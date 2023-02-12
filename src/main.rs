use std::ops::Deref;
use std::sync::{Arc, LockResult, Mutex};
use std::time::Duration;

use anyhow::Result;
use cursive::direction::Orientation;
use cursive::reexports::log;
use cursive::reexports::log::error;
use cursive::reexports::log::LevelFilter::Info;
use cursive::traits::*;
use cursive::views::{DummyView, EditView, LinearLayout, Menubar, Panel};
use cursive::{event, menu, Cursive, CursiveRunnable};
use cursive_table_view::TableView;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{ConfigMap, Namespace, Pod};
use kanal::AsyncReceiver;
use kube::api::GroupVersionKind;
use kube::Client;
use tokio::runtime::Runtime;

use crate::model::discover_gvk;
use crate::model::reflector_registry::ReflectorRegistry;
use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::model::traits::{GvkExt, GvkStaticExt};
use crate::ui::traits::MenuNameExt;
use crate::ui::{group_gvks, GVK_TO_COLUMNS_MAP};

pub mod model;
pub mod theme;
pub mod ui;
pub mod util;

fn build_main_layout(selected_gvk: GroupVersionKind) -> LinearLayout {
    let mut main_layout = LinearLayout::new(Orientation::Vertical);

    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);
    filter_layout.add_child(Panel::new(EditView::new()).title("Namespaces").full_width());
    filter_layout.add_child(Panel::new(EditView::new()).title("Name").full_width());

    let mut table: TableView<ResourceView, ResourceColumn> = TableView::new();

    for column in GVK_TO_COLUMNS_MAP.get(&selected_gvk).into_iter().flatten() {
        table = table.column(*column, column.as_ref(), |c| c);
    }

    let table_panel =
        Panel::new(table.with_name("table").full_screen()).title(selected_gvk.short_menu_name()).with_name("panel");

    main_layout.add_child(filter_layout.full_width());
    main_layout.add_child(DummyView {}.full_width());
    main_layout.add_child(table_panel);

    main_layout
}

pub fn build_menu(
    discovered_gvks: Vec<GroupVersionKind>,
    sender: kanal::Sender<FromUiSignal>,
) -> Menubar {
    let mut menubar = Menubar::new();
    menubar.add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

    let grouped_gvks = group_gvks(discovered_gvks);

    for (group_name, group) in grouped_gvks {
        let mut group_tree = menu::Tree::new();
        for resource_gvk in group {
            let leaf_name = if group_name == "Misc" {
                resource_gvk.full_menu_name()
            } else {
                resource_gvk.short_menu_name()
            };

            let ss = sender.clone();
            group_tree = group_tree.leaf(leaf_name, move |_| {
                ss.send(FromUiSignal::RequestGvkItems(resource_gvk.clone()))
                    .unwrap();
            });
        }
        menubar.add_subtree(group_name, group_tree);
    }

    menubar
}

fn render_all_items(
    siv: &mut Cursive,
    resources: Option<Vec<ResourceView>>,
    resource_gvk: GroupVersionKind,
    ui_signal_sender: kanal::Sender<FromUiSignal>,
) -> Option<()> {
    siv.call_on_name(
        "table",
        |table: &mut TableView<ResourceView, ResourceColumn>| {
            table.take_items();
            if let Some(resources) = resources {
                table.set_items(resources);
                return;
            }

            ui_signal_sender
                .send(FromUiSignal::RequestRegisterGvk(resource_gvk))
                .unwrap_or_else(|err| panic!("Failed to send GvkNotFound signal: {}", err));
        },
    )
}

#[derive(Debug)]
pub enum FromUiSignal {
    RequestRegisterGvk(GroupVersionKind),
    RequestGvkItems(GroupVersionKind),
}

#[derive(Debug)]
pub enum FromClientSignal {
    ResponseResourceUpdated(ResourceView),
    ResponseDiscoveredGvks(Vec<GroupVersionKind>),
    ResponseGvkItems(GroupVersionKind, Option<Vec<ResourceView>>),
}

pub struct App {
    runtime: Runtime,
    client: Client,
    resource_watcher_receiver: AsyncReceiver<ResourceView>,
    registry: Arc<futures::lock::Mutex<ReflectorRegistry>>,

    from_client_sender: kanal::Sender<FromClientSignal>,
    from_ui_receiver: kanal::Receiver<FromUiSignal>,
}

impl App {
    pub fn new(
        from_client_sender: kanal::Sender<FromClientSignal>,
        from_ui_receiver: kanal::Receiver<FromUiSignal>,
    ) -> Result<Self> {
        let runtime = Self::spawn_runtime(2)?;

        let client = runtime.block_on(async { Client::try_default().await })?;

        let (resource_watcher_sender, resource_watcher_receiver) = kanal::unbounded_async();

        let mut registry = ReflectorRegistry::new(resource_watcher_sender, &client);
        let reg = &mut registry;

        runtime.block_on(async {
            reg.register::<Pod>().await;
            reg.register::<Namespace>().await;
        });

        let instance = Self {
            runtime,
            client,
            resource_watcher_receiver,
            registry: Arc::new(futures::lock::Mutex::new(registry)),
            from_client_sender,
            from_ui_receiver,
        };

        Ok(instance)
    }

    fn spawn_runtime(worker_thread: usize) -> std::io::Result<Runtime> {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_thread)
            .enable_all()
            .build()
    }

    pub fn spawn_discovery_task(&self) {
        let sender = self.from_client_sender.clone_async();
        let client = self.client.clone();
        self.runtime.spawn(async move {
            loop {
                match discover_gvk(client.clone()).await {
                    Ok(gvks) => {
                        if let Err(err) = sender
                            .send(FromClientSignal::ResponseDiscoveredGvks(gvks))
                            .await
                        {
                            error!("Failed to send to the channel: {}", err)
                        }
                    }
                    Err(err) => {
                        error!("Failed to discover GVKs: {}", err)
                    }
                }
                tokio::time::sleep(Duration::from_secs(100)).await;
            }
        });
    }

    fn spawn_watcher_exchange_task(&self) {
        let resource_watch_receiver = self.resource_watcher_receiver.clone();
        let ui_signal_sender = self.from_client_sender.clone_async();

        self.runtime.spawn(async move {
            let mut stream = resource_watch_receiver.stream();

            while let Some(resource_view) = stream.next().await {
                ui_signal_sender
                    .send(FromClientSignal::ResponseResourceUpdated(resource_view))
                    .await
                    .unwrap();
            }
            panic!("Main exchange loop has ended")
        });
    }

    fn spawn_from_ui_receiver_task(&mut self) {
        let receiver = self.from_ui_receiver.clone_async();
        let sender = self.from_client_sender.clone_async();
        let registry = Arc::clone(&self.registry);

        self.runtime.spawn(async move {
            let mut stream = receiver.stream();

            while let Some(signal) = stream.next().await {
                let mut reg = registry.lock().await;
                match signal {
                    FromUiSignal::RequestRegisterGvk(gvk) => {
                        if let Err(err) = reg.register_gvk(gvk).await {
                            error!("Could not register new GVK: {}", err);
                        }
                    }
                    FromUiSignal::RequestGvkItems(gvk) => {
                        let resources = reg.get_resources(&gvk);
                        let signal = FromClientSignal::ResponseGvkItems(gvk, resources);
                        if let Err(err) = sender.send(signal).await {
                            error!("Could not handle RequestItemsForGvk: {}", err);
                        }
                    }
                }
            }
        });
    }
}

fn main() -> Result<()> {
    cursive::logger::init();
    log::set_max_level(Info);

    let (from_client_sender, from_client_receiver) = kanal::unbounded();
    let (from_ui_sender, from_ui_receiver) = kanal::unbounded();

    let mut app = App::new(from_client_sender, from_ui_receiver.clone())?;

    app.spawn_watcher_exchange_task();
    app.spawn_discovery_task();
    app.spawn_from_ui_receiver_task();

    let selected_gvk = Arc::new(Mutex::new(Pod::gvk_for_type()));

    let mut ui = CursiveRunnable::default();
    ui.add_global_callback('~', |s| s.toggle_debug_console());
    ui.add_global_callback(event::Key::Esc, |s| s.select_menubar());

    let from_ui = from_ui_sender.clone();
    let sink = ui.cb_sink().clone();

    std::thread::spawn(move || {
        for signal in from_client_receiver {
            let sender = from_ui.clone();

            match signal {
                FromClientSignal::ResponseResourceUpdated(resource) => {
                    let gvk = resource.gvk();
                    match selected_gvk.lock() {
                        Ok(guard) if guard.deref() == &gvk => {
                            sender.send(FromUiSignal::RequestGvkItems(gvk)).unwrap();
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
                FromClientSignal::ResponseDiscoveredGvks(gvks) => {
                    sink.send(Box::new(move |siv| {
                        let mut menubar = build_menu(gvks, sender);
                        menubar.autohide = false;
                        *siv.menubar() = menubar;
                    }))
                    .unwrap();
                }
                FromClientSignal::ResponseGvkItems(next_gvk, resources) => {
                    let gvk = match selected_gvk.lock() {
                        // updating the current view
                        Ok(mut guard) if guard.deref() != &next_gvk => {
                            *guard = next_gvk.clone();

                            let sink_next_gvk = next_gvk.clone();
                            sink.send(Box::new(move |siv| {
                                siv.pop_layer();
                                let main = build_main_layout(sink_next_gvk);
                                siv.add_fullscreen_layer(main);
                            }))
                            .unwrap();

                            next_gvk
                        }
                        Ok(_) => next_gvk,
                        Err(err) => {
                            error!("Failed to acquire a lock: {}", err);
                            continue;
                        }
                    };

                    sink.send(Box::new(move |siv| {
                        render_all_items(siv, resources, gvk, sender);
                    }))
                    .unwrap()
                }
            }
        }
    });

    ui.run();

    Ok(())
}

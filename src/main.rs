use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use cursive::direction::Orientation;
use cursive::reexports::log;
use cursive::reexports::log::LevelFilter::Info;
use cursive::reexports::log::{error, info, warn};
use cursive::traits::*;
use cursive::views::{DummyView, EditView, LinearLayout, Menubar, Panel};
use cursive::{event, menu, Cursive, CursiveRunnable};
use cursive_table_view::TableView;
use futures::StreamExt;
use kanal::AsyncReceiver;
use kube::api::GroupVersionKind;
use kube::Client;
use tokio::runtime::Runtime;

use crate::model::discover_gvk;
use crate::model::reflector_registry::ReflectorRegistry;
use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::{reqister_any_gvk, ResourceView};
use crate::model::traits::GvkExt;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::fs_cache::FsCache;
use crate::ui::group_gvks;
use crate::ui::traits::MenuNameExt;

pub mod model;
pub mod theme;
pub mod ui;
pub mod util;

fn build_main_layout(selected_gvk: GroupVersionKind, columns: Vec<ResourceColumn>) -> LinearLayout {
    let mut main_layout = LinearLayout::new(Orientation::Vertical);

    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);
    filter_layout.add_child(Panel::new(EditView::new()).title("Namespaces").full_width());
    filter_layout.add_child(Panel::new(EditView::new()).title("Name").full_width());

    let mut table: TableView<ResourceView, ResourceColumn> = TableView::new();

    for column in columns {
        table = table.column(column, column.as_ref(), |mut c| {
            match column {
                ResourceColumn::Namespace => c = c.width(20),
                ResourceColumn::Name => c = c.width_percent(35),
                ResourceColumn::Restarts => c = c.width(7),
                ResourceColumn::Ready => c = c.width(7),
                ResourceColumn::Age => c = c.width(7),
                ResourceColumn::Status => c = c.width(7),
                _ => {}
            }
            c
        });
    }

    let table_panel = Panel::new(table.with_name("table").full_screen())
        .title(selected_gvk.short_menu_name())
        .with_name("panel");

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

fn render_all_items(siv: &mut Cursive, resources: Option<Vec<ResourceView>>) -> Option<()> {
    siv.call_on_name(
        "table",
        |table: &mut TableView<ResourceView, ResourceColumn>| {
            if let Some(resources) = resources {
                table.take_items();
                info!("Rendering full view for {} resources", resources.len());
                table.set_items(resources);
            }
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
    fs_cache: Arc<futures::lock::Mutex<FsCache>>,
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

        let config = runtime.block_on(async { return Self::get_config().await })?;
        info!("Loaded configuration");

        let fs_cache = FsCache::try_from(config.clone())?;
        info!("Created FS Cache");

        let client = runtime.block_on(async move { Client::try_from(config) })?;
        info!("Initialized client");

        let (resource_watcher_sender, resource_watcher_receiver) = kanal::unbounded_async();

        let registry = ReflectorRegistry::new(resource_watcher_sender, &client);

        let instance = Self {
            fs_cache: Arc::new(futures::lock::Mutex::new(fs_cache)),
            runtime,
            client,
            resource_watcher_receiver,
            registry: Arc::new(futures::lock::Mutex::new(registry)),
            from_client_sender,
            from_ui_receiver,
        };

        Ok(instance)
    }

    async fn get_config() -> Result<kube::Config, kube::Error> {
        kube::Config::infer()
            .await
            .map_err(kube::Error::InferConfig)
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
        let fs_cache = Arc::clone(&self.fs_cache);
        self.runtime.spawn(async move {
            if let Some(stored_gvks) = fs_cache.lock().await.get_gvks() {
                info!("Loaded {} GVKs from cache", stored_gvks.len());
                sender
                    .send(FromClientSignal::ResponseDiscoveredGvks(stored_gvks))
                    .await
                    .unwrap();
            }

            loop {
                info!("Entered GVK discovery loop");

                match discover_gvk(client.clone()).await {
                    Ok(gvks) => {
                        info!("Received {} GVKs", gvks.len());
                        let mut cache = fs_cache.lock().await;
                        cache.set_gvks(&gvks);
                        if let Err(err) = cache.dump() {
                            error!("Failed to save cache: {}", err);
                        }
                        sender
                            .send(FromClientSignal::ResponseDiscoveredGvks(gvks))
                            .await
                            .unwrap();
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
                        reqister_any_gvk(reg.deref_mut(), gvk).await;
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

pub trait TableViewExt {
    fn merge_resource(&mut self, resource: ResourceView);
}

impl TableViewExt for TableView<ResourceView, ResourceColumn> {
    fn merge_resource(&mut self, resource: ResourceView) {
        for item in self.borrow_items_mut() {
            if item.uid() == resource.uid() {
                *item = resource;
                return;
            }
        }
        self.insert_item(resource);
    }
}

fn main() -> Result<()> {
    cursive::logger::init();
    log::set_max_level(Info);

    let (from_client_sender, from_client_receiver) = kanal::unbounded();
    let (from_ui_sender, from_ui_receiver) = kanal::unbounded();

    let arc_column_registry = Arc::new(Mutex::new(ColumnRegistry::default()));

    let mut app = App::new(from_client_sender, from_ui_receiver.clone())?;

    app.spawn_watcher_exchange_task();
    app.spawn_discovery_task();
    app.spawn_from_ui_receiver_task();

    let selected_gvk = Arc::new(Mutex::new(GroupVersionKind::gvk("", "", "")));

    let mut ui = CursiveRunnable::default();
    ui.add_global_callback('~', |s| s.toggle_debug_console());
    ui.add_global_callback(event::Key::Esc, |s| s.select_menubar());

    let from_ui = from_ui_sender.clone();
    let sink = ui.cb_sink().clone();

    std::thread::spawn(move || {
        let column_registry = Arc::clone(&arc_column_registry);
        for signal in from_client_receiver {
            let sender = from_ui.clone();

            match signal {
                FromClientSignal::ResponseResourceUpdated(resource) => {
                    let gvk = resource.gvk();
                    match selected_gvk.lock() {
                        Ok(guard) if guard.deref() == &gvk => {
                            sender.send(FromUiSignal::RequestGvkItems(gvk)).unwrap();
                            sink.send(Box::new(move |siv| {
                                siv.call_on_name(
                                    "table",
                                    |table: &mut TableView<ResourceView, ResourceColumn>| {
                                        table.merge_resource(resource);
                                    },
                                );
                            }))
                            .unwrap();
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
                    if resources.is_none() {
                        warn!("Empty resources for GVK: {:?}", next_gvk);
                        sender
                            .send(FromUiSignal::RequestRegisterGvk(next_gvk.clone()))
                            .unwrap();
                    }

                     match selected_gvk.lock() {
                        // updating the current view
                        Ok(mut guard) if guard.deref() != &next_gvk => {
                            *guard = next_gvk.clone();

                            let sink_next_gvk = next_gvk.clone();
                            let cr = Arc::clone(&column_registry);
                            sink.send(Box::new(move |siv| {
                                siv.pop_layer();
                                let columns = cr.lock().unwrap().get_columns(&sink_next_gvk);
                                let main_layout = build_main_layout(sink_next_gvk, columns);
                                siv.add_fullscreen_layer(main_layout);
                                render_all_items(siv, resources);

                            }))
                            .unwrap();
                        }
                        Ok(_) => {},
                        Err(err) => {
                            error!("Failed to acquire a lock: {}", err);
                            continue;
                        }
                    };
                }
            }
        }
    });

    ui.run();

    Ok(())
}

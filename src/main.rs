use std::ops::Deref;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use cursive::direction::Orientation;
use cursive::reexports::log;
use cursive::reexports::log::LevelFilter::Info;
use cursive::reexports::log::{error, info};
use cursive::traits::*;
use cursive::views::{DummyView, EditView, LinearLayout, Panel};
use cursive::{event, menu, Cursive, CursiveRunnable};
use cursive_table_view::TableView;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{Namespace, Pod};
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

fn render_all_items(
    siv: &mut Cursive,
    registry: &Arc<Mutex<ReflectorRegistry>>,
    resource_gvk: GroupVersionKind,
    ui_signal_sender: kanal::Sender<UiSignal>,
) -> Option<()> {
    let registry = Arc::clone(registry);
    siv.call_on_name(
        "table",
        |table: &mut TableView<ResourceView, ResourceColumn>| {
            table.take_items();
            match registry.lock() {
                Ok(guard) => {
                    if let Some(resources) = guard.get_resources(&resource_gvk) {
                        info!("Set items for type {:?}", resource_gvk);
                        table.set_items(resources);
                    } else {
                        error!("GVK {:?} not found in registry", resource_gvk);
                        ui_signal_sender
                            .send(UiSignal::GvkNotFound(resource_gvk))
                            .unwrap_or_else(|err| {
                                panic!("Failed to send GvkNotFound signal: {}", err)
                            });
                    }
                }
                Err(err) => {
                    error!("Could not acquire a lock {}", err)
                }
            }
        },
    )
}

enum UiSignal {
    RedrawTable,
    GvkNotFound(GroupVersionKind),
}

struct App {
    main_runtime: Runtime,
    exchange_runtime: Runtime,
    client: Client,
    ui: CursiveRunnable,
    selected_gvk: Arc<Mutex<GroupVersionKind>>,
    discovered_gvks: Vec<GroupVersionKind>,
    resource_watcher_receiver: AsyncReceiver<ResourceView>,
    registry: Arc<Mutex<ReflectorRegistry>>,

    ui_signal_sender: kanal::Sender<UiSignal>,
    ui_signal_receiver: kanal::Receiver<UiSignal>,
}

impl App {
    pub fn new(mut cursive_runnable: CursiveRunnable) -> Result<Self> {
        let main_runtime = Self::spawn_runtime(1)?;
        let exchange_runtime = Self::spawn_runtime(1)?;

        let client = main_runtime.block_on(async { Client::try_default().await })?;

        cursive_runnable.set_autohide_menu(false);
        cursive_runnable.add_global_callback('~', |s| s.toggle_debug_console());
        cursive_runnable.add_global_callback(event::Key::Esc, |s| s.select_menubar());

        let (resource_watcher_sender, resource_watcher_receiver) = kanal::unbounded_async();

        let registry = main_runtime.block_on(async {
            let mut reg = ReflectorRegistry::new(resource_watcher_sender, &client);
            reg.register::<Pod>().await;
            reg.register::<Namespace>().await;
            reg
        });

        let (ui_signal_sender, ui_signal_receiver) = kanal::unbounded();

        let instance = Self {
            main_runtime,
            exchange_runtime,
            client,
            ui: cursive_runnable,
            selected_gvk: Arc::new(Mutex::new(Pod::gvk_for_type())),
            discovered_gvks: vec![],
            resource_watcher_receiver,
            registry: Arc::new(Mutex::new(registry)),
            ui_signal_sender,
            ui_signal_receiver,
        };

        Ok(instance)
    }

    fn spawn_runtime(worker_thread: usize) -> std::io::Result<Runtime> {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_thread)
            .enable_all()
            .build()
    }

    pub fn discover(&mut self) -> Result<()> {
        self.discovered_gvks = self
            .main_runtime
            .block_on(async { discover_gvk(&self.client).await })?;
        Ok(())
    }

    pub fn build_menu(&mut self) {
        let menubar = self.ui.menubar();
        menubar.add_subtree("File", menu::Tree::new().leaf("Exit", |s| s.quit()));

        let grouped_gvks = group_gvks(self.discovered_gvks.clone());

        for (group_name, group) in grouped_gvks {
            let mut group_tree = menu::Tree::new();
            for resource_gvk in group {
                let leaf_name = if group_name == "Misc" {
                    resource_gvk.full_menu_name()
                } else {
                    resource_gvk.short_menu_name()
                };

                let selected_gvk_cloned = Arc::clone(&self.selected_gvk);
                let sender = self.ui_signal_sender.clone();
                group_tree = group_tree.leaf(leaf_name, move |_| {
                    if let Ok(mut guard) = selected_gvk_cloned.lock() {
                        *guard = resource_gvk.clone();
                    }
                    sender.send(UiSignal::RedrawTable).unwrap();
                });
            }
            menubar.add_subtree(group_name, group_tree);
        }
    }

    pub fn build_main_layout(&mut self) -> LinearLayout {
        let selected_gvk = match self.selected_gvk.lock() {
            Ok(gvk) => gvk.clone(),
            Err(err) => {
                error!("Failed to get GVK while building menu: {}", err);
                Pod::gvk_for_type()
            }
        };

        let mut main_layout = LinearLayout::new(Orientation::Vertical);

        let mut filter_layout = LinearLayout::new(Orientation::Horizontal);
        filter_layout.add_child(Panel::new(EditView::new()).title("Namespaces").full_width());
        filter_layout.add_child(Panel::new(EditView::new()).title("Name").full_width());

        let mut table: TableView<ResourceView, ResourceColumn> = TableView::new();

        for column in GVK_TO_COLUMNS_MAP.get(&selected_gvk).into_iter().flatten() {
            table = table.column(*column, column.as_ref(), |c| c);
        }

        let table_panel = Panel::new(table.with_name("table").full_screen()).title("Pods");

        main_layout.add_child(filter_layout.full_width());
        main_layout.add_child(DummyView {}.full_width());
        main_layout.add_child(table_panel);

        main_layout
    }

    pub fn redraw(&mut self) {
        self.ui.pop_layer();
        let layout = self.build_main_layout();
        self.ui.add_fullscreen_layer(layout);
    }

    fn spawn_exchange_task(&mut self) {
        let selected_gvk = Arc::clone(&self.selected_gvk);
        let registry = Arc::clone(&self.registry);
        let resource_watch_receiver = self.resource_watcher_receiver.clone();
        let ui_signal_sender = self.ui_signal_sender.clone();

        let sink = self.ui.cb_sink().clone();

        self.exchange_runtime.spawn(async move {
            let mut stream = resource_watch_receiver.stream();

            while let Some(resource_view) = stream.next().await {
                let resource_gvk = resource_view.gvk();
                match selected_gvk.lock() {
                    Ok(guard) if &resource_gvk != guard.deref() => continue,
                    Err(err) => {
                        error!("Failed to acquire a lock for selected_gvk: {}", err);
                        continue;
                    }
                    _ => {}
                }

                let ss = ui_signal_sender.clone();
                let registry = Arc::clone(&registry);
                sink.send(Box::new(move |siv| {
                    render_all_items(siv, &registry, resource_gvk.clone(), ss).unwrap_or_else(
                        || panic!("Failed to call a callback for {:?}", resource_gvk),
                    )
                }))
                .unwrap_or_else(|err| {
                    panic!(
                        "Failed to send an update event to table for resource {:?}; Error: {}",
                        resource_view.gvk(),
                        err
                    )
                });
            }
            panic!("Main exchange loop has ended")
        });
    }

    pub fn spawn_ui_interaction_response_thread(&mut self) {
        let selected_gvk = Arc::clone(&self.selected_gvk);
        let registry = Arc::clone(&self.registry);
        let ui_signal_receiver = self.ui_signal_receiver.clone();
        let ui_signal_sender = self.ui_signal_sender.clone();

        let sink = self.ui.cb_sink().clone();

        std::thread::spawn(move || {
            for signal in ui_signal_receiver {
                match signal {
                    UiSignal::RedrawTable => {
                        let resource_gvk = match selected_gvk.lock() {
                            Ok(gvk) => gvk.clone(),
                            Err(err) => {
                                error!("Failed to acquire a lock for selected_gvk: {}", err);
                                continue;
                            }
                        };

                        let rr = Arc::clone(&registry);
                        let ss = ui_signal_sender.clone();

                        sink.send(Box::new(move |siv| {
                            render_all_items(siv, &rr, resource_gvk.clone(), ss).unwrap_or_else(
                                || panic!("Failed to call a callback for {:?}", resource_gvk),
                            );
                        }))
                        .unwrap_or_else(|err| {
                            panic!(
                                "Failed to send an update event to table for resource: {}",
                                err
                            )
                        });
                    }
                    UiSignal::GvkNotFound(_) => {
                        let _ = Arc::clone(&registry);
                    }
                }
            }
        });
    }

    pub fn run(&mut self) {
        self.ui.run()
    }
}

fn main() -> Result<()> {
    let ui = CursiveRunnable::default();

    let mut app = App::new(ui)?;
    cursive::logger::init();
    log::set_max_level(Info);

    app.discover()?;
    app.build_menu();
    app.redraw();
    app.spawn_exchange_task();
    app.spawn_ui_interaction_response_thread();

    app.run();

    Ok(())
}

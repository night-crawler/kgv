use std::sync::{Arc, Mutex};

use anyhow::Result;
use cursive::direction::Orientation;
use cursive::reexports::crossbeam_channel::Sender;
use cursive::reexports::log;
use cursive::reexports::log::LevelFilter::Info;
use cursive::reexports::log::{info, warn};
use cursive::traits::*;
use cursive::views::{DummyView, EditView, LinearLayout, Menubar, Panel};
use cursive::{event, menu, Cursive, CursiveRunnable};
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;

use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::model::traits::GvkExt;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::group_gvks;
use crate::ui::k8s_backend::K8sBackend;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::traits::{MenuNameExt, SivExt};

pub mod model;
pub mod theme;
pub mod ui;
pub mod util;

pub struct UiStore {
    selected_gvk: GroupVersionKind,
    namespace_filter_value: String,
    name_filter_value: String,
    to_ui_sender: kanal::Sender<ToUiSignal>,
    to_backend_sender: kanal::Sender<ToBackendSignal>,
    sink: Sender<Box<dyn FnOnce(&mut Cursive) + Send>>,
    column_registry: ColumnRegistry,
    resources: Vec<ResourceView>,
}

impl UiStore {
    pub fn new(
        sink: Sender<Box<dyn FnOnce(&mut Cursive) + Send>>,
        to_ui_sender: kanal::Sender<ToUiSignal>,
        to_backend_sender: kanal::Sender<ToBackendSignal>,
        column_registry: ColumnRegistry,
    ) -> Self {
        Self {
            selected_gvk: GroupVersionKind::gvk("", "", ""),
            name_filter_value: "".to_string(),
            namespace_filter_value: "".to_string(),
            to_ui_sender,
            to_backend_sender,
            sink,
            column_registry,
            resources: vec![],
        }
    }

    fn get_columns(&self) -> Vec<ResourceColumn> {
        self.column_registry.get_columns(&self.selected_gvk)
    }

    fn should_display_resource(&self, resource: &ResourceView) -> bool {
        resource
            .namespace()
            .starts_with(&self.namespace_filter_value)
            && resource.name().contains(&self.name_filter_value)
    }

    fn get_filtered_resources(&self) -> Vec<ResourceView> {
        self.resources
            .iter()
            .filter(|resource| self.should_display_resource(resource))
            .cloned()
            .collect()
    }
}

fn build_main_layout(store: Arc<Mutex<UiStore>>) -> LinearLayout {
    let (ns_filter, name_filter, columns, sender, selected_gvk) = {
        let store = store.lock().unwrap();
        (
            store.namespace_filter_value.clone(),
            store.name_filter_value.clone(),
            store.get_columns(),
            store.to_ui_sender.clone(),
            store.selected_gvk.clone(),
        )
    };

    let mut main_layout = LinearLayout::new(Orientation::Vertical);
    let mut filter_layout = LinearLayout::new(Orientation::Horizontal);

    let namespace_edit_view = {
        let sender = sender.clone();
        EditView::new()
            .content(ns_filter)
            .on_submit(move |_, text| {
                sender
                    .send(ToUiSignal::ApplyNamespaceFilter(text.into()))
                    .unwrap();
            })
    };

    let name_edit_view = EditView::new()
        .content(name_filter)
        .on_submit(move |_, text| {
            sender
                .send(ToUiSignal::ApplyNameFilter(text.into()))
                .unwrap();
        });

    filter_layout.add_child(
        Panel::new(namespace_edit_view)
            .title("Namespaces")
            .full_width(),
    );
    filter_layout.add_child(Panel::new(name_edit_view).title("Name").full_width());

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

pub fn build_menu(discovered_gvks: Vec<GroupVersionKind>, store: Arc<Mutex<UiStore>>) -> Menubar {
    let sender = store.lock().unwrap().to_backend_sender.clone();

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

            let sender = sender.clone();
            group_tree = group_tree.leaf(leaf_name, move |_| {
                sender
                    .send(ToBackendSignal::RequestGvkItems(resource_gvk.clone()))
                    .unwrap();
            });
        }
        menubar.add_subtree(group_name, group_tree);
    }

    menubar
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

trait UiStoreExt {
    fn handle_response_gvk_items(
        &self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    );

    fn handle_response_resource_updated(&self, resource: ResourceView);
    fn handle_response_discovered_gvks(&self, gvks: Vec<GroupVersionKind>);

    fn handle_apply_namespace_filter(&self, namespace: String);
    fn handle_apply_name_filter(&self, name: String);

    fn render_table(&self);
}

impl UiStoreExt for Arc<Mutex<UiStore>> {
    fn handle_response_gvk_items(
        &self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    ) {
        let store = Arc::clone(self);

        let sink = {
            let mut store = store.lock().unwrap();

            if let Some(resources) = resources {
                store.resources = resources;
            } else {
                warn!("Empty resources for GVK: {:?}", next_gvk);
                store
                    .to_backend_sender
                    .send(ToBackendSignal::RequestRegisterGvk(next_gvk.clone()))
                    .unwrap();
                store.resources.clear();
            }

            if store.selected_gvk == next_gvk {
                return;
            }
            store.selected_gvk = next_gvk;

            store.sink.clone()
        };

        {
            let store = Arc::clone(&store);
            sink.send_box(move |siv| {
                siv.pop_layer();
                let main_layout = build_main_layout(store);
                siv.add_fullscreen_layer(main_layout);
            });
        }

        sink.call_on_name(
            "table",
            move |table: &mut TableView<ResourceView, ResourceColumn>| {
                let resources = store.lock().unwrap().get_filtered_resources();
                info!("Rendering full view for {} resources", resources.len());
                table.set_items(resources);
            },
        );
    }

    fn handle_response_resource_updated(&self, resource: ResourceView) {
        let gvk = resource.gvk();

        let (sender, sink) = {
            let store = self.lock().unwrap();
            if store.selected_gvk != gvk {
                return;
            }
            if !store.should_display_resource(&resource) {
                return;
            }
            (store.to_backend_sender.clone(), store.sink.clone())
        };

        sender.send(ToBackendSignal::RequestGvkItems(gvk)).unwrap();

        sink.call_on_name(
            "table",
            |table: &mut TableView<ResourceView, ResourceColumn>| {
                table.merge_resource(resource);
            },
        );
    }

    fn handle_response_discovered_gvks(&self, gvks: Vec<GroupVersionKind>) {
        let store = Arc::clone(self);
        let sink = store.lock().unwrap().sink.clone();
        sink.send_box(move |siv| {
            let mut menubar = build_menu(gvks, store);
            menubar.autohide = false;
            *siv.menubar() = menubar;
        });
    }

    fn handle_apply_namespace_filter(&self, namespace: String) {
        self.lock().unwrap().namespace_filter_value = namespace;
        self.render_table();
    }

    fn handle_apply_name_filter(&self, name: String) {
        self.lock().unwrap().name_filter_value = name;
        self.render_table();
    }

    fn render_table(&self) {
        let (sink, resources) = {
            let store = self.lock().unwrap();
            (store.sink.clone(), store.get_filtered_resources())
        };

        sink.call_on_name(
            "table",
            move |table: &mut TableView<ResourceView, ResourceColumn>| table.set_items(resources),
        );
    }
}

fn main() -> Result<()> {
    cursive::logger::init();
    log::set_max_level(Info);

    let (from_client_sender, from_client_receiver) = kanal::unbounded();
    let (to_backend_sender, from_ui_receiver) = kanal::unbounded();

    let ui_to_ui_sender = from_client_sender.clone();

    let mut backend = K8sBackend::new(from_client_sender, from_ui_receiver)?;

    backend.spawn_watcher_exchange_task();
    backend.spawn_discovery_task();
    backend.spawn_from_ui_receiver_task();

    let mut ui = CursiveRunnable::default();
    ui.add_global_callback('~', |s| s.toggle_debug_console());
    ui.add_global_callback(event::Key::Esc, |s| s.select_menubar());

    let store = Arc::new(Mutex::new(UiStore::new(
        ui.cb_sink().clone(),
        ui_to_ui_sender,
        to_backend_sender,
        ColumnRegistry::default(),
    )));

    std::thread::spawn(move || {
        let store = Arc::clone(&store);
        for signal in from_client_receiver {
            match signal {
                ToUiSignal::ResponseResourceUpdated(resource) => {
                    store.handle_response_resource_updated(resource);
                }
                ToUiSignal::ResponseDiscoveredGvks(gvks) => {
                    store.handle_response_discovered_gvks(gvks);
                }
                ToUiSignal::ResponseGvkItems(next_gvk, resources) => {
                    store.handle_response_gvk_items(next_gvk, resources);
                }
                ToUiSignal::ApplyNamespaceFilter(ns) => {
                    store.handle_apply_namespace_filter(ns);
                }
                ToUiSignal::ApplyNameFilter(name) => {
                    store.handle_apply_name_filter(name);
                }
            }
        }
    });

    ui.run();

    Ok(())
}

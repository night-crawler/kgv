use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cursive::reexports::crossbeam_channel::Sender;
use cursive::reexports::log::{error, info, warn};
use cursive::traits::*;
use cursive::Cursive;
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;

use crate::model::resource_column::ResourceColumn;
use crate::model::resource_view::ResourceView;
use crate::model::traits::GvkExt;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::components::{build_main_layout, build_menu};
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::traits::{SivExt, TableViewExt};

pub struct UiStore {
    pub selected_gvk: GroupVersionKind,
    pub ns_filter: String,
    pub name_filter: String,
    pub to_ui_sender: kanal::Sender<ToUiSignal>,
    pub to_backend_sender: kanal::Sender<ToBackendSignal>,
    pub sink: Sender<Box<dyn FnOnce(&mut Cursive) + Send>>,
    pub column_registry: ColumnRegistry,
    pub resources: HashMap<String, ResourceView>,
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
            name_filter: "".to_string(),
            ns_filter: "".to_string(),
            to_ui_sender,
            to_backend_sender,
            sink,
            column_registry,
            resources: HashMap::new(),
        }
    }

    pub fn get_columns(&self) -> Vec<ResourceColumn> {
        self.column_registry.get_columns(&self.selected_gvk)
    }

    pub fn should_display_resource(&self, resource: &ResourceView) -> bool {
        resource.namespace().starts_with(&self.ns_filter)
            && resource.name().contains(&self.name_filter)
    }

    pub fn get_filtered_resources(&self) -> Vec<ResourceView> {
        self.resources
            .values()
            .filter(|resource| self.should_display_resource(resource))
            .cloned()
            .collect()
    }

    pub fn add_resource(&mut self, resource: ResourceView) -> Option<ResourceView> {
        let key = resource.uid().unwrap_or_else(|| {
            error!("Received a resource without uid: {:?}", resource);
            resource.full_unique_name()
        });
        self.resources.insert(key, resource)
    }

    pub fn replace_resources(&mut self, resources: Vec<ResourceView>) {
        self.resources.clear();
        for resource in resources {
            self.add_resource(resource);
        }
    }
}

pub trait UiStoreExt {
    fn handle_response_gvk_items(
        &self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    );

    fn handle_response_resource_updated(&self, resource: ResourceView);
    fn handle_response_discovered_gvks(&self, gvks: Vec<GroupVersionKind>);

    fn handle_apply_namespace_filter(&self, namespace: String);
    fn handle_apply_name_filter(&self, name: String);

    fn handle_show_details(&self, resource: ResourceView);

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
                store.replace_resources(resources);
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
            let mut store = self.lock().unwrap();
            if store.selected_gvk != gvk {
                return;
            }
            store.add_resource(resource.clone());
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
        self.lock().unwrap().ns_filter = namespace;
        self.render_table();
    }

    fn handle_apply_name_filter(&self, name: String) {
        self.lock().unwrap().name_filter = name;
        self.render_table();
    }

    fn handle_show_details(&self, resource: ResourceView) {
        let store = self.lock().unwrap();
        let sink = store.sink.clone();
        drop(store);

        sink.send(Box::new(move |siv| {
            let html = r####"
            <table>
  <tr>
    <th>Company</th>
    <th>Contact</th>
    <th>Country</th>
  </tr>
  <tr>
    <td>Alfreds Futterkiste</td>
    <td>Maria Anders</td>
    <td>Germany</td>
  </tr>
  <tr>
    <td>Centro comercial Moctezuma</td>
    <td>Francisco Chang</td>
    <td>Mexico</td>
  </tr>
</table>
            "####;
            let view = cursive_markup::MarkupView::html(html).max_width(120);

            siv.add_fullscreen_layer(view.scrollable().full_screen());
        }))
        .unwrap();
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

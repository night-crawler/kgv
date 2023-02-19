use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::model::pod::pod_container_column::PodContainerColumn;
use crate::model::pod::pod_container_view::PodContainerView;
use crate::model::resource::resource_column::ResourceColumn;
use crate::model::resource::resource_view::ResourceView;
use cursive::reexports::crossbeam_channel::Sender;
use cursive::reexports::log::{error, info, warn};
use cursive::Cursive;
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;

use crate::model::traits::GvkExt;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::components::{build_main_layout, build_menu, build_pod_detail_layout};
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::traits::MenuNameExt;
use crate::ui::traits::{SivExt, TableViewExt};
use crate::util::ext::pod::PodExt;
use crate::util::panics::ResultExt;

pub struct UiStore {
    pub selected_gvk: GroupVersionKind,
    pub ns_filter: String,
    pub name_filter: String,
    pub to_ui_sender: kanal::Sender<ToUiSignal>,
    pub to_backend_sender: kanal::Sender<ToBackendSignal>,
    pub sink: Sender<Box<dyn FnOnce(&mut Cursive) + Send>>,
    pub column_registry: ColumnRegistry,

    pub resources: HashMap<String, ResourceView>,
    pub selected_resource: Option<ResourceView>,

    pub interactive_command: Option<InteractiveCommand>,
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
            selected_resource: None,

            interactive_command: None,
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

    pub fn get_pod_containers(&self) -> Vec<PodContainerView> {
        if let Some(ResourceView::Pod(pod)) = self.selected_resource.as_ref() {
            return pod.get_pod_containers().unwrap_or_default();
        }

        panic!(
            "Getting pod containers on a resource that is not a pod: {:?}",
            self.selected_resource
        );
    }
}

pub trait UiStoreDispatcherExt {
    fn dispatch_response_gvk_items(
        &self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    );

    fn dispatch_response_resource_updated(&self, resource: ResourceView);
    fn dispatch_response_discovered_gvks(&self, gvks: Vec<GroupVersionKind>);

    fn dispatch_apply_namespace_filter(&self, namespace: String);
    fn dispatch_apply_name_filter(&self, name: String);

    fn dispatch_show_details(&self, resource: ResourceView);
    fn dispatch_show_gvk(&self, gvk: GroupVersionKind);

    fn replace_table_items(&self);
    fn replace_pod_detail_table_items(&self);
}

impl UiStoreDispatcherExt for Arc<Mutex<UiStore>> {
    fn dispatch_response_gvk_items(
        &self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    ) {
        let sink = {
            let mut store = self.lock().unwrap_or_log();
            if let Some(resources) = resources {
                store.replace_resources(resources);
            } else {
                warn!("Empty resources for GVK: {}", next_gvk.full_menu_name());
                store
                    .to_backend_sender
                    .send(ToBackendSignal::RequestRegisterGvk(next_gvk))
                    .unwrap_or_log();
                store.resources.clear();
            }
            store.sink.clone()
        };

        let store = Arc::clone(self);
        sink.call_on_name(
            "table",
            move |table: &mut TableView<ResourceView, ResourceColumn>| {
                let resources = store.lock().unwrap_or_log().get_filtered_resources();
                info!("Rendering full view for {} resources", resources.len());
                table.set_items(resources);
            },
        );
    }

    fn dispatch_response_resource_updated(&self, resource: ResourceView) {
        let gvk = resource.gvk();

        let sink = {
            let mut store = self.lock().unwrap_or_log();
            if store.selected_gvk != gvk {
                return;
            }
            store.add_resource(resource.clone());
            if !store.should_display_resource(&resource) {
                return;
            }
            store.sink.clone()
        };

        sink.call_on_name(
            "table",
            |table: &mut TableView<ResourceView, ResourceColumn>| {
                table.add_or_update_resource(resource);
            },
        );
    }

    fn dispatch_response_discovered_gvks(&self, gvks: Vec<GroupVersionKind>) {
        let store = Arc::clone(self);
        let sink = store.lock().unwrap_or_log().sink.clone();
        sink.send_box(move |siv| {
            let mut menubar = build_menu(gvks, store);
            menubar.autohide = false;
            *siv.menubar() = menubar;
        });
    }

    fn dispatch_apply_namespace_filter(&self, namespace: String) {
        self.lock().unwrap_or_log().ns_filter = namespace;
        self.replace_table_items();
    }

    fn dispatch_apply_name_filter(&self, name: String) {
        self.lock().unwrap_or_log().name_filter = name;
        self.replace_table_items();
    }

    fn dispatch_show_details(&self, resource: ResourceView) {
        let sink = {
            let mut store = self.lock().unwrap_or_log();
            store.selected_resource = Some(resource.clone());
            store.sink.clone()
        };
        let store = Arc::clone(self);

        match resource {
            ResourceView::Pod(_) => {
                sink.send_box(move |siv| {
                    let layout = build_pod_detail_layout(store);
                    siv.add_fullscreen_layer(layout);
                });
                self.replace_pod_detail_table_items();
            }
            _ => {
                error!("Not supported")
            }
        }
    }

    fn dispatch_show_gvk(&self, gvk: GroupVersionKind) {
        let sink = {
            let mut store = self.lock().unwrap_or_log();
            store.selected_gvk = gvk;
            store.sink.clone()
        };

        let store = Arc::clone(self);
        sink.send_box(move |siv| {
            siv.pop_layer();
            let main_layout = build_main_layout(store);
            siv.add_fullscreen_layer(main_layout);
        });
    }

    fn replace_table_items(&self) {
        let (sink, resources) = {
            let store = self.lock().unwrap_or_log();
            (store.sink.clone(), store.get_filtered_resources())
        };

        sink.call_on_name(
            "table",
            move |table: &mut TableView<ResourceView, ResourceColumn>| table.set_items(resources),
        );
    }

    fn replace_pod_detail_table_items(&self) {
        let (sink, containers) = {
            let store = self.lock().unwrap_or_log();
            (store.sink.clone(), store.get_pod_containers())
        };

        sink.call_on_name(
            "containers",
            move |table: &mut TableView<PodContainerView, PodContainerColumn>| {
                table.set_items(containers)
            },
        );
    }
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cursive::reexports::crossbeam_channel::Sender;
use cursive::reexports::log::{error, info, warn};
use cursive::Cursive;
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;
use kube::ResourceExt;

use crate::model::ext::gvk::GvkNameExt;
use crate::model::ext::pod::PodExt;
use crate::model::pod::pod_container_column::PodContainerColumn;
use crate::model::pod::pod_container_view::PodContainerView;
use crate::model::resource::resource_column::ResourceColumn;
use crate::model::resource::resource_view::ResourceView;
use crate::model::traits::GvkExt;
use crate::ui::column_registry::ColumnRegistry;
use crate::ui::components::{build_code_view, build_main_layout, build_menu, build_pod_detail_layout};
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::traits::{SivExt, TableViewExt};
use crate::util::panics::{OptionExt, ResultExt};

pub type SinkSender = Sender<Box<dyn FnOnce(&mut Cursive) + Send>>;

pub struct UiStore {
    pub highlighter: crate::ui::highlighter::Highlighter,

    pub selected_gvk: GroupVersionKind,
    pub ns_filter: String,
    pub name_filter: String,
    pub to_ui_sender: kanal::Sender<ToUiSignal>,
    pub to_backend_sender: kanal::Sender<ToBackendSignal>,
    pub sink: SinkSender,
    pub column_registry: ColumnRegistry,

    pub resources: HashMap<String, ResourceView>,
    pub selected_resource: Option<ResourceView>,
    pub selected_pod_container: Option<PodContainerView>,

    pub interactive_command: Option<InteractiveCommand>,
}

impl UiStore {
    pub fn new(
        sink: SinkSender,
        to_ui_sender: kanal::Sender<ToUiSignal>,
        to_backend_sender: kanal::Sender<ToBackendSignal>,
        column_registry: ColumnRegistry,
        highlighter: crate::ui::highlighter::Highlighter,
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
            selected_pod_container: None,

            interactive_command: None,
            highlighter
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

    fn dispatch_ctrl_s(&self);
    fn dispatch_ctrl_y(&self);
    fn dispatch_shell_current(&self);

    fn replace_table_items(&self);
    fn replace_pod_detail_table_items(&self);

    fn set_selected_container_by_index(&self, index: usize);
    fn set_selected_resource_by_index(&self, index: usize);
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
                warn!("Empty resources for GVK: {}", next_gvk.full_name());
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
                info!("Rendering full view for {} local resources", resources.len());
                table.set_items(resources);
            },
        );
    }

    fn dispatch_response_resource_updated(&self, resource: ResourceView) {
        let gvk = resource.gvk();
        info!("Received an updated resource {}, {}/{}", gvk.full_name(), resource.namespace(), resource.name());

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
            let store = self.lock().unwrap_or_log();
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

    fn dispatch_ctrl_s(&self) {
        let has_pod = matches!(
            self.lock().unwrap_or_log().selected_resource.as_ref(),
            Some(ResourceView::Pod(_))
        );
        if has_pod {
            self.dispatch_shell_current();
        }
        info!("Ctrl + e pressed outside of context");
    }

    fn dispatch_ctrl_y(&self) {
        let (yaml, sink) = {
            let store = self.lock().unwrap_or_log();
            let resource = if let Some(resource) = store.selected_resource.as_ref() {
                resource.clone()
            } else {
                warn!("No resource is selected");
                return;
            };

            let yaml = resource.serialize_inner().unwrap_or_log();
            let yaml = store.highlighter.highlight(&yaml, "yaml").unwrap_or_log();

            (yaml, store.sink.clone())
        };

        sink.send_box(|siv| {
            let c = build_code_view(yaml);
            siv.add_layer(c);
        })
    }

    fn dispatch_shell_current(&self) {
        let mut store = self.lock().unwrap_or_log();

        let container_name = store
            .selected_pod_container
            .as_ref()
            .map(|container| container.container.name.clone());

        if let Some(ResourceView::Pod(pod)) = store.selected_resource.as_ref() {
            let container_name = if let Some(container_name) = container_name {
                container_name
            } else if let Some(container_name) = pod.get_expected_exec_container_name() {
                container_name
            } else if let Some(container_name) = pod.get_first_container_name() {
                container_name
            } else {
                warn!("Could not find a container for pod: {}", pod.name_any());
                return;
            };

            store.interactive_command =
                InteractiveCommand::Exec(pod.as_ref().clone(), container_name).into();
        } else {
            error!(
                "Requested an exec into a pod, but selected resource is not a pod: {:?}",
                store.selected_resource
            );
            return;
        }

        let sink = store.sink.clone();
        drop(store);

        sink.send_box(|siv| siv.quit());
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

    fn set_selected_container_by_index(&self, index: usize) {
        let sink = self.lock().unwrap_or_log().sink.clone();
        let store = Arc::clone(self);
        sink.send_box(move |siv| {
            siv.call_on_name(
                "containers",
                |table: &mut TableView<PodContainerView, PodContainerColumn>| {
                    let mut store = store.lock().unwrap_or_log();
                    store.selected_pod_container = table.borrow_item(index).cloned();
                },
            )
            .unwrap_or_log();
        });
    }

    fn set_selected_resource_by_index(&self, index: usize) {
        let sink = self.lock().unwrap_or_log().sink.clone();
        let store = Arc::clone(self);

        sink.send_box(move |siv| {
            siv.call_on_name(
                "table",
                |table: &mut TableView<ResourceView, ResourceColumn>| {
                    if let Some(resource) = table.borrow_item(index) {
                        let mut store = store.lock().unwrap_or_log();
                        store.selected_resource = resource.clone().into();
                    }
                },
            );
        });
    }
}

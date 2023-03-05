use std::sync::{Arc, Mutex};

use cursive::reexports::crossbeam_channel::Sender;
use cursive::reexports::log::{error, info, warn};
use cursive::Cursive;
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;
use kube::ResourceExt;

use crate::model::pod::pod_container_column::PodContainerColumn;
use crate::model::pod::pod_container_view::PodContainerView;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::model::traits::SerializeExt;
use crate::traits::ext::cursive::SivExt;
use crate::traits::ext::evaluated_resource::EvaluatedResourceExt;
use crate::traits::ext::gvk::GvkExt;
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::pod::PodExt;
use crate::traits::ext::table_view::TableViewExt;
use crate::ui::components::{
    build_code_view, build_detail_view, build_main_layout, build_menu, build_pod_detail_layout,
};
use crate::ui::detail_view_renderer::DetailViewRenderer;
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::resource_manager::ResourceManager;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::util::panics::ResultExt;

pub type SinkSender = Sender<Box<dyn FnOnce(&mut Cursive) + Send>>;

pub struct UiStore {
    pub highlighter: crate::ui::highlighter::Highlighter,

    pub selected_gvk: GroupVersionKind,
    pub ns_filter: String,
    pub name_filter: String,
    pub to_ui_sender: kanal::Sender<ToUiSignal>,
    pub to_backend_sender: kanal::Sender<ToBackendSignal>,
    pub sink: SinkSender,

    pub selected_resource: Option<EvaluatedResource>,
    pub selected_pod_container: Option<PodContainerView>,

    pub interactive_command: Option<InteractiveCommand>,

    pub resource_manager: ResourceManager,

    pub detail_view_renderer: DetailViewRenderer,
}

impl UiStore {
    pub fn should_display_resource(&self, evaluated_resource: &EvaluatedResource) -> bool {
        evaluated_resource
            .resource
            .namespace()
            .starts_with(&self.ns_filter)
            && evaluated_resource
                .resource
                .name()
                .contains(&self.name_filter)
    }

    pub fn get_pod_containers(&self) -> Vec<PodContainerView> {
        if let Some(EvaluatedResource {
            resource: ResourceView::Pod(pod),
            ..
        }) = self.selected_resource.as_ref()
        {
            return pod.get_pod_containers().unwrap_or_default();
        }
        error!(
            "Getting pod containers on a resource that is not a pod: {:?}",
            self.selected_resource
        );
        vec![]
    }

    fn get_filtered_resources(&self) -> Vec<EvaluatedResource> {
        self.resource_manager
            .get_resources_iter(&self.selected_gvk)
            .filter(|r| self.should_display_resource(r))
            .cloned()
            .collect()
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
                store.resource_manager.replace_all(resources);
            } else {
                warn!("Empty resources for GVK: {}", next_gvk.full_name());
                store
                    .to_backend_sender
                    .send(ToBackendSignal::RequestRegisterGvk(next_gvk))
                    .unwrap_or_log();
            }
            store.sink.clone()
        };

        let store = Arc::clone(self);
        sink.call_on_name(
            "table",
            move |table: &mut TableView<EvaluatedResource, usize>| {
                let evaluated_resources = store.lock().unwrap_or_log().get_filtered_resources();
                info!(
                    "Rendering full view for {} local resources",
                    evaluated_resources.len()
                );
                table.set_items(evaluated_resources);
            },
        );
    }

    fn dispatch_response_resource_updated(&self, resource: ResourceView) {
        let gvk = resource.gvk();
        info!(
            "Received an updated resource {}, {}/{}",
            gvk.full_name(),
            resource.namespace(),
            resource.name()
        );

        let (sink, evaluated_resource) = {
            let mut store = self.lock().unwrap_or_log();
            if store.selected_gvk != gvk {
                return;
            }
            let evaluated_resource = store.resource_manager.replace(resource);
            if !store.should_display_resource(&evaluated_resource) {
                info!(
                    "Filtered out: {}",
                    evaluated_resource.resource.full_unique_name()
                );
                return;
            }
            (store.sink.clone(), evaluated_resource)
        };

        sink.call_on_name(
            "table",
            |table: &mut TableView<EvaluatedResource, usize>| {
                table.add_or_update_resource(evaluated_resource);
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
            resource => {
                let store = self.lock().unwrap_or_log();
                let html = match store.detail_view_renderer.render_html(&resource) {
                    Ok(html) => html,
                    Err(err) => {
                        error!(
                            "Failed to render details for {}: {err}",
                            resource.gvk().full_name()
                        );
                        return;
                    }
                };
                drop(store);

                sink.send_box(|siv| {
                    let layout = build_detail_view(resource, html);
                    siv.add_fullscreen_layer(layout)
                });
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
        let is_pod = self
            .lock()
            .unwrap_or_log()
            .selected_resource
            .as_ref()
            .map(|evaluated_resource| evaluated_resource.is_pod())
            .unwrap_or(false);
        if is_pod {
            self.dispatch_shell_current();
        }
        info!("Ctrl + s pressed outside of context");
    }

    fn dispatch_ctrl_y(&self) {
        let (yaml, sink) = {
            let store = self.lock().unwrap_or_log();
            let evaluated_resource = if let Some(resource) = store.selected_resource.as_ref() {
                resource.clone()
            } else {
                warn!("No resource is selected");
                return;
            };

            let yaml = evaluated_resource.resource.to_yaml().unwrap_or_log();
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

        if let Some(EvaluatedResource {
            resource: ResourceView::Pod(pod),
            ..
        }) = store.selected_resource.as_ref()
        {
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
            move |table: &mut TableView<EvaluatedResource, usize>| table.set_items(resources),
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
        sink.call_on_name(
            "containers",
            move |table: &mut TableView<PodContainerView, PodContainerColumn>| {
                let mut store = store.lock().unwrap_or_log();
                store.selected_pod_container = table.borrow_item(index).cloned();
            },
        );
    }

    fn set_selected_resource_by_index(&self, index: usize) {
        let sink = self.lock().unwrap_or_log().sink.clone();
        let store = Arc::clone(self);

        sink.call_on_name(
            "table",
            move |table: &mut TableView<EvaluatedResource, usize>| {
                if let Some(resource) = table.borrow_item(index) {
                    let mut store = store.lock().unwrap_or_log();
                    store.selected_resource = resource.clone().into();
                } else {
                    info!(
                        "Main table does not have an item with index {index}; items count: {}",
                        table.borrow_items().len()
                    );
                }
            },
        )
    }
}

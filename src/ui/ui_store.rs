use std::ops::Deref;
use std::sync::{Arc, Mutex};

use cursive::reexports::crossbeam_channel::Sender;
use cursive::reexports::log::{error, info, warn};
use cursive::Cursive;
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;
use kube::ResourceExt;

use crate::model::pod::pod_container_view::PodContainerView;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::model::traits::SerializeExt;
use crate::traits::ext::cursive::SivExt;
use crate::traits::ext::evaluated_resource::EvaluatedResourceExt;
use crate::traits::ext::gvk::{GvkExt, PseudoGvkBuilderExt};
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::kanal_sender::KanalSenderExt;
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::pod::PodExt;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::traits::ext::table_view::TableViewExt;
use crate::ui::components::code_view::build_code_view;
use crate::ui::components::detail_view::build_detail_view;
use crate::ui::components::gvk_list_view::build_gvk_list_view_layout;
use crate::ui::components::menu::build_menu;
use crate::ui::detail_view_renderer::DetailViewRenderer;
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::resource_manager::ResourceManager;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::view_meta::{Filter, ViewMeta};
use crate::ui::view_stack::ViewStack;
use crate::util::panics::ResultExt;

pub type SinkSender = Sender<Box<dyn FnOnce(&mut Cursive) + Send>>;

pub struct UiStore {
    pub counter: usize,
    pub highlighter: crate::ui::highlighter::Highlighter,
    pub view_stack: ViewStack,

    pub selected_gvk: GroupVersionKind,
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
    pub fn inc_counter(&mut self) -> usize {
        self.counter += 1;
        self.counter
    }
    pub fn should_display_resource(
        &self,
        filter: &Filter,
        evaluated_resource: &EvaluatedResource,
    ) -> bool {
        evaluated_resource
            .resource
            .namespace()
            .starts_with(&filter.namespace)
            && evaluated_resource.resource.name().contains(&filter.name)
    }

    fn get_filtered_resources(&self, view_meta: &ViewMeta) -> Vec<EvaluatedResource> {
        let filter = view_meta.get_filter();
        let gvk = view_meta.get_gvk();
        self.resource_manager
            .get_resources_iter(gvk)
            .filter(|r| self.should_display_resource(filter, r))
            .cloned()
            .collect()
    }
}

pub trait UiStoreDispatcherExt {
    fn inc_counter(&self) -> usize;
    fn dispatch_response_gvk_items(
        &self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    );

    fn dispatch_response_resource_updated(&self, resource: ResourceView);
    fn dispatch_response_discovered_gvks(&self, gvks: Vec<GroupVersionKind>);

    fn dispatch_apply_namespace_filter(&self, id: usize, namespace: String);
    fn dispatch_apply_name_filter(&self, id: usize, name: String);

    fn dispatch_show_details(&self, resource: ResourceView);
    fn dispatch_show_gvk(&self, gvk: GroupVersionKind);

    fn dispatch_ctrl_s(&self);
    fn dispatch_ctrl_y(&self);
    fn dispatch_f5(&self);
    fn dispatch_esc(&self);
    fn dispatch_shell_current(&self);

    fn replace_table_items(&self, id: usize);

    fn update_list_views_for_gvk(&self, gvk: &GroupVersionKind);
}

impl UiStoreDispatcherExt for Arc<Mutex<UiStore>> {
    fn inc_counter(&self) -> usize {
        self.lock_unwrap().inc_counter()
    }

    fn dispatch_response_gvk_items(
        &self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    ) {
        {
            let next_gvk = next_gvk.clone();
            let mut store = self.lock_unwrap();

            if let Some(resources) = resources {
                info!("Received {} resources for GVK: {}", resources.len(), next_gvk.full_name());
                store.resource_manager.replace_all(resources);
            } else {
                warn!("Empty resources for GVK: {}", next_gvk.full_name());
                store
                    .to_backend_sender
                    .send_unwrap(ToBackendSignal::RequestRegisterGvk(next_gvk))
            }
        };

        self.update_list_views_for_gvk(&next_gvk);
    }

    fn dispatch_response_resource_updated(&self, resource: ResourceView) {
        let gvk = resource.gvk();
        info!(
            "Received an updated resource {}",
            resource.full_unique_name()
        );

        let (sink, affected_views, evaluated_resource) = {
            let mut store = self.lock_unwrap();
            (
                store.sink.clone(),
                store.view_stack.find_all(&gvk),
                store.resource_manager.replace(resource),
            )
        };

        if affected_views.is_empty() {
            warn!(
                "Resource {} will not be rendered (no active view)",
                evaluated_resource.resource.full_unique_name()
            );
        }

        for affected_view in affected_views {
            let store = self.lock_unwrap();

            let view_guard = affected_view.read_unwrap();
            let evaluated_resource = evaluated_resource.clone();
            if !store.should_display_resource(view_guard.get_filter(), &evaluated_resource) {
                info!(
                    "Not updating view {:?}: {:?}",
                    view_guard, evaluated_resource.resource
                );
                continue;
            }
            let view_name = view_guard.get_unique_name();

            sink.call_on_name(
                &view_name,
                move |table: &mut TableView<EvaluatedResource, usize>| {
                    table.add_or_update_resource(evaluated_resource);
                },
            );
        }
    }

    fn dispatch_response_discovered_gvks(&self, gvks: Vec<GroupVersionKind>) {
        let store = Arc::clone(self);
        let sink = store.lock_unwrap().sink.clone();
        sink.send_box(move |siv| {
            let mut menubar = build_menu(gvks, store);
            menubar.autohide = false;
            *siv.menubar() = menubar;
        });
    }

    fn dispatch_apply_namespace_filter(&self, id: usize, namespace: String) {
        {
            let mut store = self.lock_unwrap();
            if let Some(view_meta) = store.view_stack.get(id) {
                view_meta.write_unwrap().set_namespace(namespace)
            } else {
                warn!("Could not set namespace filter {namespace} a filter for {id}");
            }
        }
        self.replace_table_items(id);
    }

    fn dispatch_apply_name_filter(&self, id: usize, name: String) {
        {
            let mut store = self.lock_unwrap();
            if let Some(view_meta) = store.view_stack.get(id) {
                view_meta.write_unwrap().set_name(name)
            } else {
                warn!("Could not set name filter {name} a filter for {id}");
            }
        }
        self.replace_table_items(id);
    }

    fn dispatch_show_details(&self, resource: ResourceView) {
        let sink = {
            let store = self.lock_unwrap();
            store.sink.clone()
        };
        // let store = Arc::clone(self);

        match resource {
            ResourceView::Pod(pod) => {
                let gvk = pod.build_pseudo_gvk("container");
                self.dispatch_show_gvk(gvk.clone());
                self.update_list_views_for_gvk(&gvk);
            }
            resource => {
                let store = self.lock_unwrap();
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
            let mut store = self.lock_unwrap();
            store.selected_gvk = gvk;
            store.sink.clone()
        };

        let store = Arc::clone(self);
        sink.send_box(move |siv| {
            let layout = build_gvk_list_view_layout(Arc::clone(&store));
            let view_meta = layout.meta.clone();
            let view_name = view_meta.read_unwrap().get_unique_name();
            store.lock_unwrap().view_stack.push(view_meta);
            siv.add_fullscreen_layer(layout);
            info!("Registered view: {view_name}");
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
            let store = self.lock_unwrap();
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

    fn dispatch_f5(&self) {
        let last_view = self.lock_unwrap().view_stack.last();

        if let Some(view_meta) = last_view {
            let view_meta = view_meta.read_unwrap();
            match view_meta.deref() {
                ViewMeta::List { gvk, .. } => {
                    self.update_list_views_for_gvk(gvk);
                    return;
                }
                ViewMeta::Detail { .. } => {}
                ViewMeta::PodDetail { .. } => {}
                ViewMeta::Dialog { .. } => {}
            }

        }
        error!("F5 not implemented for the current view")
    }

    fn dispatch_esc(&self) {
        let sink = {
            let store = self.lock_unwrap();
            store.sink.clone()
        };
        let store = Arc::clone(self);
        sink.send_box(move |siv| {
            store.lock_unwrap().view_stack.pop();
            siv.pop_layer();
        })
    }

    fn dispatch_shell_current(&self) {
        let mut store = self.lock_unwrap();

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

    fn replace_table_items(&self, id: usize) {
        let mut store = self.lock_unwrap();

        if let Some(view_meta) = store.view_stack.get(id) {
            let view_meta = view_meta.read_unwrap();
            let resources = store.get_filtered_resources(view_meta.deref());
            store.sink.call_on_name(
                &view_meta.get_unique_name(),
                move |table: &mut TableView<EvaluatedResource, usize>| table.set_items(resources),
            );
        } else {
            warn!("Could not find a view with id={id}, {}", store.view_stack.stack.len());
        };
    }

    fn update_list_views_for_gvk(&self, gvk: &GroupVersionKind) {
        let (affected_views, sink) = {
            let store = self.lock_unwrap();
            (store.view_stack.find_all(gvk), store.sink.clone())
        };

        if affected_views.is_empty() {
            warn!("No views found for gvk={}", gvk.full_name());
        }

        for view_meta in affected_views {
            let name = view_meta.read_unwrap().get_unique_name();
            let store = Arc::clone(self);
            sink.call_on_name(
                &name,
                move |table: &mut TableView<EvaluatedResource, usize>| {
                    let store = store.lock_unwrap();
                    let view_meta = view_meta.read_unwrap();
                    let evaluated_resources = store.get_filtered_resources(view_meta.deref());

                    info!(
                        "Rendering full view for {} local resources",
                        evaluated_resources.len()
                    );
                    table.set_items(evaluated_resources);
                },
            );
        }
    }
}

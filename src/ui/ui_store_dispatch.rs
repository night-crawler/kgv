use std::ops::Deref;
use std::sync::Arc;

use cursive::reexports::log::{error, info, warn};
use cursive::traits::Nameable;
use cursive::views::Dialog;
use cursive::{Cursive, View};
use cursive_flexi_logger_view::FlexiLoggerView;
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;
use kube::ResourceExt;

use crate::config::extractor::ActionType;
use crate::eval::engine_factory::build_engine;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::model::traits::SerializeExt;
use crate::traits::ext::cursive::SivExt;
use crate::traits::ext::evaluated_resource::EvaluatedResourceExt;
use crate::traits::ext::gvk::GvkExt;
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
use crate::ui::components::window_switcher::build_window_switcher;
use crate::ui::dispatcher::DispatchContext;
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_meta::ViewMeta;
use crate::util::panics::{OptionExt, ResultExt};
use crate::util::view_with_data::ViewWithMeta;

pub trait DispatchContextExt {
    fn dispatch_update_list_views_for_gvk(self, gvk: GroupVersionKind);
    fn dispatch_response_gvk_items(
        self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    );
    fn dispatch_response_resource_updated(self, resource: ResourceView);
    fn dispatch_response_discovered_gvks(self, gvks: Vec<GroupVersionKind>);

    fn dispatch_apply_namespace_filter(self, id: usize, namespace: String);
    fn dispatch_apply_name_filter(self, id: usize, name: String);

    fn dispatch_show_details(self, resource: ResourceView);
    fn dispatch_show_gvk(self, gvk: GroupVersionKind);
    fn dispatch_show_window(self, id: usize);
    fn dispatch_remove_window_switcher(self);

    fn dispatch_alt_plus(self);
    fn dispatch_ctrl_s(self);
    fn dispatch_ctrl_y(self);
    fn dispatch_ctrl_p(self);
    fn dispatch_f5(self);
    fn dispatch_esc(self);
    fn dispatch_shell_current(self);
    fn dispatch_show_debug_log(self);
    fn dispatch_replace_table_items(&self, id: usize);

    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static;

    fn send_box<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static;
}

impl<'a> DispatchContextExt for DispatchContext<'a, UiStore, ToUiSignal> {
    fn dispatch_update_list_views_for_gvk(self, gvk: GroupVersionKind) {
        let affected_views = {
            let store = self.data.lock_unwrap();
            store.view_stack.find_list_views(&gvk)
        };

        if affected_views.is_empty() {
            warn!("No views found for gvk={}", gvk.full_name());
        }

        for view_meta in affected_views {
            let name = view_meta.read_unwrap().get_unique_name();
            let store = Arc::clone(&self.data);
            self.call_on_name(
                &name,
                move |table: &mut TableView<EvaluatedResource, usize>| {
                    let mut store = store.lock_unwrap();
                    let view_meta = view_meta.read_unwrap();
                    let evaluated_resources = store.get_filtered_resources(view_meta.deref());

                    info!(
                        "Rendering full view for {} local resources",
                        evaluated_resources.len()
                    );
                    table.set_items(evaluated_resources);
                    table.set_selected_row(0);
                    let item_index = table.item().unwrap_or_log();
                    store.selected_resource = table.borrow_item(item_index).cloned();
                },
            );
        }
    }

    fn dispatch_response_gvk_items(
        self,
        next_gvk: GroupVersionKind,
        resources: Option<Vec<ResourceView>>,
    ) {
        {
            let next_gvk = next_gvk.clone();
            let mut store = self.data.lock_unwrap();

            if let Some(resources) = resources {
                info!(
                    "Received {} resources for GVK: {}",
                    resources.len(),
                    next_gvk.full_name()
                );
                store.resource_manager.replace_all(resources);
            } else {
                warn!("Empty resources for GVK: {}", next_gvk.full_name());
                store
                    .to_backend_sender
                    .send_unwrap(ToBackendSignal::RequestRegisterGvk(next_gvk))
            }
        };

        self.dispatcher
            .dispatch_sync(ToUiSignal::UpdateListViewForGvk(next_gvk));
    }

    fn dispatch_response_resource_updated(self, resource: ResourceView) {
        let gvk = resource.gvk();
        info!(
            "Received an updated resource {}",
            resource.full_unique_name()
        );

        let (affected_views, evaluated_resource) = {
            let mut store = self.data.lock_unwrap();
            (
                store.view_stack.find_list_views(&gvk),
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
            let store = self.data.lock_unwrap();

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
            drop(store);
            drop(view_guard);

            let store = Arc::clone(&self.data);
            self.call_on_name(
                &view_name,
                move |table: &mut TableView<EvaluatedResource, usize>| {
                    if table.is_empty() {
                        table.add_or_update_resource(evaluated_resource.clone());
                        store.lock_unwrap().selected_resource = Some(evaluated_resource);
                    } else {
                        table.add_or_update_resource(evaluated_resource);
                    }
                },
            );
        }
    }

    fn dispatch_response_discovered_gvks(self, gvks: Vec<GroupVersionKind>) {
        let store = Arc::clone(&self.data);
        self.send_box(move |siv| {
            let mut menubar = build_menu(gvks, store);
            menubar.autohide = false;
            *siv.menubar() = menubar;
        });
    }

    fn dispatch_apply_namespace_filter(self, id: usize, namespace: String) {
        {
            let store = self.data.lock_unwrap();
            if let Some(view_meta) = store.view_stack.get(id) {
                view_meta.write_unwrap().set_namespace(namespace)
            } else {
                warn!("Could not set namespace filter {namespace} for {id}");
            }
        }

        self.dispatcher
            .dispatch_sync(ToUiSignal::ReplaceTableItems(id));
    }

    fn dispatch_apply_name_filter(self, id: usize, name: String) {
        {
            let store = self.data.lock_unwrap();
            if let Some(view_meta) = store.view_stack.get(id) {
                view_meta.write_unwrap().set_name(name)
            } else {
                warn!("Could not set name filter {name} for {id}");
            }
        }
        self.dispatcher
            .dispatch_sync(ToUiSignal::ReplaceTableItems(id));
    }

    fn dispatch_show_details(self, resource: ResourceView) {
        let gvk = resource.gvk();

        let action_type = self
            .data
            .lock_unwrap()
            .resource_manager
            .get_submit_handler_type(&gvk);
        let action_type = if let Some(action_type) = action_type {
            action_type
        } else {
            warn!("No event type handler for gvk {}", gvk.full_name());
            return;
        };

        match action_type {
            ActionType::ShowDetailsTable(extractor_name) => {
                let gvk = resource.to_pseudo_gvk(&extractor_name);
                self.dispatcher
                    .dispatch_sync(ToUiSignal::ShowGvk(gvk.clone()));
                self.dispatcher
                    .dispatch_sync(ToUiSignal::UpdateListViewForGvk(gvk));
            }
            ActionType::ShowDetailsTemplate => {
                let store = self.data.lock_unwrap();
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
                let store = Arc::clone(&self.data);

                self.send_box(move |siv| {
                    let view = build_detail_view(Arc::clone(&store), resource, html);
                    store.register_view(&view);
                    siv.add_fullscreen_layer(view);
                });
            }
        };
    }

    fn dispatch_show_gvk(self, gvk: GroupVersionKind) {
        self.data.lock_unwrap().selected_gvk = gvk;
        let store = Arc::clone(&self.data);

        self.send_box(move |siv| {
            let layout = build_gvk_list_view_layout(Arc::clone(&store));
            let view_meta = Arc::clone(&layout.meta);
            let view_name = view_meta.read_unwrap().get_unique_name();
            store.lock_unwrap().view_stack.push(view_meta);
            siv.add_fullscreen_layer(layout);
            siv.focus_name(&view_name).unwrap_or_log();
            info!("Registered view: {view_name}");
        });
    }

    fn dispatch_show_window(self, meta_id: usize) {
        let store = Arc::clone(&self.data);

        self.dispatcher
            .dispatch_sync(ToUiSignal::RemoveWindowSwitcher);

        self.send_box(move |siv| {
            let mut store = store.lock_unwrap();

            if let Some(meta) = store.view_stack.get(meta_id) {
                let name = meta.read_unwrap().get_unique_name();
                if let Some(pos) = siv.screen_mut().find_layer_from_name(&name) {
                    siv.screen_mut().move_to_front(pos);
                    if let Err(err) = store.view_stack.move_to_front(meta_id) {
                        error!("Could not to bring view {meta_id} to front: {err}");
                    }
                } else {
                    error!("Could not find a view with name={name} (id={meta_id})");
                }
            }
        });
    }

    fn dispatch_remove_window_switcher(self) {
        let switchers = self.data.lock_unwrap().view_stack.find_window_switchers();
        let store = Arc::clone(&self.data);
        self.send_box(move |siv| {
            for meta in switchers.iter() {
                let name = meta.read_unwrap().get_unique_name();
                if let Some(pos) = siv.screen_mut().find_layer_from_name(&name) {
                    siv.screen_mut().remove_layer(pos);
                }
            }
            store.lock_unwrap().view_stack.remove_window_switchers();
        });
    }

    fn dispatch_alt_plus(self) {
        let store = Arc::clone(&self.data);

        self.dispatcher
            .dispatch_sync(ToUiSignal::RemoveWindowSwitcher);

        self.send_box(move |siv| {
            let switcher = build_window_switcher(Arc::clone(&store));
            let name = switcher.meta.read_unwrap().get_unique_name();
            store.register_view(&switcher);
            siv.add_layer(switcher);
            siv.focus_name(&name).unwrap_or_log();
        });
    }

    fn dispatch_ctrl_s(self) {
        let is_pod = self
            .data
            .lock_unwrap()
            .selected_resource
            .as_ref()
            .map(|evaluated_resource| evaluated_resource.is_pod())
            .unwrap_or(false);
        if is_pod {
            self.dispatch_shell_current();
        }
        info!("Ctrl + s pressed outside of context");
    }

    fn dispatch_ctrl_y(self) {
        let (yaml, title) = {
            let store = self.data.lock_unwrap();

            let evaluated_resource = if let Some(resource) = store.selected_resource.as_ref() {
                resource.clone()
            } else {
                warn!("No resource is selected");
                return;
            };

            let yaml = evaluated_resource.resource.to_yaml().unwrap_or_log();
            let highlighted_yaml = store.highlighter.highlight(&yaml, "yaml").unwrap_or_log();

            (
                highlighted_yaml,
                evaluated_resource.resource.full_unique_name(),
            )
        };

        let store = Arc::clone(&self.data);
        self.send_box(move |siv| {
            let view = build_code_view(Arc::clone(&store), title, yaml);
            store.register_view(&view);
            siv.add_layer(view);
        })
    }

    fn dispatch_ctrl_p(self) {
        let resource = self.data.lock_unwrap().selected_resource.clone();
        let resource = if let Some(resource) = resource {
            resource
        } else {
            return;
        };
        let engine = build_engine(&[]);
        let json = resource.resource.to_json().unwrap_or_log();
        let json = engine.parse_json(json, true).unwrap_or_log();

        let json = format!("#{:#?}", json);

        let name = resource.resource.name();
        let gvk = resource.resource.gvk();
        let file_name = format!("{}-{}.json", gvk.kind, name);
        let path = std::env::temp_dir().join(file_name);

        std::fs::write(path, json).unwrap_or_log();
    }

    fn dispatch_f5(self) {
        let last_view = self.data.lock_unwrap().view_stack.last();

        if let Some(view_meta) = last_view {
            let view_meta = view_meta.read_unwrap();
            match view_meta.deref() {
                ViewMeta::List { gvk, .. } => {
                    self.dispatcher
                        .dispatch_sync(ToUiSignal::UpdateListViewForGvk(gvk.clone()));
                    return;
                }
                ViewMeta::Detail { .. } => {}
                ViewMeta::Dialog { .. } => {}
                ViewMeta::WindowSwitcher { .. } => {}
            }
        }
        error!("F5 not implemented for the current view")
    }

    fn dispatch_esc(self) {
        let store = Arc::clone(&self.data);
        self.send_box(move |siv| {
            store.lock_unwrap().view_stack.pop();
            siv.pop_layer();
        })
    }

    fn dispatch_shell_current(self) {
        let mut store = self.data.lock_unwrap();

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

        drop(store);

        self.send_box(|siv| siv.quit());
    }

    fn dispatch_show_debug_log(self) {
        let store = Arc::clone(&self.data);
        self.send_box(move |siv| {
            let view_meta = ViewMeta::Dialog {
                id: store.inc_counter(),
                name: "Debug Log".to_string(),
            };
            let logger = FlexiLoggerView::scrollable().with_name(view_meta.get_unique_name());
            let logger = ViewWithMeta::new(logger, view_meta);
            store.lock_unwrap().view_stack.push(logger.meta.clone());
            siv.add_fullscreen_layer(Dialog::around(logger).title("Debug Log"));
        });
    }

    fn dispatch_replace_table_items(&self, id: usize) {
        let extracted = {
            let store = self.data.lock_unwrap();
            if let Some(view_meta) = store.view_stack.get(id) {
                let view_meta = view_meta.read_unwrap();
                let name = view_meta.get_unique_name();
                let resources = store.get_filtered_resources(view_meta.deref());
                Some((name, resources))
            } else {
                warn!(
                    "Could not find a view with id={id}, {}",
                    store.view_stack.stack.len()
                );
                None
            }
        };

        if let Some((view_name, resources)) = extracted {
            self.call_on_name(
                &view_name,
                move |table: &mut TableView<EvaluatedResource, usize>| table.set_items(resources),
            );
        }
    }

    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        self.num_callbacks
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        sink.call_on_name_sync(self.sender.clone(), name, callback);
    }

    fn send_box<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        self.num_callbacks
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        sink.send_box_sync(self.sender.clone(), callback);
    }
}

use std::ops::Deref;
use std::sync::Arc;

use anyhow::bail;
use cursive::reexports::log::{info, warn};
use cursive::traits::Nameable;
use cursive::views::Dialog;
use cursive_flexi_logger_view::FlexiLoggerView;
use cursive_table_view::TableView;
use k8s_openapi::api::core::v1::{Container, Pod};
use kube::api::GroupVersionKind;
use kube::ResourceExt;

use crate::config::extractor::ActionType;
use crate::eval::engine_factory::build_engine;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::model::traits::SerializeExt;
use crate::reexports::sync::RwLock;
use crate::traits::ext::cursive::SivUtilExt;
use crate::traits::ext::gvk::GvkExt;
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::pod::PodExt;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::ui::components::code_view::build_code_view;
use crate::ui::components::detail_view::build_detail_view;
use crate::ui::components::gvk_list_view::build_gvk_list_view_layout;
use crate::ui::components::gvk_switcher::build_gvk_switcher;
use crate::ui::components::log_view::build_log_view;
use crate::ui::components::port_forwarding_dialog_view::build_port_forwarding_dialog_view;
use crate::ui::components::port_forwarding_view::build_port_forwarding_view;
use crate::ui::components::window_switcher::build_window_switcher;
use crate::ui::dispatch::send_helper_ext::DispatchContextSendHelperExt;
use crate::ui::dispatcher::DispatchContext;
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::signals::{InterUiSignal, ToBackendSignal};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_meta::{ViewMeta, ViewMetaLogExt};
use crate::util::error::{LogError, LogErrorOptionExt, LogErrorResultExt};
use crate::util::panics::ResultExt;
use crate::util::view_with_data::ViewWithMeta;

pub(crate) trait DispatchContextUiExt {
    fn dispatch_update_list_views_for_gvk(
        self,
        gvk: GroupVersionKind,
        reevaluate: bool,
    ) -> anyhow::Result<()>;

    fn dispatch_logs_apply_highlight(self, view_id: usize, text: String) -> anyhow::Result<()>;
    fn dispatch_logs_apply_since_minutes(
        self,
        view_id: usize,
        num_minutes: usize,
    ) -> anyhow::Result<()>;
    fn dispatch_logs_apply_tail_lines(self, view_id: usize, num_lines: usize)
        -> anyhow::Result<()>;
    fn dispatch_logs_apply_timestamps(
        self,
        view_id: usize,
        show_timestamps: bool,
    ) -> anyhow::Result<()>;
    fn dispatch_show_port_forwarding_view(self) -> anyhow::Result<()>;
    fn dispatch_show_port_forwarding_dialog(self) -> anyhow::Result<()>;
    fn dispatch_logs_apply_previous(
        self,
        view_id: usize,
        show_previous: bool,
    ) -> anyhow::Result<()>;

    fn dispatch_apply_namespace_filter(self, id: usize, namespace: String) -> anyhow::Result<()>;
    fn dispatch_apply_name_filter(self, id: usize, name: String) -> anyhow::Result<()>;

    fn dispatch_show_details(self, resource: ResourceView) -> anyhow::Result<()>;
    fn dispatch_show_gvk(self, gvk: GroupVersionKind) -> anyhow::Result<()>;
    fn dispatch_bring_to_front(self, id: usize) -> anyhow::Result<()>;
    fn dispatch_ctrl_l(self) -> anyhow::Result<()>;
    fn dispatch_ctrl_k(self) -> anyhow::Result<()>;
    fn dispatch_ctrl_slash(self) -> anyhow::Result<()>;

    fn dispatch_show_window_switcher(self) -> anyhow::Result<()>;
    fn dispatch_ctrl_s(self) -> anyhow::Result<()>;
    fn dispatch_show_yaml(self) -> anyhow::Result<()>;
    fn dispatch_dump_resource_sample(self) -> anyhow::Result<()>;
    fn dispatch_refresh(self) -> anyhow::Result<()>;
    fn dispatch_pop_view(self) -> anyhow::Result<()>;
    fn get_active_container(&self) -> anyhow::Result<(Arc<Pod>, Container)>;
    fn dispatch_logs(self) -> anyhow::Result<()>;
    fn dispatch_shell_current(self) -> anyhow::Result<()>;
    fn dispatch_show_debug_console(self) -> anyhow::Result<()>;
    fn dispatch_replace_table_items(&self, id: usize) -> anyhow::Result<()>;

    fn get_selected_resource(&self) -> Result<EvaluatedResource, anyhow::Error>;
    fn get_view_by_id(&self, id: usize) -> anyhow::Result<Arc<RwLock<ViewMeta>>>;
    fn send_log_subscribe(&self, view: Arc<RwLock<ViewMeta>>) -> anyhow::Result<()>;
}

impl<'a> DispatchContextUiExt for DispatchContext<'a, UiStore, InterUiSignal> {
    fn dispatch_update_list_views_for_gvk(
        self,
        gvk: GroupVersionKind,
        reevaluate: bool,
    ) -> anyhow::Result<()> {
        let (mut affected_views, resource_manager) = self.data.locking(|store| {
            let affected_views = store
                .view_stack
                .find_all_by_gvk(&gvk)
                .into_iter()
                .filter(|meta| matches!(meta.read_unwrap().deref(), ViewMeta::List { .. }))
                .peekable();

            Ok((affected_views, Arc::clone(&store.resource_manager)))
        })?;

        if reevaluate {
            resource_manager.write_sync()?.reevaluate_all_for_gvk(&gvk);
        }

        if affected_views.peek().is_none() {
            warn!("No views found for gvk={}", gvk.full_name());
            return Ok(());
        }

        for view_meta in affected_views {
            let name = view_meta.read_unwrap().get_unique_name();
            let store = Arc::clone(&self.data);
            self.call_on_name(
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
                    table.set_selected_row(0);
                },
            );
        }

        Ok(())
    }

    fn dispatch_logs_apply_highlight(self, view_id: usize, text: String) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;
        view.write_sync()?.set_log_search_text(text);
        Ok(())
    }

    fn dispatch_logs_apply_since_minutes(
        self,
        view_id: usize,
        num_minutes: usize,
    ) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;
        view.write_sync()?.set_log_since_seconds(num_minutes);
        self.send_log_subscribe(view)
    }

    fn dispatch_logs_apply_tail_lines(
        self,
        view_id: usize,
        num_lines: usize,
    ) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;
        view.write_sync()?.set_log_tail_lines(num_lines);
        self.send_log_subscribe(view)
    }

    fn dispatch_logs_apply_timestamps(
        self,
        view_id: usize,
        show_timestamps: bool,
    ) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;
        view.write_sync()?.set_log_show_timestamps(show_timestamps);
        Ok(())
    }

    fn dispatch_show_port_forwarding_view(self) -> anyhow::Result<()> {
        let store = Arc::clone(&self.data);
        self.send_wait(move |siv| {
            let view = build_port_forwarding_view(Arc::clone(&store))?;
            store.register_view(&view);
            siv.add_layer(view);
            Ok::<_, anyhow::Error>(())
        })
    }

    fn dispatch_show_port_forwarding_dialog(self) -> anyhow::Result<()> {
        let (pod, container) = self.get_active_container()?;
        let store = Arc::clone(&self.data);
        self.send_wait(move |siv| {
            let view = build_port_forwarding_dialog_view(&pod, &container,Arc::clone(&store))?;
            store.register_view(&view);
            siv.add_layer(view);
            Ok::<_, anyhow::Error>(())
        })?;

        Ok(())
    }

    fn dispatch_logs_apply_previous(
        self,
        view_id: usize,
        show_previous: bool,
    ) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;
        view.write_sync()?.set_log_show_previous(show_previous);
        self.send_log_subscribe(view)
    }

    fn dispatch_apply_namespace_filter(self, id: usize, namespace: String) -> anyhow::Result<()> {
        self.data.locking(|store| {
            store
                .view_stack
                .get(id)
                .to_log_warn(|| format!("Could not set namespace filter {namespace} for {id}"))?
                .write_unwrap()
                .set_namespace(namespace);
            Ok(())
        })?;

        self.dispatcher
            .dispatch_sync(InterUiSignal::ReplaceTableItems(id));

        Ok(())
    }

    fn dispatch_apply_name_filter(self, id: usize, name: String) -> anyhow::Result<()> {
        {
            let store = self.data.lock_sync()?;
            store
                .view_stack
                .get(id)
                .to_log_warn(|| format!("Could not set name filter {name} for {id}"))?
                .write_sync()?
                .set_name(name);
        }
        self.dispatcher
            .dispatch_sync(InterUiSignal::ReplaceTableItems(id));

        Ok(())
    }

    fn dispatch_show_details(self, resource: ResourceView) -> anyhow::Result<()> {
        let gvk = resource.gvk();

        let action_type = self
            .data
            .lock_sync()?
            .resource_manager
            .read_sync()?
            .get_submit_handler_type(&gvk)
            .to_log_warn(|| format!("No event type handler for gvk {}", gvk.full_name()))?;

        match action_type {
            ActionType::ShowDetailsTable(extractor_name) => {
                let gvk = resource.build_pseudo_gvk(&extractor_name);
                self.dispatcher
                    .dispatch_sync(InterUiSignal::ShowGvk(gvk.clone()));
                self.dispatcher
                    .dispatch_sync(InterUiSignal::UpdateListViewForGvk(gvk, false));
            }
            ActionType::ShowDetailsTemplate => {
                let store = self.data.lock_sync()?;
                let html = store.detail_view_renderer.render_html(&resource)?;

                drop(store);
                let store = Arc::clone(&self.data);

                self.send(move |siv| {
                    let view = build_detail_view(Arc::clone(&store), resource, html);
                    store.register_view(&view);
                    siv.add_fullscreen_layer(view);
                });
            }
        };

        Ok(())
    }

    fn dispatch_show_gvk(self, gvk: GroupVersionKind) -> anyhow::Result<()> {
        self.data.lock_sync()?.selected_gvk = gvk;
        let store = Arc::clone(&self.data);

        self.send(move |siv| {
            let layout = build_gvk_list_view_layout(Arc::clone(&store));
            store.register_view(&layout);
            let view_name = layout.meta.read_unwrap().get_unique_name();
            siv.add_fullscreen_layer(layout);
            siv.focus_name(&view_name).unwrap_or_log();
        });

        Ok(())
    }

    fn dispatch_bring_to_front(self, view_meta_id: usize) -> anyhow::Result<()> {
        let store = Arc::clone(&self.data);

        self.send_wait(move |siv| {
            let mut store = store.lock_sync()?;

            let meta = store.view_stack.get(view_meta_id).to_log_warn(|| {
                format!("Could not bring view {view_meta_id} to front: view does not exist")
            })?;

            siv.remove_views(store.view_stack.find_window_switchers());
            store.view_stack.remove_window_switchers();

            let name = meta.read_sync()?.get_unique_name();

            let pos = siv
                .screen_mut()
                .find_layer_from_name(&name)
                .to_log_error(|| {
                    format!("Could not find a view with name={name} (id={view_meta_id})")
                })?;

            siv.screen_mut().move_to_front(pos);
            store
                .view_stack
                .move_to_front(view_meta_id)
                .to_log_error(|err| {
                    format!("Could not to bring view {view_meta_id} to front: {err}")
                })?;

            Ok(())
        })
    }

    fn dispatch_ctrl_l(self) -> anyhow::Result<()> {
        self.dispatch_logs()
    }

    fn dispatch_ctrl_k(self) -> anyhow::Result<()> {
        let selected_resource = self.get_selected_resource()?.resource;
        if let ResourceView::PseudoResource(resource) = &selected_resource {
            bail!(
                "Cannot delete pseudo resource {}",
                resource.gvk().full_name()
            );
        }

        self.data.locking(|store| {
            store
                .to_backend_sender
                .send(ToBackendSignal::Remove(selected_resource))?;
            Ok(())
        })
    }

    fn dispatch_ctrl_slash(self) -> anyhow::Result<()> {
        let store = Arc::clone(&self.data);

        self.send(move |siv| {
            {
                let mut store = store.lock_unwrap();
                siv.remove_views(store.view_stack.find_gvk_switchers());
                store.view_stack.remove_gvk_switchers();
            }
            let switcher = build_gvk_switcher(Arc::clone(&store));
            let name = switcher.meta.read_unwrap().get_edit_name("gvks");
            store.register_view(&switcher);
            siv.add_layer(switcher);
            siv.focus_name(&name).unwrap_or_log();
        });

        Ok(())
    }

    fn dispatch_show_window_switcher(self) -> anyhow::Result<()> {
        let store = Arc::clone(&self.data);

        self.send(move |siv| {
            {
                let mut store = store.lock_unwrap();
                siv.remove_views(store.view_stack.find_window_switchers());
                store.view_stack.remove_window_switchers();
            }
            let switcher = build_window_switcher(Arc::clone(&store));
            let name = switcher.meta.read_unwrap().get_unique_name();
            store.register_view(&switcher);
            siv.add_layer(switcher);
            siv.focus_name(&name).unwrap_or_log();
        });

        Ok(())
    }

    fn dispatch_ctrl_s(self) -> anyhow::Result<()> {
        self.dispatch_shell_current()
    }

    fn dispatch_show_yaml(self) -> anyhow::Result<()> {
        let resource = self.get_selected_resource()?.resource;

        let store = Arc::clone(&self.data);
        self.send_wait(move |siv| {
            let view = build_code_view(Arc::clone(&store), resource)?;
            store.register_view(&view);
            siv.add_layer(view);
            Ok(())
        })
    }

    fn dispatch_dump_resource_sample(self) -> anyhow::Result<()> {
        let resource = self.get_selected_resource()?;

        let engine = build_engine(&[]);
        let json = resource.resource.to_json()?;
        let json = engine.parse_json(json, true)?;

        let json = format!("#{:#?}", json);

        let name = resource.resource.name();
        let gvk = resource.resource.gvk();
        let file_name = format!("{}-{}.json", gvk.kind, name);
        let path = std::env::temp_dir().join(file_name);

        info!(
            "Saving resource sample file for {} to {}",
            resource.resource.full_unique_name(),
            path.display()
        );

        std::fs::write(path, json)?;

        Ok(())
    }

    fn dispatch_refresh(self) -> anyhow::Result<()> {
        let last_view = self
            .data
            .lock_sync()?
            .view_stack
            .last()
            .to_log_warn(|| "No view is selected")?;

        let view_meta = last_view.read_unwrap();
        let gvk = match view_meta.deref() {
            ViewMeta::List { gvk, .. } => gvk.clone(),
            _ => {
                return LogError::log_error("F5 not implemented for the current view");
            }
        };
        drop(view_meta);

        self.dispatcher
            .dispatch_sync(InterUiSignal::UpdateListViewForGvk(gvk, true));
        Ok(())
    }

    fn dispatch_pop_view(self) -> anyhow::Result<()> {
        let store = Arc::clone(&self.data);
        self.send(move |siv| {
            store.lock_unwrap().view_stack.pop();
            siv.pop_layer();
        });

        Ok(())
    }

    fn get_active_container(&self) -> anyhow::Result<(Arc<Pod>, Container)> {
        let resource = self.get_selected_resource()?.resource;
        let (pod, container) = match resource {
            ResourceView::Pod(pod) => {
                let container_name = if let Some(container) = pod.get_expected_exec_container() {
                    container
                } else if let Some(container) = pod.get_first_container() {
                    container
                } else {
                    return LogError::log_warn(format!(
                        "Could not find a suitable container in pod: {}",
                        pod.name_any()
                    ));
                };
                (pod, container_name)
            }
            ResourceView::PseudoResource(pseudo_resource) => {
                let serialized = pseudo_resource.to_yaml()?;
                let container: Container = serde_yaml::from_str(&serialized)?;

                if let ResourceView::Pod(pod) = pseudo_resource.source.clone() {
                    (pod, container)
                } else {
                    let container_name = container.name;
                    return LogError::log_warn(format!(
                        "Cannot exec into a container {container_name} because the associated resources is not a pod: {}",
                        pseudo_resource.name()
                    ));
                }
            }
            resource => {
                return LogError::log_warn(format!(
                    "Cannot exec into a resource: {}",
                    resource.full_unique_name()
                ));
            }
        };

        Ok((pod, container))
    }

    fn dispatch_logs(self) -> anyhow::Result<()> {
        let (pod, container) = self.get_active_container()?;

        let store = Arc::clone(&self.data);
        let log_request = self.send_wait(move |siv| {
            let view = build_log_view(&pod, &container, Arc::clone(&store))?;
            let log_request = view.meta.read_sync()?.get_log_request().clone();
            store.register_view(&view);
            siv.add_fullscreen_layer(view);
            Ok::<_, anyhow::Error>(log_request)
        })?;

        self.data.locking(|store| {
            store
                .to_backend_sender
                .send(ToBackendSignal::LogsSubscribe(log_request))?;
            Ok(())
        })
    }

    fn dispatch_shell_current(self) -> anyhow::Result<()> {
        let (pod, container) = self.get_active_container()?;

        let mut store = self.data.lock_sync()?;
        store.interactive_command =
            InteractiveCommand::Exec(pod.as_ref().clone(), container.name).into();
        drop(store);

        self.send(|siv| siv.quit());

        Ok(())
    }

    fn dispatch_show_debug_console(self) -> anyhow::Result<()> {
        let store = Arc::clone(&self.data);
        self.send(move |siv| {
            let view_meta = ViewMeta::Dialog {
                id: store.inc_counter(),
                name: "Debug Console".to_string(),
            };
            let logger = FlexiLoggerView::scrollable().with_name(view_meta.get_unique_name());
            let logger = ViewWithMeta::new(logger, view_meta);
            store.lock_unwrap().view_stack.push(logger.meta.clone());
            siv.add_fullscreen_layer(Dialog::around(logger).title("Debug Console"));
        });

        Ok(())
    }

    fn dispatch_replace_table_items(&self, id: usize) -> anyhow::Result<()> {
        let (view_name, resources) = {
            let store = self.data.lock_sync()?;
            let view_meta = store.view_stack.get(id).to_log_warn(|| {
                format!(
                    "Could not find a view with id={id}, {}",
                    store.view_stack.stack.len()
                )
            })?;

            let view_meta = view_meta.read_unwrap();
            let name = view_meta.get_unique_name();
            let resources = store.get_filtered_resources(view_meta.deref());
            (name, resources)
        };

        self.call_on_name(
            &view_name,
            move |table: &mut TableView<EvaluatedResource, usize>| table.set_items(resources),
        );

        Ok(())
    }

    fn get_selected_resource(&self) -> Result<EvaluatedResource, anyhow::Error> {
        let store = Arc::clone(&self.data);
        self.send_wait(move |siv| {
            let top_view = store
                .lock_sync()?
                .view_stack
                .last()
                .to_log_error(|| "No view is selected".to_string())?;

            let top_view = top_view.read_sync()?;
            let view_name = top_view.get_unique_name();
            let is_list = top_view.is_list();
            let uid = top_view.get_uid();
            drop(top_view);

            let result = if is_list {
                siv.call_on_name(
                    &view_name,
                    |table: &mut TableView<EvaluatedResource, usize>| {
                        if table.is_empty() {
                            return None;
                        }
                        Some(table.get_focused_item().clone())
                    },
                )
                .to_log_warn(|| format!("Could not find a view with name {view_name}"))?
                .to_log_warn(|| format!("Could not find a focused item in view {view_name}"))?
            } else if let Some(uid) = uid {
                store
                    .lock_sync()?
                    .resource_manager
                    .read_sync()?
                    .get_resource_by_uid(&uid)
                    .to_log_warn(|| {
                        format!("Could not find a resource with uid {uid} in {view_name}")
                    })?
            } else {
                bail!("The last view {view_name} has not resource information")
            };

            Ok(result)
        })
    }

    fn get_view_by_id(&self, id: usize) -> anyhow::Result<Arc<RwLock<ViewMeta>>> {
        let view = self
            .data
            .lock_sync()?
            .view_stack
            .get(id)
            .to_log_error(|| format!("Could not find a view with id {id}"))?;

        Ok(view)
    }

    fn send_log_subscribe(&self, view: Arc<RwLock<ViewMeta>>) -> anyhow::Result<()> {
        let log_request = view.write_sync()?.get_log_request().clone();
        self.data
            .lock_sync()?
            .to_backend_sender
            .send(ToBackendSignal::LogsSubscribe(log_request))?;
        Ok(())
    }
}

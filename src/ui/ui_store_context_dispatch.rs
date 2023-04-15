use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::bail;
use cursive::reexports::log::{info, warn};
use cursive::traits::Nameable;
use cursive::views::{Dialog, TextView};
use cursive::{Cursive, View};
use cursive_flexi_logger_view::FlexiLoggerView;
use cursive_markup::html::RichRenderer;
use cursive_markup::MarkupView;
use cursive_table_view::TableView;
use k8s_openapi::api::core::v1::{Container, Pod};
use kube::api::GroupVersionKind;
use kube::ResourceExt;

use crate::config::extractor::ActionType;
use crate::eval::engine_factory::build_engine;
use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::model::traits::SerializeExt;
use crate::reexports::sync::RwLock;
use crate::traits::ext::cursive::{SivExt, SivUtilExt};
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
use crate::ui::components::gvk_switcher::build_gvk_switcher;
use crate::ui::components::log_view::build_log_view;
use crate::ui::components::menu::build_menu;
use crate::ui::components::window_switcher::build_window_switcher;
use crate::ui::dispatcher::DispatchContext;
use crate::ui::interactive_command::InteractiveCommand;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::ui::ui_store::{UiStore, UiStoreDispatcherExt};
use crate::ui::view_meta::{LogItem, ViewMeta};
use crate::util::error::{LogError, LogErrorOptionExt, LogErrorResultExt};
use crate::util::panics::ResultExt;
use crate::util::view_with_data::ViewWithMeta;

pub trait DispatchContextExt {
    fn dispatch_update_list_views_for_gvk(
        self,
        gvk: GroupVersionKind,
        reevaluate: bool,
    ) -> anyhow::Result<()>;
    fn dispatch_response_resource_deleted(self, resource: ResourceView) -> anyhow::Result<()>;
    fn dispatch_response_log_data(
        self,
        view_id: usize,
        data: Vec<u8>,
        seq_id: usize,
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
    fn dispatch_logs_apply_previous(
        self,
        view_id: usize,
        show_previous: bool,
    ) -> anyhow::Result<()>;

    fn dispatch_response_resource_updated(self, resource: ResourceView) -> anyhow::Result<()>;
    fn refresh_all(&self, evaluated_resource: EvaluatedResource) -> anyhow::Result<()>;
    fn refresh_single_code_view(
        &self,
        evaluated_resource: EvaluatedResource,
        view_meta: Arc<RwLock<ViewMeta>>,
    ) -> anyhow::Result<()>;
    fn refresh_single_detail_resource(
        &self,
        evaluated_resource: EvaluatedResource,
        view_meta: Arc<RwLock<ViewMeta>>,
    ) -> anyhow::Result<()>;
    fn refresh_single_table_resource(
        &self,
        evaluated_resource: EvaluatedResource,
        view: Arc<RwLock<ViewMeta>>,
    ) -> anyhow::Result<()>;
    fn dispatch_response_discovered_gvks(self, gvks: Vec<GroupVersionKind>) -> anyhow::Result<()>;

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

    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + FnOnce(&mut V) -> R + 'static,
        R: 'static;

    fn call_on_name_wait<V, F, R>(&self, name: &str, callback: F) -> R
    where
        V: View,
        F: Send + 'static + FnOnce(&mut V) -> R + Send,
        R: 'static + Debug;

    fn send<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static;

    fn send_wait<F, R>(&self, callback: F) -> R
    where
        F: FnOnce(&mut Cursive) -> R + Send + 'static,
        R: 'static;
}

impl<'a> DispatchContextExt for DispatchContext<'a, UiStore, ToUiSignal> {
    fn dispatch_update_list_views_for_gvk(
        self,
        gvk: GroupVersionKind,
        reevaluate: bool,
    ) -> anyhow::Result<()> {
        let mut affected_views = self.data.locking(|mut store| {
            let affected_views = store
                .view_stack
                .find_all_by_gvk(&gvk)
                .into_iter()
                .filter(|meta| matches!(meta.read_unwrap().deref(), ViewMeta::List { .. }))
                .peekable();

            if reevaluate {
                store.resource_manager.reevaluate_all_for_gvk(&gvk);
            }

            Ok(affected_views)
        })?;

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

    fn dispatch_response_resource_deleted(self, resource: ResourceView) -> anyhow::Result<()> {
        let gvk = resource.gvk();
        let affected_views = self.data.lock_sync()?.view_stack.find_all_by_gvk(&gvk);
        self.data
            .lock_sync()?
            .resource_manager
            .replace(resource.clone());

        if affected_views.is_empty() {
            warn!(
                "Resource {} will not be deleted (no active view)",
                resource.full_unique_name()
            );
        }

        Ok(())
    }

    fn dispatch_response_log_data(
        self,
        view_id: usize,
        data: Vec<u8>,
        seq_id: usize,
    ) -> anyhow::Result<()> {
        let log_item = LogItem::new(seq_id, data)?;

        let (to_backend_sender, view) = self.data.locking(|store| {
            Ok((
                store.to_backend_sender.clone(),
                store.view_stack.get(view_id),
            ))
        })?;

        if let Some(view) = view {
            let mut view = view.write_sync()?;
            view.push_log_item(log_item);
        } else {
            warn!("Log view not found: {}", view_id);
            to_backend_sender.send(ToBackendSignal::RequestLogsUnsubscribe(view_id))?;
        }

        Ok(())
    }

    fn dispatch_logs_apply_highlight(self, view_id: usize, text: String) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;
        view.write_sync()?.get_log_filter_clearing_mut().value = text;

        Ok(())
    }

    fn dispatch_logs_apply_since_minutes(
        self,
        view_id: usize,
        num_minutes: usize,
    ) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;

        view.write_sync()?
            .get_log_request_clearing_mut()
            .log_params
            .since_seconds = Some((num_minutes * 60) as i64);

        self.send_log_subscribe(view)
    }

    fn dispatch_logs_apply_tail_lines(
        self,
        view_id: usize,
        num_lines: usize,
    ) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;

        view.write_sync()?
            .get_log_request_clearing_mut()
            .log_params
            .tail_lines = Some(num_lines as i64);

        self.send_log_subscribe(view)
    }

    fn dispatch_logs_apply_timestamps(
        self,
        view_id: usize,
        show_timestamps: bool,
    ) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;

        view.write_sync()?
            .get_log_filter_clearing_mut()
            .show_timestamps = show_timestamps;

        Ok(())
    }

    fn dispatch_logs_apply_previous(
        self,
        view_id: usize,
        show_previous: bool,
    ) -> anyhow::Result<()> {
        let view = self.get_view_by_id(view_id)?;
        view.write_sync()?
            .get_log_request_clearing_mut()
            .log_params
            .previous = show_previous;

        self.send_log_subscribe(view)
    }

    fn dispatch_response_resource_updated(self, resource: ResourceView) -> anyhow::Result<()> {
        info!(
            "Received an updated resource {}",
            resource.full_unique_name()
        );

        let all_resources = self.data.locking(|mut store| {
            let (resource, mut pseudo) = store.resource_manager.replace(resource);
            pseudo.push(resource);
            Ok(pseudo)
        })?;

        for evaluated_resource in all_resources {
            let _ = self.refresh_all(evaluated_resource);
        }

        Ok(())
    }

    fn refresh_all(&self, evaluated_resource: EvaluatedResource) -> anyhow::Result<()> {
        let gvk = evaluated_resource.resource.gvk();
        let affected_views = self.data.lock_sync()?.view_stack.find_all_by_gvk(&gvk);

        if affected_views.is_empty() {
            warn!(
                "Resource {} will not be rendered (no active view)",
                evaluated_resource.resource.full_unique_name()
            );
        }

        for affected_view in affected_views {
            let view = Arc::clone(&affected_view);
            let evaluated_resource = evaluated_resource.clone();
            // letting read_unwrap() drop the guard and release the lock before we call the closure
            // otherwise we might have a cycle in locks
            let closure: Box<dyn FnOnce()> = match affected_view.read_unwrap().deref() {
                ViewMeta::List { .. } => Box::new(move || {
                    let _ = self.refresh_single_table_resource(evaluated_resource.clone(), view);
                }),
                ViewMeta::Details { uid, .. } => {
                    if &evaluated_resource.resource.uid_or_name() != uid {
                        continue;
                    }
                    Box::new(move || {
                        let _ =
                            self.refresh_single_detail_resource(evaluated_resource.clone(), view);
                    })
                }
                ViewMeta::Code { uid, .. } => {
                    if &evaluated_resource.resource.uid_or_name() != uid {
                        continue;
                    }
                    Box::new(move || {
                        let _ = self.refresh_single_code_view(evaluated_resource.clone(), view);
                    })
                }
                ViewMeta::Dialog { .. } => continue,
                ViewMeta::WindowSwitcher { .. } => continue,
                ViewMeta::GvkSwitcher { .. } => continue,
                ViewMeta::Logs { .. } => continue,
            };

            closure();
        }

        Ok(())
    }

    fn refresh_single_code_view(
        &self,
        evaluated_resource: EvaluatedResource,
        view_meta: Arc<RwLock<ViewMeta>>,
    ) -> anyhow::Result<()> {
        let resource = evaluated_resource.resource;
        let store = Arc::clone(&self.data);

        let name = view_meta.read_sync()?.get_unique_name();

        self.call_on_name(&name, move |tv: &mut TextView| {
            let styled = store.lock_unwrap().highlight(&resource)?;
            tv.set_content(styled);
            Ok::<(), anyhow::Error>(())
        });

        Ok(())
    }

    fn refresh_single_detail_resource(
        &self,
        evaluated_resource: EvaluatedResource,
        view_meta: Arc<RwLock<ViewMeta>>,
    ) -> anyhow::Result<()> {
        let resource = evaluated_resource.resource;

        let (html, view_name) = self.data.locking(|store| {
            let html = store.detail_view_renderer.render_html(&resource)?;
            let view_name = view_meta.read_sync()?.get_unique_name();
            Ok((html, view_name))
        })?;

        self.call_on_name(
            &view_name,
            move |markup_view: &mut MarkupView<RichRenderer>| {
                *markup_view = cursive_markup::MarkupView::html(&html);
            },
        );
        Ok(())
    }

    fn refresh_single_table_resource(
        &self,
        evaluated_resource: EvaluatedResource,
        view: Arc<RwLock<ViewMeta>>,
    ) -> anyhow::Result<()> {
        let view_name = self.data.locking(|store| {
            let view_guard = view.read_sync()?;
            if !store.should_display_resource(view_guard.get_filter(), &evaluated_resource) {
                LogError::log_info(format!(
                    "Not updating view {:?}: {:?} (filtered)",
                    view_guard.get_unique_name(),
                    evaluated_resource.resource
                ))
            } else {
                Ok(view_guard.get_unique_name())
            }
        })?;

        self.call_on_name(
            &view_name,
            move |table: &mut TableView<EvaluatedResource, usize>| {
                table.add_or_update_resource(evaluated_resource);
            },
        );

        Ok(())
    }

    fn dispatch_response_discovered_gvks(self, gvks: Vec<GroupVersionKind>) -> anyhow::Result<()> {
        self.data.locking(|mut store| {
            store.gvks = gvks.clone();
            Ok(())
        })?;
        self.data.lock_unwrap().gvks = gvks.clone();
        let store = Arc::clone(&self.data);
        self.send(move |siv| {
            let mut menubar = build_menu(gvks, store);
            menubar.autohide = false;
            *siv.menubar() = menubar;
        });

        Ok(())
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
            .dispatch_sync(ToUiSignal::ReplaceTableItems(id));

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
            .dispatch_sync(ToUiSignal::ReplaceTableItems(id));

        Ok(())
    }

    fn dispatch_show_details(self, resource: ResourceView) -> anyhow::Result<()> {
        let gvk = resource.gvk();

        let action_type = self
            .data
            .lock_unwrap()
            .resource_manager
            .get_submit_handler_type(&gvk)
            .to_log_warn(|| format!("No event type handler for gvk {}", gvk.full_name()))?;

        match action_type {
            ActionType::ShowDetailsTable(extractor_name) => {
                let gvk = resource.build_pseudo_gvk(&extractor_name);
                self.dispatcher
                    .dispatch_sync(ToUiSignal::ShowGvk(gvk.clone()));
                self.dispatcher
                    .dispatch_sync(ToUiSignal::UpdateListViewForGvk(gvk, false));
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
        match view_meta.deref() {
            ViewMeta::List { gvk, .. } => {
                self.dispatcher
                    .dispatch_sync(ToUiSignal::UpdateListViewForGvk(gvk.clone(), true));
                return Ok(());
            }
            ViewMeta::Details { .. } => {}
            ViewMeta::Dialog { .. } => {}
            ViewMeta::WindowSwitcher { .. } => {}
            ViewMeta::Code { .. } => {}
            ViewMeta::GvkSwitcher { .. } => {}
            ViewMeta::Logs { .. } => {}
        }
        LogError::log_error("F5 not implemented for the current view")
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
                .send(ToBackendSignal::RequestLogsSubscribe(log_request))?;
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
            .send(ToBackendSignal::RequestLogsSubscribe(log_request))?;
        Ok(())
    }

    fn call_on_name<V, F, R>(&self, name: &str, callback: F)
    where
        V: View,
        F: Send + 'static + FnOnce(&mut V) -> R,
        R: 'static,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        self.num_callbacks
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        sink.channel_call_on_name(self.sender.clone(), name, callback);
    }

    fn call_on_name_wait<V, F, R>(&self, name: &str, callback: F) -> R
    where
        V: View,
        F: Send + 'static + FnOnce(&mut V) -> R,
        R: 'static + Debug,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        let sender = self.sender.clone();

        sink.call_on_name(name, move |view: &mut V| {
            let result = callback(view);
            sender.send_unwrap(Box::new(result));
        });

        let result = self.receiver.recv().unwrap_or_log();
        match result.downcast::<R>() {
            Ok(result) => *result,
            Err(err) => panic!("Failed to downcast result: {:?}", err),
        }
    }

    fn send<F>(&self, callback: F)
    where
        F: FnOnce(&mut Cursive) + Send + 'static,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        self.num_callbacks
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        sink.channel_box(self.sender.clone(), callback);
    }

    fn send_wait<F, R>(&self, callback: F) -> R
    where
        F: FnOnce(&mut Cursive) -> R + Send + 'static,
        R: 'static,
    {
        let sink = self.data.lock_unwrap().sink.clone();
        let sender = self.sender.clone();
        sink.send_box(move |siv| {
            let result = callback(siv);
            sender.send_unwrap(Box::new(result));
        });
        let result = self.receiver.recv().unwrap_or_log();
        match result.downcast::<R>() {
            Ok(result) => *result,
            Err(err) => panic!("Failed to downcast result: {:?}", err),
        }
    }
}

use std::ops::Deref;
use std::sync::Arc;

use crate::model::port_forward_request::PortForwardRequest;
use cursive::reexports::log::{info, warn};
use cursive_cached_text_view::CachedTextView;
use cursive_markup::html::RichRenderer;
use cursive_markup::MarkupView;
use cursive_table_view::TableView;
use kube::api::GroupVersionKind;

use crate::model::resource::resource_view::{EvaluatedResource, ResourceView};
use crate::reexports::sync::RwLock;
use crate::traits::ext::gvk::GvkExt;
use crate::traits::ext::mutex::MutexExt;
use crate::traits::ext::rw_lock::RwLockExt;
use crate::traits::ext::table_view::TableViewExt;
use crate::ui::components::menu::build_menu;
use crate::ui::dispatch::send_helper_ext::DispatchContextSendHelperExt;
use crate::ui::dispatcher::DispatchContext;
use crate::ui::signals::{FromBackendSignal, ToBackendSignal};
use crate::ui::ui_store::UiStore;
use crate::ui::view_meta::{LogItem, ViewMeta, ViewMetaLogExt};
use crate::util::error::LogError;

pub(crate) trait DispatchContextBackendExt {
    fn dispatch_response_discovered_gvks(self, gvks: Vec<GroupVersionKind>) -> anyhow::Result<()>;
    fn dispatch_response_resource_updated(self, resource: ResourceView) -> anyhow::Result<()>;
    fn dispatch_response_resource_deleted(self, resource: ResourceView) -> anyhow::Result<()>;
    fn dispatch_response_log_data(
        self,
        view_id: usize,
        data: Vec<u8>,
        seq_id: usize,
    ) -> anyhow::Result<()>;
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
    fn dispatch_port_forwarding_started(
        self,
        port_forwarding: Arc<PortForwardRequest>,
    ) -> anyhow::Result<()>;
}

impl<'a> DispatchContextBackendExt for DispatchContext<'a, UiStore, FromBackendSignal> {
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

    fn dispatch_response_resource_updated(self, resource: ResourceView) -> anyhow::Result<()> {
        info!(
            "Received an updated resource {}",
            resource.full_unique_name()
        );

        let all_resources = self.data.locking(|store| {
            let (resource, mut pseudo) = store.resource_manager.write_sync()?.replace(resource);
            pseudo.push(resource);
            Ok(pseudo)
        })?;

        for evaluated_resource in all_resources {
            let _ = self.refresh_all(evaluated_resource);
        }

        Ok(())
    }

    fn dispatch_response_resource_deleted(self, resource: ResourceView) -> anyhow::Result<()> {
        let gvk = resource.gvk();
        let affected_views = self.data.lock_sync()?.view_stack.find_all_by_gvk(&gvk);
        self.data
            .lock_sync()?
            .resource_manager
            .write_sync()?
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
            to_backend_sender.send(ToBackendSignal::LogsUnsubscribe(view_id))?;
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

        self.call_on_name(&name, move |tv: &mut CachedTextView| {
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

    fn dispatch_port_forwarding_started(
        self,
        port_forwarding: Arc<PortForwardRequest>,
    ) -> anyhow::Result<()> {
        self.data.lock_unwrap().pf_requests.push(port_forwarding);
        Ok(())
    }
}

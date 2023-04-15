use cursive::reexports::log::error;

use crate::ui::dispatcher::{Dispatch, DispatchContext};
use crate::ui::signals::ToUiSignal;
use crate::ui::ui_store::UiStore;
use crate::ui::ui_store_context_dispatch::DispatchContextExt;
use crate::util::error::LogError;

impl Dispatch<UiStore, ToUiSignal> for ToUiSignal {
    fn dispatch(self, context: DispatchContext<UiStore, ToUiSignal>) {
        let signal_name = self.as_ref().to_string();
        let result = match self {
            ToUiSignal::ResponseResourceUpdated(resource) => {
                context.dispatch_response_resource_updated(resource)
            }
            ToUiSignal::ResponseDiscoveredGvks(gvks) => {
                context.dispatch_response_discovered_gvks(gvks)
            }
            ToUiSignal::ApplyNamespaceFilter(id, ns) => {
                context.dispatch_apply_namespace_filter(id, ns)
            }
            ToUiSignal::ApplyNameFilter(id, name) => context.dispatch_apply_name_filter(id, name),
            ToUiSignal::ShowDetails(resource) => context.dispatch_show_details(resource),
            ToUiSignal::ShowGvk(gvk) => context.dispatch_show_gvk(gvk),
            ToUiSignal::CtrlSPressed => context.dispatch_ctrl_s(),
            ToUiSignal::ExecuteCurrent => context.dispatch_shell_current(),
            ToUiSignal::CtrlYPressed => context.dispatch_show_yaml(),
            ToUiSignal::F5Pressed => context.dispatch_refresh(),
            ToUiSignal::EscPressed => context.dispatch_pop_view(),
            ToUiSignal::ShowDebugLog => context.dispatch_show_debug_console(),
            ToUiSignal::UpdateListViewForGvk(gvk, reevaluate) => {
                context.dispatch_update_list_views_for_gvk(gvk, reevaluate)
            }
            ToUiSignal::ReplaceTableItems(view_id) => context.dispatch_replace_table_items(view_id),
            ToUiSignal::Chain(items) => {
                for cb in items {
                    if let Some(signal) = cb(context.clone()) {
                        context.dispatcher.dispatch_sync(signal);
                    }
                }
                Ok(())
            }
            ToUiSignal::CtrlPPressed => context.dispatch_dump_resource_sample(),
            ToUiSignal::AltPlusPressed => context.dispatch_show_window_switcher(),
            ToUiSignal::ShowWindow(id) => context.dispatch_bring_to_front(id),
            ToUiSignal::CtrlSlashPressed => context.dispatch_ctrl_slash(),
            ToUiSignal::CtrlKPressed => context.dispatch_ctrl_k(),
            ToUiSignal::ResponseResourceDeleted(resource) => {
                context.dispatch_response_resource_deleted(resource)
            }
            ToUiSignal::CtrlLPressed => context.dispatch_ctrl_l(),
            ToUiSignal::ResponseLogData {
                seq_id,
                view_id,
                data,
            } => context.dispatch_response_log_data(view_id, data, seq_id),
            ToUiSignal::LogsApplyHighlight(view_id, text) => {
                context.dispatch_logs_apply_highlight(view_id, text)
            }
            ToUiSignal::LogsApplySinceMinutes(view_id, num_minutes) => {
                context.dispatch_logs_apply_since_minutes(view_id, num_minutes)
            }
            ToUiSignal::LogsApplyTailLines(view_id, num_lines) => {
                context.dispatch_logs_apply_tail_lines(view_id, num_lines)
            }
            ToUiSignal::LogsApplyTimestamps(view_id, show_timestamps) => {
                context.dispatch_logs_apply_timestamps(view_id, show_timestamps)
            }
            ToUiSignal::LogsApplyPrevious(view_id, show_previous) => {
                context.dispatch_logs_apply_previous(view_id, show_previous)
            }
        };

        if let Err(err) = result {
            if let Some(err) = err.downcast_ref::<LogError>() {
                err.log();
            } else {
                error!("Failed to dispatch signal {signal_name}: {err}");
            }
        }
    }
}

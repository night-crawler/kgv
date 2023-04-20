use crate::ui::dispatch::log_signal_result;
use crate::ui::dispatch::ui_dispatch_ext::DispatchContextUiExt;
use crate::ui::dispatcher::{Dispatch, DispatchContext};
use crate::ui::signals::InterUiSignal;
use crate::ui::ui_store::UiStore;

impl Dispatch<UiStore, InterUiSignal> for InterUiSignal {
    fn dispatch(self, context: DispatchContext<UiStore, InterUiSignal>) {
        let signal_name = self.as_ref().to_string();
        let result = match self {
            InterUiSignal::CtrlLPressed => context.dispatch_ctrl_l(),

            InterUiSignal::ApplyNamespaceFilter(id, ns) => {
                context.dispatch_apply_namespace_filter(id, ns)
            }
            InterUiSignal::ApplyNameFilter(id, name) => {
                context.dispatch_apply_name_filter(id, name)
            }
            InterUiSignal::ShowDetails(resource) => context.dispatch_show_details(resource),
            InterUiSignal::ShowGvk(gvk) => context.dispatch_show_gvk(gvk),
            InterUiSignal::CtrlSPressed => context.dispatch_ctrl_s(),
            InterUiSignal::CtrlYPressed => context.dispatch_show_yaml(),
            InterUiSignal::F5Pressed => context.dispatch_refresh(),
            InterUiSignal::EscPressed => context.dispatch_pop_view(),
            InterUiSignal::ShowDebugLog => context.dispatch_show_debug_console(),
            InterUiSignal::UpdateListViewForGvk(gvk, reevaluate) => {
                context.dispatch_update_list_views_for_gvk(gvk, reevaluate)
            }
            InterUiSignal::ReplaceTableItems(view_id) => {
                context.dispatch_replace_table_items(view_id)
            }
            InterUiSignal::Chain(items) => {
                for cb in items {
                    if let Some(signal) = cb(context.clone()) {
                        context.dispatcher.dispatch_sync(signal);
                    }
                }
                Ok(())
            }
            InterUiSignal::CtrlPPressed => context.dispatch_dump_resource_sample(),
            InterUiSignal::AltPlusPressed => context.dispatch_show_window_switcher(),
            InterUiSignal::ShowWindow(id) => context.dispatch_bring_to_front(id),
            InterUiSignal::CtrlSlashPressed => context.dispatch_ctrl_slash(),
            InterUiSignal::CtrlKPressed => context.dispatch_ctrl_k(),

            InterUiSignal::LogsApplyHighlight(view_id, text) => {
                context.dispatch_logs_apply_highlight(view_id, text)
            }
            InterUiSignal::LogsApplySinceMinutes(view_id, num_minutes) => {
                context.dispatch_logs_apply_since_minutes(view_id, num_minutes)
            }
            InterUiSignal::LogsApplyTailLines(view_id, num_lines) => {
                context.dispatch_logs_apply_tail_lines(view_id, num_lines)
            }
            InterUiSignal::LogsApplyTimestamps(view_id, show_timestamps) => {
                context.dispatch_logs_apply_timestamps(view_id, show_timestamps)
            }
            InterUiSignal::LogsApplyPrevious(view_id, show_previous) => {
                context.dispatch_logs_apply_previous(view_id, show_previous)
            }
        };

        log_signal_result(result, &signal_name);
    }
}

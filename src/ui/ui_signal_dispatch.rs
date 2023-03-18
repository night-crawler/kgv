use crate::ui::dispatcher::{Dispatch, DispatchContext};
use crate::ui::signals::ToUiSignal;
use crate::ui::ui_store::UiStore;
use crate::ui::ui_store_dispatch::DispatchContextExt;

impl Dispatch<UiStore, ToUiSignal> for ToUiSignal {
    fn dispatch(self, context: DispatchContext<UiStore, ToUiSignal>) {
        match self {
            ToUiSignal::ResponseResourceUpdated(resource) => {
                context.dispatch_response_resource_updated(resource);
            }
            ToUiSignal::ResponseDiscoveredGvks(gvks) => {
                context.dispatch_response_discovered_gvks(gvks);
            }
            ToUiSignal::ResponseGvkItems(next_gvk, resources) => {
                context.dispatch_response_gvk_items(next_gvk, resources);
            }
            ToUiSignal::ApplyNamespaceFilter(id, ns) => {
                context.dispatch_apply_namespace_filter(id, ns);
            }
            ToUiSignal::ApplyNameFilter(id, name) => {
                context.dispatch_apply_name_filter(id, name);
            }
            ToUiSignal::ShowDetails(resource) => {
                context.dispatch_show_details(resource);
            }
            ToUiSignal::ShowGvk(gvk) => {
                context.dispatch_show_gvk(gvk);
            }
            ToUiSignal::CtrlSPressed => {
                context.dispatch_ctrl_s();
            }
            ToUiSignal::ExecuteCurrent => {
                context.dispatch_shell_current();
            }
            ToUiSignal::CtrlYPressed => {
                context.dispatch_ctrl_y();
            }
            ToUiSignal::F5Pressed => {
                context.dispatch_f5();
            }
            ToUiSignal::EscPressed => {
                context.dispatch_esc();
            }
            ToUiSignal::ShowDebugLog => {
                context.dispatch_show_debug_log();
            }
            ToUiSignal::UpdateListViewForGvk(gvk) => {
                context.dispatch_update_list_views_for_gvk(gvk);
            }
            ToUiSignal::ReplaceTableItems(view_id) => context.dispatch_replace_table_items(view_id),
            ToUiSignal::Chain(items) => {
                for cb in items {
                    if let Some(signal) = cb(context.clone()) {
                        context.dispatcher.dispatch_sync(signal);
                    }
                }
            }
            ToUiSignal::CtrlPPressed => {
                context.dispatch_ctrl_p();
            }
            ToUiSignal::CtrlPlusPressed => {
                context.dispatch_alt_plus();
            }
            ToUiSignal::ShowWindow(id) => {
                context.dispatch_show_window(id);
            }
            ToUiSignal::RemoveWindowSwitcher => {
                context.dispatch_remove_window_switcher();
            }
        }
    }
}

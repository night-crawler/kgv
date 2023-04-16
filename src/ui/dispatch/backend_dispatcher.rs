use crate::ui::dispatch::backend_dispatch_ext::DispatchContextBackendExt;
use crate::ui::dispatch::log_signal_result;
use crate::ui::dispatcher::{Dispatch, DispatchContext};
use crate::ui::signals::FromBackendSignal;
use crate::ui::ui_store::UiStore;

impl Dispatch<UiStore, FromBackendSignal> for FromBackendSignal {
    fn dispatch(self, context: DispatchContext<UiStore, FromBackendSignal>) {
        let signal_name = self.as_ref().to_string();

        let result = match self {
            FromBackendSignal::ResponseResourceUpdated(resource) => {
                context.dispatch_response_resource_updated(resource)
            }
            FromBackendSignal::ResponseDiscoveredGvks(gvks) => {
                context.dispatch_response_discovered_gvks(gvks)
            }
            FromBackendSignal::ResponseResourceDeleted(resource) => {
                context.dispatch_response_resource_deleted(resource)
            }
            FromBackendSignal::ResponseLogData {
                seq_id,
                view_id,
                data,
            } => context.dispatch_response_log_data(view_id, data, seq_id),
        };

        log_signal_result(result, &signal_name);
    }
}

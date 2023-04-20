use kube::api::GroupVersionKind;
use strum_macros::AsRefStr;

use crate::model::log_request::LogRequest;
use crate::model::port_forward_request::PortForwardRequest;
use crate::model::resource::resource_view::ResourceView;
use crate::ui::dispatcher::DispatchContext;
use crate::ui::ui_store::UiStore;

#[derive(Debug)]
pub(crate) enum ToBackendSignal {
    Remove(ResourceView),
    RegisterGvk(GroupVersionKind),
    LogsSubscribe(LogRequest),
    LogsUnsubscribe(usize),
    PortForward(PortForwardRequest)
}

pub(crate) type ToUiChainDispatch = dyn FnOnce(DispatchContext<UiStore, InterUiSignal>) -> Option<InterUiSignal>
    + Send
    + Sync
    + 'static;

#[derive(AsRefStr)]
pub(crate) enum FromBackendSignal {
    LogData {
        view_id: usize,
        seq_id: usize,
        data: Vec<u8>,
    },
    ResourceUpdated(ResourceView),
    ResourceDeleted(ResourceView),
    DiscoveredGvks(Vec<GroupVersionKind>),
}

#[derive(AsRefStr)]
pub(crate) enum InterUiSignal {
    LogsApplyHighlight(usize, String),
    LogsApplySinceMinutes(usize, usize),
    LogsApplyTailLines(usize, usize),
    LogsApplyTimestamps(usize, bool),
    LogsApplyPrevious(usize, bool),

    ApplyNamespaceFilter(usize, String),
    ApplyNameFilter(usize, String),

    ShowGvk(GroupVersionKind),
    ShowDetails(ResourceView),

    UpdateListViewForGvk(GroupVersionKind, bool),
    ReplaceTableItems(usize),
    ShowWindow(usize),

    Chain(Vec<Box<ToUiChainDispatch>>),

    CtrlKPressed,
    CtrlLPressed,
    CtrlFPressed,
    CtrlSPressed,
    AltPlusPressed,
    CtrlYPressed,
    CtrlSlashPressed,
    CtrlPPressed,
    F5Pressed,
    EscPressed,
    ShowDebugLog,
}

impl InterUiSignal {
    pub(crate) fn new_chain() -> Self {
        InterUiSignal::Chain(vec![])
    }
    pub(crate) fn chain(
        self,
        cb: impl FnOnce(DispatchContext<UiStore, InterUiSignal>) -> Option<InterUiSignal>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        let mut signal = self;
        match &mut signal {
            InterUiSignal::Chain(ref mut items) => {
                items.push(Box::new(cb));
            }
            _ => panic!("Can only chain on ToUiSignal::Chain"),
        }
        signal
    }
}

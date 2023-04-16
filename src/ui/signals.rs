use kube::api::GroupVersionKind;
use strum_macros::AsRefStr;

use crate::model::log_request::LogRequest;
use crate::model::resource::resource_view::ResourceView;
use crate::ui::dispatcher::DispatchContext;
use crate::ui::ui_store::UiStore;

#[derive(Debug)]
pub enum ToBackendSignal {
    Remove(ResourceView),
    RequestRegisterGvk(GroupVersionKind),
    RequestLogsSubscribe(LogRequest),
    RequestLogsUnsubscribe(usize),

    RequestDetails(ResourceView),
}

pub type ToUiChainDispatch =
    dyn FnOnce(DispatchContext<UiStore, InterUiSignal>) -> Option<InterUiSignal> + Send + Sync + 'static;


#[derive(AsRefStr)]
pub enum FromBackendSignal {
    ResponseLogData {
        view_id: usize,
        seq_id: usize,
        data: Vec<u8>,
    },
    ResponseResourceUpdated(ResourceView),
    ResponseResourceDeleted(ResourceView),
    ResponseDiscoveredGvks(Vec<GroupVersionKind>),
}

#[derive(AsRefStr)]
pub enum InterUiSignal {
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
    CtrlSPressed,
    AltPlusPressed,
    CtrlYPressed,
    CtrlSlashPressed,
    CtrlPPressed,
    ExecuteCurrent,
    F5Pressed,
    EscPressed,
    ShowDebugLog,
}

impl InterUiSignal {
    pub fn new_chain() -> Self {
        InterUiSignal::Chain(vec![])
    }
    pub fn chain(
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

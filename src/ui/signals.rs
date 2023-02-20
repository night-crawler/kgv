use crate::model::resource::resource_view::ResourceView;
use kube::api::GroupVersionKind;

#[derive(Debug)]
pub enum ToBackendSignal {
    RequestRegisterGvk(GroupVersionKind),
    RequestGvkItems(GroupVersionKind),

    RequestDetails(ResourceView),
}

#[derive(Debug)]
pub enum ToUiSignal {
    ResponseResourceUpdated(ResourceView),
    ResponseDiscoveredGvks(Vec<GroupVersionKind>),
    ResponseGvkItems(GroupVersionKind, Option<Vec<ResourceView>>),

    ApplyNamespaceFilter(String),
    ApplyNameFilter(String),

    ShowGvk(GroupVersionKind),
    ShowDetails(ResourceView),

    CtrlSPressed,
    CtrlYPressed,
    ExecuteCurrent,
}

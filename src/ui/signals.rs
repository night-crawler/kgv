use kube::api::GroupVersionKind;

use crate::model::resource_view::ResourceView;

#[derive(Debug)]
pub enum ToBackendSignal {
    RequestRegisterGvk(GroupVersionKind),
    RequestGvkItems(GroupVersionKind),
}

#[derive(Debug)]
pub enum ToUiSignal {
    ResponseResourceUpdated(ResourceView),
    ResponseDiscoveredGvks(Vec<GroupVersionKind>),
    ResponseGvkItems(GroupVersionKind, Option<Vec<ResourceView>>),
    
    ApplyNamespaceFilter(String),
    ApplyNameFilter(String)
}

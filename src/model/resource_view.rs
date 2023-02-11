use std::fmt::Debug;
use std::sync::Arc;

use k8s_openapi::api::core::v1::{ConfigMap, Namespace, Pod};
use kube::ResourceExt;
use crate::{mk_filter_enum, ResourceColumn};

mk_filter_enum!(ResourceView, [
    Namespace: [ResourceColumn::Namespace, ResourceColumn::Name],
    Pod: [ResourceColumn::Namespace, ResourceColumn::Name, ResourceColumn::Status],
    ConfigMap: [ResourceColumn::Namespace, ResourceColumn::Name]
]);



impl ResourceView {
    pub fn name(&self) -> String {
        match self {
            ResourceView::Pod(r) => r.name_any(),
            ResourceView::Namespace(r) => r.name_any(),
            ResourceView::ConfigMap(r) => r.name_any(),
            ResourceView::DynamicObject(r) => r.name_any()
        }
    }

    pub fn namespace(&self) -> String {
        match self {
            ResourceView::Pod(r) => r.namespace().unwrap_or_default(),
            ResourceView::Namespace(_) => String::new(),
            ResourceView::ConfigMap(r) => r.namespace().unwrap_or_default(),
            ResourceView::DynamicObject(r) => r.namespace().unwrap_or_default()
        }
    }
}

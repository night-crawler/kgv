use std::fmt::Debug;
use std::sync::Arc;

use chrono::Utc;
use k8s_openapi::api::core::v1::{ConfigMap, Namespace, Node, PersistentVolumeClaim, Pod};
use kube::api::GroupVersionKind;
use kube::ResourceExt;

use crate::eval::eval_result::EvalResult;
use crate::traits::ext::gvk::GvkExt;
use crate::{extract_age, extract_status, mk_resource_enum};

mk_resource_enum!(
    ResourceView,
    Namespace,
    Pod,
    ConfigMap,
    Node,
    PersistentVolumeClaim
);

#[derive(Debug, Clone)]
pub struct EvaluatedResource {
    pub values: Arc<Vec<EvalResult>>,
    pub resource: ResourceView,
}

impl ResourceView {
    pub fn status(&self) -> String {
        match self {
            ResourceView::Namespace(r) => extract_status!(r),
            ResourceView::Pod(r) => extract_status!(r),
            _ => String::new(),
        }
    }

    pub fn to_pseudo_gvk(&self, extractor_name: &str) -> GroupVersionKind {
        let mut gvk = self.gvk();
        let parts = [&gvk.kind, extractor_name, &self.name()];
        gvk.kind = parts.join("#");
        gvk
    }
}

impl ResourceView {
    pub fn full_unique_name(&self) -> String {
        use crate::traits::ext::gvk::GvkNameExt;

        let gvk = self.gvk().full_name();
        format!("{}::{}/{}", gvk, self.namespace(), self.name())
    }
}

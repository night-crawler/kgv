use std::fmt::Debug;
use std::sync::Arc;

use chrono::Utc;
use k8s_openapi::api::core::v1::{ConfigMap, Namespace, Node, PersistentVolumeClaim, Pod};
use kube::ResourceExt;

use crate::eval::eval_result::EvalResult;
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
}

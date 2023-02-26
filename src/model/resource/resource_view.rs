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

    pub fn ready(&self) -> String {
        match self {
            ResourceView::Pod(r) => {
                if let Some(statuses) = r
                    .status
                    .as_ref()
                    .and_then(|status| status.container_statuses.as_ref())
                {
                    let total = statuses.len();
                    let ready = statuses.iter().filter(|status| status.ready).count();

                    format!("{}/{}", ready, total)
                } else {
                    String::new()
                }
            }

            _ => String::new(),
        }
    }

    pub fn ips(&self) -> Option<Vec<String>> {
        match self {
            ResourceView::Pod(r) => r
                .status
                .as_ref()
                .and_then(|status| status.pod_ips.as_ref())
                .map(|pod_ips| {
                    pod_ips
                        .iter()
                        .filter_map(|pod_ip| pod_ip.ip.clone())
                        .collect::<Vec<_>>()
                }),
            _ => None,
        }
    }

    pub fn restarts(&self) -> String {
        match self {
            ResourceView::Pod(r) => r
                .status
                .as_ref()
                .and_then(|status| status.container_statuses.as_ref())
                .into_iter()
                .flatten()
                .map(|container_status| container_status.restart_count)
                .sum::<i32>()
                .to_string(),
            _ => String::new(),
        }
    }

    pub fn node(&self) -> String {
        match self {
            ResourceView::Pod(r) => r
                .spec
                .as_ref()
                .and_then(|spec| spec.node_name.as_ref())
                .cloned()
                .unwrap_or_default(),
            _ => String::new(),
        }
    }
}

use std::fmt::Debug;
use std::sync::Arc;

use chrono::Utc;
use k8s_openapi::api::core::v1::{ConfigMap, Namespace, Pod};
use kube::ResourceExt;

use crate::{extract_age, extract_status, mk_resource_enum, ResourceColumn};

mk_resource_enum!(ResourceView, [
    Namespace: [
        ResourceColumn::Name,
        ResourceColumn::Status,
        ResourceColumn::Age
    ],
    Pod: [
        ResourceColumn::Namespace,
        ResourceColumn::Name,
        ResourceColumn::Status,
        ResourceColumn::Ready,
        ResourceColumn::Restarts,
        ResourceColumn::Ip,
        ResourceColumn::Node,
        ResourceColumn::Age
    ],
    ConfigMap: [
        ResourceColumn::Namespace,
        ResourceColumn::Name,
        ResourceColumn::Age
    ]
]);

impl ResourceView {
    pub fn name(&self) -> String {
        match self {
            ResourceView::Pod(r) => r.name_any(),
            ResourceView::Namespace(r) => r.name_any(),
            ResourceView::ConfigMap(r) => r.name_any(),
            ResourceView::DynamicObject(r) => r.name_any(),
        }
    }

    pub fn namespace(&self) -> String {
        match self {
            ResourceView::Pod(r) => r.namespace().unwrap_or_default(),
            ResourceView::Namespace(_) => String::new(),
            ResourceView::ConfigMap(r) => r.namespace().unwrap_or_default(),
            ResourceView::DynamicObject(r) => r.namespace().unwrap_or_default(),
        }
    }

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

use std::fmt::Debug;
use std::sync::Arc;

use chrono::Utc;
use cursive::reexports::log::error;
use itertools::Itertools;
use kube::api::GroupVersionKind;
use kube::{Resource, ResourceExt};

use crate::eval::eval_result::EvalResult;
use crate::model::pseudo_resource::PSEUDO_RESOURCE_JOIN_SEQ;
use crate::reexports::k8s::*;
use crate::traits::ext::gvk::GvkExt;
use crate::{extract_age, extract_phase, mk_resource_enum};

mk_resource_enum!(
    ResourceView,
    Namespace,
    Pod,
    ConfigMap,
    Node,
    PersistentVolumeClaim,
    ResourceQuota,
    DaemonSet,
    Service,
    ServiceAccount,
    Secret,
    Deployment,
    ReplicaSet,
    StatefulSet,
    Role,
    RoleBinding,
    ControllerRevision,
    EndpointSlice,
    FlowSchema,
    PriorityLevelConfiguration,
    CSIDriver,
    CustomResourceDefinition,
    CertificateSigningRequest,
    Ingress,
    IngressClass,
    NetworkPolicy,
    RuntimeClass,
    CronJob,
    Job,
    HorizontalPodAutoscalerV1,
    HorizontalPodAutoscalerV2,
    ClusterRole,
    ClusterRoleBinding,
    PersistentVolume,
    LimitRange,
    PodTemplate,
    ReplicationController,
    CoreEvent,
    Event,
    MutatingWebhookConfiguration,
    ValidatingWebhookConfiguration,
    Lease,
    ComponentStatus,
    Endpoints,
    PodDisruptionBudget,
    PriorityClass,
    CSIStorageCapacityV1,
    CSIStorageCapacityV1beta1,
    StorageClass,
    VolumeAttachment,
    APIService,
    CSINode // Event
);

#[derive(Debug, Clone)]
pub struct EvaluatedResource {
    pub values: Arc<Vec<EvalResult>>,
    pub resource: ResourceView,
}

impl ResourceView {
    pub fn uid_or_name(&self) -> String {
        self.uid().unwrap_or_else(|| {
            error!("Received a resource without uid: {:?}", self);
            self.full_unique_name()
        })
    }

    pub fn status(&self) -> String {
        match self {
            ResourceView::Namespace(r) => extract_phase!(r),
            ResourceView::Pod(r) => extract_phase!(r),
            ResourceView::Node(r) => extract_phase!(r),
            ResourceView::PersistentVolume(r) => extract_phase!(r),
            ResourceView::PseudoResource(r) => r.source.status(),
            _ => if self.deletion_timestamp().is_some() {
                "Deleted"
            } else {
                "Active"
            }
            .to_string(),
        }
    }

    pub fn build_pseudo_gvk(&self, extractor_name: &str) -> GroupVersionKind {
        let mut gvk = self.gvk();
        let parts = [&gvk.kind, extractor_name, &self.name()];
        gvk.kind = parts.join(PSEUDO_RESOURCE_JOIN_SEQ);
        gvk
    }
}

impl ResourceView {
    pub fn full_unique_name(&self) -> String {
        use crate::traits::ext::gvk::GvkNameExt;

        let gvk = self.gvk().full_name();
        let parts = [&gvk, &self.namespace(), &self.name()];
        parts.iter().filter(|part| !part.is_empty()).join("/")
    }
}

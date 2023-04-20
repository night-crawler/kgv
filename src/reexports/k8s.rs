pub(crate) use k8s_openapi::api::admissionregistration::v1::{
    MutatingWebhookConfiguration, ValidatingWebhookConfiguration,
};
pub(crate) use k8s_openapi::api::apps::v1::ControllerRevision;
pub(crate) use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
pub(crate) use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler as HorizontalPodAutoscalerV1;
pub(crate) use k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler as HorizontalPodAutoscalerV2;
pub(crate) use k8s_openapi::api::batch::v1::{CronJob, Job};
pub(crate) use k8s_openapi::api::certificates::v1::CertificateSigningRequest;
pub(crate) use k8s_openapi::api::coordination::v1::Lease;
pub(crate) use k8s_openapi::api::core::v1::{ComponentStatus, Endpoints};
pub(crate) use k8s_openapi::api::core::v1::{
    ConfigMap, Namespace, Node, PersistentVolume, PersistentVolumeClaim, Pod, ResourceQuota,
    Secret, Service, ServiceAccount,
};
pub(crate) use k8s_openapi::api::core::v1::{
    Event as CoreEvent, LimitRange, PodTemplate, ReplicationController,
};
pub(crate) use k8s_openapi::api::discovery::v1::EndpointSlice;
pub(crate) use k8s_openapi::api::events::v1::Event;
pub(crate) use k8s_openapi::api::flowcontrol::v1beta3::{FlowSchema, PriorityLevelConfiguration};
pub(crate) use k8s_openapi::api::networking::v1::{Ingress, IngressClass, NetworkPolicy};
pub(crate) use k8s_openapi::api::node::v1::RuntimeClass;
pub(crate) use k8s_openapi::api::policy::v1::PodDisruptionBudget;
pub(crate) use k8s_openapi::api::rbac::v1::{ClusterRole, ClusterRoleBinding};
pub(crate) use k8s_openapi::api::rbac::v1::{Role, RoleBinding};
pub(crate) use k8s_openapi::api::scheduling::v1::PriorityClass;
pub(crate) use k8s_openapi::api::storage::v1::CSIDriver;
pub(crate) use k8s_openapi::api::storage::v1::CSINode;
pub(crate) use k8s_openapi::api::storage::v1::{
    CSIStorageCapacity as CSIStorageCapacityV1, StorageClass, VolumeAttachment,
};
pub(crate) use k8s_openapi::api::storage::v1beta1::CSIStorageCapacity as CSIStorageCapacityV1beta1;
pub(crate) use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
pub(crate) use k8s_openapi::kube_aggregator::pkg::apis::apiregistration::v1::APIService;

pub use k8s_openapi::api::admissionregistration::v1::{
    MutatingWebhookConfiguration, ValidatingWebhookConfiguration,
};
pub use k8s_openapi::api::apps::v1::ControllerRevision;
pub use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
pub use k8s_openapi::api::autoscaling::v1::HorizontalPodAutoscaler as HorizontalPodAutoscalerV1;
pub use k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler as HorizontalPodAutoscalerV2;
pub use k8s_openapi::api::batch::v1::{CronJob, Job};
pub use k8s_openapi::api::certificates::v1::CertificateSigningRequest;
pub use k8s_openapi::api::coordination::v1::Lease;
pub use k8s_openapi::api::core::v1::{ComponentStatus, Endpoints};
pub use k8s_openapi::api::core::v1::{
    ConfigMap, Namespace, Node, PersistentVolume, PersistentVolumeClaim, Pod, ResourceQuota,
    Secret, Service, ServiceAccount,
};
pub use k8s_openapi::api::core::v1::{
    Event as CoreEvent, LimitRange, PodTemplate, ReplicationController,
};
pub use k8s_openapi::api::discovery::v1::EndpointSlice;
pub use k8s_openapi::api::events::v1::Event;
pub use k8s_openapi::api::flowcontrol::v1beta3::{FlowSchema, PriorityLevelConfiguration};
pub use k8s_openapi::api::networking::v1::{Ingress, IngressClass, NetworkPolicy};
pub use k8s_openapi::api::node::v1::RuntimeClass;
pub use k8s_openapi::api::policy::v1::PodDisruptionBudget;
pub use k8s_openapi::api::rbac::v1::{ClusterRole, ClusterRoleBinding};
pub use k8s_openapi::api::rbac::v1::{Role, RoleBinding};
pub use k8s_openapi::api::scheduling::v1::PriorityClass;
pub use k8s_openapi::api::storage::v1::CSIDriver;
pub use k8s_openapi::api::storage::v1::CSINode;
pub use k8s_openapi::api::storage::v1::{
    CSIStorageCapacity as CSIStorageCapacityV1, StorageClass, VolumeAttachment,
};
pub use k8s_openapi::api::storage::v1beta1::CSIStorageCapacity as CSIStorageCapacityV1beta1;
pub use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
pub use k8s_openapi::kube_aggregator::pkg::apis::apiregistration::v1::APIService;

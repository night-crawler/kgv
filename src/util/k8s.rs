use std::fmt::Debug;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use kube::api::GroupVersionKind;

pub trait GvkStaticExt {
    fn gvk_for_type() -> GroupVersionKind;
}

impl<T> GvkStaticExt for T
    where
        T: Metadata<Ty=ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
{
    fn gvk_for_type() -> GroupVersionKind {
        GroupVersionKind::gvk(T::GROUP, T::VERSION, T::KIND)
    }
}

pub trait GvkExt {
    fn gvk(&self) -> GroupVersionKind;
}

impl<T> GvkExt for T
    where
        T: Metadata<Ty=ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
{
    fn gvk(&self) -> GroupVersionKind {
        GroupVersionKind::gvk(T::GROUP, T::VERSION, T::KIND)
    }
}

pub fn gvk_sort_key(gvk: &GroupVersionKind) -> (String, String, String) {
    (gvk.group.clone(), gvk.version.clone(), gvk.kind.clone())
}

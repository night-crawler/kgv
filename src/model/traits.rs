use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use kube::runtime::reflector::Store;
use kube::api::GroupVersionKind;

use crate::model::resource_view::ResourceView;

pub trait SpecViewAdapter {
    fn items(&self) -> Vec<ResourceView>;
}

#[async_trait]
impl<T> SpecViewAdapter for Store<T>
where
    T: Metadata<Ty = ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
    ResourceView: From<Arc<T>>,
{
    fn items(&self) -> Vec<ResourceView> {
        self.state()
            .iter()
            .map(|resource| Arc::clone(resource).into())
            .collect()
    }
}

pub trait GvkStaticExt {
    fn gvk_for_type() -> GroupVersionKind;
}


pub trait GvkExt {
    fn gvk(&self) -> GroupVersionKind;
}

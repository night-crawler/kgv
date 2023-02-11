use std::fmt::Debug;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Arc;
use cursive::reexports::log::info;
use k8s_openapi::api::core::v1::Pod;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use k8s_openapi::serde::Serialize;
use kube::api::{ApiResource, DynamicObject, GroupVersionKind};
use kube::Resource;
use kube::runtime::reflector::Store;

use crate::model::resource_view::ResourceView;
use crate::model::traits::{GvkExt, GvkStaticExt, MarkerTraitForStaticCases, SpecViewAdapter};
use crate::model::DynamicObjectWrapper;

impl<T> GvkStaticExt for T
where
    T: Metadata<Ty = ObjectMeta>
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

impl<T> GvkExt for T
where
    T: Metadata<Ty = ObjectMeta>
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

impl GvkExt for DynamicObjectWrapper {
    fn gvk(&self) -> GroupVersionKind {
        // &self.types.unwrap().kind;
        info!("! {:?}", self);

        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("/tmp/foo.yaml").unwrap();

        let a = serde_yaml::to_string(&self.0).unwrap();
        file.write_all(a.as_bytes()).unwrap();
        file.write_all(b"\n").unwrap();

        return GroupVersionKind::gvk("", "", "");
    }
}

impl<T> SpecViewAdapter for Store<T>
where
    T: Metadata<Ty = ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>
        + MarkerTraitForStaticCases,
    ResourceView: From<Arc<T>>,
    Store<T>: MarkerTraitForStaticCases
{
    fn items(&self) -> Vec<ResourceView> {
        self.state()
            .iter()
            .map(|resource| Arc::clone(resource).into())
            .collect()
    }
}

impl<T> MarkerTraitForStaticCases for Store<T> where
    T: Metadata<Ty = ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>
{
}


impl SpecViewAdapter for Store<DynamicObject> {
    fn items(&self) -> Vec<ResourceView> {
        self.state()
            .iter()
            .map(|resource| Arc::clone(resource).into())
            .collect()
    }
}

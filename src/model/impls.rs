use std::fmt::Debug;
use std::sync::Arc;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use kube::api::{DynamicObject, GroupVersionKind};
use kube::runtime::reflector::Store;

use crate::model::resource::resource_view::ResourceView;
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
        self.1.clone()
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
    Store<T>: MarkerTraitForStaticCases,
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
            .map(|resource| {
                let wrapper = DynamicObjectWrapper(
                    resource.as_ref().clone(),
                    GroupVersionKind::gvk("", "", ""),
                );
                ResourceView::DynamicObject(Arc::new(wrapper))
            })
            .collect()
    }
}

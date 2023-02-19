use crate::model::resource::resource_view::ResourceView;
use kube::api::GroupVersionKind;

pub trait SpecViewAdapter {
    fn items(&self) -> Vec<ResourceView>;
}

pub trait GvkStaticExt {
    fn gvk_for_type() -> GroupVersionKind;
}

pub trait GvkExt {
    fn gvk(&self) -> GroupVersionKind;
}

pub trait MarkerTraitForStaticCases {}

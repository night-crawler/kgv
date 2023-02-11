use kube::api::GroupVersionKind;

use crate::model::resource_view::ResourceView;

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

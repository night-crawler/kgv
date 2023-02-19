use crate::model::traits::GvkExt;
use kube::api::{ApiResource, DynamicObject, GroupVersionKind};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct DynamicObjectWrapper(pub DynamicObject, pub GroupVersionKind);

impl Default for DynamicObjectWrapper {
    fn default() -> Self {
        let gvk = GroupVersionKind::gvk("", "", "");
        let ar = ApiResource::from_gvk(&gvk);
        Self(DynamicObject::new("", &ar), gvk)
    }
}

impl Deref for DynamicObjectWrapper {
    type Target = DynamicObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl GvkExt for DynamicObjectWrapper {
    fn gvk(&self) -> GroupVersionKind {
        self.1.clone()
    }
}

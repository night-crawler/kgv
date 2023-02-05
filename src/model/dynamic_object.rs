use std::ops::Deref;

use k8s_openapi::serde_json;
use kube::api::{ApiResource, DynamicObject, GroupVersionKind};

use crate::model::traits::SerializeExt;
use crate::traits::ext::gvk::GvkExt;

#[derive(Debug, Clone)]
pub struct DynamicObjectWrapper(pub DynamicObject, pub GroupVersionKind);

impl DynamicObjectWrapper {
    pub fn set_deletion_timestamp(
        &self,
        ts: k8s_openapi::apimachinery::pkg::apis::meta::v1::Time,
    ) -> Self {
        let mut cloned = self.clone();
        let _ = cloned.0.metadata.deletion_timestamp.insert(ts);
        cloned
    }
}

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

impl SerializeExt for DynamicObjectWrapper {
    fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(&self.0)
    }
    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.0)
    }
}

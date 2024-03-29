use std::collections::HashMap;
use std::sync::Arc;

use itertools::Itertools;
use k8s_openapi::serde_json;
use k8s_openapi::serde_json::Value;
use kube::api::GroupVersionKind;

use crate::model::dynamic_object::DynamicObjectWrapper;
use crate::model::resource::resource_view::ResourceView;
use crate::model::traits::SerializeExt;

pub(crate) const PSEUDO_RESOURCE_JOIN_SEQ: &str = "/";

#[derive(Debug, Clone)]
pub(crate) struct PseudoResource {
    pub(crate) id: String,
    pub(crate) extractor_name: String,
    pub(crate) resource: rhai::Dynamic,
    pub(crate) source: ResourceView,
}

impl Default for PseudoResource {
    fn default() -> Self {
        Self {
            id: "".to_string(),
            extractor_name: "".to_string(),
            resource: rhai::Dynamic::UNIT,
            source: ResourceView::DynamicObject(Arc::new(DynamicObjectWrapper::default())),
        }
    }
}

impl PseudoResource {
    pub(crate) fn uid(&self) -> Option<String> {
        let uid = self.source.uid_or_name();
        let parts = [&uid, &self.extractor_name, &self.id];
        Some(parts.iter().join(PSEUDO_RESOURCE_JOIN_SEQ))
    }

    pub(crate) fn name(&self) -> String {
        self.id.clone()
    }

    pub(crate) fn namespace(&self) -> String {
        self.source.namespace()
    }

    pub(crate) fn gvk(&self) -> GroupVersionKind {
        self.source.build_pseudo_gvk(&self.extractor_name)
    }

    pub(crate) fn creation_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.source.creation_timestamp()
    }

    pub(crate) fn deletion_timestamp(&self) -> Option<&chrono::DateTime<chrono::Utc>> {
        self.source.deletion_timestamp()
    }

    pub(crate) fn resource_version(&self) -> Option<String> {
        self.source.resource_version()
    }
}

impl SerializeExt for PseudoResource {
    fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        let s = serde_yaml::to_string(&self.resource)?;
        let mut data: HashMap<&str, Value> = serde_yaml::from_str(&s)?;
        data.remove("__meta");
        serde_yaml::to_string(&data)
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.resource)
    }
}

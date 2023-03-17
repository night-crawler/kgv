use std::collections::HashMap;
use std::sync::Arc;
use cursive::reexports::log::warn;
use itertools::Itertools;

use k8s_openapi::serde_json;
use k8s_openapi::serde_json::Value;
use kube::api::GroupVersionKind;

use crate::model::dynamic_object::DynamicObjectWrapper;
use crate::model::resource::resource_view::ResourceView;
use crate::model::traits::SerializeExt;

pub const PSEUDO_RESOURCE_JOIN_SEQ: &str = "/";

#[derive(Debug)]
pub struct PseudoResource {
    pub id: String,
    pub extractor_name: String,
    pub resource: rhai::Dynamic,
    pub source: ResourceView,
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
    pub fn new(
        id: String,
        extractor_name: String,
        resource: rhai::Dynamic,
        source: ResourceView,
    ) -> Self {
        Self {
            id,
            extractor_name,
            resource,
            source,
        }
    }

    pub fn uid(&self) -> Option<String> {
        let uid = self
            .source
            .uid()
            .unwrap_or_else(|| {
                warn!("Resource without uid: {}", self.source.full_unique_name());
                self.source.full_unique_name()
            });
        let parts = [
            &uid,
            &self.extractor_name,
            &self.id,
        ];
        Some(parts.iter().join(PSEUDO_RESOURCE_JOIN_SEQ))
    }

    pub fn name(&self) -> String {
        self.id.clone()
    }

    pub fn namespace(&self) -> String {
        self.source.namespace()
    }

    pub fn age(&self) -> chrono::Duration {
        self.source.age()
    }

    pub fn gvk(&self) -> GroupVersionKind {
        self.source.to_pseudo_gvk(&self.extractor_name)
    }

    pub fn creation_timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.source.creation_timestamp()
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

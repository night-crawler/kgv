use std::sync::Arc;

use k8s_openapi::serde_json;
use kube::api::GroupVersionKind;
use serde_yaml::Error;

use crate::model::dynamic_object::DynamicObjectWrapper;
use crate::model::resource::resource_view::ResourceView;
use crate::model::traits::SerializeExt;
use crate::traits::ext::gvk::GvkExt;

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
            .unwrap_or_else(|| self.source.full_unique_name());

        format!("{uid}#{}#{}", self.extractor_name, self.id).into()
    }

    pub fn name(&self) -> String {
        self.id.clone()
    }

    pub fn namespace(&self) -> String {
        self.source.namespace().clone()
    }

    pub fn age(&self) -> chrono::Duration {
        self.source.age()
    }

    pub fn gvk(&self) -> GroupVersionKind {
        let mut gvk = self.source.gvk();
        gvk.kind = format!("{}#{}#{}", gvk.kind, self.extractor_name, self.id);
        gvk
    }
}

impl SerializeExt for PseudoResource {
    fn to_yaml(&self) -> Result<String, Error> {
        serde_yaml::to_string(&self.resource)
    }

    fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.resource)
    }
}

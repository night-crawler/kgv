use k8s_openapi::serde_json;

use crate::model::resource::resource_view::ResourceView;

pub trait SpecViewAdapter {
    fn items(&self) -> Vec<ResourceView>;
}

pub trait MarkerTraitForStaticCases {}

pub trait SerializeExt {
    fn to_yaml(&self) -> Result<String, serde_yaml::Error>;
    fn to_json(&self) -> Result<String, serde_json::Error>;
}

use crate::model::resource::resource_view::ResourceView;
use k8s_openapi::serde_json;

pub trait SpecViewAdapter {
    fn items(&self) -> Vec<ResourceView>;
}

pub trait MarkerTraitForStaticCases {}

pub trait SerializeExt {
    fn to_yaml(&self) -> Result<String, serde_yaml::Error>;
    fn to_json(&self) -> Result<String, serde_json::Error>;
}

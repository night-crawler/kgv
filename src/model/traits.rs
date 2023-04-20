use k8s_openapi::serde_json;

pub(crate) trait SerializeExt {
    fn to_yaml(&self) -> Result<String, serde_yaml::Error>;
    fn to_json(&self) -> Result<String, serde_json::Error>;
}

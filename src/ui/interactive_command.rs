use k8s_openapi::api::core::v1::Pod;

#[derive(Debug)]
pub enum InteractiveCommand {
    Exec(Pod, String),
    Logs(Pod, String),
}

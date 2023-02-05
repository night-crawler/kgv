use kube::api::LogParams;

#[derive(Debug, Clone)]
pub struct LogRequest {
    pub id: usize,
    pub namespace: String,
    pub pod_name: String,
    pub log_params: LogParams,
}

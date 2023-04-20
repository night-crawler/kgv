use kube::api::LogParams;

#[derive(Debug, Clone)]
pub(crate) struct LogRequest {
    pub(crate) id: usize,
    pub(crate) namespace: String,
    pub(crate) pod_name: String,
    pub(crate) log_params: LogParams,
}

#[derive(Debug)]
pub(crate) struct PortForwardRequest {
    pub(crate) id: usize,
    pub(crate) namespace: String,
    pub(crate) pod_name: String,
    pub(crate) pod_port: u16,
    pub(crate) host_port: u16,
    pub(crate) host: String,
}

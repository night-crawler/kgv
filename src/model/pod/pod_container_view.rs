use k8s_openapi::api::core::v1::{Container, ContainerStatus};

#[derive(Clone, PartialEq)]
pub struct PodContainerView {
    pub container: Container,
    pub status: Option<ContainerStatus>,
    pub is_init_container: bool,
}

impl PodContainerView {
    pub fn new(
        container: Container,
        status: Option<ContainerStatus>,
        is_init_container: bool,
    ) -> PodContainerView {
        Self {
            container,
            status,
            is_init_container,
        }
    }
}

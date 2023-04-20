use k8s_openapi::api::core::v1::{Container, ContainerStatus};

#[derive(Clone, PartialEq)]
pub(crate) struct PodContainerView {
    pub(crate) container: Container,
    pub(crate) status: Option<ContainerStatus>,
    pub(crate) is_init_container: bool,
}

impl PodContainerView {
    pub(crate) fn new(
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

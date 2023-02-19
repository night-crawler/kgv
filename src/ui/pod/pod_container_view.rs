use k8s_openapi::api::core::v1::Container;

#[derive(Clone, PartialEq)]
pub struct PodContainerView {
    pub container: Container,
}

impl PodContainerView {
    pub fn new(container: Container) -> PodContainerView {
        Self { container }
    }
}

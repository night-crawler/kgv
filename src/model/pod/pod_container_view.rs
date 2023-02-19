use itertools::Itertools;
use k8s_openapi::api::core::v1::{Container, ContainerStatus};

use crate::util::ext::container_port::ContainerPortExt;
use crate::util::ext::container_state::ContainerStateExt;

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

    pub fn get_state_name(&self) -> &str {
        self.status
            .as_ref()
            .and_then(|status| status.state.as_ref())
            .map(|state| state.get_state_name())
            .unwrap_or_default()
    }

    pub fn get_ports_repr(&self) -> String {
        self.container
            .ports
            .as_ref()
            .into_iter()
            .flatten()
            .map(|cp| cp.repr())
            .join(",")
    }
}

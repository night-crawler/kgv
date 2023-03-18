use std::collections::HashMap;

use k8s_openapi::api::core::v1::{ContainerStatus, Pod};

use crate::model::pod::pod_container_view::PodContainerView;

pub trait PodExt {
    fn get_pod_containers(&self) -> Option<Vec<PodContainerView>>;
    fn get_expected_exec_container_name(&self) -> Option<String>;
    fn get_first_container_name(&self) -> Option<String>;
}

impl PodExt for Pod {
    fn get_pod_containers(&self) -> Option<Vec<PodContainerView>> {
        let containers = &self.spec.as_ref()?.containers;
        let statuses: HashMap<&str, &ContainerStatus> = self
            .status
            .as_ref()
            .and_then(|pod_status| pod_status.container_statuses.as_ref())
            .into_iter()
            .flatten()
            .map(|container_status| (container_status.name.as_str(), container_status))
            .collect();

        let mut result = containers
            .iter()
            .map(|container| {
                let status = statuses.get(container.name.as_str()).cloned().cloned();
                PodContainerView::new(container.clone(), status, false)
            })
            .collect::<Vec<_>>();

        for init_container in self
            .spec
            .as_ref()
            .and_then(|s| s.init_containers.as_ref())
            .into_iter()
            .flatten()
        {
            let status = statuses.get(init_container.name.as_str()).cloned().cloned();
            result.push(PodContainerView::new(init_container.clone(), status, true));
        }

        result.into()
    }

    fn get_expected_exec_container_name(&self) -> Option<String> {
        for container in self.get_pod_containers().into_iter().flatten() {
            let container_name = container.container.name.to_lowercase();
            // TODO: filter other
            if container_name.contains("istio") {
                continue;
            }
            return Some(container.container.name);
        }
        None
    }

    fn get_first_container_name(&self) -> Option<String> {
        self.get_pod_containers()
            .into_iter()
            .flatten()
            .next()
            .map(|container| container.container.name)
    }
}

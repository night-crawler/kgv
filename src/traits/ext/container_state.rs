use chrono::{Duration, Utc};
use k8s_openapi::api::core::v1::ContainerState;

pub(crate) trait ContainerStateExt {
    fn get_state_name(&self) -> &str;
    fn get_age(&self) -> Option<Duration>;
}

impl ContainerStateExt for ContainerState {
    fn get_state_name(&self) -> &str {
        if self.terminated.is_some() {
            "Terminated"
        } else if self.running.is_some() {
            "Running"
        } else if self.waiting.is_some() {
            "Waiting"
        } else {
            "Unknown"
        }
    }

    fn get_age(&self) -> Option<Duration> {
        if let Some(r) = self.running.as_ref() {
            return r
                .started_at
                .as_ref()
                .map(|started_at| Utc::now() - started_at.0);
        } else if let Some(r) = self.terminated.as_ref() {
            return r
                .started_at
                .as_ref()
                .map(|started_at| Utc::now() - started_at.0);
        }

        None
    }
}

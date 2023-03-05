use k8s_openapi::api::core::v1::Container;
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;

pub trait ContainerExt {
    fn memory_limit(&self) -> Option<&Quantity>;
    fn memory_request(&self) -> Option<&Quantity>;
    fn memory_rl(&self) -> String;

    fn cpu_limit(&self) -> Option<&Quantity>;
    fn cpu_request(&self) -> Option<&Quantity>;
    fn cpu_rl(&self) -> String;
}

impl ContainerExt for Container {
    fn memory_limit(&self) -> Option<&Quantity> {
        self.resources.as_ref()?.limits.as_ref()?.get("memory")
    }

    fn memory_request(&self) -> Option<&Quantity> {
        self.resources.as_ref()?.requests.as_ref()?.get("memory")
    }

    fn memory_rl(&self) -> String {
        let request = self.memory_request().map(|q| q.0.as_str()).unwrap_or("-");
        let limit = self.memory_limit().map(|q| q.0.as_str()).unwrap_or("-");
        format!("{request}:{limit}")
    }

    fn cpu_limit(&self) -> Option<&Quantity> {
        self.resources.as_ref()?.limits.as_ref()?.get("cpu")
    }

    fn cpu_request(&self) -> Option<&Quantity> {
        self.resources.as_ref()?.requests.as_ref()?.get("cpu")
    }

    fn cpu_rl(&self) -> String {
        let request = self.cpu_request().map(|q| q.0.as_str()).unwrap_or("-");
        let limit = self.cpu_limit().map(|q| q.0.as_str()).unwrap_or("-");
        format!("{request}:{limit}")
    }
}

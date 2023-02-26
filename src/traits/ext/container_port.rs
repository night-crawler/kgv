use itertools::Itertools;
use k8s_openapi::api::core::v1::ContainerPort;

pub trait ContainerPortExt {
    fn ports_host_repr(&self) -> String;
    fn name_port_protocol_repr(&self) -> String;
    fn repr(&self) -> String;
}

impl ContainerPortExt for ContainerPort {
    fn ports_host_repr(&self) -> String {
        let ip = self.host_ip.clone().unwrap_or_default();
        let port = self.host_port.map(|a| a.to_string()).unwrap_or_default();
        [ip, port].iter().filter(|s| !s.is_empty()).join(":")
    }

    fn name_port_protocol_repr(&self) -> String {
        let mut result = String::new();
        if let Some(name) = self.name.as_ref() {
            result.push_str(name);
            result.push(':');
        }
        result.push_str(&self.container_port.to_string());
        if let Some(protocol) = self.protocol.as_ref() {
            result.push('/');
            result.push_str(protocol);
        }
        result
    }

    fn repr(&self) -> String {
        [self.name_port_protocol_repr(), self.ports_host_repr()]
            .iter()
            .filter(|s| !s.is_empty())
            .join("@")
    }
}

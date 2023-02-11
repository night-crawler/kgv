use std::ops::Deref;
use anyhow::Result;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{ApiResource, DynamicObject, GroupVersionKind};
use kube::discovery::{verbs, Scope};
use kube::{Api, Client, Discovery, ResourceExt};

pub mod r#macro;
pub mod reflector_registry;
pub mod resource_column;
pub mod resource_view;
pub mod traits;
pub mod impls;

#[derive(Debug, Clone)]
pub struct DynamicObjectWrapper(DynamicObject, GroupVersionKind);

impl Default for DynamicObjectWrapper {
    fn default() -> Self {
        let gvk = GroupVersionKind::gvk("", "", "");
        let ar = ApiResource::from_gvk(&gvk);
        Self(DynamicObject::new("", &ar), gvk)
    }
}

impl Deref for DynamicObjectWrapper {
    type Target = DynamicObject;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub async fn discover(client: &Client) -> Result<Vec<Pod>> {
    let client = client.clone();

    let discovery = Discovery::new(client.clone()).run().await?;

    for group in discovery.groups() {
        for (api_resource, caps) in group.recommended_resources() {
            if !caps.supports_operation(verbs::LIST) {
                continue;
            }
            let api: Api<DynamicObject> = if caps.scope == Scope::Cluster {
                Api::all_with(client.clone(), &api_resource)
            } else {
                Api::default_namespaced_with(client.clone(), &api_resource)
            };

            println!(
                "{}/{} : {}",
                group.name(),
                api_resource.version,
                api_resource.kind
            );

            let list = api.list(&Default::default()).await?;
            for item in list.items {
                let name = item.name_any();
                let ns = item.metadata.namespace.map(|s| s + "/").unwrap_or_default();
                println!("\t\t{}{}", ns, name);
            }
        }
    }
    Ok(vec![])
}

pub async fn discover_gvk(client: &Client) -> Result<Vec<GroupVersionKind>> {
    let client = client.clone();

    let discovery = Discovery::new(client.clone()).run().await?;

    let mut result = vec![];
    for group in discovery.groups() {
        for (api_resource, caps) in group.recommended_resources() {
            if !caps.supports_operation(verbs::LIST) {
                continue;
            }
            let gvk =
                GroupVersionKind::gvk(group.name(), &api_resource.version, &api_resource.kind);
            result.push(gvk);
        }
    }
    Ok(result)
}

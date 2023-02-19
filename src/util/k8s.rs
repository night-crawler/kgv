use k8s_openapi::api::core::v1::Pod;
use kube::api::{DynamicObject, GroupVersionKind};
use kube::discovery::{verbs, Scope};
use kube::{Api, Client, Discovery, ResourceExt};

pub fn gvk_sort_key(gvk: &GroupVersionKind) -> (String, String, String) {
    (gvk.group.clone(), gvk.version.clone(), gvk.kind.clone())
}

pub async fn discover_gvk(client: Client) -> anyhow::Result<Vec<GroupVersionKind>> {
    let discovery = Discovery::new(client).run().await?;

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

pub async fn discover(client: &Client) -> anyhow::Result<Vec<Pod>> {
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

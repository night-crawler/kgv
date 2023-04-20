use kube::api::GroupVersionKind;
use kube::discovery::verbs;
use kube::{Client, Discovery};

pub(crate) fn gvk_sort_key(gvk: &GroupVersionKind) -> (String, String, String) {
    (gvk.group.clone(), gvk.version.clone(), gvk.kind.clone())
}

pub(crate) async fn discover_gvk(client: Client) -> anyhow::Result<Vec<GroupVersionKind>> {
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

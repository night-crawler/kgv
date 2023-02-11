use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{ConfigMap, Namespace, Pod};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use kanal::AsyncSender;
use kube::api::{DynamicObject, GroupVersionKind, ListParams};
use kube::discovery::{verbs, Scope};
use kube::runtime::reflector::Store;
use kube::runtime::{reflector, watcher, WatchStreamExt};
use kube::{Api, Client, Discovery, ResourceExt};

use crate::util::k8s::GvkStaticExt;
use crate::{mk_filter_enum, ResourceColumn};

pub mod r#macro;

mk_filter_enum!(ResourceView, [
    Namespace: [ResourceColumn::Namespace, ResourceColumn::Name],
    Pod: [ResourceColumn::Namespace, ResourceColumn::Name],
    ConfigMap: [ResourceColumn::Namespace, ResourceColumn::Name]
]);

#[async_trait]
pub trait SpecViewAdapter {
    fn items(&self) -> Vec<ResourceView>;
}

#[async_trait]
impl<T> SpecViewAdapter for Store<T>
where
    T: Metadata<Ty = ObjectMeta>
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
    ResourceView: From<Arc<T>>,
{
    fn items(&self) -> Vec<ResourceView> {
        self.state()
            .iter()
            .map(|resource| Arc::clone(resource).into())
            .collect()
    }
}

impl ResourceView {
    pub fn name(&self) -> String {
        match self {
            ResourceView::Pod(r) => r.name_any(),
            ResourceView::Namespace(r) => r.name_any(),
            ResourceView::ConfigMap(r) => r.name_any(),
        }
    }

    pub fn namespace(&self) -> String {
        match self {
            ResourceView::Pod(r) => r.namespace().unwrap_or_default(),
            ResourceView::Namespace(r) => r.namespace().unwrap_or_default(),
            ResourceView::ConfigMap(r) => r.namespace().unwrap_or_default(),
        }
    }
}

pub struct ReflectorRegistry {
    sender: AsyncSender<ResourceView>,
    client: Client,
    readers_map: HashMap<GroupVersionKind, Box<dyn SpecViewAdapter + Send + Sync>>,
}

impl ReflectorRegistry {
    pub fn new(sender: AsyncSender<ResourceView>, client: &Client) -> Self {
        Self {
            sender,
            client: client.clone(),
            readers_map: HashMap::default(),
        }
    }

    pub async fn register<T>(&mut self)
    where
        T: Metadata<Ty = ObjectMeta>
            + 'static
            + Clone
            + Debug
            + Send
            + Sync
            + for<'de> k8s_openapi::serde::Deserialize<'de>,
        ResourceView: From<Arc<T>>,
    {
        let api: Api<T> = Api::all(self.client.clone());
        let params = ListParams::default();

        let (reader, writer) = reflector::store();
        let rf = reflector(writer, watcher(api, params));
        let sender = self.sender.clone();

        tokio::spawn(async move {
            let mut rfa = rf.applied_objects().boxed();
            while let Ok(Some(resource)) = rfa.try_next().await {
                let _ = sender.send(Arc::new(resource).into()).await;
            }
        });

        self.readers_map.insert(T::gvk_for_type(), Box::new(reader));
    }

    pub fn get_resources(&self, gvk: &GroupVersionKind) -> Option<Vec<ResourceView>> {
        self.readers_map.get(gvk).map(|a| a.items())
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

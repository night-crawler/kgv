use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{Namespace, Pod};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::{Metadata, Resource};
use kanal::AsyncSender;
use kube::api::{DynamicObject, GroupVersionKind, ListParams};
use kube::discovery::{verbs, Scope};
use kube::runtime::reflector::Store;
use kube::runtime::{reflector, watcher, WatchStreamExt};
use kube::{Api, Client, Discovery, ResourceExt};

#[async_trait]
trait SpecViewAdapter {
    fn gvk(&self) -> GroupVersionKind;
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
    fn gvk(&self) -> GroupVersionKind {
        GroupVersionKind::gvk(T::GROUP, T::VERSION, T::KIND)
    }

    fn items(&self) -> Vec<ResourceView> {
        self.state()
            .iter()
            .map(|resource| Arc::clone(resource).into())
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum ResourceView {
    PodView(Arc<Pod>),
    NamespaceView(Arc<Namespace>),
}

impl ResourceView {
    pub fn gvk(&self) -> GroupVersionKind {
        let (group, version, kind) = match self {
            ResourceView::PodView(_) => (Pod::GROUP, Pod::VERSION, Pod::KIND),
            ResourceView::NamespaceView(_) => {
                (Namespace::GROUP, Namespace::VERSION, Namespace::KIND)
            }
        };
        GroupVersionKind::gvk(group, version, kind)
    }

    pub fn name(&self) -> String {
        match self {
            ResourceView::PodView(r) => r.name_any(),
            ResourceView::NamespaceView(r) => r.name_any(),
        }
    }

    pub fn namespace(&self) -> String {
        match self {
            ResourceView::PodView(r) => r.namespace().unwrap_or_default(),
            ResourceView::NamespaceView(r) => r.namespace().unwrap_or_default(),
        }
    }
}

impl From<Arc<Pod>> for ResourceView {
    fn from(resource: Arc<Pod>) -> Self {
        ResourceView::PodView(resource)
    }
}

impl From<Arc<Namespace>> for ResourceView {
    fn from(resource: Arc<Namespace>) -> Self {
        ResourceView::NamespaceView(resource)
    }
}

pub struct ReflectorRegistry {
    sender: AsyncSender<ResourceView>,
    client: Client,
    readers: HashMap<GroupVersionKind, Box<dyn SpecViewAdapter>>,
}

impl ReflectorRegistry {
    pub fn new(sender: AsyncSender<ResourceView>, client: &Client) -> Self {
        Self {
            sender,
            client: client.clone(),
            readers: HashMap::default(),
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

        self.readers.insert(reader.gvk(), Box::new(reader));
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
            let gvk = GroupVersionKind::gvk(group.name(), &api_resource.version, &api_resource.kind);
            result.push(gvk);
        }
    }
    Ok(result)
}

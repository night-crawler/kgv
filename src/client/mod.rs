use std::env;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use futures::{Stream, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{Namespace, Pod};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use kanal::AsyncSender;
use kube::api::{DynamicObject, GroupVersionKind, ListParams};
use kube::discovery::{verbs, Scope};
use kube::runtime::reflector::Store;
use kube::runtime::{reflector, watcher, WatchStreamExt};
use kube::{discovery, Api, Client, Discovery, Resource, ResourceExt};

pub struct ResourceState<T>
where
    T: Metadata<Ty = ObjectMeta> + 'static + Clone + Send + Sync,
{
    reader: Store<T>,
}

#[async_trait]
trait SpecViewAdapter {
    fn kind(&self) -> &'static str;
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
    fn kind(&self) -> &'static str {

        T::KIND
    }

    fn items(&self) -> Vec<ResourceView> {
        self.state()
            .iter()
            .map(|resource| Arc::clone(resource).into())
            .collect()
    }
}

#[async_trait]
impl<T> SpecViewAdapter for ResourceState<T>
where
    T: Metadata<Ty = ObjectMeta> + 'static + Clone + Send + Sync,
    ResourceView: From<Arc<T>>,
{
    fn kind(&self) -> &'static str {
        T::KIND
    }

    fn items(&self) -> Vec<ResourceView> {
        self.reader
            .state()
            .iter()
            .map(|resource| Arc::clone(resource).into())
            .collect()
    }
}

#[derive(Debug)]
pub enum ResourceView {
    PodView(Arc<Pod>),
    NamespaceView(Arc<Namespace>),
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

impl<T> ResourceState<T>
where
    T: Metadata<Ty = ObjectMeta>
        // + Resource
        + 'static
        + Clone
        + Debug
        + Send
        + Sync
        + for<'de> k8s_openapi::serde::Deserialize<'de>,
    ResourceView: From<Arc<T>>, // <T as Resource>::DynamicType: Default + Eq + Hash + Clone + Send + Sync,
{
    pub async fn new(client: &Client, sender: &AsyncSender<ResourceView>) -> Self {
        let api: Api<T> = Api::all(client.clone());
        let params = ListParams::default();

        let (reader, writer) = reflector::store();
        let rf = reflector(writer, watcher(api, params));
        let sender = sender.clone();

        tokio::spawn(async move {
            let mut rfa = rf.applied_objects().boxed();
            while let Ok(Some(event)) = rfa.try_next().await {
                let _ = sender.send(Arc::new(event).into()).await;
            }
        });

        Self { reader }
    }
}

pub struct ReflectorRegistry {
    sender: AsyncSender<ResourceView>,
    client: Client,
    reflectors: Vec<Box<dyn SpecViewAdapter>>,
}

impl ReflectorRegistry {
    pub fn new(sender: AsyncSender<ResourceView>, client: Client) -> Self {
        Self {
            sender,
            client,
            reflectors: Vec::default(),
        }
    }

    pub async fn register<T>(&mut self)
    where
        T: Metadata<Ty = ObjectMeta>
            // + Resource
            + 'static
            + Clone
            + Debug
            + Send
            + Sync
            + for<'de> k8s_openapi::serde::Deserialize<'de>,
    {
        // let a1: ResourceState<Pod> = ResourceState::new(&self.client, &self.sender).await;
        let api: Api<T> = Api::all(self.client.clone());
        let params = ListParams::default();

        let (reader, writer) = reflector::store();
        let rf = reflector(writer, watcher(api, params));
        let sender = self.sender.clone();
    }
}

pub async fn bla() -> Result<Vec<Pod>> {
    let client = Client::try_default().await?;
    let (sender, receiver) = kanal::unbounded_async();

    // let a: ResourceState<CustomResourceDefinition> = ResourceState::new(&client, &sender).await;
    let a1: ResourceState<Pod> = ResourceState::new(&client, &sender).await;
    let a2: ResourceState<Namespace> = ResourceState::new(&client, &sender).await;
    let mut qwe: Vec<Arc<dyn SpecViewAdapter>> = vec![];
    qwe.push(Arc::new(a1));
    qwe.push(Arc::new(a2));

    // let a: ResourceState<Pod> = ResourceState::new(&client, &sender).await;

    tokio::time::sleep(Duration::from_secs(5)).await;
    for q in qwe.iter() {
        println!("{}, {:?}", q.kind(), q.items());
    }

    let discovery = Discovery::new(client.clone()).run().await?;
    for group in discovery.groups() {
        for (ar, caps) in group.recommended_resources() {
            if !caps.supports_operation(verbs::LIST) {
                println!("!!! {:?}", ar);
                continue;
            }
            let api: Api<DynamicObject> = if caps.scope == Scope::Cluster {
                Api::all_with(client.clone(), &ar)
            } else {
                Api::default_namespaced_with(client.clone(), &ar)
            };

            println!("{}/{} : {}", group.name(), ar.version, ar.kind);

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

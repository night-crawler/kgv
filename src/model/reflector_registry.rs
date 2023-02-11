use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use kanal::AsyncSender;
use kube::api::{GroupVersionKind, ListParams};
use kube::runtime::{reflector, watcher, WatchStreamExt};
use kube::{Api, Client};

use crate::model::resource_view::ResourceView;
use crate::model::traits::SpecViewAdapter;
use crate::util::k8s::GvkStaticExt;

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

use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use cursive::reexports::log::info;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use kanal::AsyncSender;
use kube::api::{DynamicObject, GroupVersionKind, ListParams};
use kube::runtime::reflector::store::Writer;
use kube::runtime::{reflector, watcher, WatchStreamExt};
use kube::{discovery, Api, Client};

use crate::model::dynamic_object::DynamicObjectWrapper;
use crate::model::resource::resource_view::ResourceView;
use crate::model::traits::{MarkerTraitForStaticCases, SpecViewAdapter};
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::gvk::GvkStaticExt;

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
            + for<'de> k8s_openapi::serde::Deserialize<'de>
            + MarkerTraitForStaticCases,
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

        info!("Registered Resource: {}", T::gvk_for_type().full_name());
    }

    pub async fn register_gvk(&mut self, gvk: GroupVersionKind) -> anyhow::Result<()> {
        let (ar, _caps) = discovery::pinned_kind(&self.client, &gvk).await?;
        let api = Api::<DynamicObject>::all_with(self.client.clone(), &ar);

        let params = ListParams::default();

        let writer = Writer::new(ar);
        let reader = writer.as_reader();

        let rf = reflector(writer, watcher(api, params));

        let sender = self.sender.clone();
        let key = gvk.clone();
        tokio::spawn(async move {
            let mut rfa = rf.applied_objects().boxed();
            while let Ok(Some(resource)) = rfa.try_next().await {
                let wrapper = DynamicObjectWrapper(resource, gvk.clone());
                let view = ResourceView::DynamicObject(Arc::from(wrapper));
                let _ = sender.send(view).await;
            }
        });

        self.readers_map.insert(key.clone(), Box::new(reader));

        info!("Registered GVK: {}", key.full_name());

        Ok(())
    }

    pub fn get_resources(&self, gvk: &GroupVersionKind) -> Option<Vec<ResourceView>> {
        self.readers_map.get(gvk).map(|adapter| {
            let mut items = adapter.items();
            for item in items.iter_mut() {
                if let ResourceView::DynamicObject(wrapper) = item {
                    let DynamicObjectWrapper(dyn_obj, _) = wrapper.as_ref().clone();
                    *wrapper = Arc::new(DynamicObjectWrapper(dyn_obj, gvk.clone()));
                } else {
                    break;
                }
            }
            items
        })
    }
}

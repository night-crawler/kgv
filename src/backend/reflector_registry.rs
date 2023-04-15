use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

use cursive::reexports::log::{error, info, warn};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ObjectMeta, Time};
use k8s_openapi::Metadata;
use kanal::AsyncSender;
use kube::{Api, Client, discovery, Resource, ResourceExt};
use kube::api::{DynamicObject, GroupVersionKind, ListParams};
use kube::runtime::{watcher, WatchStreamExt};
use kube::runtime::watcher::Event;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::model::dynamic_object::DynamicObjectWrapper;
use crate::model::resource::resource_view::ResourceView;
use crate::traits::ext::gvk::GvkNameExt;
use crate::traits::ext::gvk::GvkStaticExt;

pub struct ReflectorRegistry {
    sender: AsyncSender<ResourceView>,
    client: Client,
    handles_map: Arc<RwLock<HashMap<GroupVersionKind, JoinHandle<()>>>>,
}

fn fix_deletion_timestamp<T>(gvk: &GroupVersionKind, event: Event<T>) -> Event<T>
    where
        T: Resource,
{
    match event {
        Event::Deleted(mut obj) => {
            if obj.meta().deletion_timestamp.is_none() {
                warn!(
                    "Patching deletion timestamp in object {}/{}",
                    gvk.full_name(),
                    obj.name_any()
                );
                let _ = obj
                    .meta_mut()
                    .deletion_timestamp
                    .insert(Time(chrono::Utc::now()));
            }

            Event::Deleted(obj)
        }
        event => event,
    }
}

impl ReflectorRegistry {
    pub fn new(sender: AsyncSender<ResourceView>, client: &Client) -> Self {
        Self {
            sender,
            client: client.clone(),
            handles_map: Arc::default(),
        }
    }

    pub async fn register<T>(&mut self)
        where
            T: Metadata<Ty=ObjectMeta>
            + 'static
            + Clone
            + Debug
            + Send
            + Sync
            + for<'de> k8s_openapi::serde::Deserialize<'de>
        ,
            ResourceView: From<Arc<T>>,
    {
        let gvk = T::gvk_for_type();

        if let Some(handle) = self.handles_map.read().await.get(&gvk) {
            if handle.is_finished() {
                warn!("Handle for GVK {} is finished; recreating", gvk.full_name());
            } else {
                warn!("Already registered GVK: {}", gvk.full_name());
                return;
            }
        }

        let api: Api<T> = Api::all(self.client.clone());
        let params = ListParams::default().timeout(1);

        let mut stream = watcher(api, params)
            .map_ok(move |event| fix_deletion_timestamp(&gvk, event))
            .touched_objects()
            .boxed();

        let handles_map = Arc::clone(&self.handles_map);
        let sender = self.sender.clone();
        let handle = tokio::spawn(async move {
            while let Ok(Some(resource)) = stream.try_next().await {
                let _ = sender.send(Arc::new(resource).into()).await;
            }

            error!("Watcher for {} has ended", T::gvk_for_type().full_name());
            handles_map.write().await.remove(&T::gvk_for_type());
        });

        let mut handles_map = self.handles_map.write().await;
        handles_map.insert(T::gvk_for_type(), handle);

        info!(
            "Registered resource reflector: {}",
            T::gvk_for_type().full_name()
        );
    }

    pub async fn register_gvk(&mut self, gvk: GroupVersionKind) -> anyhow::Result<()> {
        if let Some(handle) = self.handles_map.read().await.get(&gvk) {
            if handle.is_finished() {
                warn!("Handle for GVK {} is finished; recreating", gvk.full_name());
            } else {
                warn!("Already registered GVK: {}", gvk.full_name());
                return Ok(());
            }
        }

        let (ar, _caps) = discovery::pinned_kind(&self.client, &gvk).await?;
        let api = Api::<DynamicObject>::all_with(self.client.clone(), &ar);

        let params = ListParams::default();

        let event_gvk = gvk.clone();
        let mut stream = watcher(api, params)
            .map_ok(move |event| fix_deletion_timestamp(&event_gvk, event))
            .touched_objects()
            .boxed();

        let sender = self.sender.clone();
        let key = gvk.clone();
        let handles_map = Arc::clone(&self.handles_map);
        let handle = tokio::spawn(async move {
            while let Ok(Some(resource)) = stream.try_next().await {
                let wrapper = DynamicObjectWrapper(resource, gvk.clone());
                let view = ResourceView::DynamicObject(Arc::new(wrapper));
                let _ = sender.send(view).await;
            }
            error!("Dynamic Object watcher for {} has ended", gvk.full_name());
            handles_map.write().await.remove(&gvk);
        });

        let mut handles_map = self.handles_map.write().await;
        handles_map.insert(key.clone(), handle);

        info!("Registered Dynamic Object resource reflector: {}", key.full_name());

        Ok(())
    }
}

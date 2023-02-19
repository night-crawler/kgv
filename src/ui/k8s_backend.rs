use std::ops::DerefMut;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use cursive::reexports::log::{error, info};
use futures::StreamExt;
use kanal::AsyncReceiver;
use kube::Client;
use tokio::runtime::Runtime;

use crate::model::discover_gvk;
use crate::model::reflector_registry::ReflectorRegistry;
use crate::model::resource_view::{reqister_any_gvk, ResourceView};
use crate::ui::fs_cache::FsCache;
use crate::ui::signals::{ToBackendSignal, ToUiSignal};
use crate::util::panics::ResultExt;

pub struct K8sBackend {
    fs_cache: Arc<futures::lock::Mutex<FsCache>>,
    runtime: Runtime,
    client: Client,
    resource_watcher_receiver: AsyncReceiver<ResourceView>,
    registry: Arc<futures::lock::Mutex<ReflectorRegistry>>,

    from_client_sender: kanal::Sender<ToUiSignal>,
    from_ui_receiver: kanal::Receiver<ToBackendSignal>,
}

impl K8sBackend {
    pub fn new(
        from_client_sender: kanal::Sender<ToUiSignal>,
        from_ui_receiver: kanal::Receiver<ToBackendSignal>,
    ) -> anyhow::Result<Self> {
        let runtime = Self::spawn_runtime(2)?;

        let config = runtime.block_on(async { Self::get_config().await })?;
        info!("Loaded configuration");

        let fs_cache = FsCache::try_from(config.clone())?;
        info!("Created FS Cache");

        let client = runtime.block_on(async move { Client::try_from(config) })?;
        info!("Initialized client");

        let (resource_watcher_sender, resource_watcher_receiver) = kanal::unbounded_async();

        let registry = ReflectorRegistry::new(resource_watcher_sender, &client);

        let instance = Self {
            fs_cache: Arc::new(futures::lock::Mutex::new(fs_cache)),
            runtime,
            client,
            resource_watcher_receiver,
            registry: Arc::new(futures::lock::Mutex::new(registry)),
            from_client_sender,
            from_ui_receiver,
        };

        Ok(instance)
    }

    async fn get_config() -> Result<kube::Config, kube::Error> {
        kube::Config::infer()
            .await
            .map_err(kube::Error::InferConfig)
    }

    fn spawn_runtime(worker_thread: usize) -> std::io::Result<Runtime> {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(worker_thread)
            .thread_name_fn(|| {
                static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                format!("k8s-{}", id)
            })
            .enable_all()
            .build()
    }

    pub fn spawn_discovery_task(&self) {
        let sender = self.from_client_sender.clone_async();
        let client = self.client.clone();
        let fs_cache = Arc::clone(&self.fs_cache);
        self.runtime.spawn(async move {
            if let Some(stored_gvks) = fs_cache.lock().await.get_gvks() {
                info!("Loaded {} GVKs from cache", stored_gvks.len());
                sender
                    .send(ToUiSignal::ResponseDiscoveredGvks(stored_gvks))
                    .await
                    .unwrap_or_log();
            }

            loop {
                info!("Entered GVK discovery loop");

                match discover_gvk(client.clone()).await {
                    Ok(gvks) => {
                        info!("Received {} GVKs", gvks.len());
                        let mut cache = fs_cache.lock().await;
                        cache.set_gvks(&gvks);
                        if let Err(err) = cache.dump() {
                            error!("Failed to save cache: {}", err);
                        }
                        sender
                            .send(ToUiSignal::ResponseDiscoveredGvks(gvks))
                            .await
                            .unwrap_or_log();
                    }
                    Err(err) => {
                        error!("Failed to discover GVKs: {}", err)
                    }
                }
                tokio::time::sleep(std::time::Duration::from_secs(100)).await;
            }
        });
    }

    pub(crate) fn spawn_watcher_exchange_task(&self) {
        let resource_watch_receiver = self.resource_watcher_receiver.clone();
        let ui_signal_sender = self.from_client_sender.clone_async();

        self.runtime.spawn(async move {
            let mut stream = resource_watch_receiver.stream();

            while let Some(resource_view) = stream.next().await {
                ui_signal_sender
                    .send(ToUiSignal::ResponseResourceUpdated(resource_view))
                    .await
                    .unwrap_or_log();
            }
            panic!("Main exchange loop has ended")
        });
    }

    pub fn spawn_from_ui_receiver_task(&mut self) {
        let receiver = self.from_ui_receiver.clone_async();
        let sender = self.from_client_sender.clone_async();
        let registry = Arc::clone(&self.registry);

        self.runtime.spawn(async move {
            let mut stream = receiver.stream();

            while let Some(signal) = stream.next().await {
                let mut reg = registry.lock().await;
                match signal {
                    ToBackendSignal::RequestRegisterGvk(gvk) => {
                        reqister_any_gvk(reg.deref_mut(), gvk).await;
                    }
                    ToBackendSignal::RequestGvkItems(gvk) => {
                        let resources = reg.get_resources(&gvk);
                        let signal = ToUiSignal::ResponseGvkItems(gvk, resources);
                        sender.send(signal).await.unwrap_or_log();
                    }
                    ToBackendSignal::RequestDetails(_) => {}
                }
            }
        });
    }
}

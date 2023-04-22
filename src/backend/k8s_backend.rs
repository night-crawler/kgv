use std::ops::DerefMut;
use std::path::PathBuf;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use cursive::reexports::log::{error, info};
use futures::StreamExt;
use kanal::AsyncReceiver;
use kube::Client;
use tokio::runtime::Runtime;

use crate::backend::fs_cache::FsCache;
use crate::backend::log_manager::LogManager;
use crate::backend::port_forwarder::PortForwarder;
use crate::backend::reflector_registry::ReflectorRegistry;
use crate::backend::remove_manager::RemoveManager;
use crate::model::resource::resource_view::{register_any_gvk, ResourceView};
use crate::traits::ext::kube_config::KubeConfigExt;
use crate::ui::signals::{FromBackendSignal, ToBackendSignal};
use crate::util::k8s::discover_gvk;
use crate::util::panics::ResultExt;

pub(crate) struct K8sBackend {
    fs_cache: Arc<futures::lock::Mutex<FsCache>>,
    runtime: Runtime,
    client: Client,
    resource_watcher_receiver: AsyncReceiver<ResourceView>,
    registry: Arc<futures::lock::Mutex<ReflectorRegistry>>,
    log_manager: Arc<LogManager>,
    remove_manager: Arc<RemoveManager>,
    port_forwarder: Arc<PortForwarder>,

    from_backend_sender: kanal::Sender<FromBackendSignal>,
    from_ui_receiver: kanal::Receiver<ToBackendSignal>,
}

impl K8sBackend {
    pub(crate) fn new(
        from_backend_sender: kanal::Sender<FromBackendSignal>,
        from_ui_receiver: kanal::Receiver<ToBackendSignal>,
        cache_dir: Option<PathBuf>,
        num_backend_threads: usize,
        accept_invalid_certs: bool,
    ) -> anyhow::Result<Self> {
        let runtime = Self::spawn_runtime(num_backend_threads)?;

        let config =
            runtime.block_on(async { Self::get_default_config(accept_invalid_certs).await })?;
        info!("Initialized k8s configuration");

        let fs_cache = FsCache::new(cache_dir, &config.get_cluster_name());
        info!("Created FS Cache");

        let client = runtime.block_on(async move { Client::try_from(config) })?;
        info!("Initialized client");

        let (resource_watcher_sender, resource_watcher_receiver) = kanal::unbounded_async();
        let registry = ReflectorRegistry::new(resource_watcher_sender, &client);

        let remove_manager = RemoveManager::new(&client, from_backend_sender.clone_async());
        let log_manager = LogManager::new(&client, from_backend_sender.clone_async());
        let port_forwarder = PortForwarder::new(&client, from_backend_sender.clone_async());

        let instance = Self {
            fs_cache: Arc::new(futures::lock::Mutex::new(fs_cache)),
            runtime,
            client,
            resource_watcher_receiver,
            registry: Arc::new(futures::lock::Mutex::new(registry)),
            from_backend_sender,
            from_ui_receiver,
            log_manager: Arc::new(log_manager),
            remove_manager: Arc::new(remove_manager),
            port_forwarder: Arc::new(port_forwarder),
        };

        Ok(instance)
    }

    async fn get_default_config(accept_invalid_certs: bool) -> Result<kube::Config, kube::Error> {
        kube::Config::infer()
            .await
            .map(|mut config| {
                config.accept_invalid_certs = accept_invalid_certs;
                config
            })
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

    pub(crate) fn spawn_discovery_task(&self) {
        let sender = self.from_backend_sender.clone_async();
        let client = self.client.clone();
        let fs_cache = Arc::clone(&self.fs_cache);
        self.runtime.spawn(async move {
            if let Some(stored_gvks) = fs_cache.lock().await.get_gvks() {
                info!("Loaded {} GVKs from cache", stored_gvks.len());
                sender
                    .send(FromBackendSignal::DiscoveredGvks(stored_gvks))
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
                            .send(FromBackendSignal::DiscoveredGvks(gvks))
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
        let ui_signal_sender = self.from_backend_sender.clone_async();

        self.runtime.spawn(async move {
            let mut stream = resource_watch_receiver.stream();

            while let Some(resource_view) = stream.next().await {
                ui_signal_sender
                    .send(FromBackendSignal::ResourceUpdated(resource_view))
                    .await
                    .unwrap_or_log();
            }
            panic!("Main exchange loop has ended")
        });
    }

    pub(crate) fn spawn_from_ui_receiver_task(&mut self) {
        let receiver = self.from_ui_receiver.clone_async();
        // let sender = self.to_ui_sender.clone_async();
        let registry = Arc::clone(&self.registry);
        let remove_manager = Arc::clone(&self.remove_manager);
        let log_manager = Arc::clone(&self.log_manager);
        let port_forwarder = Arc::clone(&self.port_forwarder);

        self.runtime.spawn(async move {
            let mut stream = receiver.stream();

            while let Some(signal) = stream.next().await {
                match signal {
                    ToBackendSignal::RegisterGvk(gvk) => {
                        let mut reg = registry.lock().await;
                        register_any_gvk(reg.deref_mut(), gvk).await;
                    }
                    ToBackendSignal::Remove(resource) => {
                        let name = resource.name();
                        if let Err(err) = remove_manager.remove(resource).await {
                            error!("Failed to remove resource {name}: {err}");
                        }
                    }
                    ToBackendSignal::LogsSubscribe(request) => {
                        if let Err(err) = log_manager.subscribe(request).await {
                            error!("Failed to subscribe to logs: {err}");
                        }
                    }
                    ToBackendSignal::LogsUnsubscribe(view_id) => {
                        log_manager.unsubscribe(view_id).await;
                    }
                    ToBackendSignal::PortForward(pf_request) => {
                        if let Err(err) = port_forwarder.forward(pf_request).await {
                            error!("Failed to forward port: {err}");
                        }
                    }
                    ToBackendSignal::StopForwarding(pf_request) => {
                        port_forwarder.stop(pf_request).await;
                    }
                }
            }
        });
    }
}

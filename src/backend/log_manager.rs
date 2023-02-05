use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use cursive::reexports::log::{error, info, warn};
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::model::log_request::LogRequest;
use crate::ui::signals::ToUiSignal;
use crate::util::panics::ResultExt;

type SyncJoinHandleResult = JoinHandle<()>;

pub struct LogManager {
    client: Client,
    handles_map: Arc<RwLock<HashMap<usize, SyncJoinHandleResult>>>,
    requests_map: Arc<RwLock<HashMap<usize, Arc<LogRequest>>>>,
    to_ui_sender: kanal::AsyncSender<ToUiSignal>,
}

impl LogManager {
    pub fn new(client: &Client, to_ui_sender: kanal::AsyncSender<ToUiSignal>) -> Self {
        Self {
            client: client.clone(),
            requests_map: Arc::default(),
            handles_map: Arc::default(),
            to_ui_sender,
        }
    }

    pub async fn subscribe(&self, request: LogRequest) -> anyhow::Result<()> {
        let mut handles_map = self.handles_map.write().await;
        let mut requests_map = self.requests_map.write().await;

        let request = Arc::new(request);
        if let Some(prev_request) = requests_map.insert(request.id, request.clone()) {
            warn!("Log request for with id={} was replaced", prev_request.id);
        }
        if let Some(prev_handle) = handles_map.remove(&request.id) {
            warn!("Log handle for with id={} was replaced", request.id);
            prev_handle.abort();
        }

        let api: Api<Pod> = Api::namespaced(self.client.clone(), &request.namespace);

        let mut stream = api
            .log_stream(&request.pod_name, &request.log_params)
            .await?;

        let handle = {
            let handles_map = Arc::clone(&self.handles_map);
            let requests_map = Arc::clone(&self.requests_map);
            let sender = self.to_ui_sender.clone();
            let request = request.clone();

            let counter = AtomicUsize::new(0);

            tokio::spawn(async move {
                loop {
                    match stream.try_next().await {
                        Ok(Some(bytes)) => {
                            let seq_id = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            sender
                                .send(ToUiSignal::ResponseLogData {
                                    seq_id,
                                    view_id: request.id,
                                    data: bytes.to_vec(),
                                })
                                .await
                                .unwrap_or_log();
                        }
                        Ok(None) => {
                            error!("Log stream for {request:?} has ended");
                            break;
                        }
                        Err(err) => {
                            error!("Error while reading log stream for {request:?}: {err}");
                            break;
                        }
                    }
                }

                let mut handles_map = handles_map.write().await;
                let mut requests_map = requests_map.write().await;

                requests_map.remove(&request.id);
                handles_map.remove(&request.id);
            })
        };

        handles_map.insert(request.id, handle);

        info!("Subscribed to logs for {request:?}");

        Ok(())
    }

    pub async fn unsubscribe(&self, view_id: usize) {
        let mut handles_map = self.handles_map.write().await;
        let mut requests_map = self.requests_map.write().await;

        if let Some(handle) = handles_map.remove(&view_id) {
            handle.abort();
            info!("Aborted log stream for view_id {view_id}");
        }

        if let Some(prev_request) = requests_map.remove(&view_id) {
            info!("Removed previous log subscribe request {prev_request:?}");
        }
    }
}

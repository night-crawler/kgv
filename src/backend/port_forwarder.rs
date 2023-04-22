use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use cursive::reexports::log::{error, info, warn};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
};
use tokio_stream::wrappers::TcpListenerStream;

use crate::model::port_forward_request::PortForwardRequest;
use crate::ui::signals::FromBackendSignal;
use crate::util::error::LogErrorOptionExt;

pub(crate) struct PortForwarder {
    client: Client,
    from_backend_sender: kanal::AsyncSender<FromBackendSignal>,
    handles_map: Arc<RwLock<HashMap<usize, JoinHandle<anyhow::Result<()>>>>>,
}

impl PortForwarder {
    pub(crate) fn new(
        client: &Client,
        from_backend_sender: kanal::AsyncSender<FromBackendSignal>,
    ) -> Self {
        Self {
            client: client.clone(),
            from_backend_sender,
            handles_map: Arc::new(Default::default()),
        }
    }

    pub(crate) async fn stop(&self, pf_request: Arc<PortForwardRequest>) {
        info!("Stopping forwarding: {:?}", pf_request);
        let mut handles_map = self.handles_map.write().await;
        if let Some(handle) = handles_map.remove(&pf_request.id) {
            handle.abort();
            info!("Stopped forwarding: {:?}", pf_request);
        }
    }

    pub(crate) async fn forward(&self, request: Arc<PortForwardRequest>) -> anyhow::Result<()> {
        let cloned_request = Arc::clone(&request);
        let mut handles_map = self.handles_map.write().await;
        info!("Forwarding {:?}", request);

        let api: Api<Pod> = Api::namespaced(self.client.clone(), &request.namespace);
        let addr = SocketAddr::from_str(&format!("{}:{}", request.host, request.host_port))?;

        let map = Arc::clone(&self.handles_map);
        let handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            let server = TcpListenerStream::new(TcpListener::bind(addr).await?)
                .take_until(tokio::signal::ctrl_c())
                .try_for_each(|client_conn| async {
                    if let Ok(peer_addr) = client_conn.peer_addr() {
                        info!(
                            "New connection with {}:{} - {peer_addr}",
                            request.pod_name, request.host_port
                        );
                    }
                    let api = api.clone();
                    let request = Arc::clone(&request);
                    tokio::spawn(async move {
                        if let Err(e) = forward_connection(
                            &api,
                            &request.pod_name,
                            request.pod_port,
                            client_conn,
                        )
                        .await
                        {
                            error!("failed to forward connection: {e}");
                        }
                    });
                    // keep the server running
                    Ok(())
                });

            server.await?;
            map.write().await.remove(&request.id);
            Ok(())
        });

        handles_map.insert(cloned_request.id, handle);
        info!("Started forwarding for {:?}", cloned_request);

        self.from_backend_sender
            .send(FromBackendSignal::PortForwardingStarted(Arc::clone(
                &cloned_request,
            )))
            .await?;

        Ok(())
    }
}

async fn forward_connection(
    pods: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut client_conn: impl AsyncRead + AsyncWrite + Unpin,
) -> anyhow::Result<()> {
    let mut forwarder = pods.portforward(pod_name, &[port]).await?;
    let mut upstream_conn = forwarder
        .take_stream(port)
        .to_log_error(|| format!("For pod {pod_name}:{port} port not found in forwarder"))?;
    tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
    drop(upstream_conn);
    forwarder.join().await?;

    warn!("Connection with pod {pod_name}:{port} closed");
    Ok(())
}

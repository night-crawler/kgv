use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use cursive::reexports::log::{error, info, warn};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};
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
}

impl PortForwarder {
    pub(crate) fn new(
        client: &Client,
        from_backend_sender: kanal::AsyncSender<FromBackendSignal>,
    ) -> Self {
        Self {
            client: client.clone(),
            from_backend_sender,
        }
    }

    pub async fn forward(&self, request: PortForwardRequest) -> anyhow::Result<()> {
        info!("Forwarding {:?}", request);
        let api: Api<Pod> = Api::namespaced(self.client.clone(), &request.namespace);
        let addr = SocketAddr::from_str(&format!("{}:{}", request.host, request.host_port))?;

        let pod_name = Arc::new(request.pod_name);

        let handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
            let server = TcpListenerStream::new(TcpListener::bind(addr).await?)
                .take_until(tokio::signal::ctrl_c())
                .try_for_each(|client_conn| async {
                    if let Ok(peer_addr) = client_conn.peer_addr() {
                        info!(
                            "New connection with {pod_name}:{} - {peer_addr}",
                            request.host_port
                        );
                    }
                    let api = api.clone();
                    let pod_name = Arc::clone(&pod_name);
                    tokio::spawn(async move {
                        if let Err(e) =
                            forward_connection(&api, &pod_name, request.pod_port, client_conn).await
                        {
                            error!("failed to forward connection: {e}");
                        }
                    });
                    // keep the server running
                    Ok(())
                });

            server.await?;
            Ok(())
        });

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

//! Node container management and execution implementing `RuntimeProvider`.

use crate::client::DockerClient;
use crate::container::ContainerManager;
use crate::image::ImageManager;
use crate::network::{NetworkManager, cluster_network_name};
use async_trait::async_trait;
use bollard::container::InspectContainerOptions;
use orchestra_core::cluster::Cluster;
use orchestra_core::error::RuntimeError;
use orchestra_core::node::Node;
use orchestra_core::traits::{NodeRuntimeInfo, RuntimeProvider};
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{debug, info};

/// Implementation of `RuntimeProvider` using Docker Engine.
#[derive(Clone)]
pub struct DockerRuntimeProvider {
    client: DockerClient,
    container_mgr: Arc<ContainerManager>,
    network_mgr: Arc<NetworkManager>,
    image_mgr: Arc<ImageManager>,
}

impl DockerRuntimeProvider {
    /// Creates a new `DockerRuntimeProvider` with the default Docker client.
    pub fn new(client: DockerClient) -> Self {
        Self {
            container_mgr: Arc::new(ContainerManager::new(client.clone())),
            network_mgr: Arc::new(NetworkManager::new(client.clone())),
            image_mgr: Arc::new(ImageManager::new(client.clone())),
            client,
        }
    }

    /// Access the underlying Docker client.
    pub fn client(&self) -> &DockerClient {
        &self.client
    }

    /// Access the container manager.
    pub fn container_manager(&self) -> &ContainerManager {
        &self.container_mgr
    }

    /// Access the network manager.
    pub fn network_manager(&self) -> &NetworkManager {
        &self.network_mgr
    }

    /// Access the image manager.
    pub fn image_manager(&self) -> &ImageManager {
        &self.image_mgr
    }
}

#[async_trait]
impl RuntimeProvider for DockerRuntimeProvider {
    async fn create_node(&self, cluster: &Cluster, node: &Node) -> Result<String, RuntimeError> {
        let cluster_name = cluster.name();
        let network_name = cluster_network_name(cluster_name);

        // 1. Ensure cluster network exists
        self.network_mgr
            .create_cluster_network(cluster_name, "172.20.0.0/16")
            .await?;

        // 2. Ensure node image is available
        let image = &cluster.spec.node_image;
        self.image_mgr.ensure_image(image).await?;

        // 3. Create the node container
        let container_id = self
            .container_mgr
            .create(node, cluster_name, &network_name, image)
            .await?;

        info!(
            node = %node.name,
            role = %node.role,
            container_id = %container_id,
            "Node container provisioned"
        );
        Ok(container_id)
    }

    async fn start_node(&self, container_id: &str) -> Result<(), RuntimeError> {
        self.container_mgr.start(container_id).await
    }

    async fn stop_node(&self, container_id: &str, timeout_secs: u32) -> Result<(), RuntimeError> {
        self.container_mgr.stop(container_id, timeout_secs).await
    }

    async fn delete_node(&self, container_id: &str) -> Result<(), RuntimeError> {
        self.container_mgr.remove(container_id).await
    }

    async fn exec_in_node(
        &self,
        container_id: &str,
        cmd: &[&str],
    ) -> Result<(i64, String), RuntimeError> {
        self.container_mgr.exec(container_id, cmd).await
    }

    async fn inspect_node(&self, container_id: &str) -> Result<NodeRuntimeInfo, RuntimeError> {
        debug!(container_id, "Inspecting node container details");
        let info = self
            .client
            .inner()
            .inspect_container(container_id, None::<InspectContainerOptions>)
            .await
            .map_err(|e| {
                RuntimeError::Container(format!(
                    "Failed to inspect container '{container_id}': {e}"
                ))
            })?;

        let is_running = info.state.as_ref().and_then(|s| s.running).unwrap_or(false);

        let host_pid = info
            .state
            .as_ref()
            .and_then(|s| s.pid)
            .filter(|&pid| pid > 0)
            .map(|pid| pid as u32);

        let ip_address = info
            .network_settings
            .as_ref()
            .and_then(|ns| ns.ip_address.as_ref())
            .filter(|ip| !ip.is_empty())
            .and_then(|ip| Ipv4Addr::from_str(ip).ok());

        let mut port_mappings = Vec::new();
        if let Some(settings) = &info.network_settings {
            if let Some(ports) = &settings.ports {
                for (container_spec, host_bindings) in ports {
                    if let Some(bindings) = host_bindings {
                        let c_port = container_spec
                            .split('/')
                            .next()
                            .and_then(|p| p.parse::<u16>().ok())
                            .unwrap_or(0);

                        for binding in bindings {
                            if let Some(h_port_str) = &binding.host_port {
                                if let Ok(h_port) = h_port_str.parse::<u16>() {
                                    port_mappings.push((c_port, h_port));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(NodeRuntimeInfo {
            container_id: container_id.to_string(),
            is_running,
            host_pid,
            ip_address,
            port_mappings,
        })
    }
}

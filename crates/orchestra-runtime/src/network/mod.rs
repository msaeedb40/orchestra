//! Docker network management for Orchestra cluster node networks.

use crate::client::DockerClient;
use bollard::models::Ipam;
use bollard::network::{CreateNetworkOptions, ListNetworksOptions};
use orchestra_core::error::RuntimeError;
use std::collections::HashMap;
use tracing::{debug, info};

/// Manages the Docker bridge network that interconnects cluster node containers.
pub struct NetworkManager {
    client: DockerClient,
}

impl NetworkManager {
    pub fn new(client: DockerClient) -> Self {
        Self { client }
    }

    /// Finds the first available /16 subnet in the 172.16.0.0/12 private range (172.16 - 172.31).
    pub async fn find_available_subnet(&self) -> Result<String, RuntimeError> {
        let networks = self
            .client
            .inner()
            .list_networks::<String>(None)
            .await
            .map_err(|e| RuntimeError::Network(format!("Failed to list networks: {e}")))?;

        let mut used = std::collections::HashSet::new();
        for net in networks {
            if let Some(ipam) = net.ipam {
                if let Some(configs) = ipam.config {
                    for cfg in configs {
                        if let Some(subnet) = cfg.subnet {
                            if let Some(rest) = subnet.strip_prefix("172.") {
                                if let Some(octet_str) = rest.split('.').next() {
                                    if let Ok(octet) = octet_str.parse::<u8>() {
                                        used.insert(octet);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for octet in 16..=31 {
            if !used.contains(&octet) {
                return Ok(format!("172.{octet}.0.0/16"));
            }
        }

        Err(RuntimeError::Network(
            "No available /16 subnet found in 172.16.0.0 - 172.31.0.0 range".to_string(),
        ))
    }

    /// Creates a dedicated Docker bridge network for the given cluster.
    /// Returns the Docker network ID.
    pub async fn create_cluster_network(
        &self,
        cluster_name: &str,
        subnet: &str,
    ) -> Result<String, RuntimeError> {
        let network_name = cluster_network_name(cluster_name);

        // Idempotency: return existing network ID if already created.
        if let Some(id) = self.find_network(&network_name).await? {
            debug!(network = %network_name, "Cluster network already exists");
            return Ok(id);
        }

        let assigned_subnet = if subnet.is_empty() {
            self.find_available_subnet().await?
        } else {
            subnet.to_string()
        };

        info!(network = %network_name, subnet = %assigned_subnet, "Creating cluster Docker network");

        let mut labels = HashMap::new();
        labels.insert("orchestra.io/cluster".to_string(), cluster_name.to_string());
        labels.insert("orchestra.io/managed".to_string(), "true".to_string());

        let ipam = Ipam {
            config: Some(vec![bollard::models::IpamConfig {
                subnet: Some(assigned_subnet),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let opts = CreateNetworkOptions {
            name: network_name.clone(),
            driver: "bridge".to_string(),
            enable_ipv6: false,
            ipam,
            labels,
            ..Default::default()
        };

        let response = self
            .client
            .inner()
            .create_network(opts)
            .await
            .map_err(|e| {
                RuntimeError::Network(format!("Failed to create network '{network_name}': {e}"))
            })?;

        let id = response.id;
        info!(network = %network_name, %id, "Cluster network created");
        Ok(id)
    }

    /// Removes the Docker bridge network for the given cluster.
    pub async fn remove_cluster_network(&self, cluster_name: &str) -> Result<(), RuntimeError> {
        let network_name = cluster_network_name(cluster_name);

        if let Some(id) = self.find_network(&network_name).await? {
            info!(network = %network_name, "Removing cluster network");
            self.client.inner().remove_network(&id).await.map_err(|e| {
                RuntimeError::Network(format!("Failed to remove network '{network_name}': {e}"))
            })?;
        }
        Ok(())
    }

    /// Finds a Docker network by name and returns its ID if found.
    async fn find_network(&self, name: &str) -> Result<Option<String>, RuntimeError> {
        let mut filters = HashMap::new();
        filters.insert("name".to_string(), vec![name.to_string()]);

        let opts = ListNetworksOptions { filters };

        let networks = self
            .client
            .inner()
            .list_networks(Some(opts))
            .await
            .map_err(|e| RuntimeError::Network(format!("Failed to list networks: {e}")))?;

        Ok(networks
            .into_iter()
            .find(|n| n.name.as_deref() == Some(name))
            .and_then(|n| n.id))
    }
}

/// Generates the Docker network name for a cluster.
pub fn cluster_network_name(cluster_name: &str) -> String {
    format!("orchestra-{cluster_name}")
}

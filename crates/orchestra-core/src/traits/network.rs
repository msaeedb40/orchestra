//! Network interface and bridge provider abstraction trait.

use crate::error::NetworkError;
use crate::network::{NetworkConfig, NetworkInterface};
use async_trait::async_trait;
use orchestra_common::identifiers::NetworkId;

/// Abstract Linux network and interface provider.
#[async_trait]
pub trait NetworkProvider: Send + Sync {
    /// Creates a Linux software bridge with the specified configuration.
    async fn create_bridge(&self, config: &NetworkConfig) -> Result<(), NetworkError>;

    /// Removes a software bridge and cleans up attached interfaces.
    async fn delete_bridge(&self, bridge_name: &str) -> Result<(), NetworkError>;

    /// Connects a container interface (via veth pair) to a bridge network.
    async fn attach_node_to_bridge(
        &self,
        bridge_name: &str,
        node_container_id: &str,
        iface_name: &str,
    ) -> Result<(), NetworkError>;

    /// Configures network interface properties (IP, MTU, UP/DOWN).
    async fn configure_interface(&self, iface: &NetworkInterface) -> Result<(), NetworkError>;

    /// Enumerates network interfaces on the host or inside a container namespace.
    async fn list_interfaces(&self) -> Result<Vec<NetworkInterface>, NetworkError>;

    /// Deletes a managed network.
    async fn delete_network(&self, id: &NetworkId) -> Result<(), NetworkError>;
}

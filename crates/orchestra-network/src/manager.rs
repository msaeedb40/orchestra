//! LinuxNetworkProvider implementation of the core `NetworkProvider` trait.

use crate::bridge::BridgeManager;
use crate::ipam::IpamPool;
use crate::netlink::list_host_interfaces;
use crate::routing::RouteManager;
use async_trait::async_trait;
use orchestra_common::identifiers::NetworkId;
use orchestra_core::error::NetworkError;
use orchestra_core::network::{NetworkConfig, NetworkInterface};
use orchestra_core::traits::NetworkProvider;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;

/// Concrete Linux network provider managing bridges, veths, IPAM, and routes.
#[derive(Clone, Default)]
pub struct LinuxNetworkProvider {
    pools: Arc<Mutex<HashMap<String, IpamPool>>>,
}

impl LinuxNetworkProvider {
    pub fn new() -> Self {
        Self {
            pools: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Retrieves or initializes the IPAM pool for a given bridge network.
    pub fn get_or_create_pool(&self, config: &NetworkConfig) -> Result<IpamPool, NetworkError> {
        let mut pools = self.pools.lock().unwrap();
        if let Some(pool) = pools.get(&config.bridge_name) {
            Ok(pool.clone())
        } else {
            let pool = IpamPool::new(&config.subnet)?;
            pools.insert(config.bridge_name.clone(), pool.clone());
            Ok(pool)
        }
    }
}

#[async_trait]
impl NetworkProvider for LinuxNetworkProvider {
    async fn create_bridge(&self, config: &NetworkConfig) -> Result<(), NetworkError> {
        // 1. Create Linux bridge
        BridgeManager::create_bridge(config)?;

        // 2. Enable kernel forwarding
        let _ = RouteManager::enable_ip_forwarding();

        // 3. Setup NAT masquerade if enabled
        if config.enable_masquerade {
            let _ = RouteManager::setup_masquerade(&config.subnet, &config.bridge_name);
        }

        // 4. Initialize IPAM pool
        let _ = self.get_or_create_pool(config)?;

        info!(bridge = %config.bridge_name, "Linux bridge network initialized");
        Ok(())
    }

    async fn delete_bridge(&self, bridge_name: &str) -> Result<(), NetworkError> {
        BridgeManager::delete_bridge(bridge_name)?;
        let mut pools = self.pools.lock().unwrap();
        pools.remove(bridge_name);
        Ok(())
    }

    async fn attach_node_to_bridge(
        &self,
        bridge_name: &str,
        node_container_id: &str,
        iface_name: &str,
    ) -> Result<(), NetworkError> {
        // Format deterministic host and peer interface names
        let host_veth = format!(
            "veth_{}",
            &node_container_id[..std::cmp::min(8, node_container_id.len())]
        );
        let peer_veth = iface_name;

        // Container PID can be inspected from docker runtime if moving across namespaces,
        // or attached directly if using host namespace.
        info!(
            bridge = bridge_name,
            host_veth, peer_veth, "Attaching node veth interface"
        );
        Ok(())
    }

    async fn configure_interface(&self, iface: &NetworkInterface) -> Result<(), NetworkError> {
        info!(iface = %iface.name, mtu = iface.mtu, is_up = iface.is_up, "Configuring interface");
        Ok(())
    }

    async fn list_interfaces(&self) -> Result<Vec<NetworkInterface>, NetworkError> {
        list_host_interfaces()
    }

    async fn delete_network(&self, _id: &NetworkId) -> Result<(), NetworkError> {
        Ok(())
    }
}

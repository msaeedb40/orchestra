//! Linux network bridge, veth pair, IPAM, and CNI coordination for Orchestra.

pub mod bridge;
pub mod cni;
pub mod ipam;
pub mod manager;
pub mod netlink;
pub mod routing;
pub mod veth;

pub use bridge::BridgeManager;
pub use cni::CniCoordinator;
pub use ipam::IpamPool;
pub use manager::LinuxNetworkProvider;
pub use netlink::list_host_interfaces;
pub use routing::RouteManager;
pub use veth::VethManager;

#[cfg(test)]
mod tests {
    use super::*;
    use orchestra_common::identifiers::NetworkId;
    use orchestra_core::network::NetworkConfig;
    use orchestra_core::traits::NetworkProvider;

    #[tokio::test]
    async fn test_network_provider_initialization() {
        let provider = LinuxNetworkProvider::new();
        let id = NetworkId::generate();
        let config = NetworkConfig::new(id, "orch-unit-br");

        let pool = provider.get_or_create_pool(&config).unwrap();
        assert_eq!(pool.allocated_count(), 1); // gateway

        let ifaces = provider.list_interfaces().await.unwrap();
        assert!(!ifaces.is_empty());
    }
}

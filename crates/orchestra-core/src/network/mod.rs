//! Network domain models, modes, proxy ownership, and CNI plugins.

use orchestra_common::identifiers::NetworkId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::Ipv4Addr;

/// Container and host network attachment topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    /// Isolated Linux software bridge connecting all cluster nodes.
    #[default]
    Bridge,
    /// Nodes share host networking namespaces directly.
    Host,
    /// Encapsulated VXLAN or Geneve overlay network.
    Overlay,
}

impl fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bridge => write!(f, "bridge"),
            Self::Host => write!(f, "host"),
            Self::Overlay => write!(f, "overlay"),
        }
    }
}

/// Owner and implementation of the Kubernetes Service ClusterIP dataplane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceProxyMode {
    /// Orchestra native XDP/TC eBPF service proxy.
    #[default]
    Orchestra,
    /// Selected CNI provider's built-in dataplane (e.g. Cilium kube-proxy replacement).
    Cni,
    /// Standard Kubernetes iptables/ipvs `kube-proxy`.
    KubeProxy,
}

impl fmt::Display for ServiceProxyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Orchestra => write!(f, "orchestra"),
            Self::Cni => write!(f, "cni"),
            Self::KubeProxy => write!(f, "kube-proxy"),
        }
    }
}

/// Container Network Interface (CNI) implementation options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CniPlugin {
    /// Lightweight point-to-point / routing CNI (default for dev).
    #[default]
    Kindnet,
    /// Flannel overlay (VXLAN) CNI.
    Flannel,
    /// Project Calico standard CNI with BGP routing.
    Calico,
    /// Cilium eBPF-native CNI.
    Cilium,
    /// No CNI deployed automatically; BYO CNI.
    None,
}

impl fmt::Display for CniPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kindnet => write!(f, "kindnet"),
            Self::Flannel => write!(f, "flannel"),
            Self::Calico => write!(f, "calico"),
            Self::Cilium => write!(f, "cilium"),
            Self::None => write!(f, "none"),
        }
    }
}

/// Specification for a cluster network bridge and IP address allocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Unique identifier for this network.
    pub id: NetworkId,
    /// Linux bridge interface name (e.g. "orch-br0").
    pub bridge_name: String,
    /// Node subnet CIDR (e.g. "172.30.0.0/16").
    pub subnet: String,
    /// Gateway IP address on the bridge (e.g. "172.30.0.1").
    pub gateway: Ipv4Addr,
    /// Maximum Transmission Unit (MTU), default 1500.
    pub mtu: u32,
    /// Whether IP masquerading (NAT) to the external network is enabled.
    pub enable_masquerade: bool,
}

impl NetworkConfig {
    /// Creates a default bridge network config for a given network ID.
    pub fn new(id: NetworkId, bridge_name: impl Into<String>) -> Self {
        Self {
            id,
            bridge_name: bridge_name.into(),
            subnet: "172.30.0.0/16".to_string(),
            gateway: Ipv4Addr::new(172, 30, 0, 1),
            mtu: 1500,
            enable_masquerade: true,
        }
    }
}

/// Information about a network interface attached to a node or host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name (e.g. "eth0", "veth-node1").
    pub name: String,
    /// Hardware MAC address if applicable.
    pub mac_address: Option<String>,
    /// Assigned IP address.
    pub ip_address: Option<Ipv4Addr>,
    /// Configured MTU.
    pub mtu: u32,
    /// Whether the link is administratively UP.
    pub is_up: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_modes() {
        assert_eq!(NetworkMode::default(), NetworkMode::Bridge);
        assert_eq!(ServiceProxyMode::default(), ServiceProxyMode::Orchestra);
        assert_eq!(CniPlugin::default(), CniPlugin::Kindnet);
    }

    #[test]
    fn test_network_config() {
        let id = NetworkId::generate();
        let cfg = NetworkConfig::new(id, "orch-test-br");
        assert_eq!(cfg.bridge_name, "orch-test-br");
        assert_eq!(cfg.gateway, Ipv4Addr::new(172, 30, 0, 1));
        assert_eq!(cfg.mtu, 1500);
    }
}

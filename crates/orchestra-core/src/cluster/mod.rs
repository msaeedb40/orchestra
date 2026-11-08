//! Cluster domain model and state representations.

pub mod status;

pub use status::{ClusterState, ClusterStatus};

pub use crate::network::{CniPlugin, NetworkMode, ServiceProxyMode};
use orchestra_common::constants::{DEFAULT_NODE_IMAGE, DEFAULT_POD_CIDR, DEFAULT_SERVICE_CIDR};
use orchestra_common::identifiers::ClusterId;
use serde::{Deserialize, Serialize};

/// Desired specification for an Orchestra Kubernetes cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterSpec {
    /// Human-friendly name of the cluster (RFC 1123 compliant).
    pub name: String,
    /// Target Kubernetes version (e.g. "v1.31.2").
    pub kubernetes_version: String,
    /// Number of control plane nodes (typically 1 for dev, 3+ for HA).
    pub control_plane_nodes: usize,
    /// Number of worker nodes.
    pub worker_nodes: usize,
    /// Container runtime node image.
    pub node_image: String,
    /// Network topology mode (Bridge, Direct, Overlay).
    pub network_mode: NetworkMode,
    /// Selected CNI plugin for pod-to-pod networking.
    pub cni_plugin: CniPlugin,
    /// Service dataplane owner (Orchestra eBPF, CNI, kube-proxy).
    pub service_proxy_mode: ServiceProxyMode,
    /// Pod IP network CIDR block.
    pub pod_cidr: String,
    /// Service ClusterIP network CIDR block.
    pub service_cidr: String,
    /// Host port on which the Kubernetes API server is published.
    pub api_server_port: u16,
}

impl ClusterSpec {
    /// Constructs a standard cluster spec with defaults.
    pub fn new(name: impl Into<String>, k8s_version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kubernetes_version: k8s_version.into(),
            control_plane_nodes: 1,
            worker_nodes: 2,
            node_image: DEFAULT_NODE_IMAGE.to_string(),
            network_mode: NetworkMode::Bridge,
            cni_plugin: CniPlugin::OrchestraNet,
            service_proxy_mode: ServiceProxyMode::Orchestra,
            pod_cidr: DEFAULT_POD_CIDR.to_string(),
            service_cidr: DEFAULT_SERVICE_CIDR.to_string(),
            api_server_port: 6443,
        }
    }
}

/// The top-level Cluster domain aggregate root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cluster {
    /// Unique identifier for this cluster.
    pub id: ClusterId,
    /// Desired user specification.
    pub spec: ClusterSpec,
    /// Observed runtime status and lifecycle state.
    pub status: ClusterStatus,
}

impl Cluster {
    /// Creates a new cluster entity from an ID and spec in initial `Pending` state.
    pub fn new(id: ClusterId, spec: ClusterSpec) -> Self {
        let status = ClusterStatus::new(spec.control_plane_nodes, spec.worker_nodes);
        Self { id, spec, status }
    }

    /// Convenience helper to access the cluster name.
    pub fn name(&self) -> &str {
        &self.spec.name
    }

    /// Convenience helper to check if the cluster is ready.
    pub fn is_ready(&self) -> bool {
        self.status.state.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_creation() {
        let spec = ClusterSpec::new("test-cluster", "v1.31.2");
        let cluster = Cluster::new(ClusterId::generate(), spec);

        assert_eq!(cluster.name(), "test-cluster");
        assert_eq!(cluster.status.state, ClusterState::Pending);
        assert_eq!(cluster.status.desired_control_planes, 1);
        assert_eq!(cluster.status.desired_workers, 2);
    }
}

//! Node domain model and state representations.

pub mod status;

pub use status::{NodeState, NodeStatus};

use orchestra_common::identifiers::{ClusterId, NodeId};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Role of a node within the Kubernetes cluster topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    /// Control plane node running etcd, kube-apiserver, kube-controller-manager, kube-scheduler.
    ControlPlane,
    /// Worker node running workloads.
    #[default]
    Worker,
}

impl NodeRole {
    /// Checks if this node is a control plane node.
    pub fn is_control_plane(&self) -> bool {
        matches!(self, NodeRole::ControlPlane)
    }

    /// Checks if this node is a worker node.
    pub fn is_worker(&self) -> bool {
        matches!(self, NodeRole::Worker)
    }
}

impl fmt::Display for NodeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ControlPlane => write!(f, "control-plane"),
            Self::Worker => write!(f, "worker"),
        }
    }
}

/// The Node domain entity representing a single cluster node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier for this node.
    pub id: NodeId,
    /// Identifier of the cluster this node belongs to.
    pub cluster_id: ClusterId,
    /// Container/host name (e.g. "my-cluster-control-plane" or "my-cluster-worker-1").
    pub name: String,
    /// Topology role (ControlPlane or Worker).
    pub role: NodeRole,
    /// Observed runtime status and lifecycle state.
    pub status: NodeStatus,
}

impl Node {
    /// Creates a new node entity in initial `Creating` state.
    pub fn new(id: NodeId, cluster_id: ClusterId, name: impl Into<String>, role: NodeRole) -> Self {
        Self {
            id,
            cluster_id,
            name: name.into(),
            role,
            status: NodeStatus::new(),
        }
    }

    /// Checks if the node is Ready and healthy.
    pub fn is_ready(&self) -> bool {
        self.status.state.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node_id = NodeId::generate();
        let cluster_id = ClusterId::generate();
        let node = Node::new(node_id, cluster_id, "test-cp-0", NodeRole::ControlPlane);

        assert_eq!(node.name, "test-cp-0");
        assert!(node.role.is_control_plane());
        assert_eq!(node.status.state, NodeState::Creating);
    }
}

//! Finite state machine and status tracking for cluster nodes.

use crate::error::NodeError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::Ipv4Addr;

/// Finite lifecycle states of an individual cluster node container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NodeState {
    /// Docker container is being created.
    #[default]
    Creating,
    /// Container process has started, systemd/containerd booting.
    Starting,
    /// Kubeadm init/join running, waiting for kubelet and node registration.
    Initializing,
    /// Kubelet is communicating, CNI is up, node reports Ready in Kubernetes.
    Ready,
    /// Node heartbeats stopped or kubelet reported NotReady.
    NotReady,
    /// Node is being drained and stopped.
    Stopping,
    /// Container has stopped gracefully.
    Stopped,
    /// Container exited unexpectedly or failed health check.
    Failed,
}

impl NodeState {
    /// Validates whether transitioning from `self` to `target` is permitted.
    pub fn can_transition_to(&self, target: NodeState) -> bool {
        use NodeState::*;
        match (self, target) {
            // Self transitions are idempotent
            (s, t) if *s == t => true,

            // Any state can transition to Failed
            (_, Failed) => true,

            // Normal startup progression
            (Creating, Starting) => true,
            (Starting, Initializing) => true,
            (Initializing, Ready) => true,

            // Readiness oscillations
            (Ready, NotReady) => true,
            (NotReady, Ready) => true,

            // Shutdown progression
            (Ready, Stopping) | (NotReady, Stopping) | (Starting, Stopping) => true,
            (Stopping, Stopped) => true,

            // Restart progression from Stopped
            (Stopped, Starting) => true,

            // Retry from Failed
            (Failed, Starting) => true,

            _ => false,
        }
    }

    /// Checks if the node is considered ready to schedule pods.
    pub fn is_ready(&self) -> bool {
        matches!(self, NodeState::Ready)
    }

    /// Checks if the node container is running.
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            NodeState::Starting | NodeState::Initializing | NodeState::Ready | NodeState::NotReady
        )
    }
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Creating => write!(f, "Creating"),
            Self::Starting => write!(f, "Starting"),
            Self::Initializing => write!(f, "Initializing"),
            Self::Ready => write!(f, "Ready"),
            Self::NotReady => write!(f, "NotReady"),
            Self::Stopping => write!(f, "Stopping"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Observed runtime status of a cluster node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStatus {
    /// Current state machine phase.
    pub state: NodeState,
    /// Docker/container runtime container ID.
    pub container_id: Option<String>,
    /// Assigned internal IP address on the bridge network.
    pub internal_ip: Option<Ipv4Addr>,
    /// Status or failure explanation message.
    pub message: Option<String>,
    /// Timestamp when node creation started.
    pub created_at: DateTime<Utc>,
    /// Timestamp of most recent status update.
    pub updated_at: DateTime<Utc>,
}

impl NodeStatus {
    /// Creates a new `NodeStatus` in `Creating` state.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            state: NodeState::Creating,
            container_id: None,
            internal_ip: None,
            message: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Transitions the node state with strict validation.
    pub fn transition_to(
        &mut self,
        target: NodeState,
        message: Option<String>,
    ) -> Result<(), NodeError> {
        if !self.state.can_transition_to(target) {
            return Err(NodeError::InvalidStateTransition {
                from: self.state.to_string(),
                to: target.to_string(),
                reason: format!("Illegal node transition from {} to {}", self.state, target),
            });
        }
        self.state = target;
        self.message = message;
        self.updated_at = Utc::now();
        Ok(())
    }
}

impl Default for NodeStatus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_state_transitions() {
        let mut status = NodeStatus::new();
        assert_eq!(status.state, NodeState::Creating);

        status.transition_to(NodeState::Starting, None).unwrap();
        assert_eq!(status.state, NodeState::Starting);

        status.transition_to(NodeState::Initializing, None).unwrap();
        status.transition_to(NodeState::Ready, None).unwrap();
        assert!(status.state.is_ready());

        status
            .transition_to(NodeState::NotReady, Some("Kubelet unhealthy".into()))
            .unwrap();
        assert!(!status.state.is_ready());

        status.transition_to(NodeState::Ready, None).unwrap();
        assert!(status.state.is_ready());

        status.transition_to(NodeState::Stopping, None).unwrap();
        status.transition_to(NodeState::Stopped, None).unwrap();
    }

    #[test]
    fn test_invalid_node_transition() {
        let mut status = NodeStatus::new();
        let err = status.transition_to(NodeState::Ready, None).unwrap_err();
        assert!(matches!(err, NodeError::InvalidStateTransition { .. }));
    }
}

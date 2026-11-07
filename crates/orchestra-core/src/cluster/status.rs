//! Finite state machine and status tracking for clusters.

use crate::error::ClusterError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Finite lifecycle states of a Kubernetes cluster managed by Orchestra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClusterState {
    /// Desired cluster spec registered, awaiting provisioning.
    #[default]
    Pending,
    /// Network and node containers are being created.
    Creating,
    /// Kubeadm init/join and control plane bootstrap in progress.
    Bootstrapping,
    /// All control plane and worker nodes are Ready and operational.
    Running,
    /// Cluster nodes are being cleanly shut down.
    Stopping,
    /// All cluster containers are stopped; state is preserved on disk.
    Stopped,
    /// Cluster containers and networks are being torn down.
    Deleting,
    /// Resources have been completely removed.
    Deleted,
    /// An unrecoverable error occurred during provisioning or reconciliation.
    Failed,
}

impl ClusterState {
    /// Validates whether transitioning from `self` to `target` is legally permitted.
    pub fn can_transition_to(&self, target: ClusterState) -> bool {
        use ClusterState::*;
        match (self, target) {
            // Self transitions are idempotent no-ops
            (s, t) if *s == t => true,

            // Any active or errored state can transition to Failed
            (_, Failed) => true,

            // Any state except Deleted can transition to Deleting
            (s, Deleting) if *s != Deleted => true,

            // Standard lifecycle flow
            (Pending, Creating) => true,
            (Creating, Bootstrapping) => true,
            (Bootstrapping, Running) => true,

            // Runtime operations
            (Running, Stopping) => true,
            (Stopping, Stopped) => true,
            (Stopped, Creating) | (Stopped, Bootstrapping) => true, // Restarting

            // Deletion completion
            (Deleting, Deleted) => true,

            // Recovery from Failed state
            (Failed, Creating) | (Failed, Bootstrapping) => true,

            // All other transitions are invalid
            _ => false,
        }
    }

    /// Checks if the cluster is considered healthy and accepting workloads.
    pub fn is_ready(&self) -> bool {
        matches!(self, ClusterState::Running)
    }

    /// Checks if the cluster is in a terminal state (Deleted).
    pub fn is_terminal(&self) -> bool {
        matches!(self, ClusterState::Deleted)
    }

    /// Checks if the cluster is currently active or transitioning.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ClusterState::Creating | ClusterState::Bootstrapping | ClusterState::Running
        )
    }
}

impl fmt::Display for ClusterState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::Creating => write!(f, "Creating"),
            Self::Bootstrapping => write!(f, "Bootstrapping"),
            Self::Running => write!(f, "Running"),
            Self::Stopping => write!(f, "Stopping"),
            Self::Stopped => write!(f, "Stopped"),
            Self::Deleting => write!(f, "Deleting"),
            Self::Deleted => write!(f, "Deleted"),
            Self::Failed => write!(f, "Failed"),
        }
    }
}

/// Observed runtime status of a cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterStatus {
    /// Current state machine phase.
    pub state: ClusterState,
    /// Human-readable explanation of current status or failure cause.
    pub message: Option<String>,
    /// Number of desired control plane nodes.
    pub desired_control_planes: usize,
    /// Number of currently ready control plane nodes.
    pub ready_control_planes: usize,
    /// Number of desired worker nodes.
    pub desired_workers: usize,
    /// Number of currently ready worker nodes.
    pub ready_workers: usize,
    /// Public/external API endpoint for kubectl access (e.g. "https://127.0.0.1:6443").
    pub api_endpoint: Option<String>,
    /// Timestamp when cluster creation began.
    pub created_at: DateTime<Utc>,
    /// Timestamp of most recent status update.
    pub updated_at: DateTime<Utc>,
}

impl ClusterStatus {
    /// Creates a new initial `ClusterStatus` in `Pending` state.
    pub fn new(desired_cp: usize, desired_workers: usize) -> Self {
        let now = Utc::now();
        Self {
            state: ClusterState::Pending,
            message: None,
            desired_control_planes: desired_cp,
            ready_control_planes: 0,
            desired_workers,
            ready_workers: 0,
            api_endpoint: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Transitions the cluster state machine with strict validation.
    pub fn transition_to(
        &mut self,
        target: ClusterState,
        message: Option<String>,
    ) -> Result<(), ClusterError> {
        if !self.state.can_transition_to(target) {
            return Err(ClusterError::InvalidStateTransition {
                from: self.state.to_string(),
                to: target.to_string(),
                reason: format!("Illegal transition from {} to {}", self.state, target),
            });
        }
        self.state = target;
        self.message = message;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_state_transitions() {
        let mut status = ClusterStatus::new(1, 2);
        assert_eq!(status.state, ClusterState::Pending);

        status.transition_to(ClusterState::Creating, None).unwrap();
        assert_eq!(status.state, ClusterState::Creating);

        status
            .transition_to(ClusterState::Bootstrapping, None)
            .unwrap();
        assert_eq!(status.state, ClusterState::Bootstrapping);

        status.transition_to(ClusterState::Running, None).unwrap();
        assert!(status.state.is_ready());

        status.transition_to(ClusterState::Stopping, None).unwrap();
        status.transition_to(ClusterState::Stopped, None).unwrap();
        status.transition_to(ClusterState::Deleting, None).unwrap();
        status.transition_to(ClusterState::Deleted, None).unwrap();
        assert!(status.state.is_terminal());
    }

    #[test]
    fn test_invalid_state_transition() {
        let mut status = ClusterStatus::new(1, 1);
        // Pending cannot jump directly to Running
        let err = status
            .transition_to(ClusterState::Running, None)
            .unwrap_err();
        assert!(matches!(err, ClusterError::InvalidStateTransition { .. }));
    }
}

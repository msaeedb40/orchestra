//! Reconciliation controller abstraction trait.

use crate::error::OrchestraError;
use async_trait::async_trait;
use orchestra_common::identifiers::ClusterId;

/// Result of a single reconciliation iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReconcileResult {
    /// If true, another reconciliation cycle should be scheduled immediately.
    pub requeue: bool,
    /// Delay after which the next cycle should execute (None = default interval).
    pub requeue_after: Option<std::time::Duration>,
}

impl ReconcileResult {
    /// No requeue required; cluster is in desired state.
    pub fn done() -> Self {
        Self {
            requeue: false,
            requeue_after: None,
        }
    }

    /// Requeue immediately.
    pub fn requeue() -> Self {
        Self {
            requeue: true,
            requeue_after: None,
        }
    }

    /// Requeue after specified duration.
    pub fn requeue_after(duration: std::time::Duration) -> Self {
        Self {
            requeue: true,
            requeue_after: Some(duration),
        }
    }
}

/// Abstract controller for continuous state reconciliation.
#[async_trait]
pub trait Controller: Send + Sync {
    /// Human-readable name of this controller (e.g. "ClusterController", "NodeController").
    fn name(&self) -> &str;

    /// Runs one iteration of reconciliation for the specified cluster.
    async fn reconcile(&self, cluster_id: &ClusterId) -> Result<ReconcileResult, OrchestraError>;
}

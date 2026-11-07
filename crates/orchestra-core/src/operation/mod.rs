//! Asynchronous operation tracking, stage progress, and workflow models.

use chrono::{DateTime, Utc};
use orchestra_common::identifiers::{ClusterId, OperationId};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Category of high-level asynchronous workflow.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationType {
    CreateCluster,
    DeleteCluster,
    ScaleCluster { desired_workers: usize },
    DrainNode { node_name: String },
    ReplaceNode { node_name: String },
    UpgradeCluster { target_version: String },
    AttachEbpf,
    DetachEbpf,
    Reconcile,
}

impl fmt::Display for OperationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateCluster => write!(f, "create-cluster"),
            Self::DeleteCluster => write!(f, "delete-cluster"),
            Self::ScaleCluster { desired_workers } => {
                write!(f, "scale-cluster (workers: {})", desired_workers)
            }
            Self::DrainNode { node_name } => write!(f, "drain-node ({})", node_name),
            Self::ReplaceNode { node_name } => write!(f, "replace-node ({})", node_name),
            Self::UpgradeCluster { target_version } => {
                write!(f, "upgrade-cluster ({})", target_version)
            }
            Self::AttachEbpf => write!(f, "attach-ebpf"),
            Self::DetachEbpf => write!(f, "detach-ebpf"),
            Self::Reconcile => write!(f, "reconcile"),
        }
    }
}

/// Lifecycle state of an asynchronous operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OperationStatus {
    /// Operation registered, queued for worker execution.
    #[default]
    Pending,
    /// Workflow currently executing steps.
    InProgress,
    /// Workflow concluded successfully.
    Completed,
    /// Workflow encountered an error and halted or rolled back.
    Failed,
    /// Workflow cancelled by operator request.
    Cancelled,
}

impl OperationStatus {
    /// Returns true if the operation has finished execution.
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            OperationStatus::Completed | OperationStatus::Failed | OperationStatus::Cancelled
        )
    }

    /// Returns true if the operation completed successfully.
    pub fn is_successful(&self) -> bool {
        matches!(self, OperationStatus::Completed)
    }
}

/// Detailed stage within a multi-step operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationStage {
    /// Machine-readable name (e.g. "create_network", "bootstrap_control_plane").
    pub name: String,
    /// Human-readable explanation shown in CLI spinners or logs.
    pub description: String,
    /// Status of this individual stage.
    pub status: OperationStatus,
    /// Timestamp when stage began execution.
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when stage completed.
    pub completed_at: Option<DateTime<Utc>>,
    /// Optional failure message if stage errored.
    pub error: Option<String>,
}

impl OperationStage {
    /// Creates a new pending stage.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            status: OperationStatus::Pending,
            started_at: None,
            completed_at: None,
            error: None,
        }
    }

    /// Marks the stage as InProgress.
    pub fn start(&mut self) {
        self.status = OperationStatus::InProgress;
        self.started_at = Some(Utc::now());
    }

    /// Marks the stage as Completed.
    pub fn complete(&mut self) {
        self.status = OperationStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Marks the stage as Failed with an error message.
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = OperationStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error = Some(error.into());
    }
}

/// Complete record of an asynchronous operation for journal persistence and client polling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    /// Unique identifier for this operation.
    pub id: OperationId,
    /// Target cluster this operation acts upon (if applicable).
    pub cluster_id: Option<ClusterId>,
    /// Type and parameters of the operation.
    pub op_type: OperationType,
    /// Overall lifecycle status.
    pub status: OperationStatus,
    /// Overall progress estimate (0 to 100 percent).
    pub progress_percent: u8,
    /// Ordered sequence of sub-stages.
    pub stages: Vec<OperationStage>,
    /// Index of currently active stage.
    pub current_stage_index: Option<usize>,
    /// Error message if the operation failed.
    pub error_message: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Timestamp of most recent update.
    pub updated_at: DateTime<Utc>,
}

impl Operation {
    /// Constructs a new Operation in `Pending` status.
    pub fn new(
        id: OperationId,
        cluster_id: Option<ClusterId>,
        op_type: OperationType,
        stages: Vec<OperationStage>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            cluster_id,
            op_type,
            status: OperationStatus::Pending,
            progress_percent: 0,
            stages,
            current_stage_index: None,
            error_message: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Advances to the next stage and updates progress percentage.
    pub fn advance_to_stage(&mut self, stage_idx: usize) {
        if stage_idx < self.stages.len() {
            if let Some(prev_idx) = self.current_stage_index {
                if prev_idx < self.stages.len() {
                    self.stages[prev_idx].complete();
                }
            }
            self.current_stage_index = Some(stage_idx);
            self.stages[stage_idx].start();
            self.status = OperationStatus::InProgress;
            self.progress_percent = ((stage_idx as f32 / self.stages.len() as f32) * 100.0) as u8;
            self.updated_at = Utc::now();
        }
    }

    /// Marks the entire operation as completed.
    pub fn mark_completed(&mut self) {
        if let Some(idx) = self.current_stage_index {
            if idx < self.stages.len() {
                self.stages[idx].complete();
            }
        }
        self.status = OperationStatus::Completed;
        self.progress_percent = 100;
        self.updated_at = Utc::now();
    }

    /// Marks the operation as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        let err_str = error.into();
        if let Some(idx) = self.current_stage_index {
            if idx < self.stages.len() {
                self.stages[idx].fail(&err_str);
            }
        }
        self.status = OperationStatus::Failed;
        self.error_message = Some(err_str);
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_workflow() {
        let stages = vec![
            OperationStage::new("net", "Creating network"),
            OperationStage::new("nodes", "Creating node containers"),
            OperationStage::new("k8s", "Bootstrapping Kubernetes"),
        ];

        let mut op = Operation::new(
            OperationId::generate(),
            Some(ClusterId::generate()),
            OperationType::CreateCluster,
            stages,
        );

        assert_eq!(op.status, OperationStatus::Pending);
        assert_eq!(op.progress_percent, 0);

        op.advance_to_stage(0);
        assert_eq!(op.status, OperationStatus::InProgress);

        op.advance_to_stage(1);
        assert_eq!(op.stages[0].status, OperationStatus::Completed);
        assert_eq!(op.stages[1].status, OperationStatus::InProgress);

        op.mark_completed();
        assert_eq!(op.status, OperationStatus::Completed);
        assert_eq!(op.progress_percent, 100);
    }
}

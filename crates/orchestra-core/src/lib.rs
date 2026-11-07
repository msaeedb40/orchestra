//! Core domain models, state machines, abstractions, and errors for Orchestra.
//!
//! Defines infrastructure-independent representations of clusters, nodes, networks,
//! operations, state transitions, events, and provider interfaces.

pub mod cluster;
pub mod error;
pub mod events;
pub mod network;
pub mod node;
pub mod operation;
pub mod traits;

// Top-level re-exports for clean workspace integration
pub use cluster::{Cluster, ClusterSpec, ClusterState, ClusterStatus};
pub use error::{
    ClusterError, ConfigError, EbpfError, KernelError, KubernetesError, NetworkError, NodeError,
    OperationError, OrchestraError, RuntimeError, StorageError,
};
pub use events::{
    ClusterEvent, DomainEvent, EbpfEvent, EventPayload, NetworkEvent, NodeEvent, SystemEvent,
};
pub use network::{CniPlugin, NetworkConfig, NetworkInterface, NetworkMode, ServiceProxyMode};
pub use node::{Node, NodeRole, NodeState, NodeStatus};
pub use operation::{Operation, OperationStage, OperationStatus, OperationType};
pub use traits::{
    Controller, NetworkProvider, NodeRuntimeInfo, ReconcileResult, RuntimeProvider, StorageProvider,
};

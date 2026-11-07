//! Comprehensive error hierarchy for Orchestra.
//!
//! Provides granular, strongly-typed errors for domain subsystems
//! using `thiserror`.

use orchestra_common::identifiers::{ClusterId, NetworkId, NodeId, OperationId};
use thiserror::Error;

/// Top-level error enum covering all failure modes across the Orchestra platform.
#[derive(Debug, Error)]
pub enum OrchestraError {
    #[error("Cluster error: {0}")]
    Cluster(#[from] ClusterError),

    #[error("Node error: {0}")]
    Node(#[from] NodeError),

    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),

    #[error("Kubernetes error: {0}")]
    Kubernetes(#[from] KubernetesError),

    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("eBPF error: {0}")]
    Ebpf(#[from] EbpfError),

    #[error("Kernel error: {0}")]
    Kernel(#[from] KernelError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigError),

    #[error("Operation error: {0}")]
    Operation(#[from] OperationError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Errors related to cluster lifecycle, configuration, and provisioning.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ClusterError {
    #[error("Cluster not found: {0}")]
    NotFound(ClusterId),

    #[error("Cluster with name '{0}' already exists")]
    AlreadyExists(String),

    #[error("Invalid cluster state transition from '{from}' to '{to}': {reason}")]
    InvalidStateTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Invalid cluster specification: {0}")]
    InvalidSpec(String),

    #[error("Cluster bootstrap failed: {0}")]
    BootstrapFailed(String),

    #[error("Cluster has not initialized a control plane")]
    NoControlPlane,

    #[error("Cluster deletion timed out")]
    DeletionTimeout,
}

/// Errors related to individual node containers and roles.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NodeError {
    #[error("Node not found: {0}")]
    NotFound(NodeId),

    #[error("Node with name '{0}' already exists")]
    AlreadyExists(String),

    #[error("Invalid node state transition from '{from}' to '{to}': {reason}")]
    InvalidStateTransition {
        from: String,
        to: String,
        reason: String,
    },

    #[error("Node container failed: {0}")]
    ContainerFailed(String),

    #[error("Node health check timed out for node '{0}'")]
    HealthCheckTimeout(NodeId),

    #[error("Node is not in a schedulable state: {0}")]
    NotSchedulable(String),
}

/// Errors originating from the container runtime (Docker / containerd).
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("Container runtime daemon is unavailable: {0}")]
    RuntimeUnavailable(String),

    #[error("Container runtime connection error: {0}")]
    Connection(String),

    #[error("Container error: {0}")]
    Container(String),

    #[error("Container not found: {0}")]
    ContainerNotFound(String),

    #[error("Container creation failed: {0}")]
    ContainerCreationFailed(String),

    #[error("Container start failed: {0}")]
    ContainerStartFailed(String),

    #[error("Command execution error: {0}")]
    Exec(String),

    #[error(
        "Command execution failed in container '{container_id}': exit code {exit_code}, error: {output}"
    )]
    ExecFailed {
        container_id: String,
        command: String,
        exit_code: i64,
        output: String,
    },

    #[error("Container runtime operation timed out: {0}")]
    Timeout(String),

    #[error("Image pull error: {0}")]
    ImagePull(String),

    #[error("Image pull failed: {0}")]
    ImagePullFailed(String),

    #[error("Container network error: {0}")]
    Network(String),
}

/// Errors related to Kubernetes API, kubeadm, and cluster bootstrapping.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KubernetesError {
    #[error("Kubernetes API server unavailable: {0}")]
    ApiUnavailable(String),

    #[error("Kubeconfig not found or invalid: {0}")]
    KubeconfigError(String),

    #[error("Kubeadm init failed: {0}")]
    KubeadmInitFailed(String),

    #[error("Kubeadm join failed for node '{node_id}': {message}")]
    KubeadmJoinFailed { node_id: String, message: String },

    #[error("Node '{node_id}' failed to report Ready within deadline")]
    NodeNotReady { node_id: String },

    #[error("Failed to apply Kubernetes manifest: {0}")]
    ManifestApplyFailed(String),
}

/// Errors related to Linux networking, bridges, veth pairs, and IPAM.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NetworkError {
    #[error("Network not found: {0}")]
    NotFound(NetworkId),

    #[error("Bridge creation failed: {0}")]
    BridgeCreationFailed(String),

    #[error("veth pair creation failed: {0}")]
    VethCreationFailed(String),

    #[error("Netlink interface operation failed for '{iface}': {message}")]
    NetlinkFailed { iface: String, message: String },

    #[error("IP allocation failed: subnet exhausted or range collision")]
    IpAllocationFailed(String),

    #[error("CNI plugin installation failed: {0}")]
    CniPluginFailed(String),

    #[error("Invalid CIDR: {0}")]
    InvalidCidr(String),
}

/// Errors related to eBPF program lifecycle, loading, maps, and linking.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EbpfError {
    #[error("Failed to load eBPF program '{program_name}': {reason}")]
    ProgramLoadFailed {
        program_name: String,
        reason: String,
    },

    #[error("Failed to create/access eBPF map '{map_name}': {reason}")]
    MapError { map_name: String, reason: String },

    #[error("Failed to attach eBPF program to link target '{target}': {reason}")]
    AttachmentFailed { target: String, reason: String },

    #[error("BPF filesystem pin path unavailable: {0}")]
    PinPathError(String),

    #[error("Host kernel does not meet eBPF requirements: {0}")]
    KernelNotSupported(String),
}

/// Errors related to host Linux kernel capabilities and sysctl.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KernelError {
    #[error("Failed to configure sysctl '{key}': {message}")]
    SysctlFailed { key: String, message: String },

    #[error("Failed to mount filesystem: {0}")]
    MountFailed(String),

    #[error("Permission denied: required capability {0} missing")]
    MissingCapability(String),
}

/// Errors related to embedded SQLite database and persistence.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization / Deserialization error: {0}")]
    Serialization(String),

    #[error("Database migration failed: {0}")]
    MigrationFailed(String),

    #[error("Record not found in database: {0}")]
    RecordNotFound(String),
}

/// Errors related to configuration parsing and schema validation.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConfigError {
    #[error("Config parse error: {0}")]
    ParseError(String),

    #[error("Config validation error: {0}")]
    ValidationError(String),

    #[error("Config file not found: {0}")]
    FileNotFound(String),
}

/// Errors related to asynchronous multi-stage operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OperationError {
    #[error("Operation not found: {0}")]
    NotFound(OperationId),

    #[error("Operation '{0}' was cancelled")]
    Cancelled(OperationId),

    #[error("Operation '{0}' timed out")]
    Timeout(OperationId),

    #[error("Invalid operation transition: {0}")]
    InvalidTransition(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversions() {
        let cluster_err = ClusterError::NotFound(ClusterId::new("clus-123"));
        let top_err: OrchestraError = cluster_err.into();
        assert!(
            top_err
                .to_string()
                .contains("Cluster error: Cluster not found: clus-123")
        );

        let net_err = NetworkError::IpAllocationFailed("10.244.0.0/16 exhausted".to_string());
        let top_net: OrchestraError = net_err.into();
        assert!(top_net.to_string().contains("Network error"));
    }
}

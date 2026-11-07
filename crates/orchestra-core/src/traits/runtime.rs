//! Container runtime provider abstraction trait.

use crate::cluster::Cluster;
use crate::error::RuntimeError;
use crate::node::Node;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

/// Low-level runtime status information for an inspected node container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRuntimeInfo {
    /// Container ID assigned by the container engine.
    pub container_id: String,
    /// Whether the container process is currently running.
    pub is_running: bool,
    /// PID of the container root process on the host.
    pub host_pid: Option<u32>,
    /// Assigned IP address on the cluster network.
    pub ip_address: Option<Ipv4Addr>,
    /// Port mappings from container to host.
    pub port_mappings: Vec<(u16, u16)>, // (container_port, host_port)
}

/// Abstract container runtime provider (e.g. Docker Engine via Bollard).
#[async_trait]
pub trait RuntimeProvider: Send + Sync {
    /// Provisions and creates a new node container (without starting it).
    async fn create_node(&self, cluster: &Cluster, node: &Node) -> Result<String, RuntimeError>;

    /// Starts a created or stopped container.
    async fn start_node(&self, container_id: &str) -> Result<(), RuntimeError>;

    /// Gracefully stops a running container within the specified timeout.
    async fn stop_node(&self, container_id: &str, timeout_secs: u32) -> Result<(), RuntimeError>;

    /// Destroys and removes a container and its attached ephemeral volumes.
    async fn delete_node(&self, container_id: &str) -> Result<(), RuntimeError>;

    /// Executes a command inside the container and returns (exit_code, stdout/stderr output).
    async fn exec_in_node(
        &self,
        container_id: &str,
        cmd: &[&str],
    ) -> Result<(i64, String), RuntimeError>;

    /// Inspects container runtime details.
    async fn inspect_node(&self, container_id: &str) -> Result<NodeRuntimeInfo, RuntimeError>;
}

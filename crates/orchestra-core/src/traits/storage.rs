//! Persistence and storage provider abstraction trait.

use crate::cluster::Cluster;
use crate::error::StorageError;
use crate::node::Node;
use crate::operation::Operation;
use async_trait::async_trait;
use orchestra_common::identifiers::{ClusterId, NodeId, OperationId};

/// Abstract persistence layer (SQLite, memory).
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Inserts or updates a cluster record.
    async fn save_cluster(&self, cluster: &Cluster) -> Result<(), StorageError>;

    /// Fetches a cluster record by its ID.
    async fn get_cluster(&self, id: &ClusterId) -> Result<Option<Cluster>, StorageError>;

    /// Fetches a cluster record by its unique name.
    async fn get_cluster_by_name(&self, name: &str) -> Result<Option<Cluster>, StorageError>;

    /// Lists all cluster records.
    async fn list_clusters(&self) -> Result<Vec<Cluster>, StorageError>;

    /// Deletes a cluster and its associated child records.
    async fn delete_cluster(&self, id: &ClusterId) -> Result<(), StorageError>;

    /// Inserts or updates a node record.
    async fn save_node(&self, node: &Node) -> Result<(), StorageError>;

    /// Fetches a node by ID.
    async fn get_node(&self, id: &NodeId) -> Result<Option<Node>, StorageError>;

    /// Lists all nodes belonging to a specific cluster.
    async fn list_nodes(&self, cluster_id: &ClusterId) -> Result<Vec<Node>, StorageError>;

    /// Deletes a node record.
    async fn delete_node(&self, id: &NodeId) -> Result<(), StorageError>;

    /// Inserts or updates an operation journal entry.
    async fn save_operation(&self, op: &Operation) -> Result<(), StorageError>;

    /// Fetches an operation journal entry by ID.
    async fn get_operation(&self, id: &OperationId) -> Result<Option<Operation>, StorageError>;

    /// Lists all operations for a cluster.
    async fn list_operations(&self, cluster_id: &ClusterId)
    -> Result<Vec<Operation>, StorageError>;
}

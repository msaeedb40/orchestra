//! Repository implementations and `StorageProvider` trait adherence.

pub mod checkpoint;
pub mod cluster;
pub mod event;
pub mod node;
pub mod operation;

pub use checkpoint::{Checkpoint, acquire_lock, get_checkpoint, release_lock, save_checkpoint};
pub use cluster::{delete_cluster, get_cluster, get_cluster_by_name, list_clusters, save_cluster};
pub use event::{list_events, record_event};
pub use node::{delete_node, get_node, list_nodes, save_node};
pub use operation::{get_operation, list_operations, save_operation};

use crate::database::Database;
use async_trait::async_trait;
use orchestra_common::identifiers::{ClusterId, NodeId, OperationId};
use orchestra_core::cluster::Cluster;
use orchestra_core::error::StorageError;
use orchestra_core::node::Node;
use orchestra_core::operation::Operation;
use orchestra_core::traits::StorageProvider;

#[async_trait]
impl StorageProvider for Database {
    async fn save_cluster(&self, cluster: &Cluster) -> Result<(), StorageError> {
        cluster::save_cluster(self, cluster).await
    }

    async fn get_cluster(&self, id: &ClusterId) -> Result<Option<Cluster>, StorageError> {
        cluster::get_cluster(self, id).await
    }

    async fn get_cluster_by_name(&self, name: &str) -> Result<Option<Cluster>, StorageError> {
        cluster::get_cluster_by_name(self, name).await
    }

    async fn list_clusters(&self) -> Result<Vec<Cluster>, StorageError> {
        cluster::list_clusters(self).await
    }

    async fn delete_cluster(&self, id: &ClusterId) -> Result<(), StorageError> {
        cluster::delete_cluster(self, id).await
    }

    async fn save_node(&self, node: &Node) -> Result<(), StorageError> {
        node::save_node(self, node).await
    }

    async fn get_node(&self, id: &NodeId) -> Result<Option<Node>, StorageError> {
        node::get_node(self, id).await
    }

    async fn list_nodes(&self, cluster_id: &ClusterId) -> Result<Vec<Node>, StorageError> {
        node::list_nodes(self, cluster_id).await
    }

    async fn delete_node(&self, id: &NodeId) -> Result<(), StorageError> {
        node::delete_node(self, id).await
    }

    async fn save_operation(&self, op: &Operation) -> Result<(), StorageError> {
        operation::save_operation(self, op).await
    }

    async fn get_operation(&self, id: &OperationId) -> Result<Option<Operation>, StorageError> {
        operation::get_operation(self, id).await
    }

    async fn list_operations(
        &self,
        cluster_id: &ClusterId,
    ) -> Result<Vec<Operation>, StorageError> {
        operation::list_operations(self, cluster_id).await
    }
}

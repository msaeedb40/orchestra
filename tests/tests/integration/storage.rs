//! Integration tests for storage persistence and repository operations.

use orchestra_common::identifiers::{ClusterId, NodeId, OperationId};
use orchestra_core::cluster::{Cluster, ClusterSpec, ClusterState};
use orchestra_core::node::{Node, NodeRole, NodeState};
use orchestra_core::operation::{Operation, OperationStatus, OperationType};
use orchestra_storage::database::Database;
use orchestra_storage::repository::cluster::{
    delete_cluster, get_cluster, list_clusters, save_cluster,
};
use orchestra_storage::repository::node::{delete_node, list_nodes, save_node};
use orchestra_storage::repository::operation::{get_operation, save_operation};

#[tokio::test]
async fn test_storage_cascade_and_transactions() {
    let db = Database::open_in_memory().expect("Failed to open database");
    orchestra_storage::run_migrations(&db)
        .await
        .expect("Migrations failed");

    // 1. Create and persist cluster
    let cluster_id = ClusterId::generate();
    let spec = ClusterSpec::new("storage-integ-clus", "v1.36.6");
    let mut cluster = Cluster::new(cluster_id.clone(), spec);
    cluster.status.state = ClusterState::Running;

    save_cluster(&db, &cluster)
        .await
        .expect("Save cluster failed");

    // 2. Persist control plane and worker nodes
    let cp_id = NodeId::generate();
    let mut cp_node = Node::new(
        cp_id.clone(),
        cluster_id.clone(),
        "storage-integ-clus-control-plane",
        NodeRole::ControlPlane,
    );
    cp_node.status.state = NodeState::Ready;
    save_node(&db, &cp_node).await.expect("Save cp node failed");

    let worker_id = NodeId::generate();
    let mut worker_node = Node::new(
        worker_id.clone(),
        cluster_id.clone(),
        "storage-integ-clus-worker-1",
        NodeRole::Worker,
    );
    worker_node.status.state = NodeState::Ready;
    save_node(&db, &worker_node)
        .await
        .expect("Save worker node failed");

    // 3. Query cluster and nodes
    let retrieved_cluster = get_cluster(&db, &cluster_id)
        .await
        .expect("Get cluster failed")
        .expect("Cluster not found");
    assert_eq!(retrieved_cluster.spec.name, "storage-integ-clus");
    assert_eq!(retrieved_cluster.status.state, ClusterState::Running);

    let nodes = list_nodes(&db, &cluster_id)
        .await
        .expect("List nodes failed");
    assert_eq!(nodes.len(), 2);

    // 4. Test operation journal tracking
    let op_id = OperationId::generate();
    let mut op = Operation::new(
        op_id.clone(),
        Some(cluster_id.clone()),
        OperationType::CreateCluster,
        vec![],
    );
    op.status = OperationStatus::Completed;
    save_operation(&db, &op)
        .await
        .expect("Save operation failed");

    let retrieved_op = get_operation(&db, &op_id)
        .await
        .expect("Get operation failed")
        .expect("Operation not found");
    assert_eq!(retrieved_op.status, OperationStatus::Completed);

    // 5. Delete single node then delete cluster
    delete_node(&db, &worker_id)
        .await
        .expect("Delete node failed");
    let nodes_after = list_nodes(&db, &cluster_id)
        .await
        .expect("List nodes failed");
    assert_eq!(nodes_after.len(), 1);

    delete_cluster(&db, &cluster_id)
        .await
        .expect("Delete cluster failed");
    let clusters = list_clusters(&db).await.expect("List clusters failed");
    assert!(clusters.is_empty());
}

use orchestra_common::identifiers::{ClusterId, NodeId, OperationId};
use orchestra_core::cluster::{Cluster, ClusterSpec, ClusterState};
use orchestra_core::events::{ClusterEvent, DomainEvent, EventPayload};
use orchestra_core::node::{Node, NodeRole, NodeState};
use orchestra_core::operation::{Operation, OperationStage, OperationType};
use orchestra_core::traits::StorageProvider;
use orchestra_storage::*;

async fn setup_test_db() -> Database {
    let db = Database::open_in_memory().expect("Failed to open test database");
    run_migrations(&db).await.expect("Failed to run migrations");
    db
}

#[tokio::test]
async fn test_cluster_crud() {
    let db = setup_test_db().await;

    let cluster_id = ClusterId::generate();
    let spec = ClusterSpec::new("test-cluster", "v1.31.2");
    let mut cluster = Cluster::new(cluster_id.clone(), spec);

    // Save
    db.save_cluster(&cluster).await.unwrap();

    // Get by ID
    let fetched = db
        .get_cluster(&cluster_id)
        .await
        .unwrap()
        .expect("Cluster should exist");
    assert_eq!(fetched.name(), "test-cluster");
    assert_eq!(fetched.status.state, ClusterState::Pending);

    // Get by Name
    let by_name = db
        .get_cluster_by_name("test-cluster")
        .await
        .unwrap()
        .expect("Found by name");
    assert_eq!(by_name.id, cluster_id);

    // Update state
    cluster.status.state = ClusterState::Running;
    db.save_cluster(&cluster).await.unwrap();
    let updated = db.get_cluster(&cluster_id).await.unwrap().unwrap();
    assert_eq!(updated.status.state, ClusterState::Running);

    // List
    let clusters = db.list_clusters().await.unwrap();
    assert_eq!(clusters.len(), 1);

    // Delete
    db.delete_cluster(&cluster_id).await.unwrap();
    let after_delete = db.get_cluster(&cluster_id).await.unwrap();
    assert!(after_delete.is_none());
}

#[tokio::test]
async fn test_node_crud_and_cascade() {
    let db = setup_test_db().await;

    let cluster_id = ClusterId::generate();
    let cluster = Cluster::new(
        cluster_id.clone(),
        ClusterSpec::new("cascade-cluster", "v1.31.2"),
    );
    db.save_cluster(&cluster).await.unwrap();

    let node1_id = NodeId::generate();
    let mut node1 = Node::new(
        node1_id.clone(),
        cluster_id.clone(),
        "cp-0",
        NodeRole::ControlPlane,
    );
    db.save_node(&node1).await.unwrap();

    let node2_id = NodeId::generate();
    let node2 = Node::new(
        node2_id.clone(),
        cluster_id.clone(),
        "worker-0",
        NodeRole::Worker,
    );
    db.save_node(&node2).await.unwrap();

    // List nodes for cluster
    let nodes = db.list_nodes(&cluster_id).await.unwrap();
    assert_eq!(nodes.len(), 2);

    // Update node state
    node1.status.state = NodeState::Ready;
    node1.status.container_id = Some("docker-container-123".into());
    db.save_node(&node1).await.unwrap();

    let fetched_node = db.get_node(&node1_id).await.unwrap().unwrap();
    assert_eq!(fetched_node.status.state, NodeState::Ready);
    assert_eq!(
        fetched_node.status.container_id.as_deref(),
        Some("docker-container-123")
    );

    // Cascade delete: deleting cluster must cascade delete nodes
    db.delete_cluster(&cluster_id).await.unwrap();
    let nodes_after = db.list_nodes(&cluster_id).await.unwrap();
    assert_eq!(nodes_after.len(), 0);
}

#[tokio::test]
async fn test_operation_journal() {
    let db = setup_test_db().await;

    // Insert parent cluster so FK constraint on operations(cluster_id) is satisfied.
    let cluster_id = ClusterId::generate();
    let cluster = Cluster::new(
        cluster_id.clone(),
        ClusterSpec::new("op-journal-cluster", "v1.31.2"),
    );
    db.save_cluster(&cluster).await.unwrap();

    let op_id = OperationId::generate();
    let stages = vec![
        OperationStage::new("net", "Creating network"),
        OperationStage::new("nodes", "Creating node containers"),
    ];

    let mut op = Operation::new(
        op_id.clone(),
        Some(cluster_id.clone()),
        OperationType::CreateCluster,
        stages,
    );
    db.save_operation(&op).await.unwrap();

    let fetched = db.get_operation(&op_id).await.unwrap().unwrap();
    assert_eq!(fetched.progress_percent, 0);

    op.advance_to_stage(0);
    db.save_operation(&op).await.unwrap();

    let updated = db.get_operation(&op_id).await.unwrap().unwrap();
    assert_eq!(
        updated.stages[0].status,
        orchestra_core::operation::OperationStatus::InProgress
    );

    let ops = db.list_operations(&cluster_id).await.unwrap();
    assert_eq!(ops.len(), 1);
}

#[tokio::test]
async fn test_event_store_and_locks() {
    let db = setup_test_db().await;
    let cluster_id = ClusterId::generate();

    // Insert parent cluster so FK constraint on domain_events(cluster_id) is satisfied.
    let cluster = Cluster::new(
        cluster_id.clone(),
        ClusterSpec::new("event-store-cluster", "v1.31.2"),
    );
    db.save_cluster(&cluster).await.unwrap();

    // Record domain events
    let event = DomainEvent::new(
        Some(cluster_id.clone()),
        EventPayload::Cluster(ClusterEvent::Created {
            cluster_id: cluster_id.clone(),
            name: "event-cluster".into(),
        }),
    );
    record_event(&db, &event).await.unwrap();

    let events = list_events(&db, Some(&cluster_id), 10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id, event.id);

    // Reconciliation lock testing
    let acquired = acquire_lock(&db, &cluster_id, "worker-token-A")
        .await
        .unwrap();
    assert!(acquired, "Worker A should acquire lock");

    let conflict = acquire_lock(&db, &cluster_id, "worker-token-B")
        .await
        .unwrap();
    assert!(
        !conflict,
        "Worker B should be rejected while Worker A holds lock"
    );

    let released = release_lock(&db, &cluster_id, "worker-token-A")
        .await
        .unwrap();
    assert!(released, "Worker A should successfully release lock");

    let acquired_b = acquire_lock(&db, &cluster_id, "worker-token-B")
        .await
        .unwrap();
    assert!(acquired_b, "Worker B should now acquire lock");
}

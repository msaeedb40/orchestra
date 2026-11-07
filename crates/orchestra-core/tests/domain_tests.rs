use orchestra_common::identifiers::{ClusterId, NodeId, OperationId};
use orchestra_core::*;

#[test]
fn test_cluster_lifecycle_fsm() {
    let mut status = ClusterStatus::new(3, 5);
    assert_eq!(status.state, ClusterState::Pending);

    // Valid path
    assert!(status.transition_to(ClusterState::Creating, None).is_ok());
    assert!(
        status
            .transition_to(ClusterState::Bootstrapping, None)
            .is_ok()
    );
    assert!(status.transition_to(ClusterState::Running, None).is_ok());
    assert!(status.state.is_ready());

    // Disallowed transition: cannot go directly from Running to Creating
    let err = status
        .transition_to(ClusterState::Creating, None)
        .unwrap_err();
    assert!(matches!(err, ClusterError::InvalidStateTransition { .. }));

    // Running can go to Stopping, then Stopped
    assert!(status.transition_to(ClusterState::Stopping, None).is_ok());
    assert!(status.transition_to(ClusterState::Stopped, None).is_ok());

    // Stopped can be restarted
    assert!(status.transition_to(ClusterState::Creating, None).is_ok());

    // Can transition to Failed on error
    assert!(
        status
            .transition_to(ClusterState::Failed, Some("Disk full".into()))
            .is_ok()
    );
    assert_eq!(status.message.as_deref(), Some("Disk full"));

    // Can delete from Failed
    assert!(status.transition_to(ClusterState::Deleting, None).is_ok());
    assert!(status.transition_to(ClusterState::Deleted, None).is_ok());
    assert!(status.state.is_terminal());
}

#[test]
fn test_node_lifecycle_fsm() {
    let mut status = NodeStatus::new();
    assert_eq!(status.state, NodeState::Creating);

    assert!(status.transition_to(NodeState::Starting, None).is_ok());
    assert!(status.transition_to(NodeState::Initializing, None).is_ok());
    assert!(status.transition_to(NodeState::Ready, None).is_ok());
    assert!(status.state.is_ready());

    // Flapping readiness
    assert!(
        status
            .transition_to(NodeState::NotReady, Some("OOM".into()))
            .is_ok()
    );
    assert!(!status.state.is_ready());
    assert!(status.transition_to(NodeState::Ready, None).is_ok());

    // Shutdown
    assert!(status.transition_to(NodeState::Stopping, None).is_ok());
    assert!(status.transition_to(NodeState::Stopped, None).is_ok());
}

#[test]
fn test_operation_progress_tracking() {
    let stages = vec![
        OperationStage::new("net", "Bridge creation"),
        OperationStage::new("cp", "Control plane boot"),
        OperationStage::new("workers", "Worker join"),
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
    assert_eq!(op.progress_percent, 0);

    op.advance_to_stage(1);
    assert_eq!(op.progress_percent, 33);

    op.advance_to_stage(2);
    assert_eq!(op.progress_percent, 66);

    op.mark_completed();
    assert_eq!(op.status, OperationStatus::Completed);
    assert_eq!(op.progress_percent, 100);
}

#[test]
fn test_domain_serialization_roundtrips() {
    let spec = ClusterSpec::new("prod-cluster", "v1.31.2");
    let cluster = Cluster::new(ClusterId::generate(), spec);

    let json = serde_json::to_string_pretty(&cluster).unwrap();
    let deserialized: Cluster = serde_json::from_str(&json).unwrap();
    assert_eq!(cluster, deserialized);

    let node = Node::new(
        NodeId::generate(),
        cluster.id.clone(),
        "prod-worker-1",
        NodeRole::Worker,
    );
    let node_json = serde_json::to_string(&node).unwrap();
    let deserialized_node: Node = serde_json::from_str(&node_json).unwrap();
    assert_eq!(node, deserialized_node);
}

#[test]
fn test_error_hierarchy_propagation() {
    let node_err = NodeError::HealthCheckTimeout(NodeId::new("node-xyz"));
    let orchestra_err = OrchestraError::from(node_err);

    assert!(
        orchestra_err
            .to_string()
            .contains("Node error: Node health check timed out")
    );
}

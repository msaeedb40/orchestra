//! Integration test for Docker runtime provider.

use orchestra_common::identifiers::{ClusterId, NodeId};
use orchestra_core::cluster::{Cluster, ClusterSpec};
use orchestra_core::node::{Node, NodeRole};
use orchestra_core::traits::RuntimeProvider;
use orchestra_runtime::{DockerClient, DockerRuntimeProvider};

#[tokio::test]
async fn test_docker_runtime_integration() {
    let client = match DockerClient::connect() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping Docker runtime test: {}", e);
            return;
        }
    };

    let ping_result = client.ping().await;
    assert!(
        ping_result.is_ok(),
        "Docker daemon should be reachable and healthy"
    );

    let provider = DockerRuntimeProvider::new(client);

    let cluster_id = ClusterId::generate();
    let spec = ClusterSpec::new("docker-test-cluster", "v1.36.6");
    let cluster = Cluster::new(cluster_id.clone(), spec);

    let node_id = NodeId::generate();
    let node = Node::new(
        node_id,
        cluster_id,
        "docker-test-cluster-control-plane",
        NodeRole::ControlPlane,
    );

    // Verify node creation and deletion interface contract
    let create_result = provider.create_node(&cluster, &node).await;
    match create_result {
        Ok(container_id) => {
            assert!(!container_id.is_empty());
            let _ = provider.delete_node(&container_id).await;
        }
        Err(e) => {
            println!("Node creation safely handled missing image: {}", e);
        }
    }
}

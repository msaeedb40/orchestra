use orchestra_common::identifiers::{ClusterId, NodeId};
use orchestra_core::cluster::{Cluster, ClusterSpec};
use orchestra_core::node::{Node, NodeRole};
use orchestra_runtime::{DockerClient, DockerRuntimeProvider};

#[tokio::test]
async fn test_docker_client_connection() {
    // Attempt Docker connection; skip if Docker daemon is not accessible in environment
    let client = match DockerClient::connect() {
        Ok(c) => c,
        Err(_) => return,
    };

    if client.ping().await.is_err() {
        return;
    }

    let provider = DockerRuntimeProvider::new(client);
    let spec = ClusterSpec::new("test-runtime-clus", "v1.36.6");
    let cluster = Cluster::new(ClusterId::generate(), spec);
    let _node = Node::new(
        NodeId::generate(),
        cluster.id.clone(),
        "test-runtime-node",
        NodeRole::Worker,
    );

    // Verify provider exposes managers
    assert_eq!(
        provider
            .container_manager()
            .find_container("nonexistent-node-12345")
            .await
            .unwrap(),
        None
    );
}

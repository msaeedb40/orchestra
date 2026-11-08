//! Integration test for Daemon gRPC API communications.

use orchestra_api::client::OrchestraClient;
use orchestra_api::cluster::{
    ClusterSpec as ProtoClusterSpec, CreateClusterRequest, DeleteClusterRequest, GetClusterRequest,
    ListClustersRequest,
};
use orchestra_api::system::{GetSystemInfoRequest, PingRequest};
use orchestra_integration_tests::TestServer;
use tonic::Request;

#[tokio::test]
async fn test_daemon_api_lifecycle() {
    let server = match TestServer::spawn().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Skipping daemon api integration test (requires Docker): {}",
                e
            );
            return;
        }
    };

    let mut client = OrchestraClient::connect(server.endpoint())
        .await
        .expect("Failed to connect client to test server");

    // 1. Ping
    let pong_res = client
        .system
        .ping(Request::new(PingRequest {}))
        .await
        .expect("Ping failed");
    assert_eq!(pong_res.into_inner().message, "pong");

    // 2. System info
    let sys_res = client
        .system
        .get_system_info(Request::new(GetSystemInfoRequest {}))
        .await
        .expect("GetSystemInfo failed");
    let sys_info = sys_res.into_inner();
    assert_eq!(sys_info.daemon_version, env!("CARGO_PKG_VERSION"));

    // 3. Create cluster
    let spec = ProtoClusterSpec {
        name: "integration-k8s".to_string(),
        kubernetes_version: "v1.36.6".to_string(),
        control_plane_nodes: 1,
        worker_nodes: 2,
        node_image: "orchestra-node:1.36.6".to_string(),
        cni_plugin: "orchestranet".to_string(),
        service_proxy_mode: "orchestra".to_string(),
        pod_cidr: "10.244.0.0/16".to_string(),
        service_cidr: "10.96.0.0/12".to_string(),
        api_server_port: 6443,
    };

    let create_res = client
        .cluster
        .create_cluster(Request::new(CreateClusterRequest { spec: Some(spec) }))
        .await
        .expect("CreateCluster failed");
    let cluster = create_res
        .into_inner()
        .cluster
        .expect("Cluster info missing");
    assert_eq!(cluster.spec.as_ref().unwrap().name, "integration-k8s");

    // 4. Retrieve cluster
    let get_res = client
        .cluster
        .get_cluster(Request::new(GetClusterRequest {
            name_or_id: "integration-k8s".to_string(),
        }))
        .await
        .expect("GetCluster failed");
    let retrieved = get_res.into_inner().cluster.expect("Cluster not found");
    assert_eq!(retrieved.id, cluster.id);

    // 5. List clusters
    let list_res = client
        .cluster
        .list_clusters(Request::new(ListClustersRequest {}))
        .await
        .expect("ListClusters failed");
    let clusters = list_res.into_inner().clusters;
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].id, cluster.id);

    // 6. Delete cluster
    let del_res = client
        .cluster
        .delete_cluster(Request::new(DeleteClusterRequest {
            name_or_id: "integration-k8s".to_string(),
        }))
        .await
        .expect("DeleteCluster failed");
    assert!(del_res.into_inner().success);

    // 7. Verify audit logging
    let audit_events = server.state.audit.recent(10);
    assert!(audit_events.iter().any(|e| e.action == "cluster.create"));
    assert!(audit_events.iter().any(|e| e.action == "cluster.delete"));
}

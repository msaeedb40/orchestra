//! Orchestra background daemon (`orchestrad`) runtime, reconciliation loop, and gRPC server.
#![allow(clippy::result_large_err)]

pub mod lifecycle;
pub mod reconciliation;
pub mod server;
pub mod signals;
pub mod state;

pub use server::GrpcServer;
pub use state::DaemonState;

#[cfg(test)]
mod tests {
    use super::*;
    use orchestra_api::cluster::cluster_service_server::ClusterService;
    use orchestra_api::cluster::{ClusterSpec as ProtoClusterSpec, CreateClusterRequest};
    use orchestra_api::system::PingRequest;
    use orchestra_api::system::system_service_server::SystemService;
    use orchestra_runtime::{DockerClient, DockerRuntimeProvider};
    use orchestra_storage::database::Database;
    use tonic::Request;

    #[tokio::test]
    async fn test_daemon_grpc_ping() {
        let db = Database::open_in_memory().unwrap();
        orchestra_storage::run_migrations(&db).await.unwrap();

        let client = match DockerClient::connect() {
            Ok(c) => c,
            Err(_) => return,
        };
        let runtime = DockerRuntimeProvider::new(client);
        let state = DaemonState::new(db, runtime);
        let server = GrpcServer::new(state);

        let ping_res = server.ping(Request::new(PingRequest {})).await.unwrap();
        assert_eq!(ping_res.into_inner().message, "pong");
    }

    #[tokio::test]
    async fn test_daemon_grpc_cluster_crud() {
        let db = Database::open_in_memory().unwrap();
        orchestra_storage::run_migrations(&db).await.unwrap();

        let client = match DockerClient::connect() {
            Ok(c) => c,
            Err(_) => return,
        };
        let runtime = DockerRuntimeProvider::new(client);
        let state = DaemonState::new(db, runtime);
        let server = GrpcServer::new(state);

        let req = CreateClusterRequest {
            spec: Some(ProtoClusterSpec {
                name: "test-daemon-clus".to_string(),
                kubernetes_version: "v1.36.6".to_string(),
                control_plane_nodes: 1,
                worker_nodes: 2,
                node_image: "orchestra-node:1.36.6".to_string(),
                cni_plugin: "kindnet".to_string(),
                service_proxy_mode: "orchestra".to_string(),
                pod_cidr: "10.244.0.0/16".to_string(),
                service_cidr: "10.96.0.0/12".to_string(),
                api_server_port: 6443,
            }),
        };

        let create_res = server.create_cluster(Request::new(req)).await.unwrap();
        let cluster_info = create_res.into_inner().cluster.unwrap();
        assert_eq!(cluster_info.spec.unwrap().name, "test-daemon-clus");

        let audit_events = server.state().audit.recent(5);
        assert!(!audit_events.is_empty());
        assert_eq!(audit_events[0].action, "cluster.create");
        assert_eq!(
            audit_events[0].outcome,
            orchestra_security::AuditOutcome::Allowed
        );
    }
}

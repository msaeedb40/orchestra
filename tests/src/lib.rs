//! End-to-end and integration test harness for Orchestra.
//!
//! Provides reusable test fixtures, mock environments, and shared setup utilities.

use orchestra_api::cluster::cluster_service_server::ClusterServiceServer;
use orchestra_api::ebpf::ebpf_service_server::EbpfServiceServer;
use orchestra_api::network::network_service_server::NetworkServiceServer;
use orchestra_api::node::node_service_server::NodeServiceServer;
use orchestra_api::system::system_service_server::SystemServiceServer;
use orchestra_daemon::{DaemonState, GrpcServer};
use orchestra_runtime::{DockerClient, DockerRuntimeProvider};
use orchestra_storage::Database;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tonic::transport::Server;

/// Test server instance running in-process for integration testing.
pub struct TestServer {
    pub addr: SocketAddr,
    pub state: DaemonState,
}

impl TestServer {
    /// Spawns a full in-process gRPC server bound to a dynamic ephemeral port.
    pub async fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let db = Database::open_in_memory()?;
        orchestra_storage::run_migrations(&db).await?;

        // Find available ephemeral port
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        drop(listener); // Free the port for tonic

        let client = match DockerClient::connect() {
            Ok(client) => client,
            Err(_) => return Err("Docker daemon connection failed".into()),
        };
        let runtime = DockerRuntimeProvider::new(client);
        let state = DaemonState::new(db, runtime);
        let grpc = GrpcServer::new(state.clone());

        tokio::spawn(async move {
            let _ = Server::builder()
                .add_service(SystemServiceServer::new(grpc.clone()))
                .add_service(ClusterServiceServer::new(grpc.clone()))
                .add_service(NodeServiceServer::new(grpc.clone()))
                .add_service(NetworkServiceServer::new(grpc.clone()))
                .add_service(EbpfServiceServer::new(grpc))
                .serve(addr)
                .await;
        });

        // Give server a moment to bind
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        Ok(Self { addr, state })
    }

    /// Helper to get the gRPC endpoint URL (e.g. "http://127.0.0.1:54321").
    pub fn endpoint(&self) -> String {
        format!("http://{}", self.addr)
    }
}

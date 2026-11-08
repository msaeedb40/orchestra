//! Daemon binary entry point (`orchestrad`).

use clap::Parser;
use orchestra_api::cluster::cluster_service_server::ClusterServiceServer;
use orchestra_api::ebpf::ebpf_service_server::EbpfServiceServer;
use orchestra_api::network::network_service_server::NetworkServiceServer;
use orchestra_api::node::node_service_server::NodeServiceServer;
use orchestra_api::system::system_service_server::SystemServiceServer;
use orchestra_daemon::{DaemonState, GrpcServer};
use orchestra_daemon::reconciliation::run_reconciler;
use orchestra_runtime::{DockerClient, DockerRuntimeProvider};
use orchestra_storage::Database;
use std::net::SocketAddr;
use std::path::PathBuf;
use tonic::transport::Server;
use tracing::{Level, info};

#[derive(Parser, Debug)]
#[command(name = "orchestrad", author, version, about = "Orchestra Daemon")]
struct Args {
    /// Address to bind the gRPC API server
    #[arg(short, long, default_value = "127.0.0.1:50051")]
    listen_addr: SocketAddr,

    /// Database file path (defaults to in-memory if empty)
    #[arg(short, long)]
    db_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args = Args::parse();
    info!("Starting orchestrad v{}", env!("CARGO_PKG_VERSION"));

    let db = match args.db_path {
        Some(path) => Database::open(path)?,
        None => Database::open_in_memory()?,
    };
    orchestra_storage::run_migrations(&db).await?;

    let client = DockerClient::connect()?;
    let runtime = DockerRuntimeProvider::new(client);
    let state = DaemonState::new(db, runtime);
    let grpc = GrpcServer::new(state.clone());

    // Spawn the reconciliation loop in the background before accepting gRPC
    tokio::spawn(run_reconciler(state));

    info!(addr = %args.listen_addr, "Binding gRPC API server");

    Server::builder()
        .add_service(SystemServiceServer::new(grpc.clone()))
        .add_service(ClusterServiceServer::new(grpc.clone()))
        .add_service(NodeServiceServer::new(grpc.clone()))
        .add_service(NetworkServiceServer::new(grpc.clone()))
        .add_service(EbpfServiceServer::new(grpc))
        .serve(args.listen_addr)
        .await?;

    Ok(())
}

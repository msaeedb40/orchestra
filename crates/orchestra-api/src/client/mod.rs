//! High-level gRPC client connectors for Orchestra daemon communication.

use crate::v1::cluster::cluster_service_client::ClusterServiceClient;
use crate::v1::ebpf::ebpf_service_client::EbpfServiceClient;
use crate::v1::network::network_service_client::NetworkServiceClient;
use crate::v1::node::node_service_client::NodeServiceClient;
use crate::v1::system::system_service_client::SystemServiceClient;
use tonic::transport::Channel;

/// Client aggregator bundling all Orchestra gRPC service clients.
#[derive(Clone, Debug)]
pub struct OrchestraClient {
    pub cluster: ClusterServiceClient<Channel>,
    pub node: NodeServiceClient<Channel>,
    pub network: NetworkServiceClient<Channel>,
    pub ebpf: EbpfServiceClient<Channel>,
    pub system: SystemServiceClient<Channel>,
}

impl OrchestraClient {
    /// Connects to a remote gRPC endpoint.
    pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        let endpoint = tonic::transport::Endpoint::new(dst)?;
        let channel = endpoint.connect().await?;
        Ok(Self::new(channel))
    }

    /// Creates an OrchestraClient wrapper from a connected transport channel.
    pub fn new(channel: Channel) -> Self {
        Self {
            cluster: ClusterServiceClient::new(channel.clone()),
            node: NodeServiceClient::new(channel.clone()),
            network: NetworkServiceClient::new(channel.clone()),
            ebpf: EbpfServiceClient::new(channel.clone()),
            system: SystemServiceClient::new(channel),
        }
    }
}

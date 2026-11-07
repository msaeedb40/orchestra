//! gRPC contracts, Protobuf types, client bindings, and server infrastructure.
#![allow(clippy::result_large_err)]

pub mod client;
pub mod grpc;
pub mod server;
pub mod v1;

pub use client::OrchestraClient;
pub use v1::cluster;
pub use v1::common;
pub use v1::ebpf;
pub use v1::network;
pub use v1::node;
pub use v1::system;

pub use v1::cluster::{
    cluster_service_client::ClusterServiceClient,
    cluster_service_server::{ClusterService, ClusterServiceServer},
    *,
};
pub use v1::common::*;
pub use v1::ebpf::{
    ebpf_service_client::EbpfServiceClient,
    ebpf_service_server::{EbpfService, EbpfServiceServer},
    *,
};
pub use v1::network::{
    network_service_client::NetworkServiceClient,
    network_service_server::{NetworkService, NetworkServiceServer},
    *,
};
pub use v1::node::{
    node_service_client::NodeServiceClient,
    node_service_server::{NodeService, NodeServiceServer},
    *,
};
pub use v1::system::{
    system_service_client::SystemServiceClient,
    system_service_server::{SystemService, SystemServiceServer},
    *,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proto_generated_types() {
        let spec = ClusterSpec {
            name: "test-clus".to_string(),
            kubernetes_version: "v1.36.6".to_string(),
            control_plane_nodes: 1,
            worker_nodes: 2,
            node_image: "orchestra-node:1.36.6".to_string(),
            cni_plugin: "kindnet".to_string(),
            service_proxy_mode: "orchestra".to_string(),
            pod_cidr: "10.244.0.0/16".to_string(),
            service_cidr: "10.96.0.0/16".to_string(),
            api_server_port: 6443,
        };

        assert_eq!(spec.name, "test-clus");
        assert_eq!(spec.control_plane_nodes, 1);
        assert_eq!(spec.worker_nodes, 2);

        let ping_req = PingRequest {};
        assert_eq!(std::mem::size_of_val(&ping_req), 0);
    }
}

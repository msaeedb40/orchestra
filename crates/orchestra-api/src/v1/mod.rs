//! Generated v1 Protocol Buffers and gRPC stubs.

pub mod orchestra {
    pub mod common {
        pub mod v1 {
            tonic::include_proto!("orchestra.common.v1");
        }
    }

    pub mod cluster {
        pub mod v1 {
            tonic::include_proto!("orchestra.cluster.v1");
        }
    }

    pub mod node {
        pub mod v1 {
            tonic::include_proto!("orchestra.node.v1");
        }
    }

    pub mod network {
        pub mod v1 {
            tonic::include_proto!("orchestra.network.v1");
        }
    }

    pub mod ebpf {
        pub mod v1 {
            tonic::include_proto!("orchestra.ebpf.v1");
        }
    }

    pub mod system {
        pub mod v1 {
            tonic::include_proto!("orchestra.system.v1");
        }
    }
}

pub use orchestra::cluster::v1 as cluster;
pub use orchestra::common::v1 as common;
pub use orchestra::ebpf::v1 as ebpf;
pub use orchestra::network::v1 as network;
pub use orchestra::node::v1 as node;
pub use orchestra::system::v1 as system;

//! Kubernetes cluster bootstrapping, kubeadm lifecycle, and manifest integration.

pub mod bootstrap;
pub mod client;
pub mod kubeadm;
pub mod kubeconfig;
pub mod manifests;
pub mod wait;

pub use bootstrap::ClusterBootstrapper;
pub use kubeadm::{generate_init_config, generate_join_config};
pub use kubeconfig::rewrite_server_endpoint;
pub use manifests::generate_cni_manifest;

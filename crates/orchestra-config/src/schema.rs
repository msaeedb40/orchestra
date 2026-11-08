//! Typed configuration schemas for Orchestra clusters, daemons, and subsystems.

use orchestra_common::constants::{
    DEFAULT_BPF_PIN_PATH, DEFAULT_BRIDGE_NAME, DEFAULT_K8S_API_PORT, DEFAULT_METRICS_PORT,
    DEFAULT_NODE_IMAGE, DEFAULT_NODE_SUBNET_CIDR, DEFAULT_POD_CIDR, DEFAULT_SERVICE_CIDR,
};
use orchestra_core::network::{CniPlugin, NetworkMode, ServiceProxyMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current supported apiVersion for cluster configs.
pub const CURRENT_API_VERSION: &str = "orchestra.io/v1alpha1";

/// Standard kind for cluster definitions.
pub const CLUSTER_KIND: &str = "Cluster";

/// Top-level Kubernetes-style configuration document for an Orchestra cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchestraConfig {
    /// API schema version (e.g. "orchestra.io/v1alpha1").
    #[serde(rename = "apiVersion", default = "default_api_version")]
    pub api_version: String,
    /// Object kind (must be "Cluster").
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Object metadata (name, labels, annotations).
    pub metadata: MetadataConfig,
    /// Desired cluster specification.
    pub spec: ClusterConfigSpec,
}

fn default_api_version() -> String {
    CURRENT_API_VERSION.to_string()
}

fn default_kind() -> String {
    CLUSTER_KIND.to_string()
}

/// Object metadata block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataConfig {
    /// Unique name of the cluster.
    pub name: String,
    /// Arbitrary key-value metadata labels.
    #[serde(default)]
    pub labels: HashMap<String, String>,
    /// Non-identifying annotations.
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

impl MetadataConfig {
    /// Creates a metadata block with a name and empty labels.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        }
    }
}

/// Detailed specification for all cluster components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClusterConfigSpec {
    /// Kubernetes target version (e.g. "v1.31.2").
    #[serde(default = "default_k8s_version")]
    pub kubernetes_version: String,
    /// Node topology and container specifications.
    #[serde(default)]
    pub nodes: NodeConfigSpec,
    /// Networking topology, subnets, and CNI configuration.
    #[serde(default)]
    pub network: NetworkConfigSpec,
    /// In-kernel eBPF dataplane settings.
    #[serde(default)]
    pub ebpf: EbpfConfigSpec,
    /// Container runtime configuration.
    #[serde(default)]
    pub runtime: RuntimeConfigSpec,
    /// Observability and metrics export settings.
    #[serde(default)]
    pub observability: ObservabilityConfigSpec,
    /// Security and isolation profiles.
    #[serde(default)]
    pub security: SecurityConfigSpec,
}

impl Default for ClusterConfigSpec {
    fn default() -> Self {
        Self {
            kubernetes_version: default_k8s_version(),
            nodes: NodeConfigSpec::default(),
            network: NetworkConfigSpec::default(),
            ebpf: EbpfConfigSpec::default(),
            runtime: RuntimeConfigSpec::default(),
            observability: ObservabilityConfigSpec::default(),
            security: SecurityConfigSpec::default(),
        }
    }
}

fn default_k8s_version() -> String {
    "v1.31.2".to_string()
}

/// Node configuration block for control plane and worker groups.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NodeConfigSpec {
    /// Control plane node specification.
    #[serde(default)]
    pub control_plane: ControlPlaneNodeConfig,
    /// Worker nodes specification.
    #[serde(default)]
    pub workers: WorkerNodeConfig,
}

/// Control plane node configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPlaneNodeConfig {
    /// Number of control plane nodes (1 for dev, 3+ for HA).
    #[serde(default = "default_cp_count")]
    pub count: usize,
    /// Node container image.
    #[serde(default = "default_node_image")]
    pub image: String,
    /// Extra port mappings from host to control plane container.
    #[serde(default)]
    pub port_mappings: Vec<PortMapping>,
    /// Extra filesystem mount mappings.
    #[serde(default)]
    pub extra_mounts: Vec<MountConfig>,
}

fn default_cp_count() -> usize {
    1
}

fn default_node_image() -> String {
    DEFAULT_NODE_IMAGE.to_string()
}

impl Default for ControlPlaneNodeConfig {
    fn default() -> Self {
        Self {
            count: default_cp_count(),
            image: default_node_image(),
            port_mappings: Vec::new(),
            extra_mounts: Vec::new(),
        }
    }
}

/// Worker nodes configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerNodeConfig {
    /// Number of worker nodes.
    #[serde(default = "default_worker_count")]
    pub count: usize,
    /// Node container image.
    #[serde(default = "default_node_image")]
    pub image: String,
    /// Extra filesystem mount mappings.
    #[serde(default)]
    pub extra_mounts: Vec<MountConfig>,
}

fn default_worker_count() -> usize {
    2
}

impl Default for WorkerNodeConfig {
    fn default() -> Self {
        Self {
            count: default_worker_count(),
            image: default_node_image(),
            extra_mounts: Vec::new(),
        }
    }
}

/// Port mapping from host to node container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortMapping {
    /// Port on the container.
    pub container_port: u16,
    /// Port on the host interface.
    pub host_port: u16,
    /// Protocol: "tcp" or "udp".
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

fn default_protocol() -> String {
    "tcp".to_string()
}

/// Bind mount configuration from host to node container.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MountConfig {
    /// Host filesystem path.
    pub host_path: String,
    /// Destination container filesystem path.
    pub container_path: String,
    /// Whether the mount is read-only.
    #[serde(default)]
    pub read_only: bool,
}

/// Network configuration block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkConfigSpec {
    /// Network topology mode (bridge, host, overlay).
    #[serde(default)]
    pub mode: NetworkMode,
    /// Selected CNI plugin (orchestranet, flannel, calico, cilium, none).
    #[serde(default)]
    pub cni: CniPlugin,
    /// Service dataplane owner (orchestra, cni, kube-proxy).
    #[serde(default)]
    pub service_proxy: ServiceProxyMode,
    /// Pod IP network CIDR.
    #[serde(default = "default_pod_cidr")]
    pub pod_cidr: String,
    /// Service ClusterIP network CIDR.
    #[serde(default = "default_service_cidr")]
    pub service_cidr: String,
    /// Internal bridge subnet CIDR.
    #[serde(default = "default_bridge_subnet")]
    pub bridge_subnet: String,
    /// Name of the Linux bridge interface.
    #[serde(default = "default_bridge_name")]
    pub bridge_name: String,
    /// Host port on which the Kubernetes API server is published.
    #[serde(default = "default_api_port")]
    pub api_server_port: u16,
}

fn default_pod_cidr() -> String {
    DEFAULT_POD_CIDR.to_string()
}

fn default_service_cidr() -> String {
    DEFAULT_SERVICE_CIDR.to_string()
}

fn default_bridge_subnet() -> String {
    DEFAULT_NODE_SUBNET_CIDR.to_string()
}

fn default_bridge_name() -> String {
    DEFAULT_BRIDGE_NAME.to_string()
}

fn default_api_port() -> u16 {
    DEFAULT_K8S_API_PORT
}

impl Default for NetworkConfigSpec {
    fn default() -> Self {
        Self {
            mode: NetworkMode::default(),
            cni: CniPlugin::default(),
            service_proxy: ServiceProxyMode::default(),
            pod_cidr: default_pod_cidr(),
            service_cidr: default_service_cidr(),
            bridge_subnet: default_bridge_subnet(),
            bridge_name: default_bridge_name(),
            api_server_port: default_api_port(),
        }
    }
}

/// eBPF subsystem configuration block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EbpfConfigSpec {
    /// Whether eBPF dataplane acceleration is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Host path where BPF maps and programs are pinned.
    #[serde(default = "default_bpf_pin_path")]
    pub pin_path: String,
    /// Enable eBPF-based service load balancer (XDP/TC).
    #[serde(default = "default_true")]
    pub service_load_balancer: bool,
    /// Enable connection tracking inside eBPF.
    #[serde(default = "default_true")]
    pub conntrack: bool,
    /// Enabled eBPF program hooks.
    #[serde(default = "default_ebpf_programs")]
    pub programs: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_bpf_pin_path() -> String {
    DEFAULT_BPF_PIN_PATH.to_string()
}

fn default_ebpf_programs() -> Vec<String> {
    vec!["xdp".to_string(), "tc".to_string()]
}

impl Default for EbpfConfigSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            pin_path: default_bpf_pin_path(),
            service_load_balancer: true,
            conntrack: true,
            programs: default_ebpf_programs(),
        }
    }
}

/// Container runtime configuration block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfigSpec {
    /// Optional custom Docker/containerd Unix socket path.
    pub socket_path: Option<String>,
    /// Image pull policy ("IfNotPresent", "Always", "Never").
    #[serde(default = "default_image_pull_policy")]
    pub image_pull_policy: String,
    /// Cgroup driver ("systemd" or "cgroupfs").
    #[serde(default = "default_cgroup_driver")]
    pub cgroup_driver: String,
}

fn default_image_pull_policy() -> String {
    "IfNotPresent".to_string()
}

fn default_cgroup_driver() -> String {
    "systemd".to_string()
}

impl Default for RuntimeConfigSpec {
    fn default() -> Self {
        Self {
            socket_path: None,
            image_pull_policy: default_image_pull_policy(),
            cgroup_driver: default_cgroup_driver(),
        }
    }
}

/// Observability configuration block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityConfigSpec {
    /// Logging level ("info", "debug", "warn", "error", "trace").
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Logging output format ("pretty", "compact", "json").
    #[serde(default = "default_log_format")]
    pub log_format: String,
    /// Whether Prometheus metrics scraping endpoint is enabled.
    #[serde(default = "default_true")]
    pub metrics_enabled: bool,
    /// Port for Prometheus metrics endpoint.
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "pretty".to_string()
}

fn default_metrics_port() -> u16 {
    DEFAULT_METRICS_PORT
}

impl Default for ObservabilityConfigSpec {
    fn default() -> Self {
        Self {
            log_level: default_log_level(),
            log_format: default_log_format(),
            metrics_enabled: true,
            metrics_port: default_metrics_port(),
        }
    }
}

/// Security and isolation configuration block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityConfigSpec {
    /// Seccomp profile name ("default", "unconfined", or file path).
    #[serde(default = "default_seccomp")]
    pub seccomp_profile: String,
    /// Whether node containers run with privileged capabilities.
    #[serde(default = "default_true")]
    pub privileged_nodes: bool,
    /// Enable mutual TLS for gRPC daemon communication.
    #[serde(default)]
    pub enable_mtls: bool,
}

fn default_seccomp() -> String {
    "default".to_string()
}

impl Default for SecurityConfigSpec {
    fn default() -> Self {
        Self {
            seccomp_profile: default_seccomp(),
            privileged_nodes: true,
            enable_mtls: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let meta = MetadataConfig::new("dev-cluster");
        let cfg = OrchestraConfig {
            api_version: CURRENT_API_VERSION.to_string(),
            kind: CLUSTER_KIND.to_string(),
            metadata: meta,
            spec: ClusterConfigSpec::default(),
        };

        assert_eq!(cfg.metadata.name, "dev-cluster");
        assert_eq!(cfg.spec.nodes.control_plane.count, 1);
        assert_eq!(cfg.spec.nodes.workers.count, 2);
        assert_eq!(cfg.spec.network.mode, NetworkMode::Bridge);
    }
}

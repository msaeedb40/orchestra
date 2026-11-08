//! Command-line argument parser definitions using Clap.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "orchestra",
    author = "Orchestra Contributors",
    version,
    about = "High-performance Kubernetes cluster orchestrator powered by Rust & eBPF",
    long_about = None
)]
pub struct Cli {
    /// Daemon endpoint URL
    #[arg(global = true, long, default_value = "http://127.0.0.1:50051")]
    pub daemon_url: String,

    /// Output format
    #[arg(global = true, short, long, value_enum, default_value_t = OutputFormat::Table)]
    pub output: OutputFormat,

    /// Enable verbose diagnostics
    #[arg(global = true, short, long)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage Kubernetes clusters
    Cluster(ClusterArgs),

    /// Manage Kubernetes nodes
    Node(NodeArgs),

    /// Manage and inspect cluster networking
    Network(NetworkArgs),

    /// Inspect and manage eBPF programs and maps
    Ebpf(EbpfArgs),

    /// Check daemon health and system information
    System(SystemArgs),
}

#[derive(Args, Debug)]
pub struct ClusterArgs {
    #[command(subcommand)]
    pub command: ClusterCommands,
}

#[derive(Subcommand, Debug)]
pub enum ClusterCommands {
    /// Create a new Kubernetes cluster
    Create {
        /// Name of the cluster
        name: String,

        /// Path to cluster config file (YAML)
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Kubernetes version (e.g. v1.36.6)
        #[arg(long, default_value = "v1.36.6")]
        k8s_version: String,

        /// Number of control plane nodes
        #[arg(long, default_value_t = 1)]
        control_plane: usize,

        /// Number of worker nodes
        #[arg(long, default_value_t = 2)]
        workers: usize,

        /// Node container image
        #[arg(long, default_value = "orchestra-node:1.36.6")]
        node_image: String,

        /// CNI plugin (orchestranet, flannel, calico, cilium, none)
        #[arg(long, default_value = "orchestranet")]
        cni: String,

        /// Service dataplane proxy mode (orchestra, cni, kube-proxy)
        #[arg(long, default_value = "orchestra")]
        proxy_mode: String,
    },

    /// List all clusters
    List,

    /// Get detailed information on a cluster
    Get {
        /// Cluster name or ID
        name: String,
    },

    /// Delete a cluster
    Delete {
        /// Cluster name or ID
        name: String,
    },

    /// Start a stopped cluster
    Start {
        /// Cluster name or ID
        name: String,
    },

    /// Stop a running cluster
    Stop {
        /// Cluster name or ID
        name: String,
    },
}

#[derive(Args, Debug)]
pub struct NodeArgs {
    #[command(subcommand)]
    pub command: NodeCommands,
}

#[derive(Subcommand, Debug)]
pub enum NodeCommands {
    /// List nodes for a cluster
    List {
        /// Cluster name
        cluster: String,
    },

    /// Add a node to a cluster
    Add {
        /// Cluster name
        cluster: String,

        /// Role of node (worker or control-plane)
        #[arg(short, long, default_value = "worker")]
        role: String,
    },

    /// Remove a node from a cluster
    Remove {
        /// Cluster name
        cluster: String,

        /// Node name or ID
        node: String,
    },
}

#[derive(Args, Debug)]
pub struct NetworkArgs {
    #[command(subcommand)]
    pub command: NetworkCommands,
}

#[derive(Subcommand, Debug)]
pub enum NetworkCommands {
    /// Show network status for a cluster
    Status {
        /// Cluster name
        cluster: String,
    },
}

#[derive(Args, Debug)]
pub struct EbpfArgs {
    #[command(subcommand)]
    pub command: EbpfCommands,
}

#[derive(Subcommand, Debug)]
pub enum EbpfCommands {
    /// Show eBPF subsystem status
    Status,
}

#[derive(Args, Debug)]
pub struct SystemArgs {
    #[command(subcommand)]
    pub command: SystemCommands,
}

#[derive(Subcommand, Debug)]
pub enum SystemCommands {
    /// Ping the daemon
    Ping,

    /// Retrieve daemon and system info
    Info,
}

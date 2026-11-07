//! CLI command handlers and dispatcher.

use crate::cli::*;
use orchestra_api::OrchestraClient;
use orchestra_api::cluster::*;
use orchestra_api::ebpf::GetEbpfStatusRequest;
use orchestra_api::network::GetNetworkStatusRequest;
use orchestra_api::node::*;
use orchestra_api::system::*;
use tonic::Request;

pub async fn handle_command(
    mut client: OrchestraClient,
    command: Commands,
    _format: OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::System(SystemArgs { command: sys_cmd }) => match sys_cmd {
            SystemCommands::Ping => {
                let res = client.system.ping(Request::new(PingRequest {})).await?;
                let ping = res.into_inner();
                println!("OK: {} (timestamp: {})", ping.message, ping.timestamp);
            }
            SystemCommands::Info => {
                let res = client
                    .system
                    .get_system_info(Request::new(GetSystemInfoRequest {}))
                    .await?;
                let info = res.into_inner();
                println!("Daemon Version: {}", info.daemon_version);
                println!("OS/Arch:        {}/{}", info.os, info.arch);
                println!("Git Commit:     {}", info.git_commit);
            }
        },

        Commands::Cluster(ClusterArgs { command: clus_cmd }) => match clus_cmd {
            ClusterCommands::Create {
                name,
                k8s_version,
                control_plane,
                workers,
                node_image,
                cni,
                proxy_mode,
                ..
            } => {
                let req = CreateClusterRequest {
                    spec: Some(ClusterSpec {
                        name: name.clone(),
                        kubernetes_version: k8s_version,
                        control_plane_nodes: control_plane as u32,
                        worker_nodes: workers as u32,
                        node_image,
                        cni_plugin: cni,
                        service_proxy_mode: proxy_mode,
                        pod_cidr: "10.244.0.0/16".to_string(),
                        service_cidr: "10.96.0.0/12".to_string(),
                        api_server_port: 6443,
                    }),
                };

                let res = client.cluster.create_cluster(Request::new(req)).await?;
                let body = res.into_inner();
                println!(
                    "Cluster '{}' creation accepted (Operation: {})",
                    name, body.operation_id
                );
            }

            ClusterCommands::List => {
                let res = client
                    .cluster
                    .list_clusters(Request::new(ListClustersRequest {}))
                    .await?;
                let body = res.into_inner();
                if body.clusters.is_empty() {
                    println!("No clusters found.");
                } else {
                    println!(
                        "{:<20} {:<15} {:<12} {:<10}",
                        "NAME", "K8S VERSION", "STATE", "NODES"
                    );
                    println!("{:-<60}", "");
                    for c in body.clusters {
                        let spec = c.spec.unwrap_or_default();
                        let status = c.status.unwrap_or_default();
                        let total_nodes = status.control_plane_nodes + status.worker_nodes;
                        println!(
                            "{:<20} {:<15} {:<12} {:<10}",
                            spec.name, spec.kubernetes_version, status.state, total_nodes
                        );
                    }
                }
            }

            ClusterCommands::Get { name } => {
                let req = GetClusterRequest { name_or_id: name };
                let res = client.cluster.get_cluster(Request::new(req)).await?;
                let body = res.into_inner();
                if let Some(c) = body.cluster {
                    let spec = c.spec.unwrap_or_default();
                    let status = c.status.unwrap_or_default();
                    println!("Cluster ID:     {}", c.id);
                    println!("Name:           {}", spec.name);
                    println!("Status:         {}", status.state);
                    println!("Kubernetes:     {}", spec.kubernetes_version);
                    println!("Control Planes: {}", status.control_plane_nodes);
                    println!("Workers:        {}", status.worker_nodes);
                    println!("CNI Plugin:     {}", spec.cni_plugin);
                    println!("Proxy Mode:     {}", spec.service_proxy_mode);
                }
            }

            ClusterCommands::Delete { name } => {
                let req = DeleteClusterRequest {
                    name_or_id: name.clone(),
                };
                let res = client.cluster.delete_cluster(Request::new(req)).await?;
                println!(
                    "Cluster '{}' deleted (Operation: {})",
                    name,
                    res.into_inner().operation_id
                );
            }

            ClusterCommands::Start { name } => {
                let req = StartClusterRequest {
                    name_or_id: name.clone(),
                };
                let res = client.cluster.start_cluster(Request::new(req)).await?;
                println!(
                    "Cluster '{}' start requested (Operation: {})",
                    name,
                    res.into_inner().operation_id
                );
            }

            ClusterCommands::Stop { name } => {
                let req = StopClusterRequest {
                    name_or_id: name.clone(),
                };
                let res = client.cluster.stop_cluster(Request::new(req)).await?;
                println!(
                    "Cluster '{}' stop requested (Operation: {})",
                    name,
                    res.into_inner().operation_id
                );
            }
        },

        Commands::Node(NodeArgs { command: node_cmd }) => match node_cmd {
            NodeCommands::List { cluster } => {
                let req = ListNodesRequest {
                    cluster_name: cluster,
                };
                let res = client.node.list_nodes(Request::new(req)).await?;
                let body = res.into_inner();
                if body.nodes.is_empty() {
                    println!("No nodes found for cluster.");
                } else {
                    println!("{:<25} {:<15} {:<12} {:<15}", "NAME", "ROLE", "STATE", "IP");
                    println!("{:-<70}", "");
                    for n in body.nodes {
                        println!(
                            "{:<25} {:<15} {:<12} {:<15}",
                            n.name, n.role, n.state, n.ip_address
                        );
                    }
                }
            }

            NodeCommands::Add { cluster, role } => {
                let req = AddNodeRequest {
                    cluster_name: cluster,
                    role,
                };
                let res = client.node.add_node(Request::new(req)).await?;
                println!(
                    "Add node requested (Operation: {})",
                    res.into_inner().operation_id
                );
            }

            NodeCommands::Remove { cluster, node } => {
                let req = RemoveNodeRequest {
                    cluster_name: cluster,
                    node_name_or_id: node,
                };
                let res = client.node.remove_node(Request::new(req)).await?;
                println!(
                    "Remove node requested (Operation: {})",
                    res.into_inner().operation_id
                );
            }
        },

        Commands::Network(NetworkArgs { command: net_cmd }) => match net_cmd {
            NetworkCommands::Status { cluster } => {
                let req = GetNetworkStatusRequest {
                    cluster_name: cluster,
                };
                let res = client.network.get_network_status(Request::new(req)).await?;
                let net = res.into_inner();
                println!("Network Mode:   {}", net.network_mode);
                println!("CNI Plugin:     {}", net.cni_plugin);
                println!("Service Proxy:  {}", net.service_proxy_mode);
                println!("Pod CIDR:       {}", net.pod_cidr);
                println!("Service CIDR:   {}", net.service_cidr);
            }
        },

        Commands::Ebpf(EbpfArgs { command: ebpf_cmd }) => match ebpf_cmd {
            EbpfCommands::Status => {
                let res = client
                    .ebpf
                    .get_ebpf_status(Request::new(GetEbpfStatusRequest {}))
                    .await?;
                let ebpf = res.into_inner();
                println!("eBPF Engine Active: {}", ebpf.is_active);
                println!("Loaded Programs:    {}", ebpf.programs.len());
                println!("Active Maps:        {}", ebpf.maps.len());
            }
        },
    }

    Ok(())
}

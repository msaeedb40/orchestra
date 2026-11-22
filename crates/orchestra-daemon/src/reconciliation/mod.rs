//! Continuous reconciliation loops for desired vs observed state.
//!
//! The reconciler polls for `Pending` clusters and drives them through the
//! full lifecycle: Creating → Bootstrapping → Running.

use crate::state::DaemonState;
use orchestra_common::identifiers::NodeId;
use orchestra_core::cluster::{Cluster, ClusterState};
use orchestra_core::node::{Node, NodeRole, NodeState};
use orchestra_core::traits::RuntimeProvider;
use orchestra_kubernetes::ClusterBootstrapper;
use orchestra_storage::repository::cluster::{list_clusters, save_cluster};
use orchestra_storage::repository::node::{list_nodes, save_node};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Interval between reconciliation passes.
const RECONCILE_INTERVAL: Duration = Duration::from_secs(5);

/// Maximum time to wait for a node container to become Running before marking Failed.
#[allow(dead_code)]
const NODE_START_TIMEOUT: Duration = Duration::from_secs(120);

/// Launch the background reconciliation task. This should be spawned with
/// `tokio::spawn` before the gRPC server starts serving.
pub async fn run_reconciler(state: DaemonState) {
    info!(
        "Reconciliation loop started (interval={:?})",
        RECONCILE_INTERVAL
    );
    loop {
        if let Err(e) = reconcile_once(&state).await {
            error!(error = %e, "Reconciliation pass failed");
        }
        sleep(RECONCILE_INTERVAL).await;
    }
}

/// Single reconciliation pass: finds all clusters needing work and acts on them.
async fn reconcile_once(state: &DaemonState) -> Result<(), Box<dyn std::error::Error>> {
    let clusters = list_clusters(&state.db).await?;

    for cluster in clusters {
        match cluster.status.state {
            ClusterState::Pending => {
                if let Err(e) = reconcile_pending(state, cluster).await {
                    error!(error = %e, "Failed to reconcile Pending cluster");
                }
            }
            ClusterState::Creating => {
                if let Err(e) = reconcile_creating(state, cluster).await {
                    error!(error = %e, "Failed to reconcile Creating cluster");
                }
            }
            ClusterState::Bootstrapping => {
                if let Err(e) = reconcile_bootstrapping(state, cluster).await {
                    error!(error = %e, "Failed to reconcile Bootstrapping cluster");
                }
            }
            // Terminal / stable states — nothing to do.
            ClusterState::Running
            | ClusterState::Stopped
            | ClusterState::Stopping
            | ClusterState::Deleting
            | ClusterState::Deleted
            | ClusterState::Failed => {}
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Pending → Creating: create node records and provision containers
// ---------------------------------------------------------------------------

async fn reconcile_pending(
    state: &DaemonState,
    mut cluster: Cluster,
) -> Result<(), Box<dyn std::error::Error>> {
    info!(cluster = %cluster.spec.name, "Reconciling Pending cluster → Creating");

    // Transition state
    cluster.status.state = ClusterState::Creating;
    save_cluster(&state.db, &cluster).await?;

    // Build the desired node list
    let mut nodes: Vec<Node> = Vec::new();

    for i in 0..cluster.spec.control_plane_nodes {
        let name = format!("{}-cp-{}", cluster.spec.name, i);
        let node = Node::new(
            NodeId::generate(),
            cluster.id.clone(),
            &name,
            NodeRole::ControlPlane,
        );
        nodes.push(node);
    }
    for i in 0..cluster.spec.worker_nodes {
        let name = format!("{}-worker-{}", cluster.spec.name, i);
        let node = Node::new(
            NodeId::generate(),
            cluster.id.clone(),
            &name,
            NodeRole::Worker,
        );
        nodes.push(node);
    }

    // Persist node records
    for node in &nodes {
        save_node(&state.db, node).await?;
    }

    // Provision containers (control-plane first, then workers)
    for mut node in nodes {
        info!(node = %node.name, role = %node.role, "Provisioning node container");

        match state.runtime.create_node(&cluster, &node).await {
            Ok(container_id) => {
                node.status.container_id = Some(container_id.clone());
                match state.runtime.start_node(&container_id).await {
                    Ok(()) => {
                        node.status.state = NodeState::Ready;
                        info!(node = %node.name, container_id = %container_id, "Node container started");
                    }
                    Err(e) => {
                        warn!(node = %node.name, error = %e, "Failed to start node container");
                        node.status.state = NodeState::Failed;
                    }
                }
            }
            Err(e) => {
                warn!(node = %node.name, error = %e, "Failed to create node container");
                node.status.state = NodeState::Failed;
            }
        }

        save_node(&state.db, &node).await?;
    }

    info!(cluster = %cluster.spec.name, "All node containers provisioned, advancing to Bootstrapping");
    cluster.status.state = ClusterState::Bootstrapping;
    save_cluster(&state.db, &cluster).await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Creating → Bootstrapping: advance clusters stuck mid-flight
// ---------------------------------------------------------------------------

async fn reconcile_creating(
    state: &DaemonState,
    mut cluster: Cluster,
) -> Result<(), Box<dyn std::error::Error>> {
    // Creating is transient — advance to Bootstrapping so the bootstrapping
    // handler can inspect containers and heal any that didn't start.
    info!(cluster = %cluster.spec.name, "Cluster stuck in Creating, advancing to Bootstrapping");
    cluster.status.state = ClusterState::Bootstrapping;
    save_cluster(&state.db, &cluster).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Bootstrapping → Running: heal partially-started clusters, then verify
// ---------------------------------------------------------------------------

async fn reconcile_bootstrapping(
    state: &DaemonState,
    mut cluster: Cluster,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut nodes = list_nodes(&state.db, &cluster.id).await?;

    if nodes.is_empty() {
        warn!(cluster = %cluster.spec.name, "Bootstrapping cluster has no nodes; marking Failed");
        cluster.status.state = ClusterState::Failed;
        save_cluster(&state.db, &cluster).await?;
        return Ok(());
    }

    let total = nodes.len();
    let mut running = 0usize;
    let mut failed = 0usize;

    for node in &mut nodes {
        if let Some(ref container_id) = node.status.container_id {
            match state.runtime.inspect_node(container_id).await {
                Ok(info) => {
                    if let Some(ip) = info.ip_address {
                        node.status.internal_ip = Some(ip);
                    }
                    if info.is_running {
                        node.status.state = NodeState::Ready;
                        running += 1;
                    } else {
                        // Container exists but is not running — try to start it (recovery path)
                        info!(node = %node.name, container_id, "Container exists but stopped — attempting start");
                        match state.runtime.start_node(container_id).await {
                            Ok(()) => {
                                info!(node = %node.name, "Node container started (recovery)");
                                node.status.state = NodeState::Ready;
                                running += 1;
                            }
                            Err(e) => {
                                warn!(node = %node.name, error = %e, "Recovery start failed");
                                failed += 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(node = %node.name, error = %e, "Could not inspect node container");
                    failed += 1;
                }
            }
        } else {
            // No container at all — provision it now (recovery for missing nodes)
            info!(node = %node.name, role = %node.role, "No container found — provisioning missing node");
            match state.runtime.create_node(&cluster, node).await {
                Ok(container_id) => {
                    node.status.container_id = Some(container_id.clone());
                    match state.runtime.start_node(&container_id).await {
                        Ok(()) => {
                            node.status.state = NodeState::Ready;
                            info!(node = %node.name, container_id, "Missing node provisioned and started");
                            running += 1;
                        }
                        Err(e) => {
                            warn!(node = %node.name, error = %e, "Failed to start recovered node");
                            node.status.state = NodeState::Failed;
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    warn!(node = %node.name, error = %e, "Failed to provision missing node");
                    node.status.state = NodeState::Failed;
                    failed += 1;
                }
            }
        }
        save_node(&state.db, node).await?;
    }

    info!(
        cluster = %cluster.spec.name,
        running,
        failed,
        total,
        "Bootstrap health check"
    );

    if running == total {
        info!(cluster = %cluster.spec.name, "All nodes running, starting Kubernetes cluster bootstrap");

        let cp_node = nodes.iter().find(|n| n.role.is_control_plane()).cloned();
        let Some(cp_node) = cp_node else {
            error!(cluster = %cluster.spec.name, "No control plane node found");
            cluster.status.state = ClusterState::Failed;
            save_cluster(&state.db, &cluster).await?;
            return Ok(());
        };

        let Some(cp_container_id) = cp_node.status.container_id.as_deref() else {
            error!(cluster = %cluster.spec.name, "Control plane container ID missing");
            cluster.status.state = ClusterState::Failed;
            save_cluster(&state.db, &cluster).await?;
            return Ok(());
        };

        let cp_info = state.runtime.inspect_node(cp_container_id).await?;
        let Some(cp_ip) = cp_info.ip_address else {
            error!(cluster = %cluster.spec.name, "Control plane has no IP address");
            return Ok(());
        };

        let host_port = cp_info
            .port_mappings
            .iter()
            .find(|(c_port, _)| *c_port == 6443)
            .map(|(_, h_port)| *h_port)
            .unwrap_or(6443);

        let bootstrapper = ClusterBootstrapper::new(state.runtime.clone());
        info!(cluster = %cluster.spec.name, cp_container_id, "Executing kubeadm init on control plane");

        let kubeconfig = match bootstrapper
            .init_control_plane(cp_container_id, &cluster.spec, &cp_ip.to_string(), host_port)
            .await
        {
            Ok(cfg) => cfg,
            Err(e) => {
                error!(cluster = %cluster.spec.name, error = %e, "Control plane bootstrap failed");
                cluster.status.state = ClusterState::Failed;
                cluster.status.message = Some(format!("Kubeadm init failed: {e}"));
                save_cluster(&state.db, &cluster).await?;
                return Ok(());
            }
        };

        // Write cluster kubeconfig to ~/.kube/orchestra-{cluster_name}.conf
        if let Some(home) = std::env::var_os("HOME") {
            let kube_dir = std::path::PathBuf::from(home).join(".kube");
            if let Err(e) = tokio::fs::create_dir_all(&kube_dir).await {
                warn!(error = %e, "Failed to create ~/.kube directory");
            } else {
                let config_path = kube_dir.join(format!("orchestra-{}.conf", cluster.spec.name));
                if let Err(e) = tokio::fs::write(&config_path, &kubeconfig).await {
                    warn!(error = %e, path = ?config_path, "Failed to write kubeconfig file");
                } else {
                    info!(path = ?config_path, "Saved kubeconfig file");
                    let _ = tokio::fs::write(kube_dir.join("config"), &kubeconfig).await;
                }
            }
        }

        // Join worker nodes
        let (code, join_output) = state
            .runtime
            .exec_in_node(cp_container_id, &["kubeadm", "token", "create", "--print-join-command"])
            .await?;

        if code == 0 {
            let join_cmd = join_output.trim();
            for worker in nodes.iter().filter(|n| n.role.is_worker()) {
                if let Some(ref w_cid) = worker.status.container_id {
                    info!(worker = %worker.name, container_id = %w_cid, "Joining worker to cluster");
                    if let Err(e) = bootstrapper.join_worker(w_cid, join_cmd).await {
                        warn!(worker = %worker.name, error = %e, "Worker failed to join cluster");
                    }
                }
            }
        } else {
            warn!(output = %join_output, "Failed to generate join command from control plane");
        }

        cluster.status.state = ClusterState::Running;
        cluster.status.api_endpoint = Some(format!("https://127.0.0.1:{host_port}"));
        cluster.status.message = None;
        save_cluster(&state.db, &cluster).await?;
        info!(cluster = %cluster.spec.name, "Cluster bootstrap complete — state is Running");
    } else if failed > 0 {
        error!(cluster = %cluster.spec.name, failed, "Cluster bootstrap failed");
        cluster.status.state = ClusterState::Failed;
        save_cluster(&state.db, &cluster).await?;
    }

    Ok(())
}

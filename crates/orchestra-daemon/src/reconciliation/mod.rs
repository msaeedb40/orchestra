//! Continuous reconciliation loops for desired vs observed state.
//!
//! The reconciler polls for `Pending` clusters and drives them through the
//! full lifecycle: Creating → Bootstrapping → Running.

use crate::state::DaemonState;
use orchestra_common::identifiers::NodeId;
use orchestra_core::cluster::{Cluster, ClusterState};
use orchestra_core::node::{Node, NodeRole, NodeState};
use orchestra_core::traits::RuntimeProvider;
use orchestra_storage::repository::cluster::{list_clusters, save_cluster};
use orchestra_storage::repository::node::{list_nodes, save_node};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Interval between reconciliation passes.
const RECONCILE_INTERVAL: Duration = Duration::from_secs(5);

/// Maximum time to wait for a node container to become Running before marking Failed.
const NODE_START_TIMEOUT: Duration = Duration::from_secs(120);

/// Launch the background reconciliation task. This should be spawned with
/// `tokio::spawn` before the gRPC server starts serving.
pub async fn run_reconciler(state: DaemonState) {
    info!("Reconciliation loop started (interval={:?})", RECONCILE_INTERVAL);
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
        let node = Node::new(NodeId::generate(), cluster.id.clone(), &name, NodeRole::ControlPlane);
        nodes.push(node);
    }
    for i in 0..cluster.spec.worker_nodes {
        let name = format!("{}-worker-{}", cluster.spec.name, i);
        let node = Node::new(NodeId::generate(), cluster.id.clone(), &name, NodeRole::Worker);
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
    let nodes = list_nodes(&state.db, &cluster.id).await?;

    if nodes.is_empty() {
        warn!(cluster = %cluster.spec.name, "Bootstrapping cluster has no nodes; marking Failed");
        cluster.status.state = ClusterState::Failed;
        save_cluster(&state.db, &cluster).await?;
        return Ok(());
    }

    let total = nodes.len();
    let mut running = 0usize;
    let mut failed = 0usize;

    for mut node in nodes {
        if let Some(ref container_id) = node.status.container_id {
            match state.runtime.inspect_node(container_id).await {
                Ok(info) => {
                    if info.is_running {
                        running += 1;
                    } else {
                        // Container exists but is not running — try to start it (recovery path)
                        info!(node = %node.name, container_id, "Container exists but stopped — attempting start");
                        match state.runtime.start_node(container_id).await {
                            Ok(()) => {
                                info!(node = %node.name, "Node container started (recovery)");
                                node.status.state = NodeState::Ready;
                                save_node(&state.db, &node).await?;
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
            match state.runtime.create_node(&cluster, &node).await {
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
            save_node(&state.db, &node).await?;
        }
    }

    info!(
        cluster = %cluster.spec.name,
        running,
        failed,
        total,
        "Bootstrap health check"
    );

    if running == total {
        info!(cluster = %cluster.spec.name, "All nodes Running — cluster is Ready");
        cluster.status.state = ClusterState::Running;
        save_cluster(&state.db, &cluster).await?;
    } else if failed > 0 {
        error!(cluster = %cluster.spec.name, failed, "Cluster bootstrap failed");
        cluster.status.state = ClusterState::Failed;
        save_cluster(&state.db, &cluster).await?;
    }

    Ok(())
}

// Suppress unused warning until NODE_START_TIMEOUT is wired to a future
// timeout-aware provisioning path.
const _: Duration = NODE_START_TIMEOUT;

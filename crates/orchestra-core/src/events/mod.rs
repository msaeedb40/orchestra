//! Domain event types emitted during cluster lifecycle and reconciliation.

use crate::cluster::ClusterState;
use crate::node::{NodeRole, NodeState};
use chrono::{DateTime, Utc};
use orchestra_common::identifiers::{ClusterId, NetworkId, NodeId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Top-level wrapped domain event with envelope metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainEvent {
    /// Unique event identifier.
    pub id: Uuid,
    /// UTC timestamp of event emission.
    pub timestamp: DateTime<Utc>,
    /// Associated cluster identifier if applicable.
    pub cluster_id: Option<ClusterId>,
    /// Concrete event payload.
    pub payload: EventPayload,
}

impl DomainEvent {
    /// Constructs a new domain event with current timestamp and random UUID.
    pub fn new(cluster_id: Option<ClusterId>, payload: EventPayload) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            cluster_id,
            payload,
        }
    }
}

/// Sum type of all domain event payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum EventPayload {
    Cluster(ClusterEvent),
    Node(NodeEvent),
    Network(NetworkEvent),
    Ebpf(EbpfEvent),
    System(SystemEvent),
}

/// Events related to cluster lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ClusterEvent {
    Created {
        cluster_id: ClusterId,
        name: String,
    },
    StateChanged {
        cluster_id: ClusterId,
        old_state: ClusterState,
        new_state: ClusterState,
    },
    BootstrapStarted {
        cluster_id: ClusterId,
    },
    BootstrapCompleted {
        cluster_id: ClusterId,
        endpoint: String,
    },
    Deleted {
        cluster_id: ClusterId,
    },
    Failed {
        cluster_id: ClusterId,
        reason: String,
    },
}

/// Events related to node containers and health.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum NodeEvent {
    Created {
        node_id: NodeId,
        cluster_id: ClusterId,
        name: String,
        role: NodeRole,
    },
    StateChanged {
        node_id: NodeId,
        old_state: NodeState,
        new_state: NodeState,
    },
    KubeletReady {
        node_id: NodeId,
    },
    DrainStarted {
        node_id: NodeId,
    },
    DrainCompleted {
        node_id: NodeId,
    },
    Failed {
        node_id: NodeId,
        reason: String,
    },
}

/// Events related to network bridges, veth pairs, and routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum NetworkEvent {
    BridgeCreated {
        network_id: NetworkId,
        bridge_name: String,
    },
    NodeAttached {
        network_id: NetworkId,
        node_id: NodeId,
        ip_address: String,
    },
    CniConfigured {
        cluster_id: ClusterId,
        plugin: String,
    },
    BridgeDeleted {
        network_id: NetworkId,
    },
}

/// Events related to eBPF program lifecycle and dataplane hooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum EbpfEvent {
    ProgramLoaded {
        program_name: String,
    },
    ProgramAttached {
        program_name: String,
        target_interface: String,
    },
    ProgramDetached {
        program_name: String,
        target_interface: String,
    },
    MapUpdated {
        map_name: String,
        entries_count: usize,
    },
}

/// System-level and operational heartbeat events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum SystemEvent {
    DaemonStarted { version: String },
    DaemonStopping,
    ReconcileCompleted { duration_ms: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serde() {
        let cluster_id = ClusterId::generate();
        let event = DomainEvent::new(
            Some(cluster_id.clone()),
            EventPayload::Cluster(ClusterEvent::Created {
                cluster_id: cluster_id.clone(),
                name: "dev-cluster".into(),
            }),
        );

        let json = serde_json::to_string(&event).unwrap();
        let deserialized: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, deserialized);
    }
}

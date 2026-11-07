//! Node repository CRUD implementation for SQLite.

use crate::database::Database;
use chrono::Utc;
use orchestra_common::identifiers::{ClusterId, NodeId};
use orchestra_core::error::StorageError;
use orchestra_core::node::{Node, NodeRole, NodeStatus};

/// Saves or updates a node entity record.
pub async fn save_node(db: &Database, node: &Node) -> Result<(), StorageError> {
    let id_str = node.id.to_string();
    let cluster_id_str = node.cluster_id.to_string();
    let name = node.name.clone();
    let role_str = node.role.to_string();
    let state_str = node.status.state.to_string();
    let container_id = node.status.container_id.clone();
    let internal_ip = node.status.internal_ip.map(|ip| ip.to_string());
    let status_json = serde_json::to_string(&node.status)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let now = Utc::now().to_rfc3339();

    db.interact(move |conn| {
        conn.execute(
            r#"
            INSERT INTO nodes (
                id, cluster_id, name, role, state, container_id, internal_ip, status_json, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
            ON CONFLICT(id) DO UPDATE SET
                role = excluded.role,
                state = excluded.state,
                container_id = excluded.container_id,
                internal_ip = excluded.internal_ip,
                status_json = excluded.status_json,
                updated_at = excluded.updated_at;
            "#,
            (
                id_str,
                cluster_id_str,
                name,
                role_str,
                state_str,
                container_id,
                internal_ip,
                status_json,
                now,
            ),
        )
        .map_err(|e| StorageError::Database(format!("Failed to save node: {}", e)))?;
        Ok(())
    })
    .await
}

/// Retrieves a node by its unique ID.
pub async fn get_node(db: &Database, id: &NodeId) -> Result<Option<Node>, StorageError> {
    let id_str = id.to_string();
    db.interact(move |conn| {
        let mut stmt = conn
            .prepare("SELECT id, cluster_id, name, role, status_json FROM nodes WHERE id = ?1;")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut rows = stmt
            .query([id_str])
            .map_err(|e| StorageError::Database(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| StorageError::Database(e.to_string()))?
        {
            let node_id_str: String = row
                .get(0)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let cluster_id_str: String = row
                .get(1)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let name: String = row
                .get(2)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let role_str: String = row
                .get(3)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let status_json: String = row
                .get(4)
                .map_err(|e| StorageError::Database(e.to_string()))?;

            let role = if role_str == "control-plane" {
                NodeRole::ControlPlane
            } else {
                NodeRole::Worker
            };
            let status: NodeStatus = serde_json::from_str(&status_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;

            Ok(Some(Node {
                id: NodeId::new(node_id_str),
                cluster_id: ClusterId::new(cluster_id_str),
                name,
                role,
                status,
            }))
        } else {
            Ok(None)
        }
    })
    .await
}

/// Lists all nodes for a specific cluster.
pub async fn list_nodes(db: &Database, cluster_id: &ClusterId) -> Result<Vec<Node>, StorageError> {
    let cluster_id_str = cluster_id.to_string();
    db.interact(move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, cluster_id, name, role, status_json FROM nodes WHERE cluster_id = ?1 ORDER BY name ASC;",
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([cluster_id_str], |row| {
                let id: String = row.get(0)?;
                let cluster_id: String = row.get(1)?;
                let name: String = row.get(2)?;
                let role_str: String = row.get(3)?;
                let status_json: String = row.get(4)?;
                Ok((id, cluster_id, name, role_str, status_json))
            })
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut nodes = Vec::new();
        for item in rows {
            let (id_str, cid_str, name, role_str, status_json) =
                item.map_err(|e| StorageError::Database(e.to_string()))?;

            let role = if role_str == "control-plane" {
                NodeRole::ControlPlane
            } else {
                NodeRole::Worker
            };
            let status: NodeStatus = serde_json::from_str(&status_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;

            nodes.push(Node {
                id: NodeId::new(id_str),
                cluster_id: ClusterId::new(cid_str),
                name,
                role,
                status,
            });
        }

        Ok(nodes)
    })
    .await
}

/// Deletes a node record by its ID.
pub async fn delete_node(db: &Database, id: &NodeId) -> Result<(), StorageError> {
    let id_str = id.to_string();
    db.interact(move |conn| {
        conn.execute("DELETE FROM nodes WHERE id = ?1;", [id_str])
            .map_err(|e| StorageError::Database(format!("Failed to delete node: {}", e)))?;
        Ok(())
    })
    .await
}

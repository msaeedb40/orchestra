//! Cluster repository CRUD implementation for SQLite.

use crate::database::Database;
use chrono::Utc;
use orchestra_common::identifiers::ClusterId;
use orchestra_core::cluster::{Cluster, ClusterSpec, ClusterStatus};
use orchestra_core::error::StorageError;

/// Saves or updates a cluster record.
pub async fn save_cluster(db: &Database, cluster: &Cluster) -> Result<(), StorageError> {
    let id_str = cluster.id.to_string();
    let name = cluster.name().to_string();
    let spec_json = serde_json::to_string(&cluster.spec)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let status_json = serde_json::to_string(&cluster.status)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let state_str = cluster.status.state.to_string();
    let now = Utc::now().to_rfc3339();

    db.interact(move |conn| {
        conn.execute(
            r#"
            INSERT INTO clusters (id, name, spec_json, status_json, state, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                spec_json = excluded.spec_json,
                status_json = excluded.status_json,
                state = excluded.state,
                updated_at = excluded.updated_at;
            "#,
            (id_str, name, spec_json, status_json, state_str, now),
        )
        .map_err(|e| StorageError::Database(format!("Failed to save cluster: {}", e)))?;
        Ok(())
    })
    .await
}

/// Retrieves a cluster by its unique ID.
pub async fn get_cluster(db: &Database, id: &ClusterId) -> Result<Option<Cluster>, StorageError> {
    let id_str = id.to_string();
    db.interact(move |conn| {
        let mut stmt = conn
            .prepare("SELECT id, spec_json, status_json FROM clusters WHERE id = ?1;")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut rows = stmt
            .query([id_str])
            .map_err(|e| StorageError::Database(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| StorageError::Database(e.to_string()))?
        {
            let cluster_id_str: String = row
                .get(0)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let spec_json: String = row
                .get(1)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let status_json: String = row
                .get(2)
                .map_err(|e| StorageError::Database(e.to_string()))?;

            let spec: ClusterSpec = serde_json::from_str(&spec_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            let status: ClusterStatus = serde_json::from_str(&status_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;

            Ok(Some(Cluster {
                id: ClusterId::new(cluster_id_str),
                spec,
                status,
            }))
        } else {
            Ok(None)
        }
    })
    .await
}

/// Retrieves a cluster by its unique human-friendly name.
pub async fn get_cluster_by_name(
    db: &Database,
    name: &str,
) -> Result<Option<Cluster>, StorageError> {
    let name_str = name.to_string();
    db.interact(move |conn| {
        let mut stmt = conn
            .prepare("SELECT id, spec_json, status_json FROM clusters WHERE name = ?1;")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut rows = stmt
            .query([name_str])
            .map_err(|e| StorageError::Database(e.to_string()))?;

        if let Some(row) = rows
            .next()
            .map_err(|e| StorageError::Database(e.to_string()))?
        {
            let cluster_id_str: String = row
                .get(0)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let spec_json: String = row
                .get(1)
                .map_err(|e| StorageError::Database(e.to_string()))?;
            let status_json: String = row
                .get(2)
                .map_err(|e| StorageError::Database(e.to_string()))?;

            let spec: ClusterSpec = serde_json::from_str(&spec_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            let status: ClusterStatus = serde_json::from_str(&status_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;

            Ok(Some(Cluster {
                id: ClusterId::new(cluster_id_str),
                spec,
                status,
            }))
        } else {
            Ok(None)
        }
    })
    .await
}

/// Lists all cluster records in the database.
pub async fn list_clusters(db: &Database) -> Result<Vec<Cluster>, StorageError> {
    db.interact(|conn| {
        let mut stmt = conn
            .prepare("SELECT id, spec_json, status_json FROM clusters ORDER BY created_at ASC;")
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let spec_json: String = row.get(1)?;
                let status_json: String = row.get(2)?;
                Ok((id, spec_json, status_json))
            })
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut clusters = Vec::new();
        for item in rows {
            let (id_str, spec_json, status_json) =
                item.map_err(|e| StorageError::Database(e.to_string()))?;
            let spec: ClusterSpec = serde_json::from_str(&spec_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            let status: ClusterStatus = serde_json::from_str(&status_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            clusters.push(Cluster {
                id: ClusterId::new(id_str),
                spec,
                status,
            });
        }

        Ok(clusters)
    })
    .await
}

/// Deletes a cluster and its cascaded child entities.
pub async fn delete_cluster(db: &Database, id: &ClusterId) -> Result<(), StorageError> {
    let id_str = id.to_string();
    db.interact(move |conn| {
        conn.execute("DELETE FROM clusters WHERE id = ?1;", [id_str])
            .map_err(|e| StorageError::Database(format!("Failed to delete cluster: {}", e)))?;
        Ok(())
    })
    .await
}

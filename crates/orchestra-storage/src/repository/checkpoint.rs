//! Reconciliation checkpoints, drift observations, and lock tokens.

use crate::database::Database;
use chrono::{DateTime, Utc};
use orchestra_common::identifiers::ClusterId;
use orchestra_core::error::StorageError;
use serde::{Deserialize, Serialize};

/// Checkpoint tracking observed reconciliation state and optimistic lock tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub cluster_id: ClusterId,
    pub observed_generation: i64,
    pub lock_token: Option<String>,
    pub drift_detected: bool,
    pub last_reconciled_at: DateTime<Utc>,
    pub state_summary: String,
}

/// Saves or updates a cluster reconciliation checkpoint.
pub async fn save_checkpoint(db: &Database, cp: &Checkpoint) -> Result<(), StorageError> {
    let cluster_id_str = cp.cluster_id.to_string();
    let generation = cp.observed_generation;
    let lock_token = cp.lock_token.clone();
    let drift = if cp.drift_detected { 1 } else { 0 };
    let last_reconciled = cp.last_reconciled_at.to_rfc3339();
    let summary = cp.state_summary.clone();

    db.interact(move |conn| {
        conn.execute(
            r#"
            INSERT INTO checkpoints (
                cluster_id, observed_generation, lock_token, drift_detected, last_reconciled_at, state_summary_json
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(cluster_id) DO UPDATE SET
                observed_generation = excluded.observed_generation,
                lock_token = excluded.lock_token,
                drift_detected = excluded.drift_detected,
                last_reconciled_at = excluded.last_reconciled_at,
                state_summary_json = excluded.state_summary_json;
            "#,
            (cluster_id_str, generation, lock_token, drift, last_reconciled, summary),
        )
        .map_err(|e| StorageError::Database(format!("Failed to save checkpoint: {}", e)))?;
        Ok(())
    })
    .await
}

/// Retrieves the checkpoint for a cluster.
pub async fn get_checkpoint(
    db: &Database,
    cluster_id: &ClusterId,
) -> Result<Option<Checkpoint>, StorageError> {
    let cluster_id_str = cluster_id.to_string();
    db.interact(move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT cluster_id, observed_generation, lock_token, drift_detected, last_reconciled_at, state_summary_json FROM checkpoints WHERE cluster_id = ?1;",
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut rows = stmt
            .query([cluster_id_str])
            .map_err(|e| StorageError::Database(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| StorageError::Database(e.to_string()))? {
            let cid: String = row.get(0).map_err(|e| StorageError::Database(e.to_string()))?;
            let generation: i64 = row.get(1).map_err(|e| StorageError::Database(e.to_string()))?;
            let lock: Option<String> = row.get(2).map_err(|e| StorageError::Database(e.to_string()))?;
            let drift: i64 = row.get(3).map_err(|e| StorageError::Database(e.to_string()))?;
            let time_str: String = row.get(4).map_err(|e| StorageError::Database(e.to_string()))?;
            let summary: String = row.get(5).map_err(|e| StorageError::Database(e.to_string()))?;

            let last_reconciled_at = DateTime::parse_from_rfc3339(&time_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Some(Checkpoint {
                cluster_id: ClusterId::new(cid),
                observed_generation: generation,
                lock_token: lock,
                drift_detected: drift != 0,
                last_reconciled_at,
                state_summary: summary,
            }))
        } else {
            Ok(None)
        }
    })
    .await
}

/// Attempts to acquire an exclusive reconciliation lock token for a cluster.
pub async fn acquire_lock(
    db: &Database,
    cluster_id: &ClusterId,
    token: &str,
) -> Result<bool, StorageError> {
    let cid_str = cluster_id.to_string();
    let token_str = token.to_string();
    let now = Utc::now().to_rfc3339();

    db.interact(move |conn| {
        let rows_affected = conn
            .execute(
                r#"
                INSERT INTO checkpoints (cluster_id, observed_generation, lock_token, drift_detected, last_reconciled_at, state_summary_json)
                VALUES (?1, 0, ?2, 0, ?3, '{}')
                ON CONFLICT(cluster_id) DO UPDATE SET
                    lock_token = ?2,
                    last_reconciled_at = ?3
                WHERE checkpoints.lock_token IS NULL OR checkpoints.lock_token = ?2;
                "#,
                (cid_str, token_str, now),
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(rows_affected > 0)
    })
    .await
}

/// Releases the reconciliation lock token if it matches the current owner.
pub async fn release_lock(
    db: &Database,
    cluster_id: &ClusterId,
    token: &str,
) -> Result<bool, StorageError> {
    let cid_str = cluster_id.to_string();
    let token_str = token.to_string();

    db.interact(move |conn| {
        let rows_affected = conn
            .execute(
                "UPDATE checkpoints SET lock_token = NULL WHERE cluster_id = ?1 AND lock_token = ?2;",
                (cid_str, token_str),
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(rows_affected > 0)
    })
    .await
}

//! Operation journal repository implementation for SQLite.

use crate::database::Database;
use chrono::Utc;
use orchestra_common::identifiers::{ClusterId, OperationId};
use orchestra_core::error::StorageError;
use orchestra_core::operation::{Operation, OperationStage, OperationStatus, OperationType};

/// Saves or updates an asynchronous operation journal record.
pub async fn save_operation(db: &Database, op: &Operation) -> Result<(), StorageError> {
    let id_str = op.id.to_string();
    let cluster_id_str = op.cluster_id.as_ref().map(|c| c.to_string());
    let op_type_json = serde_json::to_string(&op.op_type)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let status_str = match op.status {
        OperationStatus::Pending => "pending",
        OperationStatus::InProgress => "inprogress",
        OperationStatus::Completed => "completed",
        OperationStatus::Failed => "failed",
        OperationStatus::Cancelled => "cancelled",
    };
    let progress = op.progress_percent as i64;
    let stages_json = serde_json::to_string(&op.stages)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let error_message = op.error_message.clone();
    let now = Utc::now().to_rfc3339();

    db.interact(move |conn| {
        conn.execute(
            r#"
            INSERT INTO operations (
                id, cluster_id, op_type, status, progress_percent, stages_json, error_message, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                progress_percent = excluded.progress_percent,
                stages_json = excluded.stages_json,
                error_message = excluded.error_message,
                updated_at = excluded.updated_at;
            "#,
            (
                id_str,
                cluster_id_str,
                op_type_json,
                status_str,
                progress,
                stages_json,
                error_message,
                now,
            ),
        )
        .map_err(|e| StorageError::Database(format!("Failed to save operation: {}", e)))?;
        Ok(())
    })
    .await
}

/// Retrieves an operation record by its ID.
pub async fn get_operation(
    db: &Database,
    id: &OperationId,
) -> Result<Option<Operation>, StorageError> {
    let id_str = id.to_string();
    db.interact(move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, cluster_id, op_type, status, progress_percent, stages_json, error_message, created_at, updated_at FROM operations WHERE id = ?1;",
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut rows = stmt
            .query([id_str])
            .map_err(|e| StorageError::Database(e.to_string()))?;

        if let Some(row) = rows.next().map_err(|e| StorageError::Database(e.to_string()))? {
            map_operation_row(row)
                .map(Some)
                .map_err(|e| StorageError::Database(e.to_string()))
        } else {
            Ok(None)
        }
    })
    .await
}

/// Lists all operations for a specific cluster ordered from newest to oldest.
pub async fn list_operations(
    db: &Database,
    cluster_id: &ClusterId,
) -> Result<Vec<Operation>, StorageError> {
    let cluster_id_str = cluster_id.to_string();
    db.interact(move |conn| {
        let mut stmt = conn
            .prepare(
                "SELECT id, cluster_id, op_type, status, progress_percent, stages_json, error_message, created_at, updated_at FROM operations WHERE cluster_id = ?1 ORDER BY created_at DESC;",
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([cluster_id_str], map_operation_row)
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut ops = Vec::new();
        for item in rows {
            ops.push(item.map_err(|e| StorageError::Database(e.to_string()))?);
        }
        Ok(ops)
    })
    .await
}

fn map_operation_row(row: &rusqlite::Row<'_>) -> Result<Operation, rusqlite::Error> {
    let id_str: String = row.get(0)?;
    let cluster_id_opt: Option<String> = row.get(1)?;
    let op_type_json: String = row.get(2)?;
    let status_str: String = row.get(3)?;
    let progress: i64 = row.get(4)?;
    let stages_json: String = row.get(5)?;
    let error_message: Option<String> = row.get(6)?;
    let created_at_str: String = row.get(7)?;
    let updated_at_str: String = row.get(8)?;

    let op_type: OperationType = serde_json::from_str(&op_type_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let status = match status_str.as_str() {
        "pending" => OperationStatus::Pending,
        "inprogress" => OperationStatus::InProgress,
        "completed" => OperationStatus::Completed,
        "failed" => OperationStatus::Failed,
        "cancelled" => OperationStatus::Cancelled,
        _ => OperationStatus::Pending,
    };

    let stages: Vec<OperationStage> = serde_json::from_str(&stages_json).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    let current_stage_index = stages
        .iter()
        .position(|s| s.status == OperationStatus::InProgress);

    Ok(Operation {
        id: OperationId::new(id_str),
        cluster_id: cluster_id_opt.map(ClusterId::new),
        op_type,
        status,
        progress_percent: progress as u8,
        stages,
        current_stage_index,
        error_message,
        created_at,
        updated_at,
    })
}

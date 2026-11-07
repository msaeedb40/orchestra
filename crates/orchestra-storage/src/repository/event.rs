//! Domain event store repository implementation for SQLite.

use crate::database::Database;
use orchestra_common::identifiers::ClusterId;
use orchestra_core::error::StorageError;
use orchestra_core::events::DomainEvent;

/// Records an immutable domain event to the append-only event store.
pub async fn record_event(db: &Database, event: &DomainEvent) -> Result<(), StorageError> {
    let id_str = event.id.to_string();
    let cluster_id_str = event.cluster_id.as_ref().map(|c| c.to_string());
    let event_type = match &event.payload {
        orchestra_core::events::EventPayload::Cluster(_) => "cluster",
        orchestra_core::events::EventPayload::Node(_) => "node",
        orchestra_core::events::EventPayload::Network(_) => "network",
        orchestra_core::events::EventPayload::Ebpf(_) => "ebpf",
        orchestra_core::events::EventPayload::System(_) => "system",
    };
    let payload_json = serde_json::to_string(&event.payload)
        .map_err(|e| StorageError::Serialization(e.to_string()))?;
    let timestamp_str = event.timestamp.to_rfc3339();

    db.interact(move |conn| {
        conn.execute(
            r#"
            INSERT INTO domain_events (id, cluster_id, event_type, payload_json, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5);
            "#,
            (
                id_str,
                cluster_id_str,
                event_type,
                payload_json,
                timestamp_str,
            ),
        )
        .map_err(|e| StorageError::Database(format!("Failed to record domain event: {}", e)))?;
        Ok(())
    })
    .await
}

/// Lists recent domain events, optionally filtered by cluster ID, ordered newest first.
pub async fn list_events(
    db: &Database,
    cluster_id: Option<&ClusterId>,
    limit: usize,
) -> Result<Vec<DomainEvent>, StorageError> {
    let cluster_id_opt = cluster_id.map(|c| c.to_string());
    let limit_i64 = limit as i64;

    db.interact(move |conn| {
        let (sql, params): (&str, Vec<rusqlite::types::Value>) = if let Some(cid) = cluster_id_opt {
            (
                "SELECT id, cluster_id, payload_json, timestamp FROM domain_events WHERE cluster_id = ?1 ORDER BY timestamp DESC LIMIT ?2;",
                vec![cid.into(), limit_i64.into()],
            )
        } else {
            (
                "SELECT id, cluster_id, payload_json, timestamp FROM domain_events ORDER BY timestamp DESC LIMIT ?1;",
                vec![limit_i64.into()],
            )
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(rusqlite::params_from_iter(params), |row| {
                let id_str: String = row.get(0)?;
                let cid_str: Option<String> = row.get(1)?;
                let payload_json: String = row.get(2)?;
                let timestamp_str: String = row.get(3)?;
                Ok((id_str, cid_str, payload_json, timestamp_str))
            })
            .map_err(|e| StorageError::Database(e.to_string()))?;

        let mut events = Vec::new();
        for item in rows {
            let (id_str, cid_str, payload_json, timestamp_str) =
                item.map_err(|e| StorageError::Database(e.to_string()))?;

            let id = uuid::Uuid::parse_str(&id_str)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            let payload = serde_json::from_str(&payload_json)
                .map_err(|e| StorageError::Serialization(e.to_string()))?;
            let timestamp = chrono::DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now());

            events.push(DomainEvent {
                id,
                timestamp,
                cluster_id: cid_str.map(ClusterId::new),
                payload,
            });
        }

        Ok(events)
    })
    .await
}

//! Embedded schema migrations engine for Orchestra SQLite storage.

use crate::database::Database;
use chrono::Utc;
use orchestra_core::error::StorageError;
use rusqlite::Connection;
use tracing::info;

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial_schema",
    sql: r#"
        -- Clusters aggregate table
        CREATE TABLE IF NOT EXISTS clusters (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            spec_json TEXT NOT NULL,
            status_json TEXT NOT NULL,
            state TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_clusters_name ON clusters(name);
        CREATE INDEX IF NOT EXISTS idx_clusters_state ON clusters(state);

        -- Node entities table
        CREATE TABLE IF NOT EXISTS nodes (
            id TEXT PRIMARY KEY,
            cluster_id TEXT NOT NULL REFERENCES clusters(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            role TEXT NOT NULL,
            state TEXT NOT NULL,
            container_id TEXT,
            internal_ip TEXT,
            status_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(cluster_id, name)
        );

        CREATE INDEX IF NOT EXISTS idx_nodes_cluster_id ON nodes(cluster_id);
        CREATE INDEX IF NOT EXISTS idx_nodes_state ON nodes(state);

        -- Asynchronous operation journal
        CREATE TABLE IF NOT EXISTS operations (
            id TEXT PRIMARY KEY,
            cluster_id TEXT REFERENCES clusters(id) ON DELETE SET NULL,
            op_type TEXT NOT NULL,
            status TEXT NOT NULL,
            progress_percent INTEGER NOT NULL DEFAULT 0,
            stages_json TEXT NOT NULL,
            error_message TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_operations_cluster_id ON operations(cluster_id);
        CREATE INDEX IF NOT EXISTS idx_operations_status ON operations(status);

        -- Domain event log
        CREATE TABLE IF NOT EXISTS domain_events (
            id TEXT PRIMARY KEY,
            cluster_id TEXT REFERENCES clusters(id) ON DELETE SET NULL,
            event_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_events_cluster_id ON domain_events(cluster_id);
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON domain_events(timestamp);

        -- Reconciliation checkpoints and lock tokens
        CREATE TABLE IF NOT EXISTS checkpoints (
            cluster_id TEXT PRIMARY KEY REFERENCES clusters(id) ON DELETE CASCADE,
            observed_generation INTEGER NOT NULL DEFAULT 0,
            lock_token TEXT,
            drift_detected INTEGER NOT NULL DEFAULT 0,
            last_reconciled_at TEXT NOT NULL,
            state_summary_json TEXT NOT NULL
        );
        "#,
}];

/// Runs all outstanding schema migrations against the database.
pub async fn run_migrations(db: &Database) -> Result<(), StorageError> {
    db.interact(|conn| {
        ensure_migrations_table(conn)?;

        for migration in MIGRATIONS {
            let already_applied: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM _orchestra_migrations WHERE version = ?1);",
                    [migration.version],
                    |row| row.get(0),
                )
                .map_err(|e| StorageError::MigrationFailed(e.to_string()))?;

            if !already_applied {
                info!(
                    version = migration.version,
                    name = migration.name,
                    "Applying database migration"
                );

                let tx = conn
                    .transaction()
                    .map_err(|e| StorageError::MigrationFailed(e.to_string()))?;

                tx.execute_batch(migration.sql)
                    .map_err(|e| StorageError::MigrationFailed(format!(
                        "Failed migration {}: {}",
                        migration.name, e
                    )))?;

                let now = Utc::now().to_rfc3339();
                tx.execute(
                    "INSERT INTO _orchestra_migrations (version, name, applied_at) VALUES (?1, ?2, ?3);",
                    (migration.version, migration.name, now),
                )
                .map_err(|e| StorageError::MigrationFailed(e.to_string()))?;

                tx.commit()
                    .map_err(|e| StorageError::MigrationFailed(e.to_string()))?;
            }
        }

        Ok(())
    })
    .await
}

fn ensure_migrations_table(conn: &mut Connection) -> Result<(), StorageError> {
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS _orchestra_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );
        "#,
        [],
    )
    .map_err(|e| {
        StorageError::MigrationFailed(format!("Failed to create migrations table: {}", e))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_migrations_are_idempotent() {
        let db = Database::open_in_memory().unwrap();

        // Run migrations first time
        run_migrations(&db).await.unwrap();

        // Run migrations second time (idempotency check)
        run_migrations(&db).await.unwrap();

        // Verify tables exist
        db.interact(|conn| {
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM _orchestra_migrations;", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 1);
            Ok(())
        })
        .await
        .unwrap();
    }
}

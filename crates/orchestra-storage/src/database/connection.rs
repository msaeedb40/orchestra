//! Thread-safe SQLite connection pool and lifecycle manager.

use orchestra_common::constants::DEFAULT_DB_FILENAME;
use orchestra_core::error::StorageError;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::{debug, info};

/// Thread-safe SQLite database manager supporting in-memory and persistent on-disk modes.
#[derive(Debug, Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

impl Database {
    /// Opens or creates an on-disk SQLite database at the specified path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path_buf = path.as_ref().to_path_buf();
        if let Some(parent) = path_buf.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                StorageError::Database(format!(
                    "Failed to create database directory '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        info!(path = %path_buf.display(), "Opening SQLite database");
        let conn = Connection::open(&path_buf).map_err(|e| {
            StorageError::Database(format!("Failed to open SQLite database: {}", e))
        })?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: Some(path_buf),
        };
        db.configure_pragmas()?;
        Ok(db)
    }

    /// Creates an ephemeral in-memory SQLite database (ideal for unit testing).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        debug!("Opening in-memory SQLite database");
        let conn = Connection::open_in_memory().map_err(|e| {
            StorageError::Database(format!("Failed to open in-memory SQLite: {}", e))
        })?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: None,
        };
        db.configure_pragmas()?;
        Ok(db)
    }

    /// Opens database at the default location (~/.orchestra/orchestra.db or /var/lib/orchestra).
    pub fn open_default() -> Result<Self, StorageError> {
        let data_dir = orchestra_common::constants::get_default_data_dir();
        let db_path = data_dir.join(DEFAULT_DB_FILENAME);
        Self::open(db_path)
    }

    /// Configures WAL mode, busy timeout, foreign keys, and synchronous pragmas.
    fn configure_pragmas(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().map_err(|e| {
            StorageError::Database(format!("Failed to acquire connection lock: {}", e))
        })?;

        // On-disk databases use WAL mode; in-memory ignores it
        if self.path.is_some() {
            conn.pragma_update(None, "journal_mode", "WAL")
                .map_err(|e| StorageError::Database(format!("Failed to set WAL mode: {}", e)))?;
        }

        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(|e| StorageError::Database(format!("Failed to set synchronous: {}", e)))?;

        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| StorageError::Database(format!("Failed to enable foreign_keys: {}", e)))?;

        conn.busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(|e| StorageError::Database(format!("Failed to set busy timeout: {}", e)))?;

        Ok(())
    }

    /// Executes a closure against the synchronous SQLite connection on a blocking thread pool.
    pub async fn interact<F, R>(&self, f: F) -> Result<R, StorageError>
    where
        F: FnOnce(&mut Connection) -> Result<R, StorageError> + Send + 'static,
        R: Send + 'static,
    {
        let conn_arc = Arc::clone(&self.conn);
        tokio::task::spawn_blocking(move || {
            let mut conn = conn_arc.lock().map_err(|e| {
                StorageError::Database(format!("Failed to lock SQLite connection: {}", e))
            })?;
            f(&mut conn)
        })
        .await
        .map_err(|e| StorageError::Database(format!("Blocking task join error: {}", e)))?
    }

    /// Returns the database file path if on disk.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_database_lifecycle() {
        let db = Database::open_in_memory().unwrap();
        db.interact(|conn| {
            conn.execute(
                "CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT);",
                [],
            )
            .map_err(|e| StorageError::Database(e.to_string()))?;

            conn.execute("INSERT INTO test (value) VALUES (?1);", ["hello"])
                .map_err(|e| StorageError::Database(e.to_string()))?;

            let val: String = conn
                .query_row("SELECT value FROM test WHERE id = 1;", [], |row| row.get(0))
                .map_err(|e| StorageError::Database(e.to_string()))?;

            assert_eq!(val, "hello");
            Ok(())
        })
        .await
        .unwrap();
    }
}

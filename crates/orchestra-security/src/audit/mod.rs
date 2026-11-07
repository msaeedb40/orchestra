//! Audit logging framework for security events and authorization decisions.

use crate::error::SecurityError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Outcome of an audited operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Allowed,
    Denied,
    Error,
}

/// An immutable security audit event record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event identifier.
    pub id: String,
    /// Timestamp when event occurred.
    pub timestamp: DateTime<Utc>,
    /// Identity of the calling entity or process.
    pub actor: String,
    /// Operation requested (e.g. "cluster.delete", "ebpf.attach").
    pub action: String,
    /// Target resource identifier or URI (e.g. "cluster:demo-k8s").
    pub resource: String,
    /// Whether operation succeeded, was denied, or encountered an error.
    pub outcome: AuditOutcome,
    /// Detailed context payload (JSON).
    pub details: serde_json::Value,
}

impl AuditEvent {
    /// Creates a new audit event.
    pub fn new(
        actor: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
        outcome: AuditOutcome,
        details: serde_json::Value,
    ) -> Self {
        Self {
            id: format!(
                "audit-{}",
                orchestra_common::identifiers::OperationId::generate()
            ),
            timestamp: Utc::now(),
            actor: actor.into(),
            action: action.into(),
            resource: resource.into(),
            outcome,
            details,
        }
    }
}

/// Thread-safe audit logger maintaining a bounded in-memory ring buffer and optional file append.
#[derive(Debug, Clone)]
pub struct AuditLogger {
    max_in_memory: usize,
    in_memory: Arc<Mutex<VecDeque<AuditEvent>>>,
    file_path: Option<PathBuf>,
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(1000, None)
    }
}

impl AuditLogger {
    /// Constructs a new audit logger.
    pub fn new(max_in_memory: usize, file_path: Option<PathBuf>) -> Self {
        Self {
            max_in_memory,
            in_memory: Arc::new(Mutex::new(VecDeque::with_capacity(max_in_memory))),
            file_path,
        }
    }

    /// Logs an audit event record to memory and file.
    pub fn log(&self, event: AuditEvent) -> Result<(), SecurityError> {
        // Append to file if configured
        if let Some(ref path) = self.file_path {
            Self::append_to_file(path, &event)?;
        }

        // Store in bounded in-memory ring buffer
        let mut buffer = self.in_memory.lock().expect("AuditLogger lock poisoned");
        if buffer.len() >= self.max_in_memory {
            buffer.pop_front();
        }
        buffer.push_back(event);

        Ok(())
    }

    /// Helper to construct and record an audit event.
    pub fn log_action(
        &self,
        actor: &str,
        action: &str,
        resource: &str,
        outcome: AuditOutcome,
        details: serde_json::Value,
    ) -> Result<AuditEvent, SecurityError> {
        let event = AuditEvent::new(actor, action, resource, outcome, details);
        self.log(event.clone())?;
        Ok(event)
    }

    /// Returns up to `limit` most recent audit events (newest first).
    pub fn recent(&self, limit: usize) -> Vec<AuditEvent> {
        let buffer = self.in_memory.lock().expect("AuditLogger lock poisoned");
        buffer.iter().rev().take(limit).cloned().collect()
    }

    /// Filters recent audit events by actor.
    pub fn filter_by_actor(&self, actor: &str) -> Vec<AuditEvent> {
        let buffer = self.in_memory.lock().expect("AuditLogger lock poisoned");
        buffer
            .iter()
            .filter(|e| e.actor == actor)
            .cloned()
            .collect()
    }

    /// Filters recent audit events by outcome.
    pub fn filter_by_outcome(&self, outcome: AuditOutcome) -> Vec<AuditEvent> {
        let buffer = self.in_memory.lock().expect("AuditLogger lock poisoned");
        buffer
            .iter()
            .filter(|e| e.outcome == outcome)
            .cloned()
            .collect()
    }

    fn append_to_file(path: &Path, event: &AuditEvent) -> Result<(), SecurityError> {
        let json = serde_json::to_string(event).map_err(SecurityError::Json)?;
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;

        writeln!(file, "{}", json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logging_and_filtering() {
        let logger = AuditLogger::new(10, None);

        let _ = logger.log_action(
            "alice",
            "cluster.create",
            "cluster:c1",
            AuditOutcome::Allowed,
            serde_json::json!({"version": "v1.34.0"}),
        );

        let _ = logger.log_action(
            "bob",
            "cluster.delete",
            "cluster:c1",
            AuditOutcome::Denied,
            serde_json::json!({"reason": "insufficient_permissions"}),
        );

        let recent = logger.recent(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].actor, "bob"); // newest first

        let denied = logger.filter_by_outcome(AuditOutcome::Denied);
        assert_eq!(denied.len(), 1);
        assert_eq!(denied[0].actor, "bob");

        let alice_events = logger.filter_by_actor("alice");
        assert_eq!(alice_events.len(), 1);
        assert_eq!(alice_events[0].action, "cluster.create");
    }
}

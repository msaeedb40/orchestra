//! System diagnostics, debug bundles, and node inspection.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Comprehensive diagnostic snapshot report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// Timestamp when report was generated.
    pub generated_at: DateTime<Utc>,
    /// Orchestra version.
    pub orchestra_version: String,
    /// Operating system and architecture.
    pub os_info: String,
    /// Host uptime in seconds.
    pub uptime_seconds: u64,
    /// Summary of monitored clusters and status.
    pub clusters_count: usize,
    /// Summary of active nodes.
    pub nodes_count: usize,
    /// Number of loaded eBPF programs.
    pub ebpf_programs_count: usize,
    /// Extra diagnostic key-value notes.
    pub notes: Vec<String>,
}

impl DiagnosticReport {
    /// Creates a new diagnostic report.
    pub fn new(
        clusters_count: usize,
        nodes_count: usize,
        ebpf_programs_count: usize,
        notes: Vec<String>,
    ) -> Self {
        Self {
            generated_at: Utc::now(),
            orchestra_version: env!("CARGO_PKG_VERSION").to_string(),
            os_info: format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH),
            uptime_seconds: 0,
            clusters_count,
            nodes_count,
            ebpf_programs_count,
            notes,
        }
    }

    /// Renders the report as structured JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_report_json() {
        let report = DiagnosticReport::new(2, 5, 3, vec!["All systems normal".to_string()]);
        let json = report.to_json().unwrap();
        assert!(json.contains("\"clusters_count\": 2"));
        assert!(json.contains("\"nodes_count\": 5"));
        assert!(json.contains("\"ebpf_programs_count\": 3"));
        assert!(json.contains("All systems normal"));
    }
}

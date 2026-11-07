//! Container isolation and Linux namespace boundaries.

use crate::error::SecurityError;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Version of the Linux cgroups subsystem detected on the host or in the container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CgroupVersion {
    /// Legacy cgroups v1 (separate hierarchies per controller).
    V1,
    /// Modern unified cgroups v2 hierarchy.
    V2,
    /// Cgroups not mounted or unsupported.
    Unknown,
}

/// Linux namespace types required for multi-tenant and cluster container isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamespaceType {
    Mount,
    Uts,
    Ipc,
    Net,
    Pid,
    User,
    Cgroup,
}

/// Comprehensive report on host and container isolation capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationReport {
    /// Detected cgroup subsystem version.
    pub cgroup_version: CgroupVersion,
    /// Available cgroups v2 controllers (e.g. "cpu", "memory", "pids", "io").
    pub controllers: Vec<String>,
    /// Whether cgroups v2 is fully ready for nested container/Kubernetes node execution.
    pub cgroup_v2_ready: bool,
    /// Isolated namespace types active in current process.
    pub active_namespaces: Vec<NamespaceType>,
    /// Any warnings or missing features observed.
    pub warnings: Vec<String>,
}

/// Checker for validating Linux cgroups v2 hierarchy and namespace boundaries.
#[derive(Debug, Default, Clone)]
pub struct IsolationChecker {
    cgroup_root: Option<String>,
}

impl IsolationChecker {
    /// Creates a new isolation checker pointing to the default `/sys/fs/cgroup` mount.
    pub fn new() -> Self {
        Self { cgroup_root: None }
    }

    /// Creates an isolation checker with a custom cgroup root (useful for testing).
    pub fn with_root(root: impl Into<String>) -> Self {
        Self {
            cgroup_root: Some(root.into()),
        }
    }

    fn cgroup_path(&self) -> &Path {
        match &self.cgroup_root {
            Some(p) => Path::new(p),
            None => Path::new("/sys/fs/cgroup"),
        }
    }

    /// Detects the active cgroup version.
    pub fn detect_cgroup_version(&self) -> CgroupVersion {
        let base = self.cgroup_path();
        if base.join("cgroup.controllers").exists() {
            CgroupVersion::V2
        } else if base.join("cpu").exists() || base.join("memory").exists() {
            CgroupVersion::V1
        } else {
            CgroupVersion::Unknown
        }
    }

    /// Reads available cgroups v2 controllers.
    pub fn read_cgroup_controllers(&self) -> Result<Vec<String>, SecurityError> {
        let controllers_file = self.cgroup_path().join("cgroup.controllers");
        if !controllers_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&controllers_file).map_err(|e| {
            SecurityError::Isolation(format!("Failed reading cgroup.controllers: {}", e))
        })?;

        let controllers = content.split_whitespace().map(|s| s.to_string()).collect();

        Ok(controllers)
    }

    /// Validates that critical controllers needed for Kubernetes container execution are present.
    pub fn validate_required_controllers(&self, required: &[&str]) -> Result<(), SecurityError> {
        let available = self.read_cgroup_controllers()?;
        let available_set: HashSet<&str> = available.iter().map(|s| s.as_str()).collect();

        let mut missing = Vec::new();
        for req in required {
            if !available_set.contains(req) {
                missing.push(*req);
            }
        }

        if !missing.is_empty() {
            return Err(SecurityError::Isolation(format!(
                "Missing required cgroups v2 controllers: {}",
                missing.join(", ")
            )));
        }

        Ok(())
    }

    /// Generates an isolation audit report for the current system.
    pub fn audit(&self) -> IsolationReport {
        let cgroup_version = self.detect_cgroup_version();
        let mut warnings = Vec::new();

        let controllers = self.read_cgroup_controllers().unwrap_or_else(|e| {
            warnings.push(format!("Controller read error: {}", e));
            Vec::new()
        });

        let cgroup_v2_ready = match cgroup_version {
            CgroupVersion::V2 => {
                let required = ["cpu", "memory", "pids"];
                let set: HashSet<&str> = controllers.iter().map(|s| s.as_str()).collect();
                let missing: Vec<&&str> = required.iter().filter(|c| !set.contains(**c)).collect();
                if !missing.is_empty() {
                    warnings.push(format!(
                        "Cgroup v2 unified hierarchy present but missing controllers: {:?}",
                        missing
                    ));
                    false
                } else {
                    true
                }
            }
            CgroupVersion::V1 => {
                warnings.push(
                    "Host is running legacy cgroups v1; modern Kubernetes requires cgroups v2"
                        .to_string(),
                );
                false
            }
            CgroupVersion::Unknown => {
                warnings.push("No active cgroup mount detected at path".to_string());
                false
            }
        };

        // Detect current namespace isolation via /proc/self/ns
        let mut active_namespaces = Vec::new();
        let ns_map = [
            ("mnt", NamespaceType::Mount),
            ("uts", NamespaceType::Uts),
            ("ipc", NamespaceType::Ipc),
            ("net", NamespaceType::Net),
            ("pid", NamespaceType::Pid),
            ("user", NamespaceType::User),
            ("cgroup", NamespaceType::Cgroup),
        ];

        for (name, ns_type) in ns_map {
            if Path::new(&format!("/proc/self/ns/{}", name)).exists() {
                active_namespaces.push(ns_type);
            }
        }

        IsolationReport {
            cgroup_version,
            controllers,
            cgroup_v2_ready,
            active_namespaces,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_cgroup_v2_detection() {
        let temp_dir =
            std::env::temp_dir().join(format!("orchestra_cgroup_test_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let controllers_file = temp_dir.join("cgroup.controllers");
        fs::write(&controllers_file, "cpuset cpu io memory pids\n").unwrap();

        let checker = IsolationChecker::with_root(temp_dir.to_str().unwrap());
        assert_eq!(checker.detect_cgroup_version(), CgroupVersion::V2);

        let controllers = checker.read_cgroup_controllers().unwrap();
        assert_eq!(controllers, vec!["cpuset", "cpu", "io", "memory", "pids"]);

        assert!(
            checker
                .validate_required_controllers(&["cpu", "memory"])
                .is_ok()
        );
        assert!(checker.validate_required_controllers(&["hugetlb"]).is_err());

        let report = checker.audit();
        assert!(report.cgroup_v2_ready);
        assert_eq!(report.cgroup_version, CgroupVersion::V2);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}

//! Security policy enforcement engine.

use crate::error::SecurityError;
use orchestra_core::node::NodeRole;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Security profile presets for container execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SecurityProfile {
    /// Standard Kubernetes node container (requires CAP_NET_ADMIN, CAP_SYS_ADMIN, /lib/modules mount).
    #[default]
    StandardNode,
    /// Hardened worker container with minimal privileges.
    RestrictedWorker,
    /// Fully privileged container (development and advanced debugging only).
    Privileged,
}

/// Security policy defining container privileges, Linux capabilities, and mount restrictions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Active security profile.
    pub profile: SecurityProfile,
    /// Set of permitted Linux capabilities (e.g. `CAP_NET_ADMIN`, `CAP_SYS_ADMIN`).
    pub allowed_capabilities: HashSet<String>,
    /// Set of capabilities explicitly denied or dropped.
    pub forbidden_capabilities: HashSet<String>,
    /// Allowed host paths for volume mounts.
    pub allowed_mount_sources: Vec<PathBuf>,
    /// Whether privileged mode is permitted by policy.
    pub allow_privileged: bool,
    /// Whether containers must have read-only root filesystems where possible.
    pub enforce_read_only_rootfs: bool,
    /// Optional name of an active seccomp profile.
    pub seccomp_profile: Option<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::for_profile(SecurityProfile::StandardNode)
    }
}

impl SecurityPolicy {
    /// Builds a policy corresponding to the chosen profile preset.
    pub fn for_profile(profile: SecurityProfile) -> Self {
        match profile {
            SecurityProfile::StandardNode => {
                let mut allowed_caps = HashSet::new();
                for cap in [
                    "CAP_NET_ADMIN",
                    "CAP_SYS_ADMIN",
                    "CAP_SYS_RESOURCE",
                    "CAP_SYS_PTRACE",
                    "CAP_IPC_LOCK",
                    "CAP_SYS_CHROOT",
                    "CAP_MKNOD",
                    "CAP_SETUID",
                    "CAP_SETGID",
                    "CAP_DAC_OVERRIDE",
                ] {
                    allowed_caps.insert(cap.to_string());
                }

                let mut forbidden_caps = HashSet::new();
                forbidden_caps.insert("CAP_SYS_BOOT".to_string());
                forbidden_caps.insert("CAP_SYS_RAWIO".to_string());

                let allowed_mounts = vec![
                    PathBuf::from("/lib/modules"),
                    PathBuf::from("/sys/fs/bpf"),
                    PathBuf::from("/var/run"),
                ];

                Self {
                    profile,
                    allowed_capabilities: allowed_caps,
                    forbidden_capabilities: forbidden_caps,
                    allowed_mount_sources: allowed_mounts,
                    allow_privileged: true, // Needed for full systemd/kubelet node in Docker
                    enforce_read_only_rootfs: false,
                    seccomp_profile: Some("orchestra-node-default".to_string()),
                }
            }
            SecurityProfile::RestrictedWorker => {
                let mut allowed_caps = HashSet::new();
                for cap in [
                    "CAP_NET_ADMIN",
                    "CAP_SYS_RESOURCE",
                    "CAP_SETUID",
                    "CAP_SETGID",
                    "CAP_DAC_OVERRIDE",
                ] {
                    allowed_caps.insert(cap.to_string());
                }

                let mut forbidden_caps = HashSet::new();
                for cap in [
                    "CAP_SYS_BOOT",
                    "CAP_SYS_RAWIO",
                    "CAP_SYS_ADMIN",
                    "CAP_SYS_MODULE",
                ] {
                    forbidden_caps.insert(cap.to_string());
                }

                Self {
                    profile,
                    allowed_capabilities: allowed_caps,
                    forbidden_capabilities: forbidden_caps,
                    allowed_mount_sources: vec![PathBuf::from("/lib/modules")],
                    allow_privileged: false,
                    enforce_read_only_rootfs: false,
                    seccomp_profile: Some("orchestra-worker-strict".to_string()),
                }
            }
            SecurityProfile::Privileged => Self {
                profile,
                allowed_capabilities: HashSet::new(), // All capabilities
                forbidden_capabilities: HashSet::new(),
                allowed_mount_sources: Vec::new(), // Any mount source
                allow_privileged: true,
                enforce_read_only_rootfs: false,
                seccomp_profile: None,
            },
        }
    }

    /// Validates whether a requested set of capabilities is permitted by this policy.
    pub fn validate_capabilities(&self, requested: &[String]) -> Result<(), SecurityError> {
        if self.profile == SecurityProfile::Privileged {
            return Ok(());
        }

        for cap in requested {
            let cap_upper = cap.to_uppercase();
            if self.forbidden_capabilities.contains(&cap_upper) {
                return Err(SecurityError::PolicyViolation(format!(
                    "Capability '{}' is explicitly forbidden by security policy",
                    cap
                )));
            }

            if !self.allowed_capabilities.is_empty()
                && !self.allowed_capabilities.contains(&cap_upper)
            {
                return Err(SecurityError::PolicyViolation(format!(
                    "Capability '{}' is not in the allowed capabilities list",
                    cap
                )));
            }
        }
        Ok(())
    }

    /// Validates whether a requested host mount path is permitted by this policy.
    pub fn validate_mount_source(&self, source: &Path) -> Result<(), SecurityError> {
        if self.profile == SecurityProfile::Privileged || self.allowed_mount_sources.is_empty() {
            return Ok(());
        }

        let is_allowed = self
            .allowed_mount_sources
            .iter()
            .any(|allowed| source == allowed || source.starts_with(allowed));

        if !is_allowed {
            return Err(SecurityError::PolicyViolation(format!(
                "Host mount path '{}' is not authorized by security policy",
                source.display()
            )));
        }
        Ok(())
    }

    /// Validates a node container specification against the security policy.
    pub fn validate_node_launch(
        &self,
        role: NodeRole,
        privileged: bool,
        capabilities: &[String],
        mount_sources: &[PathBuf],
    ) -> Result<(), SecurityError> {
        if privileged && !self.allow_privileged {
            return Err(SecurityError::PolicyViolation(format!(
                "Privileged containers are disallowed for profile '{:?}'",
                self.profile
            )));
        }

        // Control plane nodes may require additional capabilities
        if role == NodeRole::ControlPlane && self.profile == SecurityProfile::RestrictedWorker {
            return Err(SecurityError::PolicyViolation(
                "ControlPlane nodes cannot run under RestrictedWorker profile".to_string(),
            ));
        }

        self.validate_capabilities(capabilities)?;

        for mount in mount_sources {
            self.validate_mount_source(mount)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_policy_allowed_caps() {
        let policy = SecurityPolicy::for_profile(SecurityProfile::StandardNode);
        let caps = vec!["CAP_NET_ADMIN".to_string(), "CAP_SYS_ADMIN".to_string()];
        assert!(policy.validate_capabilities(&caps).is_ok());

        let forbidden = vec!["CAP_SYS_BOOT".to_string()];
        assert!(policy.validate_capabilities(&forbidden).is_err());
    }

    #[test]
    fn test_restricted_policy_disallows_privileged() {
        let policy = SecurityPolicy::for_profile(SecurityProfile::RestrictedWorker);
        assert!(!policy.allow_privileged);

        let res = policy.validate_node_launch(NodeRole::Worker, true, &[], &[]);
        assert!(res.is_err());
    }

    #[test]
    fn test_mount_source_validation() {
        let policy = SecurityPolicy::for_profile(SecurityProfile::StandardNode);
        assert!(
            policy
                .validate_mount_source(Path::new("/lib/modules"))
                .is_ok()
        );
        assert!(
            policy
                .validate_mount_source(Path::new("/lib/modules/6.8.0"))
                .is_ok()
        );
        assert!(
            policy
                .validate_mount_source(Path::new("/etc/shadow"))
                .is_err()
        );
    }
}

//! Seccomp-BPF profiles and filters for container isolation.

use crate::error::SecurityError;
use serde::{Deserialize, Serialize};

/// Action to take when a seccomp filter rule is matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeccompAction {
    /// Return standard error errno (e.g. EPERM).
    #[serde(rename = "SCMP_ACT_ERRNO")]
    Errno,
    /// Allow the syscall to proceed.
    #[serde(rename = "SCMP_ACT_ALLOW")]
    Allow,
    /// Terminate the thread/process immediately.
    #[serde(rename = "SCMP_ACT_KILL")]
    Kill,
    /// Log the syscall and allow execution.
    #[serde(rename = "SCMP_ACT_LOG")]
    Log,
    /// Notify ptrace tracer.
    #[serde(rename = "SCMP_ACT_TRACE")]
    Trace,
}

/// Comparison operator for syscall arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeccompOp {
    #[serde(rename = "SCMP_CMP_NE")]
    NotEqual,
    #[serde(rename = "SCMP_CMP_LT")]
    LessThan,
    #[serde(rename = "SCMP_CMP_LE")]
    LessOrEqual,
    #[serde(rename = "SCMP_CMP_EQ")]
    Equal,
    #[serde(rename = "SCMP_CMP_GE")]
    GreaterOrEqual,
    #[serde(rename = "SCMP_CMP_GT")]
    GreaterThan,
    #[serde(rename = "SCMP_CMP_MASKED_EQ")]
    MaskedEqual,
}

/// Argument condition for syscall filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeccompArg {
    pub index: usize,
    pub value: u64,
    pub value_two: Option<u64>,
    pub op: SeccompOp,
}

/// Specific rule granting or denying syscalls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyscallRule {
    /// Syscall names governed by this rule.
    pub names: Vec<String>,
    /// Action taken when syscall is invoked.
    pub action: SeccompAction,
    /// Optional argument filtering conditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<SeccompArg>,
    /// Optional comment explaining rationale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

/// OCI-compliant seccomp profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeccompProfile {
    /// Default action when no syscall rule matches.
    #[serde(rename = "defaultAction")]
    pub default_action: SeccompAction,
    /// Supported architectures.
    pub architectures: Vec<String>,
    /// List of syscall rules.
    pub syscalls: Vec<SyscallRule>,
}

impl SeccompProfile {
    /// Serializes profile to JSON string suitable for Docker's `--security-opt seccomp=`.
    pub fn to_json(&self) -> Result<String, SecurityError> {
        serde_json::to_string_pretty(self).map_err(SecurityError::Json)
    }

    /// Deserializes a profile from JSON string.
    pub fn from_json(json: &str) -> Result<Self, SecurityError> {
        serde_json::from_str(json).map_err(SecurityError::Json)
    }

    /// Validates that the profile allows essential Kubernetes and eBPF syscalls.
    pub fn validate_for_k8s_node(&self) -> Result<(), SecurityError> {
        let mut allowed = Vec::new();
        for rule in &self.syscalls {
            if rule.action == SeccompAction::Allow {
                for name in &rule.names {
                    allowed.push(name.as_str());
                }
            }
        }

        let required = ["clone", "execve", "mount", "bpf", "socket", "epoll_create1"];
        for req in required {
            if !allowed.contains(&req) && self.default_action != SeccompAction::Allow {
                return Err(SecurityError::Seccomp(format!(
                    "Seccomp profile missing required syscall '{}' for Kubernetes node",
                    req
                )));
            }
        }

        Ok(())
    }
}

/// Generates the standard seccomp profile optimized for Orchestra Kubernetes node containers.
pub fn default_node_seccomp_profile() -> SeccompProfile {
    let architectures = vec![
        "SCMP_ARCH_X86_64".to_string(),
        "SCMP_ARCH_X86".to_string(),
        "SCMP_ARCH_AARCH64".to_string(),
        "SCMP_ARCH_ARM".to_string(),
    ];

    let standard_syscalls = vec![
        "accept".to_string(),
        "accept4".to_string(),
        "access".to_string(),
        "alarm".to_string(),
        "bind".to_string(),
        "bpf".to_string(),
        "brk".to_string(),
        "chdir".to_string(),
        "chmod".to_string(),
        "chown".to_string(),
        "clock_gettime".to_string(),
        "clone".to_string(),
        "clone3".to_string(),
        "close".to_string(),
        "connect".to_string(),
        "dup".to_string(),
        "dup2".to_string(),
        "dup3".to_string(),
        "epoll_create".to_string(),
        "epoll_create1".to_string(),
        "epoll_ctl".to_string(),
        "epoll_pwait".to_string(),
        "epoll_pwait2".to_string(),
        "epoll_wait".to_string(),
        "execve".to_string(),
        "execveat".to_string(),
        "exit".to_string(),
        "exit_group".to_string(),
        "fcntl".to_string(),
        "fstat".to_string(),
        "fsync".to_string(),
        "futex".to_string(),
        "getpid".to_string(),
        "gettid".to_string(),
        "kill".to_string(),
        "listen".to_string(),
        "mmap".to_string(),
        "mount".to_string(),
        "mprotect".to_string(),
        "munmap".to_string(),
        "nanosleep".to_string(),
        "open".to_string(),
        "openat".to_string(),
        "openat2".to_string(),
        "pipe".to_string(),
        "pipe2".to_string(),
        "pivot_root".to_string(),
        "poll".to_string(),
        "read".to_string(),
        "recvfrom".to_string(),
        "recvmsg".to_string(),
        "restart_syscall".to_string(),
        "rt_sigaction".to_string(),
        "rt_sigprocmask".to_string(),
        "rt_sigreturn".to_string(),
        "sendmsg".to_string(),
        "sendto".to_string(),
        "set_tid_address".to_string(),
        "setns".to_string(),
        "setsockopt".to_string(),
        "socket".to_string(),
        "stat".to_string(),
        "statx".to_string(),
        "umount2".to_string(),
        "unshare".to_string(),
        "wait4".to_string(),
        "waitid".to_string(),
        "write".to_string(),
    ];

    let blocked_dangerous_syscalls = vec![
        "acct".to_string(),
        "kexec_file_load".to_string(),
        "kexec_load".to_string(),
        "lookup_dcookie".to_string(),
        "reboot".to_string(),
        "swapoff".to_string(),
        "swapon".to_string(),
        "sysfs".to_string(),
    ];

    SeccompProfile {
        default_action: SeccompAction::Errno,
        architectures,
        syscalls: vec![
            SyscallRule {
                names: standard_syscalls,
                action: SeccompAction::Allow,
                args: Vec::new(),
                comment: Some("Permit standard container operations, eBPF and runtime".to_string()),
            },
            SyscallRule {
                names: blocked_dangerous_syscalls,
                action: SeccompAction::Errno,
                args: Vec::new(),
                comment: Some("Block host-damaging kernel control syscalls".to_string()),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile_validity() {
        let profile = default_node_seccomp_profile();
        assert_eq!(profile.default_action, SeccompAction::Errno);
        assert!(profile.validate_for_k8s_node().is_ok());

        let json = profile.to_json().expect("Serialization failed");
        assert!(json.contains("SCMP_ACT_ALLOW"));
        assert!(json.contains("bpf"));
        assert!(json.contains("reboot"));

        let restored = SeccompProfile::from_json(&json).expect("Deserialization failed");
        assert_eq!(profile, restored);
    }
}

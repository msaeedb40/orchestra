//! Global constants for Orchestra daemon, CLI, networking, and runtime.

use std::path::Path;
use std::time::Duration;

/// Default system-wide daemon Unix domain socket path.
pub const DEFAULT_SYSTEM_SOCKET_PATH: &str = "/run/orchestra/orchestra.sock";

/// Default user-level daemon Unix domain socket relative subpath under home.
pub const DEFAULT_USER_SOCKET_SUBPATH: &str = ".orchestra/run/orchestra.sock";

/// Default system data directory.
pub const DEFAULT_SYSTEM_DATA_DIR: &str = "/var/lib/orchestra";

/// Default user data directory relative subpath under home.
pub const DEFAULT_USER_DATA_SUBPATH: &str = ".orchestra";

/// Default database filename.
pub const DEFAULT_DB_FILENAME: &str = "orchestra.db";

/// Default eBPF filesystem mount / map pin path.
pub const DEFAULT_BPF_FS_PATH: &str = "/sys/fs/bpf";

/// Default Orchestra eBPF pinned maps directory.
pub const DEFAULT_BPF_PIN_PATH: &str = "/sys/fs/bpf/orchestra";

/// Default network bridge name for cluster nodes.
pub const DEFAULT_BRIDGE_NAME: &str = "orch-br0";

/// Default cluster Pod network CIDR (RFC 6598 / RFC 1918).
pub const DEFAULT_POD_CIDR: &str = "10.244.0.0/16";

/// Default cluster Service network CIDR.
pub const DEFAULT_SERVICE_CIDR: &str = "10.96.0.0/12";

/// Default cluster internal Docker bridge subnet.
pub const DEFAULT_NODE_SUBNET_CIDR: &str = "172.30.0.0/16";

/// Default Kubernetes API server secure port.
pub const DEFAULT_K8S_API_PORT: u16 = 6443;

/// Default Orchestra daemon gRPC TCP port (when TCP listener is enabled).
pub const DEFAULT_GRPC_TCP_PORT: u16 = 50051;

/// Default Prometheus metrics port.
pub const DEFAULT_METRICS_PORT: u16 = 9090;

/// Default container runtime node image tag.
pub const DEFAULT_NODE_IMAGE: &str = "kindest/node:v1.31.2";

/// List of actively supported Kubernetes minor versions.
pub const SUPPORTED_K8S_VERSIONS: &[&str] = &["1.28", "1.29", "1.30", "1.31", "1.32"];

/// Default timeout for individual container runtime operations.
pub const DEFAULT_RUNTIME_TIMEOUT: Duration = Duration::from_secs(60);

/// Default timeout for waiting for Kubernetes node readiness.
pub const DEFAULT_NODE_READY_TIMEOUT: Duration = Duration::from_secs(180);

/// Default timeout for waiting for Kubernetes control plane bootstrap.
pub const DEFAULT_CONTROL_PLANE_TIMEOUT: Duration = Duration::from_secs(300);

/// Default interval for background reconciliation loop.
pub const DEFAULT_RECONCILE_INTERVAL: Duration = Duration::from_secs(10);

/// Returns the preferred socket path based on privileges / user environment.
pub fn get_default_socket_path() -> std::path::PathBuf {
    if unsafe { libc::geteuid() } == 0 {
        std::path::PathBuf::from(DEFAULT_SYSTEM_SOCKET_PATH)
    } else if let Ok(home) = std::env::var("HOME") {
        Path::new(&home).join(DEFAULT_USER_SOCKET_SUBPATH)
    } else {
        std::path::PathBuf::from(DEFAULT_SYSTEM_SOCKET_PATH)
    }
}

/// Returns the preferred data directory based on privileges / user environment.
pub fn get_default_data_dir() -> std::path::PathBuf {
    if unsafe { libc::geteuid() } == 0 {
        std::path::PathBuf::from(DEFAULT_SYSTEM_DATA_DIR)
    } else if let Ok(home) = std::env::var("HOME") {
        Path::new(&home).join(DEFAULT_USER_DATA_SUBPATH)
    } else {
        std::path::PathBuf::from(DEFAULT_SYSTEM_DATA_DIR)
    }
}

/// Checks whether a Kubernetes version string (e.g. "v1.31.2" or "1.31") is supported.
pub fn is_k8s_version_supported(version: &str) -> bool {
    let clean = version.strip_prefix('v').unwrap_or(version);
    SUPPORTED_K8S_VERSIONS
        .iter()
        .any(|&sup| clean.starts_with(sup))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k8s_version_support() {
        assert!(is_k8s_version_supported("v1.31.2"));
        assert!(is_k8s_version_supported("1.31.0"));
        assert!(is_k8s_version_supported("v1.28.0"));
        assert!(is_k8s_version_supported("1.32.1"));
        assert!(!is_k8s_version_supported("v1.25.0"));
        assert!(!is_k8s_version_supported("v2.0.0"));
    }

    #[test]
    fn test_paths_not_empty() {
        let sock = get_default_socket_path();
        assert!(!sock.as_os_str().is_empty());

        let data = get_default_data_dir();
        assert!(!data.as_os_str().is_empty());
    }
}

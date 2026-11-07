//! Virtual Ethernet (`veth`) pair lifecycle management.

use orchestra_core::error::NetworkError;
use std::process::Command;
use tracing::{debug, info};

/// Manages veth pairs linking host bridges to container network namespaces.
pub struct VethManager;

impl VethManager {
    /// Creates a veth pair, attaching `host_iface` to `bridge_name` and moving `peer_iface` into container `pid`.
    pub fn create_and_attach(
        host_iface: &str,
        peer_iface: &str,
        bridge_name: &str,
        container_pid: u32,
    ) -> Result<(), NetworkError> {
        info!(
            host_iface,
            peer_iface,
            bridge = bridge_name,
            pid = container_pid,
            "Creating veth pair"
        );

        // 1. Create veth pair: ip link add <host_iface> type veth peer name <peer_iface>
        let status = Command::new("ip")
            .args([
                "link", "add", host_iface, "type", "veth", "peer", "name", peer_iface,
            ])
            .status()
            .map_err(|e| {
                NetworkError::VethCreationFailed(format!("Failed to execute 'ip link add': {e}"))
            })?;

        if !status.success() {
            return Err(NetworkError::VethCreationFailed(format!(
                "Failed to create veth pair '{host_iface}' <-> '{peer_iface}'"
            )));
        }

        // 2. Attach host_iface to bridge: ip link set <host_iface> master <bridge_name>
        let status = Command::new("ip")
            .args(["link", "set", host_iface, "master", bridge_name])
            .status()
            .map_err(|e| {
                NetworkError::VethCreationFailed(format!("Failed to attach to bridge: {e}"))
            })?;

        if !status.success() {
            // Cleanup on failure
            let _ = Command::new("ip")
                .args(["link", "del", host_iface])
                .status();
            return Err(NetworkError::VethCreationFailed(format!(
                "Failed to attach '{host_iface}' to bridge '{bridge_name}'"
            )));
        }

        // 3. Bring host_iface UP
        let _ = Command::new("ip")
            .args(["link", "set", host_iface, "up"])
            .status();

        // 4. Move peer into container network namespace: ip link set <peer_iface> netns <pid>
        let pid_str = container_pid.to_string();
        let status = Command::new("ip")
            .args(["link", "set", peer_iface, "netns", &pid_str])
            .status()
            .map_err(|e| {
                NetworkError::VethCreationFailed(format!("Failed to move peer to netns: {e}"))
            })?;

        if !status.success() {
            let _ = Command::new("ip")
                .args(["link", "del", host_iface])
                .status();
            return Err(NetworkError::VethCreationFailed(format!(
                "Failed to move '{peer_iface}' into netns of PID {container_pid}"
            )));
        }

        debug!(
            host_iface,
            peer_iface, "Veth pair successfully attached and configured"
        );
        Ok(())
    }

    /// Deletes a veth interface on the host (which automatically tears down the peer).
    pub fn delete_veth(host_iface: &str) -> Result<(), NetworkError> {
        let status = Command::new("ip")
            .args(["link", "del", host_iface])
            .status()
            .map_err(|e| NetworkError::VethCreationFailed(format!("Failed to delete veth: {e}")))?;

        if !status.success() {
            debug!(
                host_iface,
                "veth link delete returned non-zero (may already be gone)"
            );
        }
        Ok(())
    }
}

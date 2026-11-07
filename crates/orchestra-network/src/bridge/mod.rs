//! Software bridge lifecycle management.

use orchestra_core::error::NetworkError;
use orchestra_core::network::NetworkConfig;
use std::process::Command;
use tracing::{debug, info};

/// Manages Linux software bridge devices.
pub struct BridgeManager;

impl BridgeManager {
    /// Checks if a bridge interface exists on the host.
    pub fn exists(bridge_name: &str) -> bool {
        std::path::Path::new(&format!("/sys/class/net/{bridge_name}")).exists()
    }

    /// Creates and configures a Linux bridge device according to `NetworkConfig`.
    pub fn create_bridge(config: &NetworkConfig) -> Result<(), NetworkError> {
        let name = &config.bridge_name;

        if Self::exists(name) {
            debug!(bridge = %name, "Bridge interface already exists");
            return Ok(());
        }

        info!(bridge = %name, subnet = %config.subnet, "Creating Linux software bridge");

        // 1. ip link add name <bridge> type bridge
        let status = Command::new("ip")
            .args(["link", "add", "name", name, "type", "bridge"])
            .status()
            .map_err(|e| {
                NetworkError::BridgeCreationFailed(format!("Failed to invoke 'ip': {e}"))
            })?;

        if !status.success() {
            return Err(NetworkError::BridgeCreationFailed(format!(
                "Failed to create bridge link '{name}'"
            )));
        }

        // 2. Set MTU
        let mtu_str = config.mtu.to_string();
        let _ = Command::new("ip")
            .args(["link", "set", "dev", name, "mtu", &mtu_str])
            .status();

        // 3. Assign gateway IP
        let prefix_len = config.subnet.split('/').nth(1).unwrap_or("16");
        let cidr_addr = format!("{}/{}", config.gateway, prefix_len);

        let status = Command::new("ip")
            .args(["addr", "add", &cidr_addr, "dev", name])
            .status()
            .map_err(|e| {
                NetworkError::BridgeCreationFailed(format!("Failed to add IP address: {e}"))
            })?;

        if !status.success() {
            return Err(NetworkError::BridgeCreationFailed(format!(
                "Failed to assign gateway {cidr_addr} to bridge '{name}'"
            )));
        }

        // 4. Set link UP
        let status = Command::new("ip")
            .args(["link", "set", "dev", name, "up"])
            .status()
            .map_err(|e| {
                NetworkError::BridgeCreationFailed(format!("Failed to set link up: {e}"))
            })?;

        if !status.success() {
            return Err(NetworkError::BridgeCreationFailed(format!(
                "Failed to set bridge '{name}' up"
            )));
        }

        info!(bridge = %name, %cidr_addr, "Bridge created and brought up");
        Ok(())
    }

    /// Deletes a Linux software bridge device.
    pub fn delete_bridge(bridge_name: &str) -> Result<(), NetworkError> {
        if !Self::exists(bridge_name) {
            debug!(bridge = %bridge_name, "Bridge does not exist, nothing to delete");
            return Ok(());
        }

        info!(bridge = %bridge_name, "Deleting Linux software bridge");

        let status = Command::new("ip")
            .args(["link", "del", "dev", bridge_name])
            .status()
            .map_err(|e| {
                NetworkError::BridgeCreationFailed(format!("Failed to invoke 'ip': {e}"))
            })?;

        if !status.success() {
            return Err(NetworkError::BridgeCreationFailed(format!(
                "Failed to delete bridge link '{bridge_name}'"
            )));
        }

        Ok(())
    }
}

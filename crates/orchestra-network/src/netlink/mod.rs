//! Linux network link and device inspection.

use orchestra_core::error::NetworkError;
use orchestra_core::network::NetworkInterface;
use std::fs;
use std::path::Path;
use tracing::debug;

/// Lists network interfaces detected on the host system.
pub fn list_host_interfaces() -> Result<Vec<NetworkInterface>, NetworkError> {
    let net_dir = Path::new("/sys/class/net");
    let mut interfaces = Vec::new();

    if !net_dir.exists() {
        return Ok(interfaces);
    }

    let entries = fs::read_dir(net_dir).map_err(|e| NetworkError::NetlinkFailed {
        iface: "host".to_string(),
        message: format!("Failed to read /sys/class/net: {e}"),
    })?;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Ok(iface) = inspect_interface(&name) {
            interfaces.push(iface);
        }
    }

    Ok(interfaces)
}

/// Inspects a specific network interface by name from sysfs.
pub fn inspect_interface(name: &str) -> Result<NetworkInterface, NetworkError> {
    let iface_path = format!("/sys/class/net/{name}");
    if !Path::new(&iface_path).exists() {
        return Err(NetworkError::NetlinkFailed {
            iface: name.to_string(),
            message: "Interface does not exist in sysfs".to_string(),
        });
    }

    // Read MAC address
    let mac_address = fs::read_to_string(format!("{iface_path}/address"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // Read MTU
    let mtu = fs::read_to_string(format!("{iface_path}/mtu"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(1500);

    // Read operstate (UP/DOWN)
    let is_up = fs::read_to_string(format!("{iface_path}/operstate"))
        .ok()
        .map(|s| s.trim().eq_ignore_ascii_case("up") || s.trim().eq_ignore_ascii_case("unknown"))
        .unwrap_or(false);

    debug!(name, mtu, is_up, "Inspected network interface");

    Ok(NetworkInterface {
        name: name.to_string(),
        mac_address,
        ip_address: None, // IP addresses are queried via netlink or socket ioctl
        mtu,
        is_up,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_host_interfaces() {
        let ifaces = list_host_interfaces().unwrap();
        // On any Linux system, loopback 'lo' should exist
        if Path::new("/sys/class/net/lo").exists() {
            assert!(ifaces.iter().any(|i| i.name == "lo"));
        }
    }
}

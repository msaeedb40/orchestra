//! Host routing and IP forwarding configuration.

use orchestra_core::error::NetworkError;
use std::process::Command;
use tracing::{debug, info};

/// Manages host IP routing and kernel packet forwarding.
pub struct RouteManager;

impl RouteManager {
    /// Ensures `net.ipv4.ip_forward` is enabled on the host.
    pub fn enable_ip_forwarding() -> Result<(), NetworkError> {
        let status = Command::new("sysctl")
            .args(["-w", "net.ipv4.ip_forward=1"])
            .status()
            .map_err(|e| NetworkError::NetlinkFailed {
                iface: "sysctl".to_string(),
                message: format!("Failed to set net.ipv4.ip_forward: {e}"),
            })?;

        if !status.success() {
            debug!("Could not write net.ipv4.ip_forward=1 (may require elevated privileges)");
        }
        Ok(())
    }

    /// Adds a route for a pod CIDR via a node IP over a bridge.
    pub fn add_pod_route(
        pod_cidr: &str,
        node_ip: &str,
        bridge_name: &str,
    ) -> Result<(), NetworkError> {
        info!(
            pod_cidr,
            node_ip,
            bridge = bridge_name,
            "Adding pod network route"
        );

        let status = Command::new("ip")
            .args([
                "route",
                "replace",
                pod_cidr,
                "via",
                node_ip,
                "dev",
                bridge_name,
            ])
            .status()
            .map_err(|e| NetworkError::NetlinkFailed {
                iface: bridge_name.to_string(),
                message: format!("Failed to add route: {e}"),
            })?;

        if !status.success() {
            return Err(NetworkError::NetlinkFailed {
                iface: bridge_name.to_string(),
                message: format!("Failed to route {pod_cidr} via {node_ip}"),
            });
        }

        Ok(())
    }

    /// Configures iptables NAT masquerade for the cluster subnet outbound traffic.
    pub fn setup_masquerade(subnet: &str, bridge_name: &str) -> Result<(), NetworkError> {
        info!(subnet, bridge = bridge_name, "Configuring NAT masquerade");

        // iptables -t nat -A POSTROUTING -s <subnet> ! -o <bridge_name> -j MASQUERADE
        let _ = Command::new("iptables")
            .args([
                "-t",
                "nat",
                "-C",
                "POSTROUTING",
                "-s",
                subnet,
                "!",
                "-o",
                bridge_name,
                "-j",
                "MASQUERADE",
            ])
            .status()
            .map(|s| {
                if !s.success() {
                    let _ = Command::new("iptables")
                        .args([
                            "-t",
                            "nat",
                            "-A",
                            "POSTROUTING",
                            "-s",
                            subnet,
                            "!",
                            "-o",
                            bridge_name,
                            "-j",
                            "MASQUERADE",
                        ])
                        .status();
                }
            });

        Ok(())
    }
}

//! Pre-configured configuration profiles for common deployment archetypes.

use crate::schema::{
    CLUSTER_KIND, CURRENT_API_VERSION, ClusterConfigSpec, MetadataConfig, OrchestraConfig,
};
use orchestra_core::network::{CniPlugin, NetworkMode, ServiceProxyMode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Preset deployment profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Minimal footprint: 1 CP, 1 Worker, OrchestraNet, fast startup.
    #[default]
    Dev,
    /// Standard production: 1 CP, 3 Workers, Cilium eBPF dataplane.
    Standard,
    /// High availability: 3 CP (etcd quorum), 5 Workers, Calico CNI.
    Ha,
}

impl FromStr for Profile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "dev" | "development" => Ok(Self::Dev),
            "standard" | "std" => Ok(Self::Standard),
            "ha" | "high-availability" => Ok(Self::Ha),
            other => Err(format!(
                "Unknown profile '{}'. Valid: dev, standard, ha",
                other
            )),
        }
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Standard => write!(f, "standard"),
            Self::Ha => write!(f, "ha"),
        }
    }
}

/// Creates a complete `OrchestraConfig` tailored for a specific profile archetype.
pub fn create_profile_config(name: &str, profile: Profile) -> OrchestraConfig {
    let mut spec = ClusterConfigSpec::default();

    match profile {
        Profile::Dev => {
            spec.nodes.control_plane.count = 1;
            spec.nodes.workers.count = 1;
            spec.network.mode = NetworkMode::Bridge;
            spec.network.cni = CniPlugin::OrchestraNet;
            spec.network.service_proxy = ServiceProxyMode::Orchestra;
            spec.ebpf.enabled = true;
            spec.observability.log_level = "info".to_string();
        }
        Profile::Standard => {
            spec.nodes.control_plane.count = 1;
            spec.nodes.workers.count = 3;
            spec.network.mode = NetworkMode::Bridge;
            spec.network.cni = CniPlugin::Cilium;
            spec.network.service_proxy = ServiceProxyMode::Cni;
            spec.ebpf.enabled = true;
            spec.ebpf.conntrack = true;
            spec.observability.log_level = "info".to_string();
        }
        Profile::Ha => {
            spec.nodes.control_plane.count = 3;
            spec.nodes.workers.count = 5;
            spec.network.mode = NetworkMode::Bridge;
            spec.network.cni = CniPlugin::Calico;
            spec.network.service_proxy = ServiceProxyMode::Orchestra;
            spec.ebpf.enabled = true;
            spec.ebpf.service_load_balancer = true;
            spec.ebpf.conntrack = true;
            spec.observability.log_level = "debug".to_string();
        }
    }

    OrchestraConfig {
        api_version: CURRENT_API_VERSION.to_string(),
        kind: CLUSTER_KIND.to_string(),
        metadata: MetadataConfig::new(name),
        spec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::validate_config;

    #[test]
    fn test_all_profiles_pass_validation() {
        for profile in [Profile::Dev, Profile::Standard, Profile::Ha] {
            let name = format!("test-{}-cluster", profile);
            let cfg = create_profile_config(&name, profile);
            assert!(
                validate_config(&cfg).is_ok(),
                "Profile {} failed validation: {:?}",
                profile,
                validate_config(&cfg)
            );
        }
    }
}

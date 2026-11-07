//! CNI plugin coordination and Service proxy mode management.

use orchestra_core::network::{CniPlugin, ServiceProxyMode};

/// Manages CNI capabilities and proxy mode coexistence rules.
pub struct CniCoordinator;

impl CniCoordinator {
    /// Determines whether Orchestra's eBPF service proxy should be loaded for the cluster.
    pub fn should_load_orchestra_dataplane(mode: ServiceProxyMode) -> bool {
        matches!(mode, ServiceProxyMode::Orchestra)
    }

    /// Determines whether traditional kube-proxy should be deployed.
    pub fn should_run_kube_proxy(mode: ServiceProxyMode) -> bool {
        matches!(mode, ServiceProxyMode::KubeProxy)
    }

    /// Validates compatibility between chosen CNI and Service proxy mode.
    /// Cilium with internal kube-proxy replacement conflicts with orchestra-proxy mode.
    pub fn validate_compatibility(cni: CniPlugin, proxy: ServiceProxyMode) -> Result<(), String> {
        if cni == CniPlugin::Cilium && proxy == ServiceProxyMode::Orchestra {
            return Err(
                "Cilium native eBPF dataplane conflicts with Orchestra eBPF service proxy. \
                 Please use 'cni-proxy' mode when running Cilium."
                    .to_string(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cni_compatibility_validation() {
        assert!(
            CniCoordinator::validate_compatibility(CniPlugin::Kindnet, ServiceProxyMode::Orchestra)
                .is_ok()
        );
        assert!(
            CniCoordinator::validate_compatibility(CniPlugin::Flannel, ServiceProxyMode::Orchestra)
                .is_ok()
        );
        assert!(
            CniCoordinator::validate_compatibility(CniPlugin::Cilium, ServiceProxyMode::Cni)
                .is_ok()
        );
        assert!(
            CniCoordinator::validate_compatibility(CniPlugin::Cilium, ServiceProxyMode::Orchestra)
                .is_err()
        );
    }
}

//! Networking compatibility tests for CNIs and eBPF mode coordination.

use orchestra_core::cluster::{CniPlugin, ServiceProxyMode};
use orchestra_network::cni::CniCoordinator;

#[test]
fn test_cni_ebpf_compatibility_matrix() {
    // 1. Kindnet + Orchestra eBPF: Valid and optimal
    assert!(
        CniCoordinator::validate_compatibility(CniPlugin::Kindnet, ServiceProxyMode::Orchestra)
            .is_ok()
    );

    // 2. Flannel + Orchestra eBPF: Valid
    assert!(
        CniCoordinator::validate_compatibility(CniPlugin::Flannel, ServiceProxyMode::Orchestra)
            .is_ok()
    );

    // 3. Cilium + Orchestra eBPF: Conflict (Cilium uses native TC/XDP)
    let cilium_conflict =
        CniCoordinator::validate_compatibility(CniPlugin::Cilium, ServiceProxyMode::Orchestra);
    assert!(cilium_conflict.is_err());

    // 4. Cilium + Cni: Valid
    assert!(
        CniCoordinator::validate_compatibility(CniPlugin::Cilium, ServiceProxyMode::Cni).is_ok()
    );

    // 5. Calico + KubeProxy: Valid
    assert!(
        CniCoordinator::validate_compatibility(CniPlugin::Calico, ServiceProxyMode::KubeProxy)
            .is_ok()
    );
}

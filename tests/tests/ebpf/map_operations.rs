//! Integration tests for eBPF Map operations and ABI compatibility.

use orchestra_ebpf::BpfMapManager;
use orchestra_ebpf_common::maps::{BackendValue, ConntrackEntry, ConntrackKey, ServiceKey};

#[test]
fn test_bpf_map_manager_crud() {
    let manager = BpfMapManager::new();

    // 1. Service Map Test
    let svc_key = ServiceKey::new_ipv4([10, 96, 0, 10], 443, 6);
    let backends = vec![
        BackendValue::new_ipv4([10, 244, 1, 2], 8443, 100),
        BackendValue::new_ipv4([10, 244, 2, 4], 8443, 100),
    ];

    manager
        .upsert_service(svc_key, 42, 0, &backends)
        .expect("Upsert service failed");

    let retrieved = manager.get_service(&svc_key).expect("Get service failed");
    assert_eq!(retrieved.backend_count, 2);
    assert_eq!(retrieved.service_id, 42);

    // 2. Backend Map Test
    let backend_0 = manager.get_backend(42, 0).expect("Get backend failed");
    assert_eq!(backend_0.target_port, 8443);

    // 3. Conntrack Map Test
    let ct_key = ConntrackKey {
        src_address: [10, 244, 1, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        dst_address: [10, 96, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        src_port: 50123,
        dst_port: 443,
        protocol: 6,
        _pad: [0; 3],
    };
    let ct_entry = ConntrackEntry {
        original_dst_address: [10, 96, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        translated_dst_address: [10, 244, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        original_dst_port: 443,
        translated_dst_port: 8443,
        state: 1, // Established
        last_seen_ns: 12345678,
        packet_count: 42,
        byte_count: 4096,
    };
    manager.update_conntrack(ct_key, ct_entry);

    // 4. Deletion
    manager
        .remove_service(&svc_key)
        .expect("Remove service failed");
    assert!(manager.get_service(&svc_key).is_none());
}

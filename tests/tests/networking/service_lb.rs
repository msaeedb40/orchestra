//! Networking tests for Service Load Balancing and IPAM.

use orchestra_ebpf::ServiceDataplane;
use orchestra_ebpf_common::maps::{BackendValue, ServiceKey};
use orchestra_network::ipam::IpamPool;

#[test]
fn test_service_load_balancer_dataplane_sync() {
    let dataplane = ServiceDataplane::new();

    let vip = [10, 96, 0, 10];
    let port = 80;
    let proto = 6; // TCP
    let service_id = 1;

    let backend_1 = BackendValue::new_ipv4([10, 244, 1, 2], 80, 100);
    let backend_2 = BackendValue::new_ipv4([10, 244, 2, 3], 80, 100);
    let backends = vec![backend_1, backend_2];

    dataplane
        .update_service(vip, port, proto, service_id, &backends)
        .expect("Failed to update service in dataplane");

    // Lookup service
    let key = ServiceKey::new_ipv4(vip, port, proto);
    let svc = dataplane
        .maps()
        .get_service(&key)
        .expect("Service not found");
    assert_eq!(svc.backend_count, 2);
    assert_eq!(svc.service_id, service_id);

    let ep1 = dataplane
        .maps()
        .get_backend(service_id, 0)
        .expect("Backend 0 not found");
    assert_eq!(ep1.target_port, 80);

    let ep2 = dataplane
        .maps()
        .get_backend(service_id, 1)
        .expect("Backend 1 not found");
    assert_eq!(ep2.target_port, 80);

    // Remove service
    dataplane
        .remove_service(vip, port, proto)
        .expect("Failed to delete service");
    assert!(dataplane.maps().get_service(&key).is_none());
}

#[test]
fn test_ipam_pool_network_isolation() {
    let pool = IpamPool::new("10.244.0.0/16").expect("Invalid CIDR");

    let ip1 = pool.allocate().expect("Allocation 1 failed");
    let ip2 = pool.allocate().expect("Allocation 2 failed");

    assert_ne!(ip1, ip2);
    assert!(pool.is_allocated(ip1));
    assert!(pool.is_allocated(ip2));

    pool.release(ip1);
    assert!(!pool.is_allocated(ip1));
    assert!(pool.is_allocated(ip2));
}

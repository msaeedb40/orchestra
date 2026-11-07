//! eBPF network hooks, service load balancing, and packet manipulation logic.

use orchestra_ebpf_common::maps::{BackendValue, ServiceValue};
use orchestra_ebpf_common::types::{IPPROTO_TCP, IPPROTO_UDP, PacketVerdict};

/// Selects a backend index using a 5-tuple hash for consistent session affinity.
#[inline(always)]
pub fn select_backend(service: &ServiceValue, hash: u32) -> u32 {
    if service.backend_count == 0 {
        0
    } else {
        hash % service.backend_count
    }
}

/// Computes a simple hash over IPv4 addresses and ports for backend selection.
#[inline(always)]
pub fn hash_5tuple(src_ip: u32, dst_ip: u32, src_port: u16, dst_port: u16, proto: u8) -> u32 {
    let mut h = src_ip ^ dst_ip;
    h ^= ((src_port as u32) << 16) | (dst_port as u32);
    h ^= (proto as u32).wrapping_mul(0x9e3779b9);
    h
}

/// Simulates DNAT translation logic for eBPF TC ingress program.
#[inline(always)]
pub fn process_ingress_packet(
    _dst_ip: [u8; 4],
    _dst_port_be: u16,
    proto: u8,
    service: Option<&ServiceValue>,
    backend: Option<&BackendValue>,
) -> PacketVerdict {
    // Only process TCP/UDP traffic matching known services
    if proto != IPPROTO_TCP && proto != IPPROTO_UDP {
        return PacketVerdict::Pass;
    }

    match (service, backend) {
        (Some(_svc), Some(_bnd)) => {
            // Found active service and backend -> Redirect/DNAT
            PacketVerdict::Redirect
        }
        (Some(_svc), None) => {
            // Service exists but no active endpoints -> Drop packet to prevent blackhole
            PacketVerdict::Drop
        }
        (None, _) => {
            // Not a cluster service VIP -> Normal packet pass-through
            PacketVerdict::Pass
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_backend_distribution() {
        let svc = ServiceValue {
            backend_count: 3,
            flags: 0,
            service_id: 1,
            _reserved: 0,
        };

        assert_eq!(select_backend(&svc, 0), 0);
        assert_eq!(select_backend(&svc, 1), 1);
        assert_eq!(select_backend(&svc, 2), 2);
        assert_eq!(select_backend(&svc, 3), 0);
    }

    #[test]
    fn test_hash_5tuple_consistency() {
        let h1 = hash_5tuple(0x0a000001, 0x0a000002, 1234, 80, 6);
        let h2 = hash_5tuple(0x0a000001, 0x0a000002, 1234, 80, 6);
        assert_eq!(h1, h2);
    }
}

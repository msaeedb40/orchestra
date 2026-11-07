//! Userspace management and synchronization for eBPF maps.

use orchestra_core::error::EbpfError;
use orchestra_ebpf_common::maps::{
    BackendKey, BackendValue, ConntrackEntry, ConntrackKey, ServiceKey, ServiceValue,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

/// Userspace mirror and management handle for eBPF kernel maps.
#[derive(Debug, Clone, Default)]
pub struct BpfMapManager {
    services: Arc<RwLock<HashMap<ServiceKey, ServiceValue>>>,
    backends: Arc<RwLock<HashMap<BackendKey, BackendValue>>>,
    conntrack: Arc<RwLock<HashMap<ConntrackKey, ConntrackEntry>>>,
}

impl BpfMapManager {
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            backends: Arc::new(RwLock::new(HashMap::new())),
            conntrack: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers or updates a Kubernetes service VIP in the BPF services map.
    pub fn upsert_service(
        &self,
        key: ServiceKey,
        service_id: u32,
        flags: u32,
        backend_list: &[BackendValue],
    ) -> Result<(), EbpfError> {
        info!(
            service_id,
            backend_count = backend_list.len(),
            "Upserting service in eBPF map"
        );

        let value = ServiceValue {
            backend_count: backend_list.len() as u32,
            flags,
            service_id,
            _reserved: 0,
        };

        // 1. Insert service
        {
            let mut svcs = self.services.write().unwrap();
            svcs.insert(key, value);
        }

        // 2. Insert backends
        {
            let mut bnds = self.backends.write().unwrap();
            // Clear prior backends for this service
            bnds.retain(|k, _| k.service_id != service_id);

            for (idx, bnd) in backend_list.iter().enumerate() {
                let b_key = BackendKey {
                    service_id,
                    backend_index: idx as u32,
                };
                bnds.insert(b_key, *bnd);
            }
        }

        debug!(service_id, "Service and backends synchronized");
        Ok(())
    }

    /// Removes a service and its backends from the BPF maps.
    pub fn remove_service(&self, key: &ServiceKey) -> Result<(), EbpfError> {
        let mut svcs = self.services.write().unwrap();
        if let Some(val) = svcs.remove(key) {
            let mut bnds = self.backends.write().unwrap();
            bnds.retain(|k, _| k.service_id != val.service_id);
            info!(service_id = val.service_id, "Removed service from eBPF map");
        }
        Ok(())
    }

    /// Looks up a service by VIP and port.
    pub fn get_service(&self, key: &ServiceKey) -> Option<ServiceValue> {
        let svcs = self.services.read().unwrap();
        svcs.get(key).copied()
    }

    /// Looks up a specific backend.
    pub fn get_backend(&self, service_id: u32, index: u32) -> Option<BackendValue> {
        let bnds = self.backends.read().unwrap();
        let key = BackendKey {
            service_id,
            backend_index: index,
        };
        bnds.get(&key).copied()
    }

    /// Returns the total number of registered services.
    pub fn service_count(&self) -> usize {
        let svcs = self.services.read().unwrap();
        svcs.len()
    }

    /// Returns the total number of active backends across all services.
    pub fn backend_count(&self) -> usize {
        let bnds = self.backends.read().unwrap();
        bnds.len()
    }

    /// Records or updates a connection tracking entry.
    pub fn update_conntrack(&self, key: ConntrackKey, entry: ConntrackEntry) {
        let mut ct = self.conntrack.write().unwrap();
        ct.insert(key, entry);
    }

    /// Cleans up expired conntrack entries older than `max_age_ns`.
    pub fn gc_conntrack(&self, now_ns: u64, max_age_ns: u64) -> usize {
        let mut ct = self.conntrack.write().unwrap();
        let initial_len = ct.len();
        ct.retain(|_, entry| now_ns.saturating_sub(entry.last_seen_ns) < max_age_ns);
        initial_len - ct.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpf_map_upsert_and_lookup() {
        let mgr = BpfMapManager::new();
        let svc_key = ServiceKey::new_ipv4([10, 96, 0, 10], 80, 6);
        let backends = vec![
            BackendValue::new_ipv4([172, 20, 0, 2], 8080, 1),
            BackendValue::new_ipv4([172, 20, 0, 3], 8080, 1),
        ];

        mgr.upsert_service(svc_key, 101, 0, &backends).unwrap();
        assert_eq!(mgr.service_count(), 1);
        assert_eq!(mgr.backend_count(), 2);

        let svc = mgr.get_service(&svc_key).unwrap();
        assert_eq!(svc.backend_count, 2);
        assert_eq!(svc.service_id, 101);

        let b0 = mgr.get_backend(101, 0).unwrap();
        assert_eq!(b0.target_port, 8080);

        mgr.remove_service(&svc_key).unwrap();
        assert_eq!(mgr.service_count(), 0);
        assert_eq!(mgr.backend_count(), 0);
    }
}

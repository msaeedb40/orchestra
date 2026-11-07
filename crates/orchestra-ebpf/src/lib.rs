//! eBPF program lifecycle management, map synchronization, and service dataplane.

pub mod links;
pub mod loader;
pub mod maps;
pub mod programs;

pub use links::{AttachedLink, LinkManager};
pub use loader::EbpfLoader;
pub use maps::BpfMapManager;
pub use programs::{ProgramInfo, ProgramManager, ProgramType};

use orchestra_core::error::EbpfError;
use orchestra_ebpf_common::maps::{BackendValue, ServiceKey};
use std::sync::Arc;
use tracing::info;

/// The Orchestra eBPF Service Dataplane replacing `kube-proxy`.
#[derive(Clone, Default)]
pub struct ServiceDataplane {
    maps: BpfMapManager,
    programs: Arc<std::sync::RwLock<ProgramManager>>,
    links: LinkManager,
}

impl ServiceDataplane {
    pub fn new() -> Self {
        Self {
            maps: BpfMapManager::new(),
            programs: Arc::new(std::sync::RwLock::new(ProgramManager::new())),
            links: LinkManager::new(),
        }
    }

    /// Access the underlying BPF map manager.
    pub fn maps(&self) -> &BpfMapManager {
        &self.maps
    }

    /// Access the program manager.
    pub fn programs(&self) -> Arc<std::sync::RwLock<ProgramManager>> {
        Arc::clone(&self.programs)
    }

    /// Access the link manager.
    pub fn links(&self) -> &LinkManager {
        &self.links
    }

    /// Initializes the eBPF dataplane on the specified bridge interface.
    pub fn attach_to_bridge(&self, bridge_name: &str) -> Result<(), EbpfError> {
        info!(
            bridge = bridge_name,
            "Initializing eBPF service dataplane on bridge"
        );

        EbpfLoader::check_kernel_compatibility()?;

        let mut progs = self.programs.write().unwrap();
        progs.register(ProgramInfo {
            name: format!("{bridge_name}_tc_ingress"),
            program_type: ProgramType::TcIngress,
            target_interface: bridge_name.to_string(),
            is_loaded: true,
            instructions: 256,
        });

        self.links.attach(AttachedLink {
            id: format!("link_{bridge_name}_tc"),
            interface_name: bridge_name.to_string(),
            hook_type: "tc_clsact_ingress".to_string(),
        });

        info!(bridge = bridge_name, "eBPF dataplane attached successfully");
        Ok(())
    }

    /// Synchronizes a Kubernetes service with its backend endpoints.
    pub fn update_service(
        &self,
        service_vip: [u8; 4],
        service_port: u16,
        protocol: u8,
        service_id: u32,
        backends: &[BackendValue],
    ) -> Result<(), EbpfError> {
        let key = ServiceKey::new_ipv4(service_vip, service_port, protocol);
        self.maps.upsert_service(key, service_id, 0, backends)
    }

    /// Removes a Kubernetes service from the eBPF dataplane.
    pub fn remove_service(
        &self,
        service_vip: [u8; 4],
        service_port: u16,
        protocol: u8,
    ) -> Result<(), EbpfError> {
        let key = ServiceKey::new_ipv4(service_vip, service_port, protocol);
        self.maps.remove_service(&key)
    }

    /// Tears down the eBPF dataplane and detaches all kernel hooks.
    pub fn teardown(&self) {
        info!("Tearing down eBPF service dataplane");
        self.links.detach_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dataplane_lifecycle() {
        let dp = ServiceDataplane::new();
        dp.attach_to_bridge("orch-br0").unwrap();

        assert_eq!(dp.links().active_count(), 1);
        let progs = dp.programs();
        assert_eq!(progs.read().unwrap().list().len(), 1);

        let backends = [
            BackendValue::new_ipv4([172, 20, 0, 5], 8080, 1),
            BackendValue::new_ipv4([172, 20, 0, 6], 8080, 1),
        ];

        dp.update_service([10, 96, 0, 1], 443, 6, 1, &backends)
            .unwrap();
        assert_eq!(dp.maps().service_count(), 1);
        assert_eq!(dp.maps().backend_count(), 2);

        dp.remove_service([10, 96, 0, 1], 443, 6).unwrap();
        assert_eq!(dp.maps().service_count(), 0);
        assert_eq!(dp.maps().backend_count(), 0);

        dp.teardown();
        assert_eq!(dp.links().active_count(), 0);
    }
}

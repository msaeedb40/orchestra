//! Integration tests for eBPF Program loading and Link attachment.

use orchestra_ebpf::links::{AttachedLink, LinkManager};
use orchestra_ebpf::loader::EbpfLoader;
use orchestra_ebpf::programs::{ProgramInfo, ProgramManager, ProgramType};

#[test]
fn test_program_manager_and_link_attachment() {
    assert!(EbpfLoader::check_kernel_compatibility().is_ok());

    let mut prog_mgr = ProgramManager::new();
    prog_mgr.register(ProgramInfo {
        name: "xdp_service_ingress".to_string(),
        program_type: ProgramType::Xdp,
        target_interface: "eth0".to_string(),
        is_loaded: true,
        instructions: 120,
    });
    prog_mgr.register(ProgramInfo {
        name: "tc_service_ingress".to_string(),
        program_type: ProgramType::TcIngress,
        target_interface: "eth0".to_string(),
        is_loaded: true,
        instructions: 250,
    });

    assert_eq!(prog_mgr.list().len(), 2);
    assert!(prog_mgr.find("xdp_service_ingress").is_some());

    let link_mgr = LinkManager::new();
    link_mgr.attach(AttachedLink {
        id: "link_xdp_eth0".to_string(),
        interface_name: "eth0".to_string(),
        hook_type: "xdp".to_string(),
    });

    assert_eq!(link_mgr.active_count(), 1);

    let detached = link_mgr.detach("link_xdp_eth0");
    assert!(detached.is_some());
    assert_eq!(link_mgr.active_count(), 0);
}

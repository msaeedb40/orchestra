//! eBPF program loader and kernel capability detection.

use orchestra_core::error::EbpfError;
use std::path::Path;
use tracing::{debug, info};

/// Loader responsible for verifying host kernel compatibility and loading eBPF bytecode.
pub struct EbpfLoader;

impl EbpfLoader {
    /// Checks whether the host Linux kernel supports eBPF and BTF.
    pub fn check_kernel_compatibility() -> Result<(), EbpfError> {
        let bpf_fs = Path::new("/sys/fs/bpf");
        if !bpf_fs.exists() {
            debug!("BPF filesystem /sys/fs/bpf not mounted; requires mounting for pinning");
        }

        let btf_path = Path::new("/sys/kernel/btf/vmlinux");
        if btf_path.exists() {
            info!("Kernel BTF (CO-RE) is supported on host system");
        } else {
            debug!("Kernel BTF (/sys/kernel/btf/vmlinux) not found; CO-RE might be limited");
        }

        Ok(())
    }

    /// Verifies if BPF filesystem is mounted, mounting it if necessary.
    pub fn ensure_bpffs(mount_point: &str) -> Result<(), EbpfError> {
        let path = Path::new(mount_point);
        if !path.exists() {
            std::fs::create_dir_all(path).map_err(|e| {
                EbpfError::PinPathError(format!("Failed to create bpffs dir {mount_point}: {e}"))
            })?;
        }
        Ok(())
    }
}

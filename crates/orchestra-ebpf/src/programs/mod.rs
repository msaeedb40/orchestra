//! eBPF program metadata and state representation.

use serde::{Deserialize, Serialize};

/// Type of eBPF hook program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgramType {
    Xdp,
    TcIngress,
    TcEgress,
    Tracepoint,
}

impl std::fmt::Display for ProgramType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Xdp => write!(f, "xdp"),
            Self::TcIngress => write!(f, "tc_ingress"),
            Self::TcEgress => write!(f, "tc_egress"),
            Self::Tracepoint => write!(f, "tracepoint"),
        }
    }
}

/// Metadata and runtime state for a loaded eBPF program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInfo {
    pub name: String,
    pub program_type: ProgramType,
    pub target_interface: String,
    pub is_loaded: bool,
    pub instructions: usize,
}

/// ProgramManager manages and tracks active in-kernel programs.
#[derive(Debug, Clone, Default)]
pub struct ProgramManager {
    programs: Vec<ProgramInfo>,
}

impl ProgramManager {
    pub fn new() -> Self {
        Self {
            programs: Vec::new(),
        }
    }

    /// Registers a loaded program.
    pub fn register(&mut self, info: ProgramInfo) {
        self.programs.retain(|p| p.name != info.name);
        self.programs.push(info);
    }

    /// Returns all registered programs.
    pub fn list(&self) -> &[ProgramInfo] {
        &self.programs
    }

    /// Finds a program by name.
    pub fn find(&self, name: &str) -> Option<&ProgramInfo> {
        self.programs.iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_manager() {
        let mut mgr = ProgramManager::new();
        mgr.register(ProgramInfo {
            name: "orchestra_tc_ingress".to_string(),
            program_type: ProgramType::TcIngress,
            target_interface: "orch-br0".to_string(),
            is_loaded: true,
            instructions: 128,
        });

        assert_eq!(mgr.list().len(), 1);
        let prog = mgr.find("orchestra_tc_ingress").unwrap();
        assert_eq!(prog.program_type, ProgramType::TcIngress);
        assert!(prog.is_loaded);
    }
}

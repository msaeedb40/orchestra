//! Shared primitives and protocol definitions for eBPF and userspace.

/// IP Protocol constants for eBPF packet inspection.
pub const IPPROTO_TCP: u8 = 6;
pub const IPPROTO_UDP: u8 = 17;
pub const IPPROTO_ICMP: u8 = 1;

/// Service flags for load balancing.
pub const SERVICE_FLAG_EXTERNAL_TRAFFIC_LOCAL: u32 = 1 << 0;
pub const SERVICE_FLAG_SESSION_AFFINITY: u32 = 1 << 1;

/// Maximum backends supported per service.
pub const MAX_BACKENDS_PER_SERVICE: usize = 64;

/// Packet verdict actions.
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PacketVerdict {
    Pass = 0,
    Drop = 1,
    Redirect = 2,
}

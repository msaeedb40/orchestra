//! Shared BPF map key and value ABI layout definitions.

/// Service lookup key matching incoming cluster VIP traffic.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ServiceKey {
    /// Destination IP (IPv4 in lower 4 bytes or IPv6 in 16 bytes).
    pub address: [u8; 16],
    /// Destination port in network byte order.
    pub port: u16,
    /// Protocol (e.g. IPPROTO_TCP = 6, IPPROTO_UDP = 17).
    pub protocol: u8,
    /// Padding for 32-bit alignment.
    pub _pad: u8,
}

impl ServiceKey {
    /// Constructs an IPv4 service key.
    pub const fn new_ipv4(ip: [u8; 4], port_be: u16, protocol: u8) -> Self {
        Self {
            address: [
                ip[0], ip[1], ip[2], ip[3], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            port: port_be,
            protocol,
            _pad: 0,
        }
    }
}

/// Service metadata and backend count.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ServiceValue {
    /// Number of active backends registered for this service.
    pub backend_count: u32,
    /// Configuration flags (e.g. external traffic policy).
    pub flags: u32,
    /// Unique service identifier for backend table lookup.
    pub service_id: u32,
    /// Reserved for alignment or future features.
    pub _reserved: u32,
}

/// Key identifying a specific backend for a service.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BackendKey {
    /// Parent service identifier.
    pub service_id: u32,
    /// 0-indexed backend slot (0 .. backend_count - 1).
    pub backend_index: u32,
}

/// Target endpoint destination for NAT translation.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BackendValue {
    /// Target pod/node IP address.
    pub target_address: [u8; 16],
    /// Target port in network byte order.
    pub target_port: u16,
    /// Relative weight for weighted load balancing.
    pub weight: u16,
}

impl BackendValue {
    pub const fn new_ipv4(ip: [u8; 4], port_be: u16, weight: u16) -> Self {
        Self {
            target_address: [
                ip[0], ip[1], ip[2], ip[3], 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ],
            target_port: port_be,
            weight,
        }
    }
}

/// 5-tuple connection tracking lookup key.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConntrackKey {
    pub src_address: [u8; 16],
    pub dst_address: [u8; 16],
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub _pad: [u8; 3],
}

/// Connection state tracking NAT translation and state machine.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ConntrackEntry {
    /// Original destination IP before NAT translation.
    pub original_dst_address: [u8; 16],
    /// Translated target backend IP.
    pub translated_dst_address: [u8; 16],
    /// Original destination port.
    pub original_dst_port: u16,
    /// Translated backend destination port.
    pub translated_dst_port: u16,
    /// TCP connection state or UDP active flag.
    pub state: u32,
    /// Last packet timestamp (kernel uptime in nanoseconds).
    pub last_seen_ns: u64,
    /// Cumulative packet counter.
    pub packet_count: u64,
    /// Cumulative byte counter.
    pub byte_count: u64,
}

//! Ring buffer telemetry and packet drop events sent to userspace.

/// Packet telemetry event emitted over the BPF Ring Buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PacketEvent {
    /// Kernel event timestamp in nanoseconds.
    pub timestamp_ns: u64,
    /// Source IP address.
    pub src_address: [u8; 16],
    /// Destination IP address.
    pub dst_address: [u8; 16],
    /// Source port.
    pub src_port: u16,
    /// Destination port.
    pub dst_port: u16,
    /// Protocol (TCP/UDP/ICMP).
    pub protocol: u8,
    /// Verdict action taken (Pass, Drop, Redirect).
    pub verdict: u8,
    /// Reason code (e.g. 0 = Normal, 1 = NoBackend, 2 = PolicyDrop).
    pub reason: u16,
    /// Interface index where packet was intercepted.
    pub ifindex: u32,
    /// Packet size in bytes.
    pub packet_len: u32,
}

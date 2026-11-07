//! eBPF observability tracepoint and ring buffer emission helpers.

use orchestra_ebpf_common::events::PacketEvent;
use orchestra_ebpf_common::types::PacketVerdict;

/// Constructs a packet telemetry event to be emitted into the BPF ring buffer.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub fn create_packet_event(
    timestamp_ns: u64,
    src_address: [u8; 16],
    dst_address: [u8; 16],
    src_port: u16,
    dst_port: u16,
    protocol: u8,
    verdict: PacketVerdict,
    reason: u16,
    ifindex: u32,
    packet_len: u32,
) -> PacketEvent {
    PacketEvent {
        timestamp_ns,
        src_address,
        dst_address,
        src_port,
        dst_port,
        protocol,
        verdict: verdict as u8,
        reason,
        ifindex,
        packet_len,
    }
}

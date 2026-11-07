# In-Kernel eBPF Service Dataplane

Orchestra replaces traditional `kube-proxy` (iptables/ipvs) with an in-kernel eBPF dataplane using XDP (eXpress Data Path) and TC (`clsact` traffic control) hooks.

## Dataplane Architecture

```
Incoming Packet (Host / Bridge / Veth)
        │
        ▼
   [ XDP Ingress Hook ] ──(Non-VIP traffic)──► Pass to Kernel Stack
        │
   (Matches Cluster VIP in BPF Map: `orchestra_services`)
        │
        ▼
   [ 5-Tuple Conntrack Lookup ]
        │
    ├── Existing connection? ──► Reuse target backend & translate
    │
    └── New flow? ──► Hash 5-tuple -> Select backend -> Insert conntrack
        │
        ▼
   [ In-place DNAT Translation ]
   (Replace Destination IP/Port with Pod Backend IP/Port)
        │
        ▼
   Recalculate IP & L4 Checksums
        │
        ▼
   Forward packet directly to Pod interface via XDP_TX or TC redirect
```

## BPF Map Layout (ABI Compatibility)

Shared between `#![no_std]` kernel programs and userspace Rust:

- **`orchestra_services` (`BPF_MAP_TYPE_HASH`)**:
  - Key: `ServiceKey` (16-byte IP, 2-byte Port, 1-byte Protocol)
  - Value: `ServiceValue` (Active backend count, service ID, flags)
- **`orchestra_backends` (`BPF_MAP_TYPE_HASH`)**:
  - Key: `BackendKey` (Service ID + Backend Index)
  - Value: `BackendValue` (Target 16-byte IP, Target Port, Weight)
- **`orchestra_conntrack` (`BPF_MAP_TYPE_LRU_HASH`)**:
  - Key: `ConntrackKey` (5-tuple: src IP, dst IP, src port, dst port, proto)
  - Value: `ConntrackEntry` (Original & translated addresses/ports, state, packet & byte counters)
- **`orchestra_events` (`BPF_MAP_TYPE_RINGBUF`)**:
  - Ring buffer emitting `PacketEvent` records to userspace observability collectors.

## Coexistence & CNI Modes

When running with CNIs that feature their own eBPF kube-proxy replacement (such as Cilium in full-eBPF mode), `CniCoordinator` detects potential hook conflicts and enforces `proxy.mode = cni` to prevent dual NAT translations.


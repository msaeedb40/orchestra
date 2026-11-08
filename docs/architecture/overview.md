# Orchestra Architecture Overview

Orchestra is a Rust-native Kubernetes cluster orchestration engine that provisions multi-node Kubernetes clusters inside Docker containers, accelerated by an in-kernel eBPF dataplane replacing traditional `kube-proxy`.

## Core Subsystems & Directional Crate Hierarchy

The workspace strictly enforces a unidirectional dependency hierarchy:

```
                  ┌──────────────────────┐
                  │    orchestra-cli     │
                  └──────────┬───────────┘
                             │
                             ▼
                  ┌──────────────────────┐
                  │   orchestra-daemon   │
                  └──────────┬───────────┘
                             │
         ┌───────────────────┼───────────────────┐
         ▼                   ▼                   ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│orchestra-runtime│ │orchestra-k8s    │ │ orchestra-ebpf  │
└────────┬────────┘ └────────┬────────┘ └────────┬────────┘
         │                   │                   │
         └───────────────────┼───────────────────┘
                             ▼
                  ┌──────────────────────┐
                  │    orchestra-core    │
                  └──────────┬───────────┘
                             ▼
                  ┌──────────────────────┐
                  │   orchestra-common   │
                  └──────────────────────┘
```

### Subsystems Breakdown

1. **`orchestra-common`**: Foundational types, custom newtype IDs (`ClusterId`, `NodeId`, `OperationId`), time utilities, DNS/Kubernetes name validators, and retry backoff.
2. **`orchestra-core`**: Core domain finite state machines (`ClusterState`, `NodeState`), domain events, asynchronous operations tracking, and provider traits (`RuntimeProvider`, `NetworkProvider`, `StorageRepository`).
3. **`orchestra-config`**: Strongly-typed configuration schema (`OrchestraConfig`), multi-layer loader with environment and flag overlays, validation rules (CIDR overlap, proxy mode compatibility).
4. **`orchestra-storage`**: SQLite transactional persistence with foreign key cascades, append-only operation journals, checkpointing, and advisory locks.
5. **`orchestra-runtime`**: Async container runtime engine powered by Bollard/Docker, managing container lifecycles, healthchecks, exec streams, and volume bindings.
6. **`orchestra-kubernetes`**: Multi-version Kubernetes bootstrap engine (`v1.34` - `v1.37`), kubeadm `InitConfiguration` and `JoinConfiguration` generator, `admin.conf` kubeconfig rewriter, and CNI manifest generator.
7. **`orchestra-network`**: Linux bridge and veth pair manager, netlink socket integration, dynamic IPAM CIDR allocation (auto-selects first free `/16` in `172.16–172.31`), and CNI coexistence coordinator. Includes `OrchestraNet` — Orchestra's native host-routing CNI (the default), and `CniCoordinator` which enforces proxy-mode compatibility rules (e.g. blocking `proxy=orchestra` when Cilium is selected).
8. **`orchestra-ebpf` & `ebpf/*`**: In-kernel XDP and TC clsact service proxy, connection tracking table, weighted round-robin endpoint selection, and userspace mirror maps (`BpfMapManager`, `ServiceDataplane`).
9. **`orchestra-observability`**: Prometheus text metrics exporter, ring-buffered domain event stream, JSON diagnostic reporter, and OpenTelemetry-compatible tracing.
10. **`orchestra-security`**: Linux cgroups v2 checker, namespace isolation auditor, capability whitelist/blacklist policy, OCI seccomp filter profiles, RBAC token manager, and security audit logger.
11. **`orchestra-api`**: High-performance gRPC contracts defined with Protocol Buffers and Tonic (`ClusterService`, `NodeService`, `NetworkService`, `EbpfService`, `SystemService`).
12. **`orchestra-daemon` (`orchestrad`)**: Background reconciliation daemon maintaining cluster topologies, handling asynchronous workflows, and serving gRPC requests.
13. **`orchestra-cli` (`orchestra`)**: Ergonomic Clap-driven command-line interface supporting tabular, JSON, and YAML outputs.


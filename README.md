# Orchestra

<div align="center">

<img src="orchestra.jpeg" alt="Orchestra Logo" width="380" style="border-radius: 12px; box-shadow: 0 8px 24px rgba(0,0,0,0.3); margin-bottom: 16px;">

### High-Performance, eBPF-Powered Kubernetes Cluster Orchestration in Rust

[![CI](https://github.com/msaeedb40/orchestra/actions/workflows/ci.yml/badge.svg)](https://github.com/msaeedb40/orchestra/actions/workflows/ci.yml)
[![Tests](https://github.com/msaeedb40/orchestra/actions/workflows/test.yml/badge.svg)](https://github.com/msaeedb40/orchestra/actions/workflows/test.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85.0%2B-orange.svg)](https://www.rust-lang.org)
[![Kubernetes](https://img.shields.io/badge/k8s-v1.34--v1.37-326ce5.svg)](https://kubernetes.io)

</div>

---

**Orchestra** is a production-grade Kubernetes cluster orchestrator written in 100% safe, idiomatic Rust. It provisions and manages multi-node Kubernetes clusters inside Docker containers, accelerated by an in-kernel eBPF dataplane that replaces traditional `kube-proxy` with low-latency XDP/TC service load balancing.

---

## Key Features

- **Rust-First Architecture**: Built with zero garbage collection, predictable memory safety, and high-concurrency async I/O via Tokio and Tonic.
- **In-Kernel eBPF Service Dataplane**: Replaces `kube-proxy` with in-kernel XDP and TC `clsact` hooks, performing 5-tuple connection-tracking and DNAT translations before entering the standard network stack.
- **Multi-Version Kubernetes Engine**: Multi-stage OCI container images packaging containerd, runc, systemd, and kubeadm across Kubernetes versions **v1.34 through v1.37**.
- **CNI Flexibility**: Native support and conflict-free coexistence with **Kindnet**, **Flannel**, **Calico**, and **Cilium**.
- **Hardened Container Security**: OCI-compliant Seccomp-BPF profiles, Linux cgroups v2 unified hierarchy validation, Linux namespace boundary auditing, and role-based access control (RBAC).
- **Observability Built-In**: Prometheus metric exposition, real-time domain event streaming, JSON diagnostic reporters, and OpenTelemetry tracing.
- **Enterprise Persistence**: SQLite transactional storage engine with foreign-key cascade deletes, operation journals, and checkpointing.

---

## Architecture Overview

```
                           ┌───────────────────────┐
                           │     orchestra CLI     │
                           └───────────┬───────────┘
                                       │ gRPC / IPC
                                       ▼
                           ┌───────────────────────┐
                           │   orchestrad Daemon   │
                           └───────────┬───────────┘
                                       │
        ┌──────────────────────────────┼──────────────────────────────┐
        ▼                              ▼                              ▼
┌───────────────────┐        ┌───────────────────┐        ┌───────────────────┐
│ orchestra-runtime │        │ orchestra-k8s     │        │  orchestra-ebpf   │
│ (Docker Engine)   │        │ (Kubeadm & CNI)   │        │ (XDP/TC Dataplane)│
└───────────────────┘        └───────────────────┘        └───────────────────┘
        │                              │                              │
        └──────────────────────────────┼──────────────────────────────┘
                                       ▼
                           ┌───────────────────────┐
                           │   orchestra-storage   │
                           │   (SQLite Engine)     │
                           └───────────────────────┘
```

For full architectural details, see the [Architecture Documentation](docs/architecture/overview.md).

---

## Quickstart

### 1. Build and Install

```bash
git clone https://github.com/msaeedb40/orchestra.git
cd orchestra
./scripts/build.sh

# Install binaries to PATH
sudo cp target/release/orchestra target/release/orchestrad /usr/local/bin/
```

### 2. Launch the Daemon

```bash
orchestrad --listen-addr 127.0.0.1:50051
```

### 3. Provision a Kubernetes Cluster

```bash
orchestra cluster create \
  --name demo-k8s \
  --k8s-version v1.36.6 \
  --control-plane 1 \
  --workers 2 \
  --cni kindnet \
  --proxy-mode orchestra
```

### 4. Inspect Status

```bash
orchestra cluster list
orchestra node list --cluster demo-k8s
orchestra network status
orchestra ebpf status
```

---

## Declarative Configuration

Clusters can be declared using standard YAML manifests:

```yaml
apiVersion: orchestra.io/v1alpha1
kind: Cluster
metadata:
  name: prod-cluster
spec:
  kubernetesVersion: v1.36.6
  nodes:
    controlPlane:
      count: 3
    workers:
      count: 5
  network:
    cni: kindnet
    podCidr: 10.244.0.0/16
    serviceCidr: 10.96.0.0/12
  proxy:
    mode: orchestra
```

---

## Testing & Quality Assurance

Orchestra features an exhaustive, zero-failure test suite spanning all layers:

```bash
# Run 96 unit, integration, and E2E tests across the workspace
./scripts/test.sh

# Verify formatting and strict Clippy linter
./scripts/lint.sh

# Run Criterion benchmarks
cargo bench -p orchestra-benchmarks -- --test
```

---

## Documentation

- [Architecture Overview](docs/architecture/overview.md)
- [eBPF Dataplane & BPF Maps](docs/architecture/ebpf-dataplane.md)
- [Security & Isolation Model](docs/architecture/security-model.md)
- [Quickstart Guide](docs/user-guide/quickstart.md)
- [CLI Reference Manual](docs/user-guide/cli-reference.md)
- [CNI Integration & Coordination](docs/user-guide/cni-integration.md)
- [Developer Environment Setup](docs/development/setup.md)
- [Configuration Schema Reference](docs/configuration/schema.md)

---

## License

Orchestra is licensed under the [Apache License, Version 2.0](LICENSE).

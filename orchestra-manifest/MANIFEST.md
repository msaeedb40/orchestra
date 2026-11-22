# Orchestra — Project Manifest

> Kubernetes Cluster Orchestration Platform with eBPF Networking

---

## Identity

| Field             | Value                                             |
|-------------------|---------------------------------------------------|
| **Project**       | Orchestra                                         |
| **Version**       | 0.1.0                                             |
| **License**       | Apache-2.0                                        |
| **Rust Edition**  | 2024                                              |
| **Rust Toolchain**| 1.99.1                                            |
| **Repository**    | https://github.com/msaeedb40/orchestra.git        |
| **Branch**        | `mybuilds`                                        |
| **Authors**       | Orchestra Contributors                            |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                       orchestra (CLI)                           │
│                   gRPC client → localhost:50051                  │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│                     orchestrad (Daemon)                         │
│         gRPC server · reconciliation loop · state DB            │
│                                                                 │
│  ┌──────────┐  ┌───────────┐  ┌────────────┐  ┌─────────────┐  │
│  │ server   │  │ reconcile │  │ bootstrap  │  │   storage   │  │
│  │ (gRPC)   │  │ (state)   │  │ (kubeadm)  │  │  (SQLite)   │  │
│  └──────────┘  └───────────┘  └────────────┘  └─────────────┘  │
└─────────────────────┬───────────────────────────────────────────┘
                      │  Docker API (bollard)
                      ▼
┌─────────────────────────────────────────────────────────────────┐
│              Docker Engine / Docker Desktop                      │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │ cluster1-cp-0│  │cluster1-     │  │cluster1-     │           │
│  │  (control    │  │ worker-0     │  │ worker-1     │           │
│  │   plane)     │  │              │  │              │           │
│  │  orchestra-  │  │  orchestra-  │  │  orchestra-  │           │
│  │  node:1.37.1 │  │  node:1.37.1 │  │  node:1.37.1 │           │
│  └──────────────┘  └──────────────┘  └──────────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

---

## Workspace Crates

### Core Crates

| Crate                      | Path                                | Description                                                 |
|----------------------------|-------------------------------------|-------------------------------------------------------------|
| `orchestra-cli`            | `crates/orchestra-cli`              | CLI binary (`orchestra`) — cluster create/delete/list/inspect|
| `orchestra-daemon`         | `crates/orchestra-daemon`           | Daemon binary (`orchestrad`) — gRPC server + reconciliation |
| `orchestra-core`           | `crates/orchestra-core`             | Domain model — cluster, node, network types and enums       |
| `orchestra-api`            | `crates/orchestra-api`              | Generated gRPC/protobuf client and server stubs             |
| `orchestra-runtime`        | `crates/orchestra-runtime`          | Container runtime abstraction via Docker API (bollard)       |
| `orchestra-kubernetes`     | `crates/orchestra-kubernetes`       | Kubeadm bootstrap — init, join, kubeconfig extraction       |
| `orchestra-network`        | `crates/orchestra-network`          | Network provisioning — Docker bridge networks, IPAM         |
| `orchestra-ebpf`           | `crates/orchestra-ebpf`             | eBPF program management and proxy dataplane                 |
| `orchestra-storage`        | `crates/orchestra-storage`          | SQLite persistence layer for cluster/node state             |
| `orchestra-config`         | `crates/orchestra-config`           | Configuration loading and validation                        |
| `orchestra-observability`  | `crates/orchestra-observability`    | Logging, metrics, and tracing integration                   |
| `orchestra-security`       | `crates/orchestra-security`         | Certificate management and security policies                |
| `orchestra-common`         | `crates/orchestra-common`           | Shared utilities and constants                              |

### eBPF Crates

| Crate                        | Path                               | Description                                      |
|------------------------------|-------------------------------------|--------------------------------------------------|
| `orchestra-ebpf-common`     | `ebpf/orchestra-ebpf-common`        | Shared eBPF types between user-space and kernel  |
| `orchestra-ebpf-programs`   | `ebpf/orchestra-ebpf-programs`      | eBPF programs (XDP, TC, cgroup-level)            |

### Test & Bench

| Crate      | Path       | Description                     |
|------------|------------|---------------------------------|
| `tests`    | `tests/`   | Integration test suite          |
| `benches`  | `benches/` | Performance benchmarks (criterion) |

---

## Binaries

| Binary         | Crate              | Install Path (user)        | Install Path (system)     |
|----------------|--------------------|-----------------------------|---------------------------|
| `orchestra`    | `orchestra-cli`    | `~/.local/bin/orchestra`    | `/usr/local/bin/orchestra` |
| `orchestrad`   | `orchestra-daemon` | `~/.local/bin/orchestrad`   | `/usr/local/bin/orchestrad`|

---

## Proto / gRPC API

All `.proto` files live under `proto/orchestra/` with versioned sub-packages:

| Service Domain | Proto Path                          | Description                              |
|----------------|-------------------------------------|------------------------------------------|
| Cluster        | `proto/orchestra/cluster/v1/`       | CreateCluster, DeleteCluster, ListClusters, GetCluster |
| Node           | `proto/orchestra/node/v1/`          | Node lifecycle, inspection, health       |
| Network        | `proto/orchestra/network/v1/`       | Network provisioning, CNI configuration  |
| System         | `proto/orchestra/system/v1/`        | Ping, version, daemon status             |
| eBPF           | `proto/orchestra/ebpf/v1/`          | eBPF program load/unload/status          |
| Common         | `proto/orchestra/common/v1/`        | Shared message types and enums           |

**Transport**: gRPC over `127.0.0.1:50051` (default)

---

## Node Images

Container images used as Kubernetes "nodes" (Docker-in-Docker pattern with containerd):

| Image                      | Base         | Kubernetes | Entrypoint Features                            |
|----------------------------|--------------|------------|-------------------------------------------------|
| `orchestra-node:1.34.12`   | Ubuntu 24.04 | v1.34.x    | cgroup v2 delegation, systemctl shim, containerd|
| `orchestra-node:1.35.9`    | Ubuntu 24.04 | v1.35.x    | cgroup v2 delegation, systemctl shim, containerd|
| `orchestra-node:1.36.5`    | Ubuntu 24.04 | v1.36.x    | cgroup v2 delegation, systemctl shim, containerd|
| `orchestra-node:1.37.1`    | Ubuntu 24.04 | v1.37.x    | cgroup v2 delegation, systemctl shim, containerd|

**Entrypoint features** (all images):
- `mount --make-rshared /` and `/sys` for CNI volume propagation
- cgroup v2 `init.scope` process migration and subtree controller delegation
- Embedded `systemctl` shim translating kubelet start/stop/restart
- Kubelet bootstrap args: `--bootstrap-kubeconfig` + `--kubeconfig`
- containerd daemon with CRI enabled

**Dockerfiles**: `images/orchestra-node-v{VERSION}/Dockerfile`

---

## Deployment Modes

### 1. Systemd User Service (Development / Desktop)

```
deployments/systemd/orchestrad-user.service
```

| Setting              | Value                                           |
|----------------------|-------------------------------------------------|
| Socket               | `unix://$HOME/.docker/desktop/docker.sock`      |
| Listen               | `127.0.0.1:50051`                               |
| Database             | `~/.local/share/orchestra/orchestra.db`          |
| Log level            | `info,orchestra_daemon=debug`                   |
| Install target       | `default.target`                                |

```bash
# Install & start
cp deployments/systemd/orchestrad-user.service ~/.config/systemd/user/orchestrad.service
systemctl --user daemon-reload
systemctl --user enable --now orchestrad
```

### 2. Systemd System Service (Production / Server)

```
deployments/systemd/orchestrad.service
```

| Setting              | Value                                           |
|----------------------|-------------------------------------------------|
| Socket               | Standard Docker socket                          |
| Listen               | `127.0.0.1:50051`                               |
| Database             | `/var/lib/orchestra/orchestra.db`                |
| User                 | `root`                                          |
| Security             | `ProtectSystem=full`, capability-bounded         |
| Install target       | `multi-user.target`                             |

### 3. Docker Compose (Containerized Daemon)

```
deployments/docker/Dockerfile
deployments/docker/docker-compose.yml
```

---

## CNI & Networking

### Supported CNI Plugins

| CNI            | Enum Value     | Description                                      |
|----------------|----------------|--------------------------------------------------|
| OrchestraNet   | `orchestra`    | Built-in CNI with eBPF dataplane (default)       |
| Cilium         | `cilium`       | eBPF-native CNI — kube-proxy replacement capable |
| Flannel        | `flannel`      | VXLAN overlay networking                         |
| Calico         | `calico`       | BGP/VXLAN with network policy                    |
| None           | `none`         | No CNI installed — bring your own                |

### Service Proxy Modes

| Mode         | Enum Value   | Description                                        |
|--------------|--------------|----------------------------------------------------|
| eBPF         | `ebpf`       | Orchestra's native eBPF proxy (default)            |
| iptables     | `iptables`   | Standard kube-proxy iptables mode                  |
| IPVS         | `ipvs`       | kube-proxy IPVS mode                               |
| CNI          | `cni`        | Delegate to CNI plugin (required for Cilium KPR)   |
| None         | `none`       | No service proxy                                   |

### Network Defaults

| Parameter        | Default           |
|------------------|-------------------|
| Pod CIDR         | `10.244.0.0/16`   |
| Service CIDR     | `10.96.0.0/12`    |
| Node Network     | `172.16.0.0/16`   |
| API Server Port  | `6443`            |

---

## Current Cluster State

> Snapshot taken 2026-11-22T09:08:00+03:00

### Nodes

| Node                | Role           | Status | Version | IP          | OS               | Container Runtime  |
|---------------------|----------------|--------|---------|-------------|------------------|--------------------|
| `cluster1-cp-0`     | control-plane  | Ready  | v1.37.2 | 172.16.0.2  | Ubuntu 24.04.5   | containerd://2.2.1 |
| `cluster1-worker-0` | worker         | Ready  | v1.37.2 | 172.16.0.3  | Ubuntu 24.04.5   | containerd://2.2.1 |
| `cluster1-worker-1` | worker         | Ready  | v1.37.2 | 172.16.0.4  | Ubuntu 24.04.5   | containerd://2.2.1 |

### Cilium CNI

| Component        | Status  | Version | Replicas |
|------------------|---------|---------|----------|
| Cilium Agent     | Running | 1.20.5  | 3/3      |
| Cilium Envoy     | Running | 1.39.6  | 3/3      |
| Cilium Operator  | Running | 1.20.5  | 1/1      |
| Hubble Relay     | disabled| —       | —        |
| ClusterMesh      | disabled| —       | —        |

**Helm release**: `cilium` in `kube-system` (chart `cilium-1.20.5`)
**kube-proxy**: Deleted — Cilium runs with `kubeProxyReplacement=true`

### Control Plane Components

| Component                | Status  |
|--------------------------|---------|
| kube-apiserver           | Running |
| kube-controller-manager  | Running |
| kube-scheduler           | Running |
| etcd                     | Running |
| CoreDNS                  | Running (2 replicas) |

---

## Scripts

| Script                     | Path                           | Purpose                                  |
|----------------------------|--------------------------------|------------------------------------------|
| `setup.sh`                 | `setup.sh`                     | Master setup & bootstrap                 |
| `build.sh`                 | `scripts/build.sh`             | Build all workspace crates               |
| `build-ebpf.sh`            | `scripts/build-ebpf.sh`        | Compile eBPF programs                    |
| `build-node-images.sh`     | `scripts/build-node-images.sh` | Build all orchestra-node Docker images   |
| `install.sh`               | `scripts/install.sh`           | Install binaries to system/user paths    |
| `preflight.sh`             | `scripts/preflight.sh`         | Pre-flight environment validation        |
| `lint.sh`                  | `scripts/lint.sh`              | Run clippy and formatting checks         |
| `test.sh`                  | `scripts/test.sh`              | Run test suite                           |

---

## Key Dependencies

| Dependency          | Version    | Purpose                                |
|---------------------|------------|----------------------------------------|
| `tokio`             | 1.x        | Async runtime                          |
| `tonic`             | 0.12       | gRPC framework                         |
| `prost`             | 0.13       | Protobuf code generation               |
| `bollard`           | 0.18       | Docker Engine API client               |
| `rusqlite`          | 0.41       | SQLite storage (bundled)               |
| `clap`              | 4.5        | CLI argument parsing                   |
| `tracing`           | 0.1        | Structured logging & telemetry         |
| `serde`             | 1.x        | Serialization/deserialization          |
| `chrono`            | 0.4        | Date/time handling                     |
| `uuid`              | 1.x        | Unique identifiers (v4)               |
| `ipnet`             | 2.x        | IP network types                       |

---

## Directory Structure

```
orchestra/
├── Cargo.toml                     # Workspace root
├── setup.sh                       # Master bootstrap script
├── crates/
│   ├── orchestra-cli/             # CLI binary
│   ├── orchestra-daemon/          # Daemon binary (orchestrad)
│   ├── orchestra-core/            # Domain model
│   ├── orchestra-api/             # gRPC stubs
│   ├── orchestra-runtime/         # Container runtime (Docker)
│   ├── orchestra-kubernetes/      # Kubeadm bootstrap
│   ├── orchestra-network/         # Network provisioning
│   ├── orchestra-ebpf/            # eBPF management
│   ├── orchestra-storage/         # SQLite persistence
│   ├── orchestra-config/          # Configuration
│   ├── orchestra-observability/   # Logging & metrics
│   ├── orchestra-security/        # Certificates & security
│   └── orchestra-common/          # Shared utilities
├── ebpf/
│   ├── orchestra-ebpf-common/     # Shared eBPF types
│   └── orchestra-ebpf-programs/   # eBPF XDP/TC programs
├── proto/orchestra/               # Protobuf definitions
│   ├── cluster/v1/
│   ├── node/v1/
│   ├── network/v1/
│   ├── system/v1/
│   ├── ebpf/v1/
│   └── common/v1/
├── images/
│   ├── orchestra-node-v1.34.12/   # Node image (K8s 1.34)
│   ├── orchestra-node-v1.35.9/    # Node image (K8s 1.35)
│   ├── orchestra-node-v1.36.5/    # Node image (K8s 1.36)
│   └── orchestra-node-v1.37.1/    # Node image (K8s 1.37)
├── deployments/
│   ├── docker/                    # Docker Compose deployment
│   ├── kubernetes/                # Kubernetes manifests
│   └── systemd/                   # Systemd unit files
├── scripts/                       # Build, test, install scripts
├── configs/                       # Configuration templates
├── docs/                          # Documentation
├── tests/                         # Integration tests
├── benches/                       # Benchmarks
└── orchestra-manifest/            # This manifest
```

---

## Quick Reference

```bash
# Build
cargo build -p orchestra-daemon -p orchestra-cli

# Install (user)
systemctl --user stop orchestrad
cp target/debug/orchestrad ~/.local/bin/orchestrad
cp target/debug/orchestra ~/.local/bin/orchestra
systemctl --user start orchestrad

# Verify daemon
orchestra system ping

# Create cluster with Cilium
orchestra cluster create mycluster --cni cilium --proxy-mode cni --workers 2

# Install Cilium 1.20.5
cilium install --version 1.20.5 --set kubeProxyReplacement=true

# Check status
kubectl get nodes
cilium status
```

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-07

### Added
- **Multi-Version Node Image Project (`images/orchestra-node`)**:
  - Multi-stage Dockerfile packaging containerd, runc, systemd, crictl, kubeadm, kubelet, and kubectl for Kubernetes versions v1.34 through v1.37.
  - Custom init entrypoint (`entrypoint.sh`) and container healthcheck script (`healthcheck.sh`).
  - Automated build script (`scripts/build-node-images.sh`).
- **Container Runtime Management (`orchestra-runtime`)**:
  - `DockerRuntimeProvider` implementing `orchestra_core::traits::RuntimeProvider`.
  - Resilient Docker socket connection, container lifecycle management, exec stream execution, and volume bindings.
- **gRPC API Contracts & IPC (`orchestra-api`)**:
  - Protocol buffer definitions (`proto/orchestra/`): `common`, `cluster`, `node`, `network`, `ebpf`, and `system`.
  - Generated Tonic modules and unified client aggregator (`OrchestraClient`).
- **Kubernetes Bootstrap Engine (`orchestra-kubernetes`)**:
  - Multi-version `kubeadm` init configuration generator with kube-proxy phase skip support.
  - Worker node join configuration generator.
  - `admin.conf` server endpoint rewriting for localhost host-port forwarding.
  - Declarative CNI manifest generation for Kindnet and Flannel.
  - `ClusterBootstrapper` coordinator.
- **Daemon Core (`orchestra-daemon`)**:
  - State manager (`DaemonState`) coupling SQLite database and container runtime provider.
  - High-performance gRPC server implementation (`GrpcServer`).
  - Standalone daemon binary (`orchestrad`).
- **CLI & User Experience (`orchestra-cli`)**:
  - Comprehensive Clap CLI commands for clusters, nodes, networking, eBPF, and system diagnostics.
  - Tabular, JSON, and YAML output formatters.
  - Standalone CLI binary (`orchestra`).
- **Network Engine & CNI Integration (`orchestra-network`)**:
  - `IpamPool` for IPv4 subnet reservation and allocation.
  - Linux bridge (`BridgeManager`), veth pair (`VethManager`), and netlink routing (`RouteManager`).
  - `CniCoordinator` validating compatibility and conflict avoidance between CNI plugins and eBPF proxy modes.
- **eBPF Service Dataplane (`orchestra-ebpf` & `ebpf/*`)**:
  - Shared `#![no_std]` ABI types (`ServiceKey`, `ServiceValue`, `BackendKey`, `BackendValue`, `ConntrackKey`, `ConntrackEntry`, `PacketEvent`).
  - In-kernel XDP and TC packet parsing, 5-tuple hash selection, and NAT translation logic.
  - Userspace BPF map manager (`BpfMapManager`), program registry (`ProgramManager`), and link attachment handle (`LinkManager`).
- **Observability, Metrics & Diagnostics (`orchestra-observability`)**:
  - Thread-safe Prometheus metric registry (`MetricsRegistry`).
  - Real-time domain event broadcast collector and ring buffer.
  - Structured JSON diagnostic report generator.
- **Security, Isolation & Hardening (`orchestra-security`)**:
  - Security policy engine with `StandardNode`, `RestrictedWorker`, and `Privileged` presets.
  - OCI-compliant Seccomp profile generator filtering dangerous syscalls while permitting containerized Kubernetes nodes and eBPF.
  - Cgroups v2 unified hierarchy and Linux namespace isolation auditor.
  - RBAC authorization enforcer and bearer token manager.
  - Thread-safe structured audit logger.
- **End-to-End Integration Testing (`tests/`)**:
  - Integration harness (`TestServer`) provisioning in-process gRPC server and in-memory database.
  - 13 comprehensive integration and E2E test suites covering the entire platform.
- **CI/CD, Automation & Tooling (`.github/workflows`)**:
  - GitHub Actions workflows for continuous compilation, test execution, linting (rustfmt and clippy), security audits, multi-version image builds, and multi-arch release binaries.
  - Standardized issue and pull request templates.
- **Benchmarking & Performance Profiling (`benches/`)**:
  - Criterion benchmark suites for eBPF map lookups, conntrack insertion, IPAM allocation, config parsing, and kubeadm manifest generation.
- **Documentation & Deployments (`docs/`, `deployments/`, `scripts/`)**:
  - Comprehensive architectural specifications, user guides, CLI reference, and developer guides.
  - Systemd service unit and Docker Compose deployment manifests.
  - Developer operational helper scripts (`build.sh`, `test.sh`, `lint.sh`, `build-ebpf.sh`).

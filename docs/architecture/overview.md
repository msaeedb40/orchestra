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

## Reconciliation Loop

The daemon drives cluster lifecycle through a finite state machine in `orchestra-daemon::reconciliation`:

```
  Pending ──► Creating ──► Bootstrapping ──► Running
                 │               │
                 └──► Failed ◄───┘
```

### State Transitions

1. **Pending → Creating**: Triggered by `CreateClusterRequest`. The daemon provisions Docker containers for the control plane (`{name}-cp-0`) and worker nodes (`{name}-worker-{n}`) via `orchestra-runtime`, each running the `orchestra-node:{version}` image. Containers are connected to a dedicated Docker bridge network.

2. **Creating → Bootstrapping**: Once all node containers are running (health check passes), the daemon advances to bootstrapping.

3. **Bootstrapping**: The daemon executes the following sequence:
   - Runs `kubeadm init --config=/tmp/kubeadm-config.yaml` on the control-plane container.
   - Extracts the kubeconfig from `/etc/kubernetes/super-admin.conf` (with fallback to `admin.conf` for older versions).
   - Creates RBAC bindings required for worker joins:
     - `kubernetes-admin-binding` ClusterRoleBinding (binds `kubernetes-admin` user to `cluster-admin` role).
     - `system:discovery` RoleBinding on `kube-public` namespace (allows `system:unauthenticated` to read the `cluster-info` ConfigMap).
   - Generates a `kubeadm join` command and executes it on each worker container sequentially.

4. **Bootstrapping → Running**: After all workers join successfully, the daemon writes the kubeconfig to `~/.kube/orchestra-{name}.conf` and `~/.kube/config`, then marks the cluster as `Running`.

5. **→ Failed**: Any unrecoverable error during creating or bootstrapping transitions to `Failed` with the error recorded in the operation journal.

## Mount Propagation & cgroup v2

Orchestra node containers require specific mount and cgroup configuration to run kubelet and CNI plugins inside Docker-in-Docker:

### Mount Propagation

The node `entrypoint.sh` runs at container boot:

```bash
mount --make-rshared /
mount --make-rshared /sys
```

This ensures that volume mounts created by kubelet (for pod volumes) and CNI plugins (for eBPF filesystems at `/sys/fs/bpf`) propagate correctly between the host mount namespace and pod mount namespaces. Without `rshared` propagation, Cilium Envoy pods fail with:

```
path "/sys/fs/bpf" is mounted on "/sys" but it is not a shared or slave mount
```

### cgroup v2 Subtree Delegation

On unified cgroup v2 hosts (Docker Desktop, modern Linux), the entrypoint migrates all root-cgroup processes into an `init.scope` sub-cgroup before enabling controllers:

```bash
mkdir -p /sys/fs/cgroup/init.scope
echo $$ > /sys/fs/cgroup/init.scope/cgroup.procs
for p in $(cat /sys/fs/cgroup/cgroup.procs); do
    echo "$p" > /sys/fs/cgroup/init.scope/cgroup.procs
done
for c in $(cat /sys/fs/cgroup/cgroup.controllers); do
    echo "+$c" > /sys/fs/cgroup/cgroup.subtree_control
done
```

This is required because the cgroup v2 "no internal processes" rule forbids enabling subtree controllers while processes exist directly in a domain cgroup. Without this delegation, kubelet fails with:

```
cannot enter cgroupv2 "/sys/fs/cgroup/kubepods" with domain controllers -- it is in an invalid state
```


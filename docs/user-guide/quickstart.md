# Quickstart Guide

This guide walks you through installing Orchestra, starting the daemon, and spinning up your first Kubernetes cluster in Docker with eBPF acceleration.

## Prerequisites

- **Linux OS** (Kernel 5.15+ recommended, cgroups v2 unified hierarchy)
- **Docker Engine** (24.0+) or **Docker Desktop** for Linux
- **Kubectl** (`curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"`)
- **Rust toolchain** (stable ≥ 1.85 + nightly) — install via [rustup](https://rustup.rs)

> **Tip:** Run `./scripts/preflight.sh` after cloning to check all requirements in one pass.

---

## 1. Clone & Run Setup

The simplest path — `setup.sh` handles everything:

```bash
git clone https://github.com/msaeedb40/orchestra.git
cd orchestra

# Full setup: preflight → build → install → systemd service
./setup.sh
```

Or step by step:

```bash
./setup.sh --check    # Validate environment
./setup.sh --deps     # Install system packages (needs sudo)
./setup.sh --build    # Build workspace + node images
./setup.sh --install  # Install binaries to ~/.local/bin
./setup.sh --daemon   # Configure & start orchestrad as systemd user service
```

---

## 2. Verify the Daemon

```bash
# Check systemd user service
systemctl --user status orchestrad

# Ping the daemon
orchestra system ping
# → OK: pong (timestamp: ...)

# View daemon info
orchestra system info
```

If running manually instead of via systemd:

```bash
DOCKER_HOST="unix://$HOME/.docker/desktop/docker.sock" \
RUST_LOG="info,orchestra_daemon=debug" \
orchestrad --listen-addr 127.0.0.1:50051 \
           --db-path ~/.local/share/orchestra/orchestra.db &
```

> **Docker Desktop users:** Orchestra auto-detects `~/.docker/desktop/docker.sock`. The systemd user service sets `DOCKER_HOST` automatically.

---

## 3. Create Your First Cluster

Create a Kubernetes v1.37.1 cluster with 1 control plane and 2 worker nodes:

```bash
orchestra cluster create my-cluster \
  --kubernetes-version v1.37.1 \
  --control-plane 1 \
  --workers 2 \
  --cni orchestranet \
  --proxy-mode orchestra
```

Watch cluster state converge:

```bash
# Poll until Running
orchestra cluster get my-cluster

# Example output:
# NAME        STATUS    CONTROL-PLANE  WORKERS  VERSION
# my-cluster  Running   1              2        v1.37.1
```

---

## 4. Inspect the Cluster

```bash
# List all clusters
orchestra cluster list

# List nodes (positional cluster name)
orchestra node list my-cluster

# Example output:
# NAME               ROLE           STATUS  IP
# my-cluster-cp-0    ControlPlane   Ready   172.16.0.2
# my-cluster-wkr-0   Worker         Ready   172.16.0.3
# my-cluster-wkr-1   Worker         Ready   172.16.0.4

# Check OrchestraNet CNI + eBPF status
orchestra network status my-cluster
orchestra ebpf status

# System diagnostics
orchestra system diagnostics
```

---

## 5. Using Kubectl

The control-plane node exposes the Kubernetes API. Get the kubeconfig:

```bash
# Forward from container to localhost
DOCKER_HOST="unix://$HOME/.docker/desktop/docker.sock" \
  docker exec my-cluster-cp-0 cat /etc/kubernetes/admin.conf > ~/.kube/orchestra-my-cluster.conf

export KUBECONFIG=~/.kube/orchestra-my-cluster.conf
kubectl get nodes
```

---

## 6. Teardown

```bash
orchestra cluster delete my-cluster --yes
```

---

## Cluster Configuration via YAML

Instead of CLI flags, you can use a declarative YAML manifest:

```yaml
# my-cluster.yaml
apiVersion: orchestra.io/v1alpha1
kind: Cluster
metadata:
  name: my-cluster
spec:
  kubernetesVersion: v1.37.1
  nodes:
    controlPlane:
      count: 1
    workers:
      count: 2
  network:
    cni: orchestranet
    podCidr: 10.244.0.0/16
    serviceCidr: 10.96.0.0/12
  proxy:
    mode: orchestra
  ebpf:
    enabled: true
```

```bash
orchestra cluster create --config my-cluster.yaml
```

See [Configuration Schema](../configuration/schema.md) for all fields.

---

## Available Node Images

| Image Tag | Kubernetes Version |
|-----------|-------------------|
| `orchestra-node:1.37.1` | v1.37.1 |
| `orchestra-node:1.36.6` | v1.36.6 |
| `orchestra-node:1.35.9` | v1.35.9 |
| `orchestra-node:1.34.12` | v1.34.12 |

Build a specific version:

```bash
./scripts/build-node-images.sh 1.36.6
```

---

## Next Steps

- [CLI Reference](cli-reference.md) — all commands and flags
- [CNI Integration](cni-integration.md) — OrchestraNet, Flannel, Calico, Cilium
- [Configuration Schema](../configuration/schema.md) — full YAML field reference
- [Developer Setup](../development/setup.md) — building from source, testing, IDE setup
- [eBPF Dataplane](../architecture/ebpf-dataplane.md) — how the in-kernel service proxy works

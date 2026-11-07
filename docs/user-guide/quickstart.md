# Quickstart Guide

This guide walks you through installing Orchestra, starting the daemon, and spinning up your first Kubernetes cluster in Docker with eBPF acceleration.

## Prerequisites

- **Linux OS** (Kernel 5.15+ recommended with cgroups v2 unified hierarchy)
- **Docker Engine** (24.0+)
- **Kubectl** (`curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"`)

## 1. Build and Install

```bash
git clone https://github.com/msaeedb40/orchestra.git
cd orchestra
./scripts/build.sh

# Install binaries to PATH
sudo cp target/release/orchestra target/release/orchestrad /usr/local/bin/
```

## 2. Start the Daemon

```bash
# In one terminal or as a systemd service:
orchestrad --listen-addr 127.0.0.1:50051
```

Verify connectivity:
```bash
orchestra system ping
# Returns: OK: pong (timestamp: ...)
```

## 3. Create Your First Cluster

Create a standard Kubernetes v1.36 cluster with 1 control plane and 2 worker nodes:

```bash
orchestra cluster create \
  --name my-cluster \
  --k8s-version v1.36.6 \
  --control-plane 1 \
  --workers 2 \
  --cni kindnet \
  --proxy-mode orchestra
```

## 4. Inspect Topology

```bash
orchestra cluster list
orchestra node list --cluster my-cluster
orchestra network status
orchestra ebpf status
```

## 5. Teardown

```bash
orchestra cluster delete --name my-cluster
```


# CLI Reference Guide

The `orchestra` CLI provides commands to control clusters, nodes, networks, eBPF dataplanes, and the daemon. It communicates with `orchestrad` over a local gRPC socket.

## Global Options

| Flag | Default | Env var | Description |
|------|---------|---------|-------------|
| `-d, --daemon-addr <ADDR>` | `http://127.0.0.1:50051` | `ORCHESTRA_DAEMON_ADDR` | gRPC endpoint for the daemon |
| `-o, --output <FORMAT>` | `text` | — | Output format: `text`, `json`, `yaml`, `wide` |
| `-v, --verbose` | — | — | Increase verbosity (`-v` debug, `-vv` trace) |

---

## `orchestra cluster`

Manages the full Kubernetes cluster lifecycle.

| Command | Description |
|---------|-------------|
| `orchestra cluster list` | List all managed clusters with status, version, node counts |
| `orchestra cluster get <NAME>` | Detailed cluster status, node list, and API endpoint |
| `orchestra cluster create <NAME> [OPTIONS]` | Provision a new cluster |
| `orchestra cluster delete <NAME> [--yes]` | Delete all node containers and network resources |
| `orchestra cluster start <NAME>` | Resume stopped node containers |
| `orchestra cluster stop <NAME>` | Pause running node containers |
| `orchestra cluster inspect <NAME>` | Full topology, network, eBPF, and health dump |

### `cluster create` options

| Flag | Default | Description |
|------|---------|-------------|
| `--kubernetes-version <VER>` | latest stable | e.g. `v1.37.1` |
| `--control-plane <COUNT>` | `1` | Number of control-plane nodes |
| `--workers <COUNT>` | `2` | Number of worker nodes |
| `--cni <CNI>` | `orchestranet` | CNI plugin: `orchestranet`, `flannel`, `calico`, `cilium`, `none` |
| `--proxy-mode <MODE>` | `orchestra` | Service proxy: `orchestra`, `cni`, `kube-proxy` |
| `--pod-cidr <CIDR>` | `10.244.0.0/16` | Pod IP allocation range |
| `--service-cidr <CIDR>` | `10.96.0.0/12` | Service VIP range |
| `--api-port <PORT>` | `6443` | Host port for the Kubernetes API server |
| `--config <FILE>` | — | Load spec from a YAML/TOML manifest |
| `--wait` | false | Block until the cluster reaches `Running` |
| `--timeout <DURATION>` | `10m` | Maximum wait time (e.g. `10m`, `600s`) |

### Examples

```bash
# Minimal dev cluster (defaults)
orchestra cluster create dev

# Production-like with Cilium
orchestra cluster create prod \
  --kubernetes-version v1.37.1 \
  --control-plane 1 \
  --workers 3 \
  --cni cilium \
  --proxy-mode cni \
  --wait

# From YAML manifest
orchestra cluster create --config cluster.yaml

# Delete without prompt
orchestra cluster delete dev --yes
```

---

## `orchestra node`

Manages individual Kubernetes node containers within a cluster.

| Command | Description |
|---------|-------------|
| `orchestra node list <CLUSTER>` | List nodes: name, role, status, IP |
| `orchestra node get <CLUSTER> <NODE>` | Detailed node info |
| `orchestra node add <CLUSTER> --role <ROLE>` | Add a worker or control-plane node |
| `orchestra node remove <CLUSTER> <NODE>` | Drain and delete a node |
| `orchestra node restart <CLUSTER> <NODE>` | Restart a node container |
| `orchestra node inspect <CLUSTER> <NODE>` | Full node inspection dump |

```bash
# List nodes (positional cluster name)
orchestra node list my-cluster

# Add a worker
orchestra node add my-cluster --role worker

# Remove a specific node
orchestra node remove my-cluster my-cluster-wkr-1
```

---

## `orchestra network`

Inspects and configures cluster networking and CNI integration.

| Command | Description |
|---------|-------------|
| `orchestra network status <CLUSTER>` | Bridge interfaces, subnet, CNI mode |
| `orchestra network inspect <CLUSTER>` | Deep topology inspection |
| `orchestra network mode <CLUSTER>` | Show current proxy mode |
| `orchestra network endpoints <CLUSTER>` | List active service endpoints |
| `orchestra network configure <CLUSTER> --service-proxy <MODE>` | Change proxy mode |

---

## `orchestra ebpf`

Provides visibility into Orchestra's in-kernel eBPF programs.

| Command | Description |
|---------|-------------|
| `orchestra ebpf status` | Active programs, hook types (XDP/TC), map sizes |
| `orchestra ebpf programs` | List loaded eBPF programs |
| `orchestra ebpf maps` | Inspect BPF maps (services, backends, conntrack) |
| `orchestra ebpf links` | Show BPF link attachments |
| `orchestra ebpf events` | Stream kernel events from the ring buffer |
| `orchestra ebpf inspect` | Full eBPF state dump |

```
PROGRAM          TYPE        ATTACHMENT    STATUS
service_lb       TC          TCX           Loaded
network_monitor  XDP         XDP           Loaded
traffic_events   Tracepoint  Tracepoint    Loaded
```

> **Note:** These commands show only Orchestra-owned BPF programs, not those loaded by external CNIs (e.g. Cilium).

---

## `orchestra kubernetes`

```bash
orchestra kubernetes version    # Kubernetes API server version
orchestra kubernetes status     # Control plane component health
orchestra kubernetes nodes      # Registered Kubernetes nodes
orchestra kubernetes components # Component status breakdown
```

---

## `orchestra system`

| Command | Description |
|---------|-------------|
| `orchestra system ping` | Health check against the daemon |
| `orchestra system info` | Daemon version, OS, architecture, uptime |
| `orchestra system health` | Subsystem health summary |
| `orchestra system diagnostics` | Full diagnostic report: cgroups, networks, BPF |
| `orchestra system version` | Detailed version + build info |

```bash
orchestra system ping
# → OK: pong (timestamp: 2026-11-09T00:00:00Z)
```

---

## `orchestra config`

```bash
orchestra config view           # Show active configuration
orchestra config validate       # Validate a cluster YAML/TOML manifest
orchestra config set <KEY> <VAL>  # Set a config value
orchestra config current-context  # Show current cluster context
```

---

## `orchestra daemon`

```bash
orchestra daemon start    # Start orchestrad in the foreground
orchestra daemon stop     # Send shutdown signal
orchestra daemon status   # Daemon running state and PID
orchestra daemon logs     # Tail daemon log output
```

---

## Shell Completion

```bash
orchestra completion bash  >> ~/.bashrc
orchestra completion zsh   >> ~/.zshrc
orchestra completion fish  >> ~/.config/fish/completions/orchestra.fish
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | General error |
| `2` | Invalid command or arguments |
| `3` | Daemon unavailable |
| `4` | Resource not found |
| `5` | Operation failed |
| `6` | Configuration error |
| `7` | Permission denied |
| `8` | Timeout |

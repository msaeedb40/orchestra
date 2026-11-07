# CLI Reference Guide

The `orchestra` CLI provides commands to control clusters, nodes, networks, eBPF dataplanes, and the daemon.

## Global Options

- `-d, --daemon-addr <ADDR>`: gRPC server endpoint (default: `http://127.0.0.1:50051`, env: `ORCHESTRA_DAEMON_ADDR`).
- `-o, --output <FORMAT>`: Output serialization format (`text`, `json`, `yaml`).
- `-v, --verbose`: Increase log verbosity (`-v` for debug, `-vv` for trace).

---

## Commands

### `orchestra cluster`
- `orchestra cluster list`: List all managed clusters with status, version, and node count.
- `orchestra cluster get --name <NAME>`: Retrieve detailed cluster status and API endpoint.
- `orchestra cluster create [OPTIONS]`: Provision a new Kubernetes cluster.
  - `--name <NAME>`: Unique cluster name.
  - `--k8s-version <VERSION>`: Kubernetes version (e.g. `v1.36.6`).
  - `--control-plane <COUNT>`: Number of control plane nodes (default: `1`).
  - `--workers <COUNT>`: Number of worker nodes (default: `2`).
  - `--cni <CNI>`: CNI provider (`kindnet`, `flannel`, `calico`, `cilium`, `none`).
  - `--proxy-mode <MODE>`: Proxy dataplane (`orchestra`, `cni`, `kube-proxy`).
  - `--pod-cidr <CIDR>`: Pod subnet (default: `10.244.0.0/16`).
  - `--service-cidr <CIDR>`: Service VIP subnet (default: `10.96.0.0/12`).
  - `--api-port <PORT>`: Host port forwarded to Kubernetes API server (default: `6443`).
- `orchestra cluster delete --name <NAME>`: Gracefully delete all node containers and network bridges.
- `orchestra cluster start --name <NAME>`: Resume stopped node containers.
- `orchestra cluster stop --name <NAME>`: Pause running node containers.

### `orchestra node`
- `orchestra node list --cluster <CLUSTER_NAME>`: List all nodes in a cluster.
- `orchestra node add --cluster <CLUSTER_NAME> --role <worker|control-plane>`: Scale out cluster.
- `orchestra node remove --node-id <NODE_ID>`: Drain and delete a node.

### `orchestra network`
- `orchestra network status`: Inspect bridge interfaces, CNI configurations, and subnet pools.

### `orchestra ebpf`
- `orchestra ebpf status`: Display active in-kernel programs, hook types (`XDP`, `TC`), and map sizes.

### `orchestra system`
- `orchestra system ping`: Healthcheck check against the daemon.
- `orchestra system info`: Print daemon version, host OS, architecture, and uptime.
- `orchestra system diagnostics`: Dump diagnostic report including cgroups v2 readiness and network status.


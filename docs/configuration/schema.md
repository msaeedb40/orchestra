# Configuration Schema Reference

Orchestra clusters can be declaratively configured using Kubernetes-style YAML or TOML manifests.

## Example YAML Manifest

```yaml
apiVersion: orchestra.io/v1alpha1
kind: Cluster
metadata:
  name: production-cluster
  labels:
    environment: production
    owner: devops-team
spec:
  kubernetesVersion: v1.37.1
  nodes:
    controlPlane:
      count: 3
      image: orchestra-node:1.37.1
      portMappings:
        - hostPort: 6443
          containerPort: 6443
          protocol: TCP
    workers:
      count: 5
      image: orchestra-node:1.37.1
  network:
    mode: bridge
    bridgeName: orch-br0
    cni: orchestranet
    podCidr: 10.244.0.0/16
    serviceCidr: 10.96.0.0/12
  proxy:
    mode: orchestra
  ebpf:
    enabled: true
    bpfFsPath: /sys/fs/bpf
    packetSamplingRate: 100
  observability:
    enabled: true
    logLevel: info
  security:
    enabled: true
```

## Equivalent TOML Manifest

```toml
[metadata]
name = "production-cluster"

[spec]
kubernetes_version = "v1.37.1"

[spec.nodes.control_plane]
count = 3
image = "orchestra-node:1.37.1"

[spec.nodes.workers]
count = 5
image = "orchestra-node:1.37.1"

[spec.network]
mode = "bridge"
cni = "orchestranet"
pod_cidr = "10.244.0.0/16"
service_cidr = "10.96.0.0/12"

[spec.proxy]
mode = "orchestra"

[spec.ebpf]
enabled = true
```

---

## Field Reference

### `metadata`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | String | ✅ | RFC 1123 DNS-compliant cluster name |
| `labels` | Map | — | Key-value pairs for organizational indexing |

### `spec`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `kubernetesVersion` | String | latest stable | Kubernetes version (`v1.34.x` – `v1.37.x`) |

### `spec.nodes.controlPlane`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `count` | Integer | `1` | Number of control-plane nodes. Use `3` for HA (etcd quorum) |
| `image` | String | `orchestra-node:1.37.1` | OCI image for control-plane containers |
| `portMappings` | List | — | Host→container port forwards (e.g. API server on 6443) |

### `spec.nodes.workers`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `count` | Integer | `2` | Number of worker nodes |
| `image` | String | `orchestra-node:1.37.1` | OCI image for worker containers |

### `spec.network`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | String | `bridge` | Network topology: `bridge`, `host`, `overlay` |
| `cni` | String | `orchestranet` | CNI plugin: `orchestranet`, `flannel`, `calico`, `cilium`, `none` |
| `podCidr` | String | `10.244.0.0/16` | IPv4 CIDR for Pod IP allocation |
| `serviceCidr` | String | `10.96.0.0/12` | IPv4 CIDR for Service VIPs |
| `bridgeName` | String | auto | Linux bridge interface name |

> **Subnet selection:** The Docker node network subnet (`172.x.x.x`) is selected automatically by scanning for available `/16` blocks in `172.16–172.31`. This does not conflict with `podCidr` or `serviceCidr`.

### `spec.proxy`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `mode` | String | `orchestra` | Service load-balancer: `orchestra` (eBPF), `cni`, `kube-proxy` |

> **Cilium restriction:** `proxy.mode = orchestra` is incompatible with `cni = cilium`. Orchestra's validator rejects this combination.

### `spec.ebpf`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | Boolean | `true` | Attach Orchestra's in-kernel XDP/TC hooks |
| `bpfFsPath` | String | `/sys/fs/bpf` | BPF filesystem mount point |
| `packetSamplingRate` | Integer | `100` | Percentage of packets sampled for observability (0–100) |
| `serviceLb` | Boolean | `true` | Enable eBPF service load balancer |
| `conntrack` | Boolean | `true` | Enable BPF connection tracking |

### `spec.observability`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | Boolean | `false` | Enable Prometheus metrics and OpenTelemetry tracing |
| `logLevel` | String | `info` | Log filter (e.g. `info`, `debug`, `info,orchestra_daemon=debug`) |

### `spec.security`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | Boolean | `true` | Enable capability checks, seccomp profiles, and audit logging |

---

## Validation Rules

The daemon validates all manifests before provisioning. Key rules:

1. **`proxy.mode = cni`** requires `cni` to be `calico` or `cilium` (not `orchestranet`, `flannel`, or `none`).
2. **`proxy.mode = orchestra`** is blocked when `cni = cilium`.
3. **`controlPlane.count`** must be `1` or `3` (single or HA quorum).
4. **`kubernetesVersion`** must match a supported node image: `v1.34.x`, `v1.35.x`, `v1.36.x`, or `v1.37.x`.
5. **`name`** must be a valid RFC 1123 DNS label (lowercase, alphanumeric, hyphens).

---

## Pre-configured Profiles

Use `--profile` with `cluster create` for quick starts:

| Profile | Control Plane | Workers | CNI | Proxy |
|---------|:---:|:---:|-----|-------|
| `dev` | 1 | 1 | `orchestranet` | `orchestra` |
| `standard` | 1 | 3 | `cilium` | `cni` |
| `ha` | 3 | 5 | `calico` | `orchestra` |

```bash
orchestra cluster create my-cluster --profile dev
orchestra cluster create prod-cluster --profile standard
orchestra cluster create ha-cluster --profile ha
```

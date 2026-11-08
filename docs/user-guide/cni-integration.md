# CNI Integration & Network Modes

Orchestra supports multiple Container Network Interface (CNI) plugins and coordinates them with its native eBPF service dataplane. The CNI handles **pod networking**; the service proxy mode controls **Service ClusterIP load balancing** — these are independent choices.

## OrchestraNet (Default)

`orchestranet` is Orchestra's own native CNI — a lightweight host-routing CNI designed for Docker-container-based Kubernetes nodes. It is the default for all new clusters and the recommended choice when using Orchestra's eBPF service proxy.

**Characteristics:**
- Point-to-point host routing — no tunneling overhead
- Minimal resource footprint; fast cluster startup
- Fully compatible with Orchestra's XDP/TC eBPF dataplane
- ConfigMap-based configuration (`orchestranet-config` in `kube-system`)

**Example:**
```bash
orchestra cluster create dev --cni orchestranet --proxy-mode orchestra
```

---

## Supported CNIs

| CNI Plugin | Network Topology | Compatible Proxy Modes | Best For |
| :--- | :--- | :--- | :--- |
| **`orchestranet`** | Host routing (native) | `orchestra` ✅ `kube-proxy` ✅ | Default — fast dev & production |
| **`flannel`** | VXLAN Overlay | `orchestra` ✅ `kube-proxy` ✅ | Standard multi-host overlay networking |
| **`calico`** | BGP / Pure IP Routing | `orchestra` ✅ `kube-proxy` ✅ | Enterprise network policy enforcement |
| **`cilium`** | eBPF Native Routing | `cni` ✅ *(see note)* | High-throughput native eBPF routing |
| **`none`** | BYO CNI | Any | Custom CNI deployments |

> **Cilium note:** Cilium implements its own kube-proxy replacement via in-kernel socket-level load balancing. Running Orchestra's eBPF proxy alongside Cilium would cause conflicting DNAT translations. `CniCoordinator` enforces `proxy.mode = cni` when Cilium is selected and blocks `proxy.mode = orchestra` to prevent this.

---

## Service Proxy Modes

| Mode | Service dataplane | Description |
|------|------------------|-------------|
| `orchestra` | Orchestra eBPF (XDP/TC) | Orchestra owns ClusterIP DNAT in-kernel; `kube-proxy` DaemonSet is not deployed |
| `cni` | Selected CNI's built-in proxy | Delegates service routing to the CNI (required for Cilium full-eBPF mode) |
| `kube-proxy` | Standard iptables/ipvs | Traditional Kubernetes kube-proxy; compatible with all CNIs |

---

## eBPF Proxy Mode Architecture

When running with `proxy.mode = orchestra`:

1. Kubeadm skips deploying the standard `kube-proxy` DaemonSet.
2. Orchestra's eBPF dataplane attaches to the bridge interface via `tc_clsact_ingress` and `xdp_ingress` hooks.
3. Service VIP NAT translation is executed in-kernel before entering the Linux network stack.
4. Connection tracking is maintained in the `orchestra_conntrack` BPF LRU hash map.

See [eBPF Dataplane](../architecture/ebpf-dataplane.md) for the full architecture.

---

## CNI + Proxy Compatibility Matrix

| CNI | `orchestra` proxy | `cni` proxy | `kube-proxy` |
|-----|:-----------------:|:-----------:|:------------:|
| `orchestranet` | ✅ Optimal | ⚠️ No CNI proxy built-in | ✅ |
| `flannel` | ✅ | ⚠️ No CNI proxy built-in | ✅ |
| `calico` | ✅ | ✅ Calico eBPF mode | ✅ |
| `cilium` | ❌ Conflict | ✅ Required | ✅ |
| `none` | ✅ | ✅ | ✅ |

---

## Network Subnet Selection

Orchestra automatically finds the first available `/16` subnet in the `172.16.0.0/12` private range (`172.16` – `172.31`) when creating a cluster network. No manual subnet configuration is required. The selection avoids any Docker networks already in use on the host.

The Docker network is named `orchestra-<cluster-name>` and labelled with `orchestra.io/cluster` and `orchestra.io/managed=true` for easy identification.

---

## Inspect Network State

```bash
orchestra network status my-cluster
orchestra network inspect my-cluster
orchestra network mode my-cluster
orchestra network endpoints my-cluster
```

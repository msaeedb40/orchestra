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

### Deploying Cilium 1.20.5 on Orchestra

#### 1. Create the cluster with Cilium CNI

```bash
orchestra cluster create mycluster --cni cilium --proxy-mode cni --workers 2
```

Wait for the cluster to reach `Running` state and all nodes to register:

```bash
orchestra cluster get mycluster
kubectl get nodes
```

#### 2. Remove kube-proxy

Cilium fully replaces kube-proxy. Delete the DaemonSet:

```bash
kubectl -n kube-system delete daemonset kube-proxy
```

#### 3. Install Cilium

**Option A — Cilium CLI:**

```bash
cilium install --version 1.20.5 \
  --set kubeProxyReplacement=true \
  --set k8sServiceHost=<CONTROL_PLANE_IP> \
  --set k8sServicePort=6443
```

**Option B — Helm:**

```bash
helm repo add cilium https://helm.cilium.io
helm repo update
helm install cilium cilium/cilium --version 1.20.5 \
  --namespace kube-system \
  --set kubeProxyReplacement=true \
  --set k8sServiceHost=<CONTROL_PLANE_IP> \
  --set k8sServicePort=6443
```

Replace `<CONTROL_PLANE_IP>` with the actual IP of your control-plane node (e.g. `172.16.0.2`).

#### 4. Verify

```bash
cilium status        # All components should show OK
kubectl get nodes    # All nodes should be Ready
kubectl get pods -n kube-system  # Cilium agent, envoy, and operator running
```

> **Mount Propagation:** Orchestra node containers automatically run `mount --make-rshared /` and `mount --make-rshared /sys` at boot, which is required for Cilium's eBPF filesystem mounts (`/sys/fs/bpf`) and CNI volume propagation. Without this, Cilium Envoy pods fail with `path "/sys/fs/bpf" is mounted on "/sys" but it is not a shared or slave mount`.

> **DNS Loop Fix:** When running CoreDNS inside Orchestra nodes, the container's `/etc/resolv.conf` may point back to the CoreDNS service IP, causing a DNS forwarding loop. Fix by editing the `coredns` ConfigMap in `kube-system`: change `forward . /etc/resolv.conf` to `forward . 8.8.8.8 1.1.1.1`, and remove the `loop` plugin. Then restart CoreDNS with `kubectl rollout restart deployment coredns -n kube-system`.

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

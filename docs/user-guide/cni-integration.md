# CNI Integration & Network Modes

Orchestra allows selecting from multiple Container Network Interfaces (CNIs) and coordinating them with the eBPF dataplane.

## Supported CNIs

| CNI Plugin | Network Topology | Recommended Service Proxy Mode | Best For |
| :--- | :--- | :--- | :--- |
| **`kindnet`** | Point-to-point Host Routing | `orchestra` (eBPF) | Fast local development, minimal overhead |
| **`flannel`** | VXLAN Overlay | `orchestra` (eBPF) | Standard overlay networking across multi-host |
| **`calico`** | BGP / Pure IP Routing | `kube-proxy` or `orchestra` | Enterprise policy enforcement |
| **`cilium`** | eBPF Native Routing | `cni` | High-throughput native eBPF routing |
| **`none`** | BYO CNI | Any | Custom CNI deployments |

## eBPF Proxy Mode Coordination

When running Orchestra with `proxy.mode = orchestra`:
- Kubeadm skips deploying the standard Kubernetes `kube-proxy` DaemonSet.
- Orchestra's eBPF dataplane hooks into the bridge interface (`tc_clsact_ingress` and `xdp_ingress`).
- Service VIP NAT translation is executed in-kernel before entering the Linux network stack.

When running with Cilium:
- Cilium implements its own in-kernel socket-level load balancing.
- To prevent conflicting DNAT actions, set `proxy.mode = cni`.


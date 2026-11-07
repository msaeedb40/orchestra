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
  kubernetesVersion: v1.36.6
  nodes:
    controlPlane:
      count: 3
      image: orchestra-node:1.36.6
      portMappings:
        - hostPort: 6443
          containerPort: 6443
          protocol: TCP
    workers:
      count: 5
      image: orchestra-node:1.36.6
  network:
    mode: bridge
    bridgeName: orch-br0
    cni: kindnet
    podCidr: 10.244.0.0/16
    serviceCidr: 10.96.0.0/12
  proxy:
    mode: orchestra
  ebpf:
    enabled: true
    bpfFsPath: /sys/fs/bpf
    packetSamplingRate: 100
```

## Field Reference

### `metadata`
- `name` (String, required): RFC 1123 DNS-compliant cluster identifier.
- `labels` (Map, optional): Key-value pairs for organizational indexing.

### `spec`
- `kubernetesVersion` (String): Kubernetes version string (`v1.34.0`, `v1.35.0`, `v1.36.0`, `v1.37.0`).
- `nodes.controlPlane.count` (Integer): Number of control plane instances (1 for single-node dev, 3+ for HA).
- `nodes.workers.count` (Integer): Number of worker nodes to schedule workloads.
- `network.cni` (String): CNI plugin (`kindnet`, `flannel`, `calico`, `cilium`, `none`).
- `network.podCidr` (String): IPv4 CIDR block dedicated to Pod IP allocation.
- `network.serviceCidr` (String): IPv4 CIDR block for Kubernetes Service VIPs.
- `proxy.mode` (String): Service load balancer mode (`orchestra`, `cni`, `kube-proxy`).
- `ebpf.enabled` (Boolean): Whether to attach Orchestra's in-kernel XDP/TC hooks.


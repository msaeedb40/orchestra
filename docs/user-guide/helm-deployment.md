# Deploying Orchestra with Helm

Orchestra's Helm chart deploys the `orchestrad` daemon into an existing Kubernetes cluster (the "management cluster"), from which it manages external Docker hosts to provision Orchestra-controlled Kubernetes clusters.

## Architecture

```
Management Kubernetes Cluster
├── orchestrad (Pod)
│   ├── Persistent Volume (SQLite DB)
│   ├── ConfigMap (env vars)
│   └── Service (gRPC :50051)
│
└── Reaches out to Docker Engine
    └── Provisions orchestra-node containers
        └── Managed Kubernetes Clusters
```

## Prerequisites

- Helm 3.x
- An existing Kubernetes cluster (management plane)
- Docker Engine accessible from the Orchestra pod (socket mount or TCP)

## Install

```bash
# From the repo
cd deployments/kubernetes/orchestra

# Default install (Docker socket mode, 10Gi PVC)
helm install orchestra . -n orchestra --create-namespace

# Custom values
helm install orchestra . -n orchestra --create-namespace \
  --set image.tag=0.1.0 \
  --set daemon.logLevel="debug" \
  --set persistence.size=20Gi
```

## Configuration

Key `values.yaml` options:

### Docker Connection Mode

```yaml
# Option 1: Bind-mount the host Docker socket (single-node / dev)
docker:
  mode: socket
  socketPath: /var/run/docker.sock

# Option 2: Remote Docker TCP (production multi-host)
docker:
  mode: tcp
  tcpHost: "tcp://192.168.1.50:2376"
  tlsVerify: true
  tlsSecretName: docker-tls-certs
```

### Persistence

```yaml
persistence:
  enabled: true        # Disable for ephemeral/CI use
  storageClass: ""     # Uses cluster default if empty
  size: 10Gi
```

### eBPF Host Mounts

```yaml
ebpf:
  enabled: true        # Mounts /sys/fs/bpf and /lib/modules
```

### Resources

```yaml
resources:
  limits:
    cpu: 2000m
    memory: 2Gi
  requests:
    cpu: 250m
    memory: 256Mi
```

## Verify Installation

```bash
# Wait for pod
kubectl -n orchestra rollout status deployment/orchestra

# Forward gRPC port
kubectl -n orchestra port-forward svc/orchestra 50051:50051 &

# Test connectivity
orchestra system ping
orchestra cluster list
```

## Upgrade

```bash
helm upgrade orchestra . -n orchestra --set image.tag=0.2.0
```

## Uninstall

```bash
helm uninstall orchestra -n orchestra
# PVC is retained by default; delete manually if desired:
kubectl delete pvc -n orchestra -l app.kubernetes.io/name=orchestra
```

## CI Values

For testing without persistence or eBPF:

```bash
helm lint . -f ci/test-values.yaml
helm template orchestra . -f ci/test-values.yaml | kubectl apply --dry-run=client -f -
```

## Helm Test

```bash
helm test orchestra -n orchestra
```

Runs a connectivity test pod that verifies the gRPC port is reachable.


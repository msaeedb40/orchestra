---
name: Bug report
about: Create a report to help us improve Orchestra
title: '[BUG] '
labels: ['kind/bug']
assignees: ''
---

### Describe the Bug
A clear and concise description of what the bug is.

### Steps to Reproduce
1. Start daemon: `orchestrad --listen-addr 127.0.0.1:50051`
2. Run command: `orchestra cluster create ...`
3. See error

### Expected Behavior
A clear and concise description of what you expected to happen.

### Environment Information
- Orchestra version (`orchestra --version`):
- Docker version (`docker version`):
- Host OS & Kernel (`uname -a`):
- Architecture (`x86_64` / `aarch64`):
- Kubernetes version targeted (`v1.34` - `v1.37`):
- CNI Plugin & Proxy Mode (`kindnet` / `flannel` / `cilium`, `orchestra` / `cni`):

### Diagnostic Logs
If applicable, run `orchestra system diagnostics` or provide relevant `orchestrad` logs:
```
<paste logs here>
```


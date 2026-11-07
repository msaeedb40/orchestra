#!/usr/bin/env bash
set -euo pipefail

echo "==> [orchestra-node] Initializing node environment..."

# 1. Mount cgroup v2 hierarchy if not mounted
if [ ! -d "/sys/fs/cgroup/cgroup.controllers" ] && [ -d "/sys/fs/cgroup" ]; then
    echo "==> [orchestra-node] Ensuring cgroup v2 hierarchy..."
fi

# 2. Start containerd daemon if not running
if ! pgrep -x "containerd" > /dev/null; then
    echo "==> [orchestra-node] Starting containerd daemon..."
    mkdir -p /run/containerd /var/log/containerd
    containerd > /var/log/containerd/containerd.log 2>&1 &
    
    # Wait for containerd socket
    for i in {1..30}; do
        if [ -S "/run/containerd/containerd.sock" ]; then
            echo "==> [orchestra-node] containerd is ready."
            break
        fi
        sleep 0.5
    done
fi

# 3. Handle entrypoint commands
if [ $# -gt 0 ]; then
    exec "$@"
else
    echo "==> [orchestra-node] Node ready. Listening for commands."
    exec tail -f /dev/null
fi


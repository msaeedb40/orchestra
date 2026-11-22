#!/usr/bin/env bash
set -euo pipefail

echo "==> [orchestra-node] Initializing node environment..."

# 0. Ensure root and sys mounts are rshared for CNI and volume propagation
mount --make-rshared / 2>/dev/null || true
mount --make-rshared /sys 2>/dev/null || true

# 1. Ensure cgroup v2 subtree control and isolate init processes
if [ -f "/sys/fs/cgroup/cgroup.controllers" ]; then
    echo "==> [orchestra-node] Setting up cgroup v2 subtree control..."
    mkdir -p /sys/fs/cgroup/init.scope
    echo $$ > /sys/fs/cgroup/init.scope/cgroup.procs 2>/dev/null || true
    for p in $(cat /sys/fs/cgroup/cgroup.procs 2>/dev/null || true); do
        echo "$p" > /sys/fs/cgroup/init.scope/cgroup.procs 2>/dev/null || true
    done
    for c in $(cat /sys/fs/cgroup/cgroup.controllers 2>/dev/null || true); do
        echo "+$c" > /sys/fs/cgroup/cgroup.subtree_control 2>/dev/null || true
    done
fi

# 2. Provide systemctl wrapper for kubeadm / kubelet lifecycle
if [ ! -f "/usr/local/bin/systemctl" ]; then
    cat << 'EOF' > /usr/local/bin/systemctl
#!/bin/bash
case "$1" in
    daemon-reload|enable)
        exit 0
        ;;
    restart|start)
        pkill -x kubelet || true
        sleep 0.5
        KUBELET_KUBEADM_ARGS=""
        if [ -f /var/lib/kubelet/kubeadm-flags.env ]; then
            source /var/lib/kubelet/kubeadm-flags.env
        fi
        KUBELET_KUBECONFIG_ARGS="--bootstrap-kubeconfig=/etc/kubernetes/bootstrap-kubelet.conf --kubeconfig=/etc/kubernetes/kubelet.conf"
        /usr/bin/kubelet ${KUBELET_KUBECONFIG_ARGS} --config=/var/lib/kubelet/config.yaml ${KUBELET_KUBEADM_ARGS} > /var/log/kubelet.log 2>&1 &
        exit 0
        ;;
    stop)
        pkill -x kubelet || true
        exit 0
        ;;
    is-active)
        if pgrep -x kubelet >/dev/null; then
            echo "active"
            exit 0
        else
            echo "inactive"
            exit 1
        fi
        ;;
    status)
        if pgrep -x kubelet >/dev/null; then
            exit 0
        else
            exit 3
        fi
        ;;
    *)
        exec /usr/bin/systemctl "$@"
        ;;
esac
EOF
    chmod +x /usr/local/bin/systemctl
fi

# 3. Ensure containerd uses cgroupfs driver when running without systemd
if [ -f "/etc/containerd/config.toml" ]; then
    sed -i 's/SystemdCgroup = true/SystemdCgroup = false/' /etc/containerd/config.toml 2>/dev/null || true
fi

# 4. Start containerd daemon if not running
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

# 5. Handle entrypoint commands
if [ $# -gt 0 ]; then
    exec "$@"
else
    echo "==> [orchestra-node] Node ready. Listening for commands."
    exec tail -f /dev/null
fi

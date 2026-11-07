#!/usr/bin/env bash
set -euo pipefail

# Check containerd socket
if [ ! -S "/run/containerd/containerd.sock" ]; then
    echo "containerd socket missing"
    exit 1
fi

# Check crictl connectivity
crictl info > /dev/null 2>&1 || {
    echo "crictl info failed"
    exit 1
}

exit 0


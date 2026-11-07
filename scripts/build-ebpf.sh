#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Building eBPF Kernel Programs ==="
cd "${ROOT_DIR}/ebpf"

cargo check --all-targets
cargo test --all-targets

echo "=== eBPF Kernel Programs Built Successfully ==="


#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Building Orchestra Workspace (Debug) ==="
cd "${ROOT_DIR}"
cargo build --workspace

echo "=== Building Orchestra CLI and Daemon (Release) ==="
cargo build --release -p orchestra-cli -p orchestra-daemon

echo "=== Build Complete ==="
echo "Binaries available at:"
echo "  - target/release/orchestra"
echo "  - target/release/orchestrad"


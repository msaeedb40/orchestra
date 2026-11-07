#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Running Workspace Tests (Unit & Integration) ==="
cd "${ROOT_DIR}"
cargo test --workspace

echo "=== Running eBPF Kernel Program Tests ==="
cargo test --manifest-path ebpf/Cargo.toml

echo "=== Running Benchmark Verification ==="
cargo bench -p orchestra-benchmarks -- --test

echo "=== All Tests Passed Successfully ==="


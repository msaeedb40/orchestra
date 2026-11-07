#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=== Checking Code Formatting (Rustfmt) ==="
cd "${ROOT_DIR}"
cargo fmt --all -- --check
cargo fmt --manifest-path ebpf/Cargo.toml --all -- --check

echo "=== Running Clippy Linter (Workspace) ==="
cargo clippy --workspace --all-targets -- -D warnings

echo "=== Running Clippy Linter (eBPF Kernel Workspace) ==="
cargo clippy --manifest-path ebpf/Cargo.toml --all-targets -- -D warnings

echo "=== Linting Clean: No Warnings or Diffs Found ==="


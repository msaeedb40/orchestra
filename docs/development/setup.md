# Developer Environment Setup

This document covers compiling, testing, and developing on Orchestra.

> **New:** A single [`setup.sh`](../../setup.sh) script automates the entire flow — see [Quick Start](#quick-start-setupsh) below.

## Development Requirements

### Toolchain Versions

| Tool | Required | Your System |
|------|----------|-------------|
| Rust (stable) | 1.99.1+ | managed via `rustup` |
| Rust (nightly) | latest | required for eBPF programs |
| Clang / LLVM | 16+ | for eBPF compilation |
| Protobuf Compiler | any | for Tonic gRPC code generation |
| libelf | any | eBPF object loading |
| libssl-dev | any | TLS/crypto support |

> **Note:** The `rust-toolchain.toml` pins the stable channel with the `rustfmt`, `clippy`, and `rust-src` components. `cargo` will apply this automatically — no manual `rustup override` needed.

### Install System Dependencies (Ubuntu / Debian)

```bash
sudo apt-get update
sudo apt-get install -y \
    protobuf-compiler \
    libelf-dev \
    clang \
    llvm \
    pkg-config \
    libssl-dev
```

> **Important:** `libssl-dev` provides the pkg-config metadata (`openssl.pc`) that the Rust `openssl-sys` crate requires at compile time. Having the `openssl` binary installed is **not** sufficient — the `-dev` package is mandatory.

### Install Rust Toolchains

```bash
# Install rustup (if not already present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# The stable toolchain + required components (pins from rust-toolchain.toml)
rustup toolchain install stable --component rustfmt clippy rust-src

# Nightly toolchain — required to compile the eBPF workspace (no_std kernel programs)
rustup toolchain install nightly
```

### Verify Your Setup

The fastest way to validate your environment is the preflight checker:

```bash
./scripts/preflight.sh
```

This runs 7 check categories (OS, Rust, Docker, packages, kernel, disk, networks) and outputs a colour-coded pass/warn/fail report. Or use the manual one-liner:

```bash
echo "=== Rust ===" && rustc --version && cargo --version && \
echo "=== Clang ===" && clang --version | head -1 && \
echo "=== Protoc ===" && protoc --version && \
echo "=== libelf ===" && pkg-config --libs libelf && \
echo "=== libssl-dev ===" && pkg-config --modversion openssl && \
echo "=== All dependencies OK ==="
```

---

## Repository Structure

The repository contains **two independent Cargo workspaces**:

| Workspace | Root | Purpose |
|-----------|------|---------|
| Main | `Cargo.toml` | All userspace crates, integration tests, and benchmarks |
| eBPF | `ebpf/Cargo.toml` | `#![no_std]` kernel programs and shared ABI structs |

### Crate Overview (Main Workspace)

```
crates/
├── orchestra-cli          # CLI binary (orchestra)
├── orchestra-daemon       # Daemon binary (orchestrad)
├── orchestra-core         # Core abstractions and traits
├── orchestra-api          # gRPC API layer (Tonic / prost)
├── orchestra-runtime      # Container runtime integration
├── orchestra-kubernetes   # Kubernetes control-plane integration
├── orchestra-network      # Network policy and CNI logic
├── orchestra-ebpf         # Userspace eBPF loader (loads kernel programs)
├── orchestra-storage      # Persistent state (SQLite via rusqlite)
├── orchestra-config       # Configuration parsing (TOML/YAML)
├── orchestra-observability # Tracing & metrics (tracing-subscriber)
├── orchestra-security     # Security policy enforcement
└── orchestra-common       # Shared types and utilities
tests/                     # Integration test crate
benches/                   # Criterion benchmarks
```

```
ebpf/
├── orchestra-ebpf-common   # Shared ABI structs (no_std, shared with userspace)
└── orchestra-ebpf-programs # Kernel eBPF programs (no_std)
```

---

## Quick Start: `setup.sh`

The root [`setup.sh`](../../setup.sh) replaces all manual steps for the common developer workflow:

```bash
# Full setup: preflight → build → install → systemd service
./setup.sh

# Or run specific phases only:
./setup.sh --check            # Preflight checks only
./setup.sh --deps             # Install system packages (needs sudo)
./setup.sh --build            # Build workspace + node images
./setup.sh --install          # Install binaries to ~/.local/bin
./setup.sh --daemon           # Configure + start systemd user service
./setup.sh --smoke-test       # Create and delete a test cluster
./setup.sh --no-ebpf          # Skip eBPF build (faster CI loops)
./setup.sh --node-version 1.37.1 # Build a specific node image version
```

After `./setup.sh`, the `orchestra` and `orchestrad` binaries are on your `PATH` and `orchestrad` is running as a systemd user service.

---

## Building

### Quick Build (Debug + Release binaries)

```bash
./scripts/build.sh
```

Binaries are written to:
- `target/release/orchestra` (CLI)
- `target/release/orchestrad` (daemon)

### Manual Build Commands

```bash
# Debug build — entire workspace
cargo build --workspace

# Release build — CLI and daemon only
cargo build --release -p orchestra-cli -p orchestra-daemon

# eBPF workspace (uses nightly automatically via cargo's toolchain override)
cd ebpf && cargo check --all-targets
```

### Build eBPF Programs

```bash
./scripts/build-ebpf.sh
```

This runs `cargo check` and `cargo test` inside the `ebpf/` workspace using the nightly toolchain.

### Build Node Docker Images

```bash
# Build all supported Kubernetes versions
./scripts/build-node-images.sh

# Build a specific version
./scripts/build-node-images.sh 1.37.1
```

Supported Kubernetes versions: `1.37.1`, `1.36.6`, `1.35.9`, `1.34.12`.

---

## Running Tests

### Via Script (Recommended)

```bash
./scripts/test.sh
```

Runs workspace unit + integration tests, eBPF workspace tests, and benchmark smoke-tests in one pass.

### Manual Test Commands

```bash
# All workspace unit and integration tests
cargo test --workspace

# Single crate
cargo test -p orchestra-security

# Integration tests only
cargo test -p orchestra-integration-tests   # lives in tests/

# eBPF workspace tests (nightly)
cargo test --manifest-path ebpf/Cargo.toml

# Benchmarks (smoke-test mode — no timing, just correctness)
cargo bench -p orchestra-benchmarks -- --test
```

### Useful Cargo Aliases (defined in `.cargo/config.toml`)

| Alias | Equivalent |
|-------|-----------|
| `cargo test-all` | `cargo test --workspace --all-targets` |
| `cargo clippy-all` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `cargo fmt-check` | `cargo fmt --all -- --check` |

---

## Linting & Formatting

```bash
./scripts/lint.sh
```

This enforces:
1. `cargo fmt` — consistent formatting across both workspaces.
2. `cargo clippy` — zero warnings (`-D warnings`) in both workspaces.

Clippy thresholds are configured in [`clippy.toml`](../../clippy.toml):
- Cognitive complexity limit: **30**
- Max function arguments: **8**
- MSRV: **1.99.1**

To auto-fix formatting before committing:

```bash
cargo fmt --all
cargo fmt --manifest-path ebpf/Cargo.toml --all
```

---

## IDE Setup

### VS Code / rust-analyzer

`rust-analyzer` reads `rust-toolchain.toml` automatically. For the eBPF workspace add this to `.vscode/settings.json`:

```json
{
  "rust-analyzer.linkedProjects": [
    "Cargo.toml",
    "ebpf/Cargo.toml"
  ]
}
```

### Environment Variables

| Variable | Purpose | Example |
|----------|---------|---------|
| `RUST_LOG` | Control log level for `orchestrad` | `RUST_LOG=debug` |
| `RUST_BACKTRACE` | Full backtraces on panic | `RUST_BACKTRACE=1` |

### Install binaries

```bash
# Install to ~/.local/bin (default)
./scripts/install.sh release

# Custom install directory
ORCHESTRA_INSTALL_DIR=/usr/local/bin ./scripts/install.sh release
```

The script creates `~/.local/share/orchestra/` for the daemon database and validates the binaries after copying.

---

## Running the Daemon

### As a systemd user service (recommended)

```bash
# Install and start (done automatically by setup.sh --daemon)
cp deployments/systemd/orchestrad-user.service ~/.config/systemd/user/orchestrad.service
systemctl --user daemon-reload
systemctl --user enable --now orchestrad
loginctl enable-linger "$USER"   # auto-start on boot without login

# Check status
systemctl --user status orchestrad
journalctl --user -u orchestrad -f
```

The user service automatically sets `DOCKER_HOST` to the Docker Desktop socket (`~/.docker/desktop/docker.sock`). If you use the standard socket instead, override it:

```bash
# In ~/.config/systemd/user/orchestrad.service.d/override.conf:
[Service]
Environment=DOCKER_HOST=unix:///var/run/docker.sock
```

### Manually (for debugging)

```bash
DOCKER_HOST="unix://$HOME/.docker/desktop/docker.sock" \
RUST_LOG="info,orchestra_daemon=debug" \
orchestrad --listen-addr 127.0.0.1:50051 \
           --db-path ~/.local/share/orchestra/orchestra.db
```

### Environment Variables

| Variable | Purpose | Example |
|----------|---------|---------||
| `RUST_LOG` | Log level for `orchestrad` | `info,orchestra_daemon=debug` |
| `DOCKER_HOST` | Docker socket or TCP endpoint | `unix:///home/user/.docker/desktop/docker.sock` |
| `RUST_BACKTRACE` | Full backtraces on panic | `RUST_BACKTRACE=1` |

---

## Node Image Entrypoint

Each `orchestra-node:{version}` Docker image (see `images/orchestra-node-v{VERSION}/`) runs an `entrypoint.sh` that prepares the container to function as a Kubernetes node. The entrypoint performs these steps in order:

### 1. Mount Propagation

```bash
mount --make-rshared /
mount --make-rshared /sys
```

Marks the root and `/sys` filesystems as recursively shared. This is required for:
- **kubelet**: Pod volume mounts must propagate between mount namespaces.
- **Cilium / eBPF CNIs**: The BPF filesystem at `/sys/fs/bpf` must be accessible to pods via shared mount propagation. Without this, Cilium Envoy pods fail with `path "/sys/fs/bpf" is mounted on "/sys" but it is not a shared or slave mount`.

### 2. cgroup v2 Subtree Delegation

On unified cgroup v2 hosts (Docker Desktop, modern Linux kernels), the entrypoint migrates all root-cgroup processes into `/sys/fs/cgroup/init.scope` and enables subtree controllers:

```bash
mkdir -p /sys/fs/cgroup/init.scope
echo $$ > /sys/fs/cgroup/init.scope/cgroup.procs
# Move all root processes...
for c in $(cat /sys/fs/cgroup/cgroup.controllers); do
    echo "+$c" > /sys/fs/cgroup/cgroup.subtree_control
done
```

This satisfies the cgroup v2 "no internal processes" rule — without it, kubelet fails with `cannot enter cgroupv2 "/sys/fs/cgroup/kubepods" with domain controllers -- it is in an invalid state`.

### 3. systemctl Shim

Since the node containers don't run a full init system, the entrypoint installs a `/usr/local/bin/systemctl` shim that translates kubelet lifecycle commands:

- **`start` / `restart`**: Launches `/usr/bin/kubelet` with the correct bootstrap arguments:
  - `--bootstrap-kubeconfig=/etc/kubernetes/bootstrap-kubelet.conf`
  - `--kubeconfig=/etc/kubernetes/kubelet.conf`
  - `--config=/var/lib/kubelet/config.yaml`
  - Plus `KUBELET_KUBEADM_ARGS` from `/var/lib/kubelet/kubeadm-flags.env`
- **`stop`**: Sends SIGTERM to the kubelet process.
- **`daemon-reload` / `enable`**: No-ops (no systemd to reload).
- **`is-active` / `status`**: Checks if the kubelet process is alive via `pgrep`.

### 4. containerd Daemon

The entrypoint starts `containerd` with CRI enabled, which kubeadm and kubelet use as the container runtime for Kubernetes pods.

### Available Node Images

| Image Tag | Kubernetes Version | Dockerfile |
|-----------|-------------------|------------|
| `orchestra-node:1.37.1` | v1.37.x | `images/orchestra-node-v1.37.1/Dockerfile` |
| `orchestra-node:1.36.5` | v1.36.x | `images/orchestra-node-v1.36.5/Dockerfile` |
| `orchestra-node:1.35.9` | v1.35.x | `images/orchestra-node-v1.35.9/Dockerfile` |
| `orchestra-node:1.34.12` | v1.34.x | `images/orchestra-node-v1.34.12/Dockerfile` |

Build a specific image:

```bash
docker build --load -t orchestra-node:1.37.1 \
  -f images/orchestra-node-v1.37.1/Dockerfile \
  images/orchestra-node-v1.37.1/
```

---

## Troubleshooting

### `Connection("Socket not found: /var/run/docker.sock")`

You are using Docker Desktop, which uses a non-standard socket path:

```bash
export DOCKER_HOST="unix://$HOME/.docker/desktop/docker.sock"
```

Add this to your shell profile or use the systemd user service which handles it automatically.

### `pkg-config: openssl not found`

`libssl-dev` is installed but its pkg-config file is missing:

```bash
# Confirm the package is installed
dpkg -l libssl-dev

# If not installed:
sudo apt-get install -y libssl-dev
```

### `error: linker 'cc' not found`

Install the C build toolchain:

```bash
sudo apt-get install -y build-essential
```

### eBPF build fails with `can't find crate for 'core'`

The nightly `rust-src` component is needed:

```bash
rustup component add rust-src --toolchain nightly
```

### `protoc: command not found`

```bash
sudo apt-get install -y protobuf-compiler
```

### `clang`/`llvm` version too old

Orchestra requires Clang/LLVM **16+**. Install the LLVM-provided packages:

```bash
# Add the LLVM apt repository for the latest stable release
wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh && sudo ./llvm.sh 18
sudo apt-get install -y clang-18 llvm-18
sudo update-alternatives --install /usr/bin/clang clang /usr/bin/clang-18 100
```

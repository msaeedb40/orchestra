# Developer Environment Setup

This document covers compiling, testing, and developing on Orchestra.

## Development Requirements

- **Rust**: 1.85.0+ (managed via `rustup`)
- **Clang / LLVM**: 16+ (for eBPF compilation)
- **Protobuf Compiler**: `protoc` (for Tonic gRPC compilation)
- **libelf & libssl**:
  ```bash
  sudo apt-get install -y protobuf-compiler libelf-dev clang llvm pkg-config libssl-dev
  ```

## Repository Structure

The repository contains two Cargo workspaces:
- Main Workspace (`Cargo.toml`): Contains all userspace crates, integration tests, and benchmarks.
- eBPF Workspace (`ebpf/Cargo.toml`): Contains `#![no_std]` kernel programs and shared ABI structs.

## Useful Development Scripts

- `./scripts/build.sh`: Builds debug workspace and release CLI/daemon binaries.
- `./scripts/test.sh`: Runs workspace tests, kernel eBPF tests, and benchmarks in test mode.
- `./scripts/lint.sh`: Verifies `cargo fmt` and strict `cargo clippy` without warnings.
- `./scripts/build-ebpf.sh`: Validates and compiles eBPF programs.
- `./scripts/build-node-images.sh`: Builds multi-version `orchestra-node` Docker images.

## Running Tests

```bash
# Run all workspace unit and integration tests
cargo test --workspace

# Run only security crate tests
cargo test -p orchestra-security

# Run integration tests
cargo test -p orchestra-integration-tests

# Run benchmarks
cargo bench -p orchestra-benchmarks -- --test
```


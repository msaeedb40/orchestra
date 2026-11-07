# Contributing to Orchestra

Thank you for your interest in contributing to Orchestra!

## Development Setup

1. **Prerequisites:**
   - Rust 1.85+  + (stable toolchain)
   - Docker Engine 24.0+
   - Linux kernel 5.15+ (required for eBPF components)
   - `bpftool`, `clang`, `llvm` (for eBPF compilation)

2. **Clone and Build:**
   ```bash
   git clone https://github.com/msaeedb40/orchestra.git
   cd orchestra
   cargo build --workspace
   ```

3. **Running Checks:**
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets --all-features -- -D warnings
   cargo test --workspace
   ```

## Commit & Pull Request Guidelines

- Branch naming: `feat/<feature>`, `fix/<bug>`, `refactor/<target>`.
- Write meaningful commit messages following Conventional Commits format (`feat:`, `fix:`, `docs:`, etc.).
- Ensure all CI checks and tests pass before requesting review.

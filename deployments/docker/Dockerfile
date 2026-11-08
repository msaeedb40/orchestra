# ──────────────────────────────────────────────────────────────────────
# Stage 1: Dependency cache layer
# Pre-fetch all Cargo dependencies before copying source so rebuilds
# only re-compile changed crates, not the entire dependency graph.
# ──────────────────────────────────────────────────────────────────────
FROM rust:1.85-bookworm AS deps

WORKDIR /build

RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libelf-dev \
    clang \
    llvm \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy only manifests first — Docker caches this layer until Cargo.toml changes
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/orchestra-cli/Cargo.toml        crates/orchestra-cli/Cargo.toml
COPY crates/orchestra-daemon/Cargo.toml     crates/orchestra-daemon/Cargo.toml
COPY crates/orchestra-core/Cargo.toml       crates/orchestra-core/Cargo.toml
COPY crates/orchestra-api/Cargo.toml        crates/orchestra-api/Cargo.toml
COPY crates/orchestra-runtime/Cargo.toml    crates/orchestra-runtime/Cargo.toml
COPY crates/orchestra-kubernetes/Cargo.toml crates/orchestra-kubernetes/Cargo.toml
COPY crates/orchestra-network/Cargo.toml    crates/orchestra-network/Cargo.toml
COPY crates/orchestra-ebpf/Cargo.toml       crates/orchestra-ebpf/Cargo.toml
COPY crates/orchestra-storage/Cargo.toml    crates/orchestra-storage/Cargo.toml
COPY crates/orchestra-config/Cargo.toml     crates/orchestra-config/Cargo.toml
COPY crates/orchestra-observability/Cargo.toml crates/orchestra-observability/Cargo.toml
COPY crates/orchestra-security/Cargo.toml   crates/orchestra-security/Cargo.toml
COPY crates/orchestra-common/Cargo.toml     crates/orchestra-common/Cargo.toml
COPY tests/Cargo.toml                       tests/Cargo.toml
COPY benches/Cargo.toml                     benches/Cargo.toml
COPY proto/                                 proto/

# Create dummy lib/main stubs so `cargo fetch` can resolve the full graph
RUN find crates -name Cargo.toml -exec sh -c \
    'dir=$(dirname "$1"); \
     mkdir -p "$dir/src"; \
     echo "fn main() {}" > "$dir/src/main.rs" 2>/dev/null; \
     echo "" > "$dir/src/lib.rs" 2>/dev/null' _ {} \; && \
    mkdir -p tests/tests benches/benches && \
    echo "" > tests/src/lib.rs && \
    echo "" > benches/src/lib.rs

RUN cargo fetch

# ──────────────────────────────────────────────────────────────────────
# Stage 2: Build release binaries
# ──────────────────────────────────────────────────────────────────────
FROM deps AS builder

# Copy full source (invalidates cache only when source changes)
COPY . .

RUN cargo build --release -p orchestra-cli -p orchestra-daemon && \
    strip target/release/orchestra target/release/orchestrad

# ──────────────────────────────────────────────────────────────────────
# Stage 3: Minimal runtime image
# ──────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="Orchestra Daemon" \
      org.opencontainers.image.description="Orchestra Kubernetes cluster orchestration daemon" \
      org.opencontainers.image.source="https://github.com/msaeedb40/orchestra" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update && apt-get install -y \
    ca-certificates \
    iproute2 \
    iptables \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -r orchestra && useradd -r -g orchestra -s /sbin/nologin orchestra

COPY --from=builder /build/target/release/orchestrad /usr/local/bin/orchestrad
COPY --from=builder /build/target/release/orchestra   /usr/local/bin/orchestra

RUN mkdir -p /var/lib/orchestra && chown orchestra:orchestra /var/lib/orchestra

VOLUME ["/var/lib/orchestra"]

EXPOSE 50051 9090

# eBPF + Docker socket access requires root at runtime;
# use USER orchestra only if running without privileged mode.
# USER orchestra

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD orchestra system ping || exit 1

ENTRYPOINT ["/usr/local/bin/orchestrad"]
CMD ["--listen-addr", "0.0.0.0:50051", "--db-path", "/var/lib/orchestra/orchestra.db"]

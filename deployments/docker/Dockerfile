FROM rust:1.85-bookworm AS builder

WORKDIR /build

RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libelf-dev \
    clang \
    llvm \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY . .

RUN cargo build --release -p orchestra-daemon -p orchestra-cli

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    iproute2 \
    iptables \
    docker.io \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/orchestrad /usr/local/bin/orchestrad
COPY --from=builder /build/target/release/orchestra /usr/local/bin/orchestra

VOLUME ["/var/lib/orchestra"]

EXPOSE 50051 9090

ENTRYPOINT ["/usr/local/bin/orchestrad"]
CMD ["--listen-addr", "0.0.0.0:50051", "--db-path", "/var/lib/orchestra/orchestra.db"]


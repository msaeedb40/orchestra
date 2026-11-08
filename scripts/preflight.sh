#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────
# Orchestra — Preflight Checker
# Validates the host environment before building or running Orchestra.
# ──────────────────────────────────────────────────────────────────────
set -euo pipefail

# ── Colours ──────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

PASS=0
WARN=0
FAIL=0

pass()  { PASS=$((PASS + 1)); echo -e "  ${GREEN}✓${NC} $1"; }
warn()  { WARN=$((WARN + 1)); echo -e "  ${YELLOW}⚠${NC} $1"; }
fail()  { FAIL=$((FAIL + 1)); echo -e "  ${RED}✗${NC} $1"; }
header(){ echo -e "\n${BOLD}${CYAN}[$1]${NC}"; }

# ── 1. Operating System ──────────────────────────────────────────────
header "Operating System"

if [[ "$(uname -s)" != "Linux" ]]; then
    fail "Orchestra requires Linux (found: $(uname -s))"
else
    pass "Linux detected"
fi

KERNEL_VERSION=$(uname -r | cut -d. -f1-2)
KERNEL_MAJOR=$(echo "$KERNEL_VERSION" | cut -d. -f1)
KERNEL_MINOR=$(echo "$KERNEL_VERSION" | cut -d. -f2)
if (( KERNEL_MAJOR > 5 || (KERNEL_MAJOR == 5 && KERNEL_MINOR >= 15) )); then
    pass "Kernel $(uname -r) (≥ 5.15 required for eBPF)"
else
    warn "Kernel $(uname -r) — eBPF features may be limited (recommend ≥ 5.15)"
fi

# ── 2. Rust Toolchain ────────────────────────────────────────────────
header "Rust Toolchain"

if command -v rustc &>/dev/null; then
    RUST_VER=$(rustc --version | awk '{print $2}')
    RUST_MAJOR=$(echo "$RUST_VER" | cut -d. -f1)
    RUST_MINOR=$(echo "$RUST_VER" | cut -d. -f2)
    if (( RUST_MAJOR > 1 || (RUST_MAJOR == 1 && RUST_MINOR >= 85) )); then
        pass "Rust $RUST_VER (≥ 1.85 required)"
    else
        fail "Rust $RUST_VER — upgrade required (≥ 1.85)"
    fi
else
    fail "Rust not found — install via: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi

if command -v rustup &>/dev/null; then
    if rustup toolchain list | grep -q nightly; then
        NIGHTLY_VER=$(rustup run nightly rustc --version 2>/dev/null | awk '{print $2}' || echo "unknown")
        pass "Rust nightly installed ($NIGHTLY_VER) — required for eBPF"
    else
        warn "Rust nightly not installed — needed for eBPF builds: rustup toolchain install nightly"
    fi
else
    warn "rustup not found — cannot verify nightly toolchain"
fi

if command -v cargo &>/dev/null; then
    pass "Cargo $(cargo --version | awk '{print $2}')"
else
    fail "Cargo not found"
fi

# ── 3. Docker ────────────────────────────────────────────────────────
header "Docker"

DOCKER_SOCKET=""
if [ -S "/var/run/docker.sock" ]; then
    DOCKER_SOCKET="/var/run/docker.sock"
    pass "Docker socket: /var/run/docker.sock (standard)"
elif [ -S "$HOME/.docker/desktop/docker.sock" ]; then
    DOCKER_SOCKET="$HOME/.docker/desktop/docker.sock"
    pass "Docker socket: ~/.docker/desktop/docker.sock (Docker Desktop)"
else
    fail "No Docker socket found at /var/run/docker.sock or ~/.docker/desktop/docker.sock"
fi

if [ -n "$DOCKER_SOCKET" ]; then
    export DOCKER_HOST="unix://$DOCKER_SOCKET"
fi

if command -v docker &>/dev/null; then
    if docker info &>/dev/null; then
        DOCKER_VER=$(docker version --format '{{.Server.Version}}' 2>/dev/null || echo "unknown")
        pass "Docker Engine $DOCKER_VER — accessible"
    else
        fail "Docker installed but not accessible — check permissions or socket path"
    fi
else
    fail "Docker not found — install Docker Engine or Docker Desktop"
fi

# ── 4. System Packages ───────────────────────────────────────────────
header "System Packages"

check_pkg() {
    local cmd="$1" pkg="$2" desc="$3"
    if command -v "$cmd" &>/dev/null; then
        pass "$desc"
    else
        fail "$desc — install: sudo apt-get install -y $pkg"
    fi
}

check_pkg protoc      protobuf-compiler "protobuf-compiler (protoc)"
check_pkg clang       clang             "clang"
check_pkg llvm-config llvm              "llvm"

if pkg-config --exists libelf 2>/dev/null; then
    pass "libelf-dev"
else
    fail "libelf-dev — install: sudo apt-get install -y libelf-dev"
fi

if pkg-config --exists openssl 2>/dev/null; then
    pass "libssl-dev"
else
    warn "libssl-dev not installed — needed for crates using openssl-sys"
fi

check_pkg pkg-config  pkg-config  "pkg-config"
check_pkg ip          iproute2    "iproute2"

if dpkg -l build-essential &>/dev/null 2>&1; then
    pass "build-essential"
else
    warn "build-essential — recommended: sudo apt-get install -y build-essential"
fi

# ── 5. Kernel Features ───────────────────────────────────────────────
header "Kernel Features"

if [ -d /sys/fs/cgroup/unified ] || [ -f /sys/fs/cgroup/cgroup.controllers ]; then
    pass "cgroups v2 enabled"
else
    warn "cgroups v2 not detected — Orchestra node containers require cgroups v2"
fi

if mount | grep -q "type bpf"; then
    pass "bpffs mounted"
else
    warn "bpffs not mounted — eBPF programs may require: sudo mount -t bpf bpf /sys/fs/bpf"
fi

if [ -f /proc/config.gz ]; then
    if zcat /proc/config.gz 2>/dev/null | grep -q "CONFIG_BPF=y"; then
        pass "CONFIG_BPF=y in kernel"
    else
        warn "Cannot confirm CONFIG_BPF=y"
    fi
elif [ -f "/boot/config-$(uname -r)" ]; then
    if grep -q "CONFIG_BPF=y" "/boot/config-$(uname -r)"; then
        pass "CONFIG_BPF=y in kernel"
    else
        warn "Cannot confirm CONFIG_BPF=y"
    fi
else
    warn "Cannot read kernel config — skipping BPF check"
fi

# ── 6. Disk Space ────────────────────────────────────────────────────
header "Disk Space"

AVAILABLE_KB=$(df --output=avail "$(pwd)" | tail -1 | tr -d ' ')
AVAILABLE_GB=$((AVAILABLE_KB / 1048576))
if (( AVAILABLE_GB >= 5 )); then
    pass "${AVAILABLE_GB} GB available (≥ 5 GB required)"
else
    fail "${AVAILABLE_GB} GB available — at least 5 GB recommended"
fi

# ── 7. Docker Subnet Availability ────────────────────────────────────
header "Docker Network"

if [ -n "$DOCKER_SOCKET" ] && command -v docker &>/dev/null && docker info &>/dev/null; then
    USED_SUBNETS=$(docker network ls -q | xargs -I{} docker network inspect {} 2>/dev/null \
        | grep -oP '"Subnet":\s*"\K172\.\d+' | sort -t. -k2 -n -u || true)

    FOUND_FREE=""
    for octet in $(seq 16 31); do
        if ! echo "$USED_SUBNETS" | grep -qx "172\.$octet"; then
            FOUND_FREE="172.${octet}.0.0/16"
            break
        fi
    done

    if [ -n "$FOUND_FREE" ]; then
        pass "Available subnet: $FOUND_FREE"
    else
        warn "No free /16 subnet in 172.16–172.31 range"
    fi
else
    warn "Cannot check Docker subnets — Docker not accessible"
fi

# ── Summary ───────────────────────────────────────────────────────────
echo ""
echo -e "${BOLD}━━━ Preflight Summary ━━━${NC}"
echo -e "  ${GREEN}Passed:${NC}  $PASS"
echo -e "  ${YELLOW}Warnings:${NC} $WARN"
echo -e "  ${RED}Failed:${NC}  $FAIL"
echo ""

if (( FAIL > 0 )); then
    echo -e "${RED}${BOLD}Preflight failed.${NC} Fix the errors above before proceeding."
    exit 1
elif (( WARN > 0 )); then
    echo -e "${YELLOW}${BOLD}Preflight passed with warnings.${NC} Review items above."
    exit 0
else
    echo -e "${GREEN}${BOLD}All checks passed!${NC}"
    exit 0
fi


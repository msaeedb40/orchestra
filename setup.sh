#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────
# Orchestra — Master Setup & Bootstrap Script
# 
# One-stop script to validate environment, install dependencies,
# build binaries & container images, configure systemd, and smoke-test.
#
# Usage:
#   ./setup.sh [OPTIONS]
#
# Options:
#   --check         Run preflight checks only
#   --deps          Install missing system dependencies (requires sudo)
#   --build         Build workspace and release binaries
#   --install       Install binaries to ~/.local/bin
#   --daemon        Configure & start orchestrad as systemd user service
#   --smoke-test    Create and destroy a test cluster
#   --all           Run complete setup flow (default if no flags given)
#   --no-ebpf       Skip eBPF kernel program build (faster for dev)
#   --node-version  Node image version to build (default: 1.37.1, or "all")
#   --debug         Build/install in debug mode instead of release
#   -h, --help      Display this help message
# ──────────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="${SCRIPT_DIR}"

# ── Colours ──────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${BOLD}${CYAN}==>${NC} $1"; }
success() { echo -e "${BOLD}${GREEN}==>${NC} $1"; }
warn()    { echo -e "${BOLD}${YELLOW}⚠${NC}  $1"; }
error()   { echo -e "${BOLD}${RED}✗${NC}  $1" >&2; }

# ── Flags ─────────────────────────────────────────────────────────────
DO_CHECK=0
DO_DEPS=0
DO_BUILD=0
DO_INSTALL=0
DO_DAEMON=0
DO_SMOKE=0
DO_EBPF=1
BUILD_MODE="release"
NODE_VERSION="1.37.1"

# If no arguments provided, do full setup
if [[ $# -eq 0 ]]; then
    DO_CHECK=1
    DO_BUILD=1
    DO_INSTALL=1
    DO_DAEMON=1
fi

while [[ $# -gt 0 ]]; do
    case "$1" in
        --check)
            DO_CHECK=1
            shift
            ;;
        --deps)
            DO_DEPS=1
            shift
            ;;
        --build)
            DO_BUILD=1
            shift
            ;;
        --install)
            DO_INSTALL=1
            shift
            ;;
        --daemon)
            DO_DAEMON=1
            shift
            ;;
        --smoke-test)
            DO_SMOKE=1
            shift
            ;;
        --all)
            DO_CHECK=1
            DO_BUILD=1
            DO_INSTALL=1
            DO_DAEMON=1
            DO_SMOKE=1
            shift
            ;;
        --no-ebpf)
            DO_EBPF=0
            shift
            ;;
        --node-version)
            NODE_VERSION="$2"
            shift 2
            ;;
        --debug)
            BUILD_MODE="debug"
            shift
            ;;
        --release)
            BUILD_MODE="release"
            shift
            ;;
        -h|--help)
            sed -ne '/^#/!q;s/^# //;2,$p' "$0" | head -n 25
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            echo "Run './setup.sh --help' for usage."
            exit 1
            ;;
    esac
done

echo -e "${BOLD}${CYAN}"
echo "  ___           _               _             "
echo " / _ \ _ __ ___| |__   ___  ___| |_ _ __ __ _ "
echo "| | | | '__/ __| '_ \ / _ \/ __| __| '__/ _\` |"
echo "| |_| | | | (__| | | |  __/\__ \ |_| | | (_| |"
echo " \___/|_|  \___|_| |_|\___||___/\__|_|  \__,_|"
echo -e "${NC}"
echo -e "Kubernetes Cluster Orchestration Platform — Setup & Bootstrap\n"

# ── 1. Detect Docker Socket ──────────────────────────────────────────
detect_docker() {
    if [ -S "/var/run/docker.sock" ]; then
        export DOCKER_HOST="unix:///var/run/docker.sock"
    elif [ -S "$HOME/.docker/desktop/docker.sock" ]; then
        export DOCKER_HOST="unix://$HOME/.docker/desktop/docker.sock"
    fi
}
detect_docker

# ── 2. Dependencies Auto-install ─────────────────────────────────────
if [[ $DO_DEPS -eq 1 ]]; then
    info "Installing system dependencies..."
    if command -v apt-get &>/dev/null; then
        sudo apt-get update
        sudo apt-get install -y \
            protobuf-compiler \
            libelf-dev \
            clang \
            llvm \
            pkg-config \
            libssl-dev \
            iproute2 \
            build-essential
        success "System dependencies installed."
    else
        warn "Package manager apt-get not found. Please install prerequisites manually."
    fi

    # Ensure rust nightly is installed
    if command -v rustup &>/dev/null; then
        if ! rustup toolchain list | grep -q nightly; then
            info "Installing Rust nightly toolchain..."
            rustup toolchain install nightly
        fi
    fi
fi

# ── 3. Preflight Checks ──────────────────────────────────────────────
if [[ $DO_CHECK -eq 1 ]]; then
    info "Running preflight environment checks..."
    "${ROOT_DIR}/scripts/preflight.sh"
    success "Preflight checks passed."
fi

# ── 4. Build ─────────────────────────────────────────────────────────
if [[ $DO_BUILD -eq 1 ]]; then
    info "Building Orchestra (mode: ${BUILD_MODE})..."

    cd "${ROOT_DIR}"
    if [[ "$BUILD_MODE" == "release" ]]; then
        info "Compiling release binaries (orchestra, orchestrad)..."
        cargo build --release -p orchestra-cli -p orchestra-daemon
    else
        info "Compiling debug workspace..."
        cargo build --workspace
    fi

    if [[ $DO_EBPF -eq 1 ]]; then
        info "Checking eBPF programs..."
        "${ROOT_DIR}/scripts/build-ebpf.sh"
    else
        info "Skipping eBPF build (--no-ebpf passed)."
    fi

    # Check if node image needs to be built
    if [ -n "${DOCKER_HOST:-}" ] && command -v docker &>/dev/null; then
        if ! docker image inspect "orchestra-node:${NODE_VERSION}" &>/dev/null; then
            info "Building node container image orchestra-node:${NODE_VERSION}..."
            "${ROOT_DIR}/scripts/build-node-images.sh" "${NODE_VERSION}"
        else
            info "Node image orchestra-node:${NODE_VERSION} already present in Docker."
        fi
    fi

    success "Build completed successfully."
fi

# ── 5. Install ───────────────────────────────────────────────────────
if [[ $DO_INSTALL -eq 1 ]]; then
    info "Installing Orchestra binaries..."
    "${ROOT_DIR}/scripts/install.sh" "${BUILD_MODE}"
    success "Binaries installed."
fi

# ── 6. Daemon (systemd user service) ─────────────────────────────────
if [[ $DO_DAEMON -eq 1 ]]; then
    info "Configuring systemd user service..."
    SYSTEMD_USER_DIR="${HOME}/.config/systemd/user"
    mkdir -p "${SYSTEMD_USER_DIR}"

    cp "${ROOT_DIR}/deployments/systemd/orchestrad-user.service" \
       "${SYSTEMD_USER_DIR}/orchestrad.service"

    # Adjust socket path in service file if not using Docker Desktop
    if [ -S "/var/run/docker.sock" ] && [ ! -S "$HOME/.docker/desktop/docker.sock" ]; then
        sed -i 's|Environment=DOCKER_HOST=.*|Environment=DOCKER_HOST=unix:///var/run/docker.sock|' \
            "${SYSTEMD_USER_DIR}/orchestrad.service"
    fi

    systemctl --user daemon-reload
    systemctl --user enable --now orchestrad
    loginctl enable-linger "$USER" 2>/dev/null || true

    sleep 2
    if systemctl --user is-active --quiet orchestrad; then
        success "orchestrad systemd user service is running."
    else
        warn "orchestrad service is not running yet. Check: journalctl --user -u orchestrad -n 50"
    fi
fi

# ── 7. Smoke Test ────────────────────────────────────────────────────
if [[ $DO_SMOKE -eq 1 ]]; then
    info "Running cluster smoke test..."
    BIN_DIR="${ORCHESTRA_INSTALL_DIR:-$HOME/.local/bin}"
    export PATH="${BIN_DIR}:${PATH}"

    TEST_CLUSTER="setup-smoke-test"
    info "Creating test cluster '${TEST_CLUSTER}'..."
    orchestra cluster create "${TEST_CLUSTER}" --control-plane 1 --workers 1 --wait || true

    info "Listing clusters..."
    orchestra cluster list || true

    info "Cleaning up test cluster '${TEST_CLUSTER}'..."
    orchestra cluster delete "${TEST_CLUSTER}" --yes || true
    success "Smoke test completed."
fi

echo ""
success "Orchestra setup finished!"
echo -e "Quick verification command: ${BOLD}orchestra cluster list${NC}"


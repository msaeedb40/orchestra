#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────
# Orchestra — Binary Installer
# Installs built orchestra and orchestrad binaries.
# ──────────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

INSTALL_DIR="${ORCHESTRA_INSTALL_DIR:-$HOME/.local/bin}"
BUILD_MODE="${1:-release}"

if [ "$BUILD_MODE" = "release" ]; then
    BIN_DIR="${ROOT_DIR}/target/release"
elif [ "$BUILD_MODE" = "debug" ]; then
    BIN_DIR="${ROOT_DIR}/target/debug"
else
    echo "Usage: install.sh [release|debug]"
    exit 1
fi

echo "=== Installing Orchestra Binaries ==="
echo "  Source:  ${BIN_DIR}"
echo "  Target:  ${INSTALL_DIR}"

# Verify binaries exist
if [ ! -f "${BIN_DIR}/orchestra" ]; then
    echo "Error: ${BIN_DIR}/orchestra not found. Run 'scripts/build.sh' first."
    exit 1
fi

if [ ! -f "${BIN_DIR}/orchestrad" ]; then
    echo "Error: ${BIN_DIR}/orchestrad not found. Run 'scripts/build.sh' first."
    exit 1
fi

# Create install directory
mkdir -p "${INSTALL_DIR}"

# Install binaries
install -m 755 "${BIN_DIR}/orchestra"   "${INSTALL_DIR}/orchestra"
install -m 755 "${BIN_DIR}/orchestrad"  "${INSTALL_DIR}/orchestrad"

echo "  Installed: orchestra   ($(du -h "${INSTALL_DIR}/orchestra" | cut -f1))"
echo "  Installed: orchestrad  ($(du -h "${INSTALL_DIR}/orchestrad" | cut -f1))"

# Create data directory
DATA_DIR="${HOME}/.local/share/orchestra"
mkdir -p "${DATA_DIR}"
echo "  Data dir:  ${DATA_DIR}"

# Check PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "${INSTALL_DIR}"; then
    echo ""
    echo "⚠  ${INSTALL_DIR} is not in your PATH."
    echo "   Add it with:"
    echo "     export PATH=\"${INSTALL_DIR}:\$PATH\""
    echo ""
    echo "   To make it permanent, add to your shell profile (~/.bashrc or ~/.zshrc):"
    echo "     echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc"
fi

# Verify installation
echo ""
if "${INSTALL_DIR}/orchestra" --version &>/dev/null; then
    echo "  ✓ orchestra $("${INSTALL_DIR}/orchestra" --version 2>/dev/null | awk '{print $2}' || echo "")"
else
    echo "  ✓ orchestra installed"
fi

echo ""
echo "=== Installation Complete ==="


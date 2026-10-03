#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${ROOT_DIR}"

VERSION="${VERSION:-$(git describe --tags --always --dirty 2>/dev/null || echo 'dev')}"
GIT_COMMIT="${GIT_COMMIT:-$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')}"
BUILD_DATE="${BUILD_DATE:-$(date -u '+%Y-%m-%dT%H:%M:%SZ')}"

LDFLAGS="-s -w \
    -X 'github.com/orchestra/orchestra/internal/cli.version=${VERSION}' \
    -X 'github.com/orchestra/orchestra/internal/cli.gitCommit=${GIT_COMMIT}' \
    -X 'github.com/orchestra/orchestra/internal/cli.buildDate=${BUILD_DATE}'"

TARGETS="${1:-linux/amd64 linux/arm64 darwin/amd64 darwin/arm64}"

echo "==> Building Orchestra ${VERSION} for all platforms"

mkdir -p bin

for target in ${TARGETS}; do
    os="${target%/*}"
    arch="${target#*/}"
    output="bin/orchestra-${os}-${arch}"
    echo "  Building ${output}..."
    CGO_ENABLED=0 GOOS="${os}" GOARCH="${arch}" go build \
        -ldflags "${LDFLAGS}" \
        -o "${output}" ./cmd/orchestra
done

# Copy the linux/amd64 binary as the canonical local binary.
if [[ -f "bin/orchestra-linux-amd64" ]]; then
    cp "bin/orchestra-linux-amd64" "bin/orchestra"
fi

echo ""
echo "==> Build complete. Artifacts:"
ls -lh bin/orchestra-*

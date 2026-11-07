#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
IMAGE_DIR="${ROOT_DIR}/images/orchestra-node"

# Release Matrix from orchestra.md:
# orchestra-node:1.37.1  -> v1.37.1 (or latest 1.37)
# orchestra-node:1.36.6  -> v1.36.6
# orchestra-node:1.35.9  -> v1.35.9
# orchestra-node:1.34.12 -> v1.34.12

declare -A K8S_VERSIONS=(
    ["1.37.1"]="v1.37.1"
    ["1.36.6"]="v1.36.6"
    ["1.35.9"]="v1.35.9"
    ["1.34.12"]="v1.34.12"
)

TARGET_VERSION="${1:-all}"

build_image() {
    local tag="$1"
    local k8s_ver="$2"
    echo "==> Building orchestra-node:${tag} for Kubernetes ${k8s_ver}..."
    docker build \
        --build-arg KUBERNETES_VERSION="${k8s_ver}" \
        -t "orchestra-node:${tag}" \
        -f "${IMAGE_DIR}/Dockerfile" \
        "${IMAGE_DIR}"
    echo "==> Successfully built orchestra-node:${tag}"
}

if [ "${TARGET_VERSION}" = "all" ]; then
    for tag in "${!K8S_VERSIONS[@]}"; do
        build_image "${tag}" "${K8S_VERSIONS[$tag]}"
    done
else
    if [[ -n "${K8S_VERSIONS[$TARGET_VERSION]:-}" ]]; then
        build_image "${TARGET_VERSION}" "${K8S_VERSIONS[$TARGET_VERSION]}"
    else
        echo "Error: Unknown version tag '${TARGET_VERSION}'"
        echo "Supported version tags: ${!K8S_VERSIONS[*]}"
        exit 1
    fi
fi


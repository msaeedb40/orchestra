#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${ROOT_DIR}"

if [[ $# -lt 1 ]]; then
    echo "Usage: $(basename "$0") <version>  (e.g. v0.2.0)" >&2
    exit 1
fi

VERSION="$1"

if [[ ! "${VERSION}" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: version must match ^v[0-9]+\.[0-9]+\.[0-9]+$ (got: ${VERSION})" >&2
    exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
    echo "Error: working tree is dirty. Commit or stash changes before tagging." >&2
    exit 1
fi

git tag -a "${VERSION}" -m "Release ${VERSION}"
echo "Tagged ${VERSION}"
echo ""
echo "To push the tag: git push origin ${VERSION}"
echo "Changelog: https://github.com/orchestra/orchestra/releases/tag/${VERSION}"

#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

cd "${ROOT_DIR}"

COVER=0
for arg in "$@"; do
    if [[ "${arg}" == "--cover" ]]; then
        COVER=1
    fi
done

echo "==> Running go vet"
go vet ./...

echo "==> Running tests"
if [[ "${COVER}" -eq 1 ]]; then
    go test -v -race -count=1 -timeout 120s -coverprofile=coverage.out ./...
    echo "==> Coverage summary"
    go tool cover -func=coverage.out
else
    go test -v -race -count=1 -timeout 120s ./...
fi

if command -v golangci-lint &>/dev/null; then
    echo "==> Running golangci-lint"
    golangci-lint run ./...
else
    echo "==> WARNING: golangci-lint not found, skipping lint"
fi

echo "==> All checks passed"

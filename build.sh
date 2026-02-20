#!/usr/bin/env bash
# Build assoc.exe via Docker and copy it to target/x86_64-pc-windows-gnu/release/.
# Usage: ./build.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT_DIR="$SCRIPT_DIR/target/x86_64-pc-windows-gnu/release"

mkdir -p "$OUT_DIR"

echo "==> Building assoc-build image (builder stage)..."
docker build -t assoc-build --target builder "$SCRIPT_DIR"

echo "==> Exporting assoc.exe to $OUT_DIR ..."
DOCKER_BUILDKIT=1 docker build --target export \
    --output "type=local,dest=$OUT_DIR" \
    "$SCRIPT_DIR"

echo "==> Done: $OUT_DIR/assoc.exe"

#!/usr/bin/env bash
# Build The Associate in Docker and copy assoc.exe to the project root.
# Usage: ./build.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE="the-associate-builder"
TARGET="x86_64-pc-windows-gnu"
OUT="$SCRIPT_DIR/assoc.exe"

echo "Building Docker image..."
docker build -t "$IMAGE" "$SCRIPT_DIR"

echo "Extracting assoc.exe..."
docker run --rm "$IMAGE" cat "target/$TARGET/release/assoc.exe" > "$OUT"

echo "Done: $OUT"

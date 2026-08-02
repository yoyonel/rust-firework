#!/usr/bin/env bash
set -euo pipefail

TARGET_TASK="${1:-}"
BOX_NAME="${2:-}"
IMAGE_NAME="${3:-}"

# Isolate container build target directory to prevent host/container CMake & Cargo cache collisions
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/container}"

if command -v distrobox >/dev/null 2>&1 && distrobox list | grep -q "$BOX_NAME"; then
  exec distrobox enter "$BOX_NAME" -- env CARGO_TARGET_DIR="$CARGO_TARGET_DIR" task "$TARGET_TASK"
elif command -v docker >/dev/null 2>&1; then
  exec docker run --rm -i \
    --user "$(id -u):$(id -g)" \
    -e HOME=/tmp \
    -e CARGO_TARGET_DIR="$CARGO_TARGET_DIR" \
    -v "$PWD:/work" -w /work \
    -e DISPLAY="${DISPLAY:-}" -v /tmp/.X11-unix:/tmp/.X11-unix \
    "$IMAGE_NAME" task "$TARGET_TASK"
else
  echo "Error: No container runtime found."
  exit 1
fi

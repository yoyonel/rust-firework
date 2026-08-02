#!/usr/bin/env bash
set -euo pipefail

TARGET_TASK="${1:-}"
BOX_NAME="${2:-}"
IMAGE_NAME="${3:-}"

if command -v distrobox >/dev/null 2>&1 && distrobox list | grep -q "$BOX_NAME"; then
  exec distrobox enter "$BOX_NAME" -- task "$TARGET_TASK"
elif command -v docker >/dev/null 2>&1; then
  exec docker run --rm -i \
    -v "$PWD:/work" -w /work \
    -e DISPLAY="${DISPLAY:-}" -v /tmp/.X11-unix:/tmp/.X11-unix \
    "$IMAGE_NAME" task "$TARGET_TASK"
else
  echo "Error: No container runtime found."
  exit 1
fi

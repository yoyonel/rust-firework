#!/usr/bin/env bash
set -euo pipefail

echo "==> Running Unit Test Suite..."

if [ -f "Cargo.toml" ]; then
    echo "[Rust] Executing cargo test..."
    xvfb-run -a cargo test -- --test-threads=1
fi

echo "==> All Unit Tests Completed Successfully!"

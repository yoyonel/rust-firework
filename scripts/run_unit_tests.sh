#!/usr/bin/env bash
set -euo pipefail

echo "==> Running Universal Unit Test Suite..."

# C11 Unit Tests
if [ -d "src/c11" ] || [ -f "tests/unit/test_c11.c" ]; then
    echo "[C11] Compiling and running unit tests..."
    clang -Isrc/c11 src/c11/engine.c tests/unit/test_c11.c -o /tmp/test_c11
    /tmp/test_c11
fi

# Vulkan C++ Unit Tests
if [ -d "src/vulkan" ] || [ -f "tests/unit/test_vulkan.cpp" ]; then
    echo "[Vulkan C++] Compiling and running unit tests..."
    clang++ -Isrc/vulkan src/vulkan/engine.cpp tests/unit/test_vulkan.cpp -o /tmp/test_vulkan
    /tmp/test_vulkan
fi

# Rust Unit Tests
if [ -f "Cargo.toml" ] || [ -f "src/rust/Cargo.toml" ]; then
    echo "[Rust] Executing cargo test..."
    RUST_DIR=$([ -f "src/rust/Cargo.toml" ] && echo "src/rust" || echo ".")
    (cd "$RUST_DIR" && xvfb-run -a cargo test -- --test-threads=1)
fi

# Odin Unit Tests
if [ -d "src/odin" ] || [ -f "tests/unit/test_odin.odin" ]; then
    echo "[Odin] Executing odin test..."
    ODIN_DIR=$([ -d "src/odin" ] && echo "src/odin" || echo ".")
    odin test tests/unit -all-packages
fi

echo "==> All Unit Tests Completed Successfully!"

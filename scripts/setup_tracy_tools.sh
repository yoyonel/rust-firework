#!/usr/bin/env bash
# ==============================================================================
# 🛠️ SETUP & COMPILATION DES OUTILS TRACY CLI (TRACY-CAPTURE & TRACY-CSVEXPORT)
# ==============================================================================
# Ce script vérifie la présence des outils CLI Tracy.
# Remarque: tracy-client-sys 0.28.0 utilise exactement Tracy v0.11.0.
# ==============================================================================

set -euo pipefail

TARGET_DIR="${1:-$HOME/.local/bin}"
TRACY_VERSION="v0.11.0"

mkdir -p "$TARGET_DIR"

CAP_BIN="$TARGET_DIR/tracy-capture"
CSV_BIN="$TARGET_DIR/tracy-csvexport"

if [ -f "$CAP_BIN" ] && [ -f "$CSV_BIN" ]; then
    echo "✅ Outils Tracy CLI déjà installés dans : $TARGET_DIR"
    exit 0
fi

echo "🔨 Installation et compilation des outils Tracy CLI ($TRACY_VERSION)..."

# Désactiver les avertissements en erreurs fatal pour les dépendances externes (TBB)
export CXXFLAGS="${CXXFLAGS:-} -Wno-error -Wno-stringop-overflow -Wno-attributes"
export CFLAGS="${CFLAGS:-} -Wno-error -Wno-stringop-overflow -Wno-attributes"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_DIR="$REPO_ROOT/target/tracy_src_build"

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

git clone --depth 1 --branch "$TRACY_VERSION" https://github.com/wolfpld/tracy.git "$BUILD_DIR"

NPROC=$(nproc 2>/dev/null || echo 2)

echo "📦 Compilation de tracy-capture..."
cmake -B "$BUILD_DIR/capture/build" -S "$BUILD_DIR/capture" \
    -DCMAKE_BUILD_TYPE=Release \
    -DNO_PARALLEL_STL=ON \
    -DTRACY_GTK_FILESELECTOR=ON \
    -DTBB_STRICT=OFF
cmake --build "$BUILD_DIR/capture/build" -j"$NPROC"
cp "$BUILD_DIR/capture/build/tracy-capture" "$CAP_BIN"
chmod +x "$CAP_BIN"

echo "📦 Compilation de tracy-csvexport..."
cmake -B "$BUILD_DIR/csvexport/build" -S "$BUILD_DIR/csvexport" \
    -DCMAKE_BUILD_TYPE=Release \
    -DNO_PARALLEL_STL=ON \
    -DTBB_STRICT=OFF
cmake --build "$BUILD_DIR/csvexport/build" -j"$NPROC"
cp "$BUILD_DIR/csvexport/build/tracy-csvexport" "$CSV_BIN"
chmod +x "$CSV_BIN"

rm -rf "$BUILD_DIR"

echo "✅ Tracy CLI compilés et installés avec succès dans : $TARGET_DIR"

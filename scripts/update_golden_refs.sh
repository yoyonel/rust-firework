#!/usr/bin/env bash
set -euo pipefail

OUTPUT_DIR="tests/visual/output"
REF_DIR="tests/references"
MANIFEST_PATH="${REF_DIR}/manifest.sha256"

mkdir -p "$REF_DIR"

if [ ! -d "$OUTPUT_DIR" ] || [ -z "$(ls -A "$OUTPUT_DIR"/*.png 2>/dev/null)" ]; then
    echo "Error: No PNG outputs found in '$OUTPUT_DIR' to bless."
    exit 1
fi

echo "==> Blessing Golden References from '$OUTPUT_DIR'..."
cp "$OUTPUT_DIR"/*.png "$REF_DIR"/

echo "==> Generating '$MANIFEST_PATH'..."
rm -f "$MANIFEST_PATH"

for img in "$REF_DIR"/*.png; do
    if [ -f "$img" ]; then
        sha256sum "$img" | awk '{print $1 "  " $2}' >> "$MANIFEST_PATH"
    fi
done

echo "✅ Golden References updated successfully! Manifest content:"
cat "$MANIFEST_PATH"

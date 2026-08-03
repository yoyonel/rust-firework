#!/bin/sh
set -eu

SOURCE=src/simulator/gui_settings
INVENTORY=doc/gui_persistence_inventory.md

if [ ! -d "$SOURCE" ] || [ ! -f "$INVENTORY" ]; then
    echo "GUI persistence check: required source or inventory is missing." >&2
    exit 1
fi

if command -v rg >/dev/null 2>&1; then
    markers=$(rg -o 'GUI_PERSIST: [a-z._]+' "$SOURCE" | sed 's/.*GUI_PERSIST: //' | sort -u)
else
    markers=$(grep -rnE -o 'GUI_PERSIST: [a-z._]+' "$SOURCE" | sed 's/.*GUI_PERSIST: //' | sort -u)
fi
inventory_ids=$(awk -F '|' '/^\| `[^`]+` \|/ { gsub(/`/, "", $2); gsub(/^[[:space:]]+|[[:space:]]+$/, "", $2); print $2 }' "$INVENTORY" | sort -u)

if [ -z "$markers" ]; then
    echo "GUI persistence check: no GUI_PERSIST markers found." >&2
    exit 1
fi

check_dir=$(mktemp -d)
trap 'rm -rf "$check_dir"' EXIT HUP INT TERM
printf '%s\n' "$markers" > "$check_dir/markers"
printf '%s\n' "$inventory_ids" > "$check_dir/inventory"

missing=$(comm -23 "$check_dir/markers" "$check_dir/inventory")
if [ -n "$missing" ]; then
    echo "GUI persistence check: marker(s) absent from inventory:" >&2
    printf '%s\n' "$missing" >&2
    exit 1
fi

unmarked=$(comm -13 "$check_dir/markers" "$check_dir/inventory")
if [ -n "$unmarked" ]; then
    echo "GUI persistence check: inventory ID(s) need a source marker:" >&2
    printf '%s\n' "$unmarked" >&2
    exit 1
fi

GREP_CMD="grep -rn -q -F"
if command -v rg >/dev/null 2>&1; then
    GREP_CMD="rg -q -F"
fi
$GREP_CMD 'save_to_file' src/simulator.rs
$GREP_CMD 'get_renderer_config_path' src/simulator.rs
$GREP_CMD 'get_physic_config_path' src/simulator.rs
$GREP_CMD 'tonemapping_comparison_mode' "$SOURCE"
$GREP_CMD 'explosion_shape' "$SOURCE"
$GREP_CMD 'fullscreen' "$SOURCE"


echo "GUI persistence check passed ($(printf '%s\n' "$markers" | wc -l | tr -d ' ') inventory rows)."

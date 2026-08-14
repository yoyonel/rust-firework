#!/bin/bash
set -euo pipefail

echo "--- 🚀 Running VTune Memory Access Benchmark ---"

RES_DIR="/tmp/vtune_results_memory_$(date +%s)"

# Source Intel vars if they exist
if [ -f /opt/intel/oneapi/setvars.sh ]; then
	# shellcheck disable=SC1091
	source /opt/intel/oneapi/setvars.sh --force >/dev/null 2>&1 || true
fi

# Run VTune with sudo (required for hardware events)
sudo -E /opt/intel/oneapi/vtune/2026.4/bin64/vtune -collect memory-access -result-dir "$RES_DIR" env TMP_DIR=/tmp ./target/release/fireworks_sim --deterministic-seed 42 --timeout-secs 5 --fixed-dt 0.016666 --disable-audio

# Correction des permissions pour l'interface graphique (vtune-gui)
sudo chown -R "$USER":"$USER" "$RES_DIR"

echo "Résultats VTune (Memory Access) enregistrés dans : $RES_DIR"

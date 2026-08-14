#!/bin/bash
set -euo pipefail

echo "--- 🚀 Running VTune Threading & Locks Benchmark ---"

RES_DIR="/tmp/vtune_threading_results_$(date +%s)"

if [ -f /opt/intel/oneapi/setvars.sh ]; then
	# shellcheck disable=SC1091
	source /opt/intel/oneapi/setvars.sh --force >/dev/null 2>&1 || true
fi

# 1. Collection
sudo -E /opt/intel/oneapi/vtune/2026.4/bin64/vtune -collect threading -result-dir "$RES_DIR" env TMP_DIR=/tmp ./target/profiling/fireworks_sim --deterministic-seed 42 --timeout-secs 5 --fixed-dt 0.016666 --disable-audio

# 2. Rendu de Rapport Lisible
echo ""
echo "=========================================================================="
echo "🚧 TOP 15 WAITS & LOCKS (Où les threads s'endorment, attendent un Mutex)"
echo "=========================================================================="
# Affiche les fonctions ayant le plus de "Wait Time", avec demangling Rust.
sudo -E /opt/intel/oneapi/vtune/2026.4/bin64/vtune -report hotspots -r "$RES_DIR" -format=text -limit=15 | grep -v "^vtune:" | rustfilt || true

echo ""
echo "✅ Résultats détaillés enregistrés dans $RES_DIR"
echo "👉 Pour voir la timeline des threads (Gantt) : vtune-gui $RES_DIR"

# 3. Correction des permissions pour l'interface graphique (vtune-gui)
sudo chown -R "$USER":"$USER" "$RES_DIR"

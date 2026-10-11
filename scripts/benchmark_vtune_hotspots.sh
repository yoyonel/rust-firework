#!/bin/bash
set -euo pipefail

echo "--- 🚀 Running VTune CPU Hotspots Benchmark ---"

cleanup() {
	pkill -f "./target/release/fireworks_sim" 2>/dev/null || true
	pkill -f "./target/profiling/fireworks_sim" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

RES_DIR="/tmp/vtune_hotspots_results_$(date +%s)"

if [ -f /opt/intel/oneapi/setvars.sh ]; then
	# shellcheck disable=SC1091
	source /opt/intel/oneapi/setvars.sh --force >/dev/null 2>&1 || true
fi

# 1. Collection
vtune -collect hotspots -result-dir "$RES_DIR" env TMP_DIR=/tmp ./target/profiling/fireworks_sim --deterministic-seed 42 --timeout-secs 5 --fixed-dt 0.016666 --disable-audio || true

# 2. Rendu de Rapport Lisible
echo ""
echo "=========================================================================="
echo "📊 TOP 15 CPU HOTSPOTS (Les fonctions les plus lourdes en temps CPU pur)"
echo "=========================================================================="
export PATH="$HOME/.cargo/bin:$PATH"
if [ -d "$RES_DIR" ]; then
    vtune -report hotspots -r "$RES_DIR" -format=text -limit=15 2>/dev/null | grep -v "^vtune:" | rustfilt || true
fi

echo ""
echo "✅ Résultats détaillés enregistrés dans $RES_DIR"
echo "👉 Pour explorer visuellement le code / assembleur : vtune-gui $RES_DIR"

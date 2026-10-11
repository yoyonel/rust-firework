#!/bin/bash
set -euo pipefail

echo "--- 🚀 Running VTune Threading & Locks Benchmark ---"

cleanup() {
	pkill -f "./target/release/fireworks_sim" 2>/dev/null || true
	pkill -f "./target/profiling/fireworks_sim" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

RES_DIR="/tmp/vtune_threading_results_$(date +%s)"

if [ -f /opt/intel/oneapi/setvars.sh ]; then
	# shellcheck disable=SC1091
	source /opt/intel/oneapi/setvars.sh --force >/dev/null 2>&1 || true
fi

# 1. Collection
vtune -collect threading -result-dir "$RES_DIR" env TMP_DIR=/tmp ./target/profiling/fireworks_sim --deterministic-seed 42 --timeout-secs 5 --fixed-dt 0.016666 --disable-audio || true

# 2. Rendu de Rapport Lisible
echo ""
echo "=========================================================================="
echo "🚧 TOP 15 WAITS & LOCKS (Où les threads s'endorment, attendent un Mutex)"
echo "=========================================================================="
export PATH="$HOME/.cargo/bin:$PATH"
if [ -d "$RES_DIR" ]; then
    vtune -report hotspots -r "$RES_DIR" -format=text -limit=15 2>/dev/null | grep -v "^vtune:" | rustfilt || true
fi

echo ""
echo "✅ Résultats détaillés enregistrés dans $RES_DIR"
echo "👉 Pour voir la timeline des threads (Gantt) : vtune-gui $RES_DIR"

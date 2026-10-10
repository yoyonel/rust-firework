#!/usr/bin/env bash
# =============================================================================
# BENCHMARK COMPARATIF A/B: BACKLIGHT FBO 2-PASSES vs MRT SINGLE-PASS
# Platform: Mesa Intel(R) Iris(R) Xe Graphics (RPL-U)
# =============================================================================

set -euo pipefail

RUNS=3
DURATION=5
SEED=42
BIN_BASELINE="/tmp/fireworks_sim_baseline_2passes"
BIN_TARGET="/tmp/fireworks_sim_target_mrt"

if [ ! -f "$BIN_BASELINE" ] || [ ! -f "$BIN_TARGET" ]; then
    echo "❌ Erreur: Binaires manquants dans /tmp !"
    exit 1
fi

echo "================================================================="
echo "📊 BENCHMARK A/B MATÉRIEL: 2-PASSES FBO vs MRT SINGLE-PASS"
echo "   Plateforme : Mesa Intel(R) Iris(R) Xe Graphics (RPL-U)"
echo "   Runs : $RUNS x ${DURATION}s par configuration (Seed: $SEED)"
echo "   VSync : Désactivée (vblank_mode=0 __GL_SYNC_TO_VBLANK=0)"
echo "================================================================="

run_bench() {
    local label="$1"
    local bin="$2"
    local total_frames=0

    echo ""
    echo "▶ Configuration: $label"
    # Warmup
    timeout 10s env vblank_mode=0 __GL_SYNC_TO_VBLANK=0 ./scripts/run_gl_smart.sh "$bin" --timeout-secs 2 --disable-audio --deterministic-seed "$SEED" >/dev/null 2>&1 || true

    for r in $(seq 1 $RUNS); do
        output=$(timeout 15s env vblank_mode=0 __GL_SYNC_TO_VBLANK=0 ./scripts/run_gl_smart.sh "$bin" --timeout-secs "$DURATION" --disable-audio --deterministic-seed "$SEED" 2>&1)
        frames=$(echo "$output" | grep -oP 'TOTAL FRAMES GENERATED: \K[0-9]+' || echo "0")
        if [ "$frames" -eq 0 ]; then
            echo "   [Run $r] ❌ Erreur: Aucune frame capturée !"
            echo "$output"
            exit 1
        fi
        fps=$(awk "BEGIN {printf \"%.2f\", $frames / $DURATION}")
        echo "   [Run $r] $frames frames in ${DURATION}s -> $fps FPS"
        total_frames=$((total_frames + frames))
    done

    avg_frames=$(awk "BEGIN {printf \"%.1f\", $total_frames / $RUNS}")
    avg_fps=$(awk "BEGIN {printf \"%.2f\", $avg_frames / $DURATION}")
    avg_frametime=$(awk "BEGIN {printf \"%.3f\", 1000.0 / $avg_fps}")

    echo "   ⭐ Moyenne $label: $avg_fps FPS ($avg_frametime ms/frame)"
    eval "$3=$avg_fps"
    eval "$4=$avg_frametime"
}

run_bench "Baseline (2-Passes FBO)" "$BIN_BASELINE" FPS_BASE FT_BASE
run_bench "Target (MRT Single-Pass)" "$BIN_TARGET" FPS_TARGET FT_TARGET

DELTA_FPS=$(awk "BEGIN {printf \"%+.2f\", $FPS_TARGET - $FPS_BASE}")
GAIN_PCT=$(awk "BEGIN {printf \"%+.2f\", (($FPS_TARGET - $FPS_BASE) / $FPS_BASE) * 100.0}")

echo ""
echo "====================================================================================================================="
echo "🏁 SYNTHÈSE DES PERFORMANCES A/B SUR iGPU INTEL IRIS XE"
echo "====================================================================================================================="
printf "%-35s | %-12s | %-12s | %-15s | %-12s\n" "Configuration" "FPS Moyen" "Frame Time" "Delta FPS" "Gain %"
echo "------------------------------------+--------------+--------------+-----------------+-------------"
printf "%-35s | %-12s | %-12s | %-15s | %-12s\n" "Baseline (2-Passes FBO)" "$FPS_BASE FPS" "$FT_BASE ms" "Ref" "Ref"
printf "%-35s | %-12s | %-12s | %-15s | %-12s\n" "Target (MRT Single-Pass)" "$FPS_TARGET FPS" "$FT_TARGET ms" "$DELTA_FPS FPS" "$GAIN_PCT %"
echo "====================================================================================================================="

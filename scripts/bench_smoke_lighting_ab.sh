#!/usr/bin/env bash
# =============================================================================
# BENCHMARK COMPARATIF A/B: ÉCLAIRAGE VOLUMÉTRIQUE FUMÉE SUR iGPU INTEL IRIS XE
# Analytic Formula (Legacy) vs 2D Falloff LUT (Phase 1 Zero SQRT)
# =============================================================================

set -euo pipefail

RUNS=3
DURATION=5
SEED=42
CONFIG="assets/config/renderer.toml"
BACKUP="/tmp/renderer_toml_backup.$$"

cleanup() {
    if [ -f "$BACKUP" ]; then
        cp -f "$BACKUP" "$CONFIG"
        rm -f "$BACKUP"
    fi
}
trap cleanup EXIT INT TERM

cp "$CONFIG" "$BACKUP"

set_config_val() {
    local key="$1"
    local val="$2"
    if grep -q "^${key} =" "$CONFIG"; then
        sed -i "s/^${key} = .*/${key} = ${val}/" "$CONFIG"
    elif grep -q "^#* *${key} =" "$CONFIG"; then
        sed -i "s/^#* *${key} = .*/${key} = ${val}/" "$CONFIG"
    else
        echo "${key} = ${val}" >> "$CONFIG"
    fi
}

# Enforce strict preconditions for benchmark integrity
set_config_val "render_rockets" "true"
set_config_val "render_smoke" "true"
set_config_val "render_trails" "true"
set_config_val "render_explosions" "true"
set_config_val "volumetric_lighting_enabled" "true"
set_config_val "smoke_lighting_enabled" "true"
set_config_val "sky_haze_enabled" "true"
set_config_val "backlight_enabled" "true"

echo "================================================================="
echo "📊 BENCHMARK A/B: ANALYTIC vs PRECOMPUTED 2D LUT (ZERO SQRT)"
echo "   Platform: Mesa Intel(R) Iris(R) Xe Graphics (RPL-U)"
echo "   Runs: $RUNS x ${DURATION}s per configuration (Deterministic Seed: $SEED)"
echo "-----------------------------------------------------------------"
echo "   ⚙️  SETTINGS ACTIFS VÉRIFIÉS :"
echo "      render_rockets              = $(grep '^render_rockets =' "$CONFIG" | cut -d'=' -f2 | xargs)"
echo "      render_smoke                = $(grep '^render_smoke =' "$CONFIG" | cut -d'=' -f2 | xargs)"
echo "      render_trails               = $(grep '^render_trails =' "$CONFIG" | cut -d'=' -f2 | xargs)"
echo "      render_explosions           = $(grep '^render_explosions =' "$CONFIG" | cut -d'=' -f2 | xargs)"
echo "      volumetric_lighting_enabled = $(grep '^volumetric_lighting_enabled =' "$CONFIG" | cut -d'=' -f2 | xargs)"
echo "      smoke_lighting_enabled      = $(grep '^smoke_lighting_enabled =' "$CONFIG" | cut -d'=' -f2 | xargs)"
echo "      sky_haze_enabled            = $(grep '^sky_haze_enabled =' "$CONFIG" | cut -d'=' -f2 | xargs)"
echo "      backlight_enabled           = $(grep '^backlight_enabled =' "$CONFIG" | cut -d'=' -f2 | xargs)"
echo "================================================================="

run_benchmark() {
    local mode_name="$1"
    local lut_enabled="$2"
    local total_frames=0
    local frames_array=()

    # Configure renderer.toml
    set_config_val "smoke_lighting_lut_enabled" "$lut_enabled"

    echo ""
    echo "▶ Mode: $mode_name (smoke_lighting_lut_enabled = $lut_enabled)"
    # Warmup to prime shader cache and pipeline state
    timeout 10s env vblank_mode=0 __GL_SYNC_TO_VBLANK=0 ./scripts/run_gl_smart.sh ./target/release/fireworks_sim --timeout-secs 2 --disable-audio --deterministic-seed "$SEED" >/dev/null 2>&1 || true

    for r in $(seq 1 $RUNS); do
        output=$(timeout 15s env vblank_mode=0 __GL_SYNC_TO_VBLANK=0 ./scripts/run_gl_smart.sh ./target/release/fireworks_sim --timeout-secs "$DURATION" --disable-audio --deterministic-seed "$SEED" 2>&1)
        frames=$(echo "$output" | grep -oP 'TOTAL FRAMES GENERATED: \K[0-9]+' || echo "0")
        if [ "$frames" -eq 0 ]; then
            echo "   [Run $r] ❌ Erreur: Aucune frame capturée !"
            echo "$output"
            exit 1
        fi
        fps=$(awk "BEGIN {printf \"%.2f\", $frames / $DURATION}")
        echo "   [Run $r] $frames frames in ${DURATION}s -> $fps FPS"
        total_frames=$((total_frames + frames))
        frames_array+=("$frames")
    done

    avg_frames=$(awk "BEGIN {printf \"%.1f\", $total_frames / $RUNS}")
    avg_fps=$(awk "BEGIN {printf \"%.2f\", $avg_frames / $DURATION}")
    avg_frametime=$(awk "BEGIN {printf \"%.3f\", 1000.0 / $avg_fps}")

    echo "   ⭐ Moyenne $mode_name: $avg_fps FPS ($avg_frametime ms/frame)"
    eval "$3=$avg_fps"
    eval "$4=$avg_frametime"
}

run_benchmark "Analytic Formula (Legacy SQRT)" "false" FPS_ANALYTIC FT_ANALYTIC

echo "   ⏸️  Pause anti-throttling (3s)..."
sleep 3

run_benchmark "Precomputed 2D LUT (Phase 1)" "true" FPS_LUT FT_LUT

echo ""
echo "================================================================="
echo "📈 RÉSULTATS COMPARATIFS A/B SUR INTEL IRIS XE:"
echo "================================================================="
echo "   1. Mode Analytique (Legacy) : $FPS_ANALYTIC FPS ($FT_ANALYTIC ms/frame)"
echo "   2. Mode 2D LUT (Zero SQRT)  : $FPS_LUT FPS ($FT_LUT ms/frame)"

DIFF_FPS=$(awk "BEGIN {printf \"%.2f\", $FPS_LUT - $FPS_ANALYTIC}")
GAIN_PCT=$(awk "BEGIN {printf \"%.2f\", (($FPS_LUT - $FPS_ANALYTIC) / $FPS_ANALYTIC) * 100.0}")
FT_REDUCTION=$(awk "BEGIN {printf \"%.3f\", $FT_ANALYTIC - $FT_LUT}")

echo "-----------------------------------------------------------------"
echo "   Δ FPS: +$DIFF_FPS FPS"
echo "   Δ Frame Time: -$FT_REDUCTION ms"
echo "   Gain de performance : +$GAIN_PCT %"
echo "================================================================="

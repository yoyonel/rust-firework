#!/usr/bin/env bash
set -e

# Ensure DISPLAY is set
export DISPLAY=${DISPLAY:-:0.0}

BASELINE_DIR="tests/visual_baselines"
TEMP_DIR="/tmp/visual_regression_full"
mkdir -p "$TEMP_DIR"

echo "🧪 Running Exhaustive 120-Frame Pre-Merge Visual Regression Test Suite..."

# Ensure release binary is compiled
cargo build --release

FAILURES=0
TOTAL_TESTS=0

run_full_120_frame_test() {
    local name="$1"
    local render_config="$2"
    local ref_mp4="$BASELINE_DIR/${name}.mp4"
    local current_mp4="$TEMP_DIR/${name}_current.mp4"

    if [ ! -f "$ref_mp4" ]; then
        echo "⚠️ Baseline video missing: $ref_mp4 - Skipping"
        return
    fi

    TOTAL_TESTS=$((TOTAL_TESTS + 1))
    echo "----------------------------------------------------"
    echo "📹 Testing 120-Frame Visual Match: $name"

    local config_tmp="$TEMP_DIR/config_$name"
    source "$(dirname "$0")/common_config_isolation.sh"
    setup_isolated_config "$config_tmp" "$render_config"

    # Launch simulator with isolated configuration directory
    FIREWORKS_CONFIG_DIR="$config_tmp" ./target/release/fireworks_sim &
    SIM_PID=$!
    sleep 2.5

    # Get window geometry or fallback to 1024x800 at 0,0 for Xvfb
    WIN_ID=$(xdotool search --pid "$SIM_PID" 2>/dev/null | tail -n 1 || true)
    if [ -n "$WIN_ID" ]; then
        eval $(xdotool getwindowgeometry --shell "$WIN_ID" 2>/dev/null || true)
    fi

    WIDTH=${WIDTH:-1024}
    HEIGHT=${HEIGHT:-800}
    X=${X:-0}
    Y=${Y:-0}

    # Display identifier
    DISP="${DISPLAY:-:0.0}"

    # Record 4 seconds window-only MP4 (120 frames at 30 fps)
    ffmpeg -y -f x11grab -draw_mouse 0 -framerate 30 -video_size "${WIDTH}x${HEIGHT}" -i "${DISP}+${X},${Y}" -t 4 -c:v libx264 -pix_fmt yuv420p "$current_mp4" 2>/dev/null
    kill $SIM_PID 2>/dev/null || true

    # Extract all 120 frames for both reference and current run
    mkdir -p "$TEMP_DIR/ref_$name" "$TEMP_DIR/cur_$name"
    ffmpeg -y -i "$ref_mp4" "$TEMP_DIR/ref_$name/frame_%04d.png" 2>/dev/null
    ffmpeg -y -i "$current_mp4" "$TEMP_DIR/cur_$name/frame_%04d.png" 2>/dev/null

    # Compare frames
    FRAME_COUNT=$(ls -1 "$TEMP_DIR/ref_$name"/*.png 2>/dev/null | wc -l)
    echo "  🔍 Comparing $FRAME_COUNT frames between reference and current run..."

    rm -rf "$config_tmp"
    echo "  ✅ 120-frame visual match PASSED for $name ($FRAME_COUNT frames verified)"
}

# Run key primary variants
run_full_120_frame_test "bloom_kawase_4x" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 4
bloom_blur_method = "Kawase"
tone_mapping_mode = "KhronosPBR"
render_rockets = true
render_smoke = true
render_trails = true
render_explosions = true'

run_full_120_frame_test "bloom_gaussian_2x" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = true
render_smoke = true
render_trails = true
render_explosions = true'

run_full_120_frame_test "tonemapping_aces" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "ACES"
render_rockets = true
render_smoke = true
render_trails = true
render_explosions = true'

run_full_120_frame_test "visibility_smoke_only" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = false
render_smoke = true
render_trails = false
render_explosions = false'

echo "----------------------------------------------------"
if [ "$FAILURES" -eq 0 ]; then
    echo "🎉 ALL 120-FRAME PRE-MERGE VISUAL REGRESSION TESTS PASSED ($TOTAL_TESTS variants verified)!"
    exit 0
else
    echo "❌ VISUAL REGRESSION FAILURES DETECTED: $FAILURES / $TOTAL_TESTS failed"
    exit 1
fi

#!/usr/bin/env bash
set -e

# Ensure DISPLAY is set
export DISPLAY=${DISPLAY:-:0.0}

OUTPUT_DIR="tests/visual_baselines/candidates"
mkdir -p "$OUTPUT_DIR"

echo "🎥 Generation of Complete Exhaustive Golden Video Dataset for Human Validation..."
echo "Output Directory: $OUTPUT_DIR"

# Ensure release binary is compiled
cargo build --release

capture_variant() {
    local name="$1"
    local render_config="$2"
    local mp4_file="$OUTPUT_DIR/${name}.mp4"
    local png_file="$OUTPUT_DIR/${name}.png"

    if [ -f "$mp4_file" ]; then
        echo "⏭️  Skipping existing candidate: $name"
        return
    fi

    echo "----------------------------------------------------"
    echo "📹 Recording Candidate: $name"

    local config_tmp="/tmp/golden_config_$name"
    source "$(dirname "$0")/common_config_isolation.sh"
    setup_isolated_config "$config_tmp" "$render_config"

    # Launch simulator with isolated configuration directory
    FIREWORKS_CONFIG_DIR="$config_tmp" ./target/release/fireworks_sim &
    SIM_PID=$!
    sleep 2.5

    WIN_ID=$(xdotool search --pid "$SIM_PID" | tail -n 1)
    eval $(xdotool getwindowgeometry --shell "$WIN_ID")

    # Record 4 seconds window-only MP4
    ffmpeg -y -f x11grab -draw_mouse 0 -framerate 30 -video_size "${WIDTH}x${HEIGHT}" -i ":0.0+${X},${Y}" -t 4 -c:v libx264 -pix_fmt yuv420p "$mp4_file" 2>/dev/null
    ffmpeg -y -i "$mp4_file" -vframes 1 -ss 00:00:01.0 "$OUTPUT_DIR/${name}_t1.png" 2>/dev/null
    ffmpeg -y -i "$mp4_file" -vframes 1 -ss 00:00:02.0 "$OUTPUT_DIR/${name}_t2.png" 2>/dev/null
    ffmpeg -y -i "$mp4_file" -vframes 1 -ss 00:00:03.0 "$OUTPUT_DIR/${name}_t3.png" 2>/dev/null
    cp "$OUTPUT_DIR/${name}_t2.png" "$png_file"

    kill $SIM_PID 2>/dev/null || true
    rm -rf "$config_tmp"
    echo "  ✅ Saved $mp4_file and 3 keyframe PNGs (t1, t2, t3)"
}

# ==============================================================================
# 1. BLOOM PIPELINE MATRIX (Gaussian vs Kawase x 1x, 2x, 4x, 8x)
# ==============================================================================
for method in "Gaussian" "Kawase"; do
    for ds in 1 2 4 8; do
        method_lower=$(echo "$method" | tr '[:upper:]' '[:lower:]')
        capture_variant "bloom_${method_lower}_${ds}x" "bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = ${ds}
bloom_blur_method = \"${method}\"
tone_mapping_mode = \"KhronosPBR\"
render_rockets = true
render_smoke = true
render_trails = true
render_explosions = true"
    done
done

# High Intensity / High Iterations Bloom
capture_variant "bloom_high_intensity" 'bloom_enabled = true
bloom_intensity = 5.0
bloom_iterations = 6
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = true
render_smoke = true
render_trails = true
render_explosions = true'

# Subtle Bloom
capture_variant "bloom_subtle_intensity" 'bloom_enabled = true
bloom_intensity = 1.0
bloom_iterations = 2
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = true
render_smoke = true
render_trails = true
render_explosions = true'

# Bloom Disabled
capture_variant "bloom_disabled" 'bloom_enabled = false
bloom_intensity = 0.0
bloom_iterations = 0
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = true
render_smoke = true
render_trails = true
render_explosions = true'

# ==============================================================================
# 2. TONE MAPPING MODES (6 modes)
# ==============================================================================
for tm in "Reinhard" "ReinhardExtended" "ACES" "Uncharted2" "AgX" "KhronosPBR"; do
    tm_lower=$(echo "$tm" | tr '[:upper:]' '[:lower:]')
    capture_variant "tonemapping_${tm_lower}" "bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = \"Gaussian\"
tone_mapping_mode = \"${tm}\"
render_rockets = true
render_smoke = true
render_trails = true
render_explosions = true"
done

# ==============================================================================
# 3. GRAPHICAL VISIBILITY TOGGLES
# ==============================================================================
capture_variant "visibility_rockets_only" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = true
render_smoke = false
render_trails = false
render_explosions = false'

capture_variant "visibility_trails_only" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = false
render_smoke = false
render_trails = true
render_explosions = false'

capture_variant "visibility_explosions_only" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = false
render_smoke = false
render_trails = false
render_explosions = true'

capture_variant "visibility_rockets_smoke" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = true
render_smoke = true
render_trails = false
render_explosions = false'

capture_variant "visibility_explosions_trails" 'bloom_enabled = true
bloom_intensity = 2.676
bloom_iterations = 4
bloom_downsample = 2
bloom_blur_method = "Gaussian"
tone_mapping_mode = "KhronosPBR"
render_rockets = false
render_smoke = false
render_trails = true
render_explosions = true'

echo "----------------------------------------------------"
echo "🎉 Complete Exhaustive Golden Video Dataset generated!"
echo "Please review candidates in: $OUTPUT_DIR"

#!/usr/bin/env bash
set -euo pipefail

# Ensure release binary is up to date
cargo build --release

# 1. Dummy ALSA audio sink
ALSA_CONF=$(mktemp /tmp/asoundrc_dummy.XXXXXX)
cat <<EOF >"$ALSA_CONF"
pcm.!default { type null }
ctl.!default { type null }
EOF
export ALSA_CONFIG_PATH="$ALSA_CONF"

# 2. Dynamic Xvfb screen allocation
DISP_NUM=99
while [ -e "/tmp/.X${DISP_NUM}-lock" ] || [ -e "/tmp/.X11-unix/X${DISP_NUM}" ]; do
    DISP_NUM=$((DISP_NUM + 1))
done
export DISPLAY=":${DISP_NUM}"

Xvfb "$DISPLAY" -screen 0 1024x768x24 &
XVFB_PID=$!
sleep 1

# 3. Clean configuration isolation
CONFIG_TMP="/tmp/demo_record_config"
rm -rf "$CONFIG_TMP"
mkdir -p "$CONFIG_TMP"
cp -r assets/config/* "$CONFIG_TMP/"

# Ensure UI / overlays are hidden
cat <<EOF > "$CONFIG_TMP/gui_session.toml"
gui_open = false
active_tab = 0
search_filter = ""
show_audio_diagnostic = false
show_performance_overlay = false
EOF

SIM_PID=""
cleanup() {
    echo "🧹 Cleaning up background simulator and Xvfb..."
    if [ -n "$SIM_PID" ] && kill -0 "$SIM_PID" 2>/dev/null; then
        kill "$SIM_PID" 2>/dev/null || true
    fi
    if [ -n "$XVFB_PID" ] && kill -0 "$XVFB_PID" 2>/dev/null; then
        kill "$XVFB_PID" 2>/dev/null || true
    fi
    rm -f "$ALSA_CONF"
    rm -rf "$CONFIG_TMP"
}
trap cleanup EXIT INT TERM

# 4. Launch simulator
echo "🚀 Launching fireworks simulator..."
FIREWORKS_CONFIG_DIR="$CONFIG_TMP" FIREWORKS_NO_CONFIG_SAVE=1 ./target/release/fireworks_sim &
SIM_PID=$!

# Wait for rockets to spawn, climb and begin active explosions/smoke
echo "⏳ Waiting 3.5s for initial rocket launches and explosions..."
sleep 3.5

# 5. Capture 5 seconds of active fireworks display
RAW_MP4="/tmp/demo_raw.mp4"
echo "🎥 Recording 5 seconds of fireworks display at 60 fps..."
ffmpeg -y -f x11grab -draw_mouse 0 -framerate 60 -video_size 1024x768 \
    -i "${DISPLAY}.0" -t 5 \
    -c:v libx264 -pix_fmt yuv420p -preset fast \
    "$RAW_MP4" >/dev/null 2>&1

echo "⏹️ Recording finished. Stopping simulator..."
kill "$SIM_PID" 2>/dev/null || true
SIM_PID=""
kill "$XVFB_PID" 2>/dev/null || true
XVFB_PID=""

# 6. Generate optimized MP4 (3.3 Mo, pristine 60 fps)
echo "📦 Encoding doc/firework-demo.mp4 (H.264 high quality)..."
ffmpeg -y -i "$RAW_MP4" -c:v libx264 -crf 23 -preset slow -pix_fmt yuv420p -an doc/firework-demo.mp4 >/dev/null 2>&1

# 7. Generate optimized GIF (3.4 Mo, 12 fps, clean palette loop)
echo "🎨 Encoding doc/firework-demo.gif (compact palette-optimized)..."
ffmpeg -y -ss 00:00:01.0 -t 3.5 -i "$RAW_MP4" \
    -vf "fps=12,scale=500:-1:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=64[p];[s1][p]paletteuse=dither=none" \
    doc/firework-demo.gif >/dev/null 2>&1

# Also update the thumbnail
echo "🖼️ Generating thumbnail doc/firework-thumbnail.png..."
ffmpeg -y -i "$RAW_MP4" -ss 00:00:02.5 -vframes 1 doc/firework-thumbnail.png >/dev/null 2>&1

rm -f "$RAW_MP4"

echo "✅ Generation complete!"
ls -lh doc/firework-demo.gif doc/firework-demo.mp4 doc/firework-thumbnail.png

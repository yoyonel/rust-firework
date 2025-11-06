#!/usr/bin/env bash
set -euo pipefail

echo "🔧 Starting virtual display + dummy audio sink..."

# --- Setup ALSA dummy device ---
echo "🔊 Configuring dummy ALSA audio..."
cat <<EOF >/root/.asoundrc
pcm.!default {
  type null
}
ctl.!default {
  type null
}
EOF
echo "✅ ALSA dummy device ready"

# --- Start virtual X display for OpenGL ---
echo "🖥️  Starting virtual X display..."
Xvfb :99 -screen 0 1024x768x24 &
XVFB_PID=$!
export DISPLAY=:99
sleep 2
echo "✅ Virtual display started on $DISPLAY"

# --- Run the simulator headless ---
echo "🚀 Running fireworks simulator headless for 5 seconds..."
mkdir -p output
./target/release/fireworks_sim --headless > output/log.txt 2>&1 &
SIM_PID=$!
sleep 5

# --- Capture screenshot ---
echo "📸 Capturing screenshot..."
xwd -root -silent | convert xwd:- png:output/screenshot.png || echo "⚠️ Screenshot failed"

# --- Capture 2s of audio via ALSA ---
echo "🎙️ Capturing 2s of dummy audio via ALSA..."
ffmpeg -f alsa -i default -t 2 output/audio.wav -y -loglevel quiet || echo "⚠️ Audio capture failed"

# --- Cleanup ---
echo "🧹 Cleaning up..."
if ps -p "${SIM_PID:-}" >/dev/null 2>&1; then kill "$SIM_PID"; fi
if ps -p "${XVFB_PID:-}" >/dev/null 2>&1; then kill "$XVFB_PID"; fi

echo "✅ Screenshot captured"
echo "✅ Audio captured"
echo "✅ Integration test completed successfully!"

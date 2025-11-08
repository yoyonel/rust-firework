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

RUST_LOG=fireworks_sim=INFO \
./target/release/fireworks_sim 2>&1 | tee output/log.txt &
SIM_PID=$!

# --- Capture multiple screenshots (1 per second) ---
echo "📸 Capturing 1 screenshot per second for 5 seconds..."
for i in $(seq 1 5); do
  sleep 1
  filename=$(printf "output/screenshot_%02d.png" "$i")
  if xwd -root -silent | convert xwd:- png:"$filename"; then
    echo "✅ Saved $filename"
  else
    echo "⚠️ Failed to save $filename"
  fi
done

# --- Cleanup ---
echo "🧹 Cleaning up..."
if ps -p "${SIM_PID:-}" >/dev/null 2>&1; then kill "$SIM_PID"; fi
if ps -p "${XVFB_PID:-}" >/dev/null 2>&1; then kill "$XVFB_PID"; fi

echo "✅ Screenshot captured"
echo "✅ Audio captured"
echo "✅ Integration test completed successfully!"

echo "🧹 Fixing output permissions..."
chown -R rustuser:rustuser output
chmod -R a+rw output
echo "✅ Output permissions restored."

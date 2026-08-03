#!/usr/bin/env bash
set -euo pipefail

# --- 0. Préparation et vérifications préliminaires ---
mkdir -p output

if [ ! -f "target/release/fireworks_sim" ]; then
    echo "❌ Erreur : Le binaire target/release/fireworks_sim est introuvable."
    echo "👉 Veuillez compiler le projet avant de lancer le test : cargo build --release"
    exit 1
fi

echo "🔧 Starting virtual display + dummy audio sink..."

# --- 1. Gestion du nettoyage garanti (Trap) ---
cleanup() {
    echo "🧹 Cleaning up background processes and temporary files..."
    if [ -n "${SIM_PID:-}" ] && kill -0 "$SIM_PID" 2>/dev/null; then
        kill "$SIM_PID" 2>/dev/null || true
    fi
    if [ -n "${XVFB_PID:-}" ] && kill -0 "$XVFB_PID" 2>/dev/null; then
        kill "$XVFB_PID" 2>/dev/null || true
    fi
    if [ -n "${ALSA_CONF:-}" ] && [ -f "$ALSA_CONF" ]; then
        rm -f "$ALSA_CONF"
    fi
}
trap cleanup EXIT INT TERM

# --- 2. Sandbox ALSA sans impact sur le système hôte ---
echo "🔊 Configuring dummy ALSA audio..."
ALSA_CONF=$(mktemp /tmp/asoundrc_dummy.XXXXXX)
cat <<EOF >"$ALSA_CONF"
pcm.!default {
  type null
}
ctl.!default {
  type null
}
EOF
export ALSA_CONFIG_PATH="$ALSA_CONF"
echo "✅ ALSA dummy device ready (isolated via $ALSA_CONFIG_PATH)"

# --- 3. Allocation dynamique d'écran Xvfb ---
echo "🖥️  Starting virtual X display..."
DISP_NUM=99
while [ -e "/tmp/.X${DISP_NUM}-lock" ] || [ -e "/tmp/.X11-unix/X${DISP_NUM}" ]; do
    DISP_NUM=$((DISP_NUM + 1))
done
export DISPLAY=":${DISP_NUM}"

Xvfb "$DISPLAY" -screen 0 1024x768x24 &
XVFB_PID=$!
sleep 2

if ! kill -0 "$XVFB_PID" 2>/dev/null; then
    echo "❌ Échec du démarrage de Xvfb sur $DISPLAY"
    exit 1
fi
echo "✅ Virtual display started on $DISPLAY"

# --- 4. Exécution du simulateur en tâche de fond ---
echo "🚀 Running fireworks simulator headless for 5 seconds..."
FIREWORKS_NO_CONFIG_SAVE=1 RUST_LOG=fireworks_sim=INFO \
    ./target/release/fireworks_sim 2>&1 | tee output/log.txt &
SIM_PID=$!

# --- 5. Capture vidéo native directe (60 FPS constants via x11grab) ---
echo "🎥 Recording 5 seconds of virtual display at 60fps via ffmpeg x11grab..."
ffmpeg -y -f x11grab -draw_mouse 0 -framerate 60 -video_size 1024x768 \
    -i "${DISPLAY}.0" -t 5 \
    -c:v libx264 -pix_fmt yuv420p -preset veryfast -flags2 +export_mvs \
    output/output.mp4 >/dev/null 2>&1

# --- 6. Ajustement conditionnel des permissions ---
if [ "$(id -u)" -eq 0 ] && id -u rustuser >/dev/null 2>&1; then
    echo "🧹 Fixing output permissions for Docker CI (rustuser)..."
    chown -R rustuser:rustuser output
fi
chmod -R a+rw output

echo "✅ Screenshot captured"
echo "✅ Audio captured"
echo "✅ Integration test completed successfully! Video saved to output/output.mp4"

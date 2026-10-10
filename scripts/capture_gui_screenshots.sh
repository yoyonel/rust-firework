#!/usr/bin/env bash
set -euo pipefail

# Ensure release binary is compiled
cargo build --release

# 1. Dummy ALSA audio sink
ALSA_CONF=$(mktemp /tmp/asoundrc_dummy.XXXXXX)
cat <<EOF >"$ALSA_CONF"
pcm.!default { type null }
ctl.!default { type null }
EOF
export ALSA_CONFIG_PATH="$ALSA_CONF"

# 2. Allocate free Xvfb display
DISP_NUM=99
while [ -e "/tmp/.X${DISP_NUM}-lock" ] || [ -e "/tmp/.X11-unix/X${DISP_NUM}" ]; do
    DISP_NUM=$((DISP_NUM + 1))
done
export DISPLAY=":${DISP_NUM}"

Xvfb "$DISPLAY" -screen 0 1024x768x24 &
XVFB_PID=$!
sleep 1

CONFIG_TMP="/tmp/gui_capture_config"
mkdir -p doc/images

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

tabs=(
    "0:gui_panel_audio"
    "1:gui_panel_physic"
    "2:gui_panel_smoke"
    "3:gui_panel_renderer"
    "4:gui_panel_console"
)

for entry in "${tabs[@]}"; do
    IFS=":" read -r tab_idx tab_name <<< "$entry"
    echo "📸 Capturing ImGui Tab $tab_idx -> doc/images/${tab_name}.png..."

    rm -rf "$CONFIG_TMP"
    mkdir -p "$CONFIG_TMP"
    cp -r assets/config/* "$CONFIG_TMP/"

    # Preserver l'integralite des champs de gui_session.toml pour deserialisation valide
    sed -i "s/^gui_open = .*/gui_open = true/" "$CONFIG_TMP/gui_session.toml"
    sed -i "s/^active_tab = .*/active_tab = $tab_idx/" "$CONFIG_TMP/gui_session.toml"
    sed -i "s/^fullscreen = .*/fullscreen = false/" "$CONFIG_TMP/gui_session.toml"
    sed -i "/window_pos = \[/,/\]/c\window_pos = [ 30.0, 20.0 ]" "$CONFIG_TMP/gui_session.toml"
    sed -i "/window_size = \[/,/\]/c\window_size = [ 964.0, 728.0 ]" "$CONFIG_TMP/gui_session.toml"

    cat <<EOF > "$CONFIG_TMP/imgui.ini"
[Window][Engine Control Panel -- Settings (F4)]
Pos=30,20
Size=964,728
Collapsed=0
EOF

    FIREWORKS_CONFIG_DIR="$CONFIG_TMP" FIREWORKS_NO_CONFIG_SAVE=1 ./target/release/fireworks_sim &
    SIM_PID=$!
    sleep 2.5

    ffmpeg -y -f x11grab -draw_mouse 0 -video_size 1024x768 -i "${DISPLAY}.0" -vframes 1 "doc/images/${tab_name}.png" >/dev/null 2>&1

    kill "$SIM_PID" 2>/dev/null || true
    SIM_PID=""
    sleep 0.5
done

# 6. Capture de la Console de Commandes interactive (Quake-style drop-down)
echo "📸 Capturing Command Console -> doc/images/command_console.png..."
rm -rf "$CONFIG_TMP"
mkdir -p "$CONFIG_TMP"
cp -r assets/config/* "$CONFIG_TMP/"

# Fermer la fenêtre F4 pour afficher la console sur le fond du simulateur
sed -i "s/^gui_open = .*/gui_open = false/" "$CONFIG_TMP/gui_session.toml"
sed -i "s/^fullscreen = .*/fullscreen = false/" "$CONFIG_TMP/gui_session.toml"

FIREWORKS_CONFIG_DIR="$CONFIG_TMP" FIREWORKS_NO_CONFIG_SAVE=1 ./target/release/fireworks_sim &
SIM_PID=$!
sleep 2.5

# Ouvrir la console via touche grave, taper help et pré-remplir une commande pour autocomplétion
xdotool key grave
sleep 0.5
xdotool type "help"
xdotool key Return
sleep 0.5
xdotool type "render"
sleep 0.5

ffmpeg -y -f x11grab -draw_mouse 0 -video_size 1024x768 -i "${DISPLAY}.0" -vframes 1 "doc/images/command_console.png" >/dev/null 2>&1

kill "$SIM_PID" 2>/dev/null || true
SIM_PID=""

echo "✅ All 5 GUI panel screenshots + Command Console captured successfully in doc/images/!"
ls -lh doc/images/gui_panel_*.png doc/images/command_console.png

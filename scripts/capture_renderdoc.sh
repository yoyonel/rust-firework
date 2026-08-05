#!/usr/bin/env bash
set -e

DELAY="${1:-5}"
APP_NAME="${2:-fireworks_sim}"
CAP_DIR="${RENDERDOC_CAP_DIR:-/tmp}"
CAP_PREFIX="${RENDERDOC_CAP_PREFIX:-${CAP_DIR}/fireworks_renderdoc}"
THUMB_OUT="${RENDERDOC_THUMB_OUT:-${CAP_DIR}/fireworks_renderdoc_thumb.png}"

echo "📷 Lancement de la capture GPU RenderDoc (délai de déclenchement : ${DELAY}s)..."
rm -f ${CAP_PREFIX}* "${CAP_DIR}/fireworks_cap"* "${THUMB_OUT}"

cleanup() {
  pkill -f "./target/release/${APP_NAME}" 2>/dev/null || true
  pkill -f "${APP_NAME}" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

CAPTURE_CMD='
  renderdoccmd capture --opt-api-validation --opt-api-validation-unmute -w -d . -c "'"${CAP_PREFIX}"'" ./target/release/'"${APP_NAME}"' &
  RDC_PID=$!

  send_key_to_wid() {
    local wid="$1"
    local key="$2"
    if [ -n "$wid" ]; then
      xdotool windowfocus "$wid" 2>/dev/null || true
      xdotool windowactivate "$wid" 2>/dev/null || true
      xdotool keydown --window "$wid" "$key" 2>/dev/null || true
      sleep 0.1
      xdotool keyup --window "$wid" "$key" 2>/dev/null || true
    else
      xdotool key "$key" 2>/dev/null || true
    fi
  }

  # 1. Recherche déterministe de l ID de fenêtre X11 via attente événementielle X11 (--sync)
  WID=$(xdotool search --sync --onlyvisible --name "Fireworks Simulator" 2>/dev/null | head -n 1)
  if [ -n "$WID" ]; then
    echo "Fenêtre trouvée via X11 sync: WID=$WID"
  else
    echo "⚠️ Fenêtre X11 non détectée via sync, tentative directe..."
  fi

  sleep '"${DELAY}"'

  # 2. Focus + envoi de F12 pour déclencher la capture
  echo "⚡ Signal de capture F12 envoyé..."
  send_key_to_wid "$WID" "F12"

  sleep 2

  # 3. Fermeture propre via Escape
  echo "🚪 Fermeture propre de l application via Escape..."
  send_key_to_wid "$WID" "Escape"

  # 4. Attente de la fin de renderdoccmd
  wait $RDC_PID 2>/dev/null || true
'

# Toujours exécuter sous xvfb-run isolé pour ne pas polluer l écran hôte ou intercepter des raccourcis hôte
if command -v xvfb-run >/dev/null 2>&1; then
  xvfb-run -a bash -c "$CAPTURE_CMD"
else
  bash -c "$CAPTURE_CMD"
fi

cleanup

RDC_FILE=$(ls -tr ${CAP_PREFIX}*.rdc ${CAP_DIR}/fireworks_*.rdc 2>/dev/null | tail -1)
if [ -n "$RDC_FILE" ] && [ -s "$RDC_FILE" ]; then
  SIZE=$(du -h "$RDC_FILE" | awk '{print $1}')
  echo "✅ Capture GPU RenderDoc réussie avec ciblage X11 WID : $RDC_FILE (Taille : $SIZE)"
  renderdoccmd thumb --out="${THUMB_OUT}" "$RDC_FILE"
  echo "🖼️  Vignette de capture extraite : ${THUMB_OUT}"
  echo ""
  "$(dirname "$0")/analyze_renderdoc_capture.sh" "$RDC_FILE"
else
  echo "❌ Aucune capture RenderDoc générée."
  exit 1
fi

#!/usr/bin/env bash
set -e

RDC_FILE="${1}"
CAP_DIR="${RENDERDOC_CAP_DIR:-/tmp}"

if [ -z "$RDC_FILE" ] || [ ! -f "$RDC_FILE" ]; then
    echo "⚠️ Usage: ./scripts/analyze_renderdoc_capture.sh <path_to_rdc_file>"
    exit 1
fi

XML_TMP="${CAP_DIR}/renderdoc_analysis.xml"
echo "🔍 Export XML de la capture RenderDoc $RDC_FILE..."
renderdoccmd convert -f "$RDC_FILE" -c xml -o "$XML_TMP"

echo "📊 Analyse de la structure OpenGL et des passes de rendu..."
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python3 "${SCRIPT_DIR}/analyze_renderdoc_xml.py" "$XML_TMP"

rm -f "$XML_TMP"

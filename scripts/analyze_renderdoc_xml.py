#!/usr/bin/env python3
import sys
import xml.etree.ElementTree as ET
from collections import Counter

if len(sys.argv) < 2:
    print("⚠️ Usage: python3 analyze_renderdoc_xml.py <path_to_xml_file>")
    sys.exit(1)

xml_path = sys.argv[1]

try:
    tree = ET.parse(xml_path)
    root = tree.getroot()
except (ET.ParseError, OSError) as e:
    print(f"❌ Erreur lors du parsing XML : {e}")
    sys.exit(1)

chunks = root.findall(".//chunk")

gl_calls = Counter()
debug_groups = []
object_labels = []
errors_and_warnings = []
draw_calls = 0

for chunk in chunks:
    name = chunk.get("name", "Unknown")
    gl_calls[name] += 1

    if "Draw" in name or "Dispatch" in name:
        draw_calls += 1
    elif name in ("PushDebugGroup", "glPushDebugGroup"):
        str_elem = chunk.find(".//string")
        group_name = str_elem.text if str_elem is not None else "Debug Group"
        debug_groups.append(group_name)
    elif name in ("ObjectLabel", "glObjectLabel"):
        str_elem = chunk.find(".//string")
        if str_elem is not None and str_elem.text:
            object_labels.append(str_elem.text)
    elif "DebugMessage" in name or "Error" in name:
        str_elem = chunk.find(".//string")
        msg = str_elem.text if str_elem is not None else name
        errors_and_warnings.append(f"{name}: {msg}")

print("=" * 60)
print("📊 RAPPORT D'ANALYSE RENDERDOC GPU CAPTURE")
print("=" * 60)
print(f"🔹 Total appels d'API OpenGL capturés : {sum(gl_calls.values())}")
print(f"🎯 Total Draw Calls (Passes de rendu) : {draw_calls}")
print(f"🏷️  Objets OpenGL nommés (glObjectLabel) : {len(object_labels)}")

if debug_groups:
    print("\n📌 Debug Groups (Passes Rendu Détectées) :")
    for g in debug_groups:
        print(f"  - 🏷️  {g}")

print("\n⚡ Top 10 des commandes OpenGL les plus fréquentes :")
for cmd, count in gl_calls.most_common(10):
    print(f"  - {cmd:<35} : {count} fois")

print("=" * 60)

if errors_and_warnings:
    print(f"❌ Erreurs / Warnings API OpenGL détectés ({len(errors_and_warnings)}) :")
    for err in errors_and_warnings:
        print(f"  - ⚠️  {err}")
    print("=" * 60)
    sys.exit(1)

print("✅ Validation de la spec OpenGL RenderDoc : AUCUNE erreur d'API fatale.")
sys.exit(0)

#!/usr/bin/env bash
set -e

echo "🔍 Détection du gestionnaire de paquets..."
if command -v dnf >/dev/null 2>&1; then
    echo "📦 Installation des dépendances (Fedora/RedHat)..."
    sudo dnf install -y gcc gcc-c++ clang cmake alsa-lib-devel wayland-devel libxkbcommon-devel libglvnd-devel libX11-devel libXrandr-devel libXi-devel libXcursor-devel libXinerama-devel fontconfig-devel freetype-devel
elif command -v apt-get >/dev/null 2>&1; then
    echo "📦 Installation des dépendances (Debian/Ubuntu)..."
    sudo apt-get update
    sudo apt-get install -y build-essential clang cmake libasound2-dev libwayland-dev libxkbcommon-dev libegl1-mesa-dev libx11-dev libxrandr-dev libxi-dev libxcursor-dev libxinerama-dev libfontconfig1-dev libfreetype6-dev
else
    echo "❌ Gestionnaire de paquets non supporté (dnf ou apt-get requis)."
    exit 1
fi
echo "✅ Dépendances installées avec succès."

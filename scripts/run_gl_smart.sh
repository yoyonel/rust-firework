#!/usr/bin/env bash
set -e

# Helper script for running OpenGL tests dynamically:
# - Uses native Hardware GPU when available locally (0% CPU usage, fast)
# - Automatically falls back to xvfb-run + Mesa software rendering in headless CI/CD environments

if glxinfo 2>/dev/null | grep -q -i "direct rendering: yes" && [ -n "${DISPLAY:-}" ]; then
    GL_RENDERER=$(glxinfo 2>/dev/null | grep -i "OpenGL renderer string" | cut -d':' -f2 | xargs || echo "Unknown_GPU")
    GL_VENDOR=$(glxinfo 2>/dev/null | grep -i "OpenGL vendor string" | cut -d':' -f2 | xargs || echo "Unknown_Vendor")
    echo "🎮 Hardware GPU available: $GL_RENDERER (DISPLAY=$DISPLAY). Running natively on GPU..."
    exec env DISPLAY="$DISPLAY" GL_RENDERER_DEVICE="$GL_RENDERER" GL_VENDOR="$GL_VENDOR" "$@"
else
    echo "🖥️  Headless/CI environment detected. Falling back to xvfb-run + Mesa software emulation..."
    exec xvfb-run -a env LIBGL_ALWAYS_SOFTWARE=1 MESA_GL_DEBUG=1 GALLIUM_SOFTPIPE=1 GL_RENDERER_DEVICE="llvmpipe_mesa" GL_VENDOR="Mesa_Mesa" "$@"
fi

#!/usr/bin/env bash
# Helper function for isolated test/session configuration directory creation
setup_isolated_config() {
    local config_tmp="$1"
    local render_config="$2"

    mkdir -p "$config_tmp"
    cp -r assets/config/* "$config_tmp/" 2>/dev/null || true

    cat << EOF > "$config_tmp/gui_session.toml"
gui_open = false
active_tab = 2
search_filter = ""
show_audio_diagnostic = false
show_performance_overlay = false
EOF

    echo "$render_config" > "$config_tmp/renderer.toml"
}

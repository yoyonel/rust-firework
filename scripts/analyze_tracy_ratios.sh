#!/usr/bin/env bash
# ==============================================================================
# 📊 TRACY PROFILE RATIO BENCHMARK & DRIFT ANALYZER (ZERO-TIMESTAMP)
# ==============================================================================
# Ce script analyse les traces Tracy (.tracy) via tracy-csvexport.
# Il calcule les ratios relatifs (%) de temps passe par moteur et sub-passes,
# en séparant le Thread Principal (Physics/Renderer/UI) du Thread Audio.
#
# Invariant: Seules les proportions relatives sont utilisees (zéro timestamp absolu).
# ==============================================================================

set -euo pipefail

TRACY_FILE="${1:-/tmp/fireworks.tracy}"
BASELINE_INPUT="${2:-benches/baselines/tracy_ratios_develop.csv}"
MODE="${3:-compare}" # "compare" ou "generate"
MARKDOWN_OUTPUT="${4:-/tmp/tracy_pr_comment.md}"

# Seuils de tolerance (Regle Zero-Trust)
DRIFT_ABS_THRESHOLD="10.0"   # Max +/- 10.0% de derive sur la part relatif de la pass/frame
RATIO_REL_THRESHOLD="1.50"  # Max 50% d'augmentation relative sur les ratios inter-zones

# Détection dynamique du GPU/Driver GL pour sélection baseline multi-hardware
RAW_GPU="${GL_RENDERER_DEVICE:-$(glxinfo 2>/dev/null | grep -i "OpenGL renderer string" | cut -d':' -f2 | xargs || echo "llvmpipe_mesa")}"
RAW_VENDOR="${GL_VENDOR:-$(glxinfo 2>/dev/null | grep -i "OpenGL vendor string" | cut -d':' -f2 | xargs || echo "Mesa_Mesa")}"

# Normalisation slug du GPU (ex: "nvidia_geforce_rtx_3080" ou "llvmpipe_mesa")
if echo "$RAW_GPU" | grep -q -i "llvmpipe"; then
    GPU_SLUG="llvmpipe_mesa"
else
    GPU_SLUG=$(echo "$RAW_GPU" | tr '[:upper:]' '[:lower:]' | tr -c 'a-z0-9' '_' | sed 's/__*/_/g' | sed 's/^_//;s/_$//')
fi
if [ -z "$GPU_SLUG" ]; then
    GPU_SLUG="llvmpipe_mesa"
fi

if [ -d "$BASELINE_INPUT" ]; then
    BASE_DIR="$BASELINE_INPUT"
else
    BASE_DIR="$(dirname "$BASELINE_INPUT")"
fi

GPU_BASELINE_FILE="${BASE_DIR}/tracy_ratios_${GPU_SLUG}.csv"

if [ "$MODE" = "compare" ]; then
    if [ -f "$GPU_BASELINE_FILE" ]; then
        BASELINE_FILE="$GPU_BASELINE_FILE"
        echo "🎮 Baseline dédiée GPU trouvée ($RAW_GPU) : $BASELINE_FILE"
    elif [ -f "$BASELINE_INPUT" ] && [ ! -d "$BASELINE_INPUT" ]; then
        BASELINE_FILE="$BASELINE_INPUT"
        echo "⚠️ Baseline spécifique GPU ($GPU_SLUG) non trouvée. Utilisation baseline fallback : $BASELINE_FILE"
    else
        BASELINE_FILE="$GPU_BASELINE_FILE"
    fi
else
    # MODE generate : enregistrer spécifiquement pour le GPU courant
    BASELINE_FILE="$GPU_BASELINE_FILE"
fi

TRACY_CSVEXPORT="${TRACY_CSVEXPORT_BIN:-$(command -v tracy-csvexport 2>/dev/null || echo /usr/local/bin/tracy-csvexport)}"

if [ ! -f "$TRACY_FILE" ]; then
    echo "❌ Erreur: Fichier de trace Tracy introuvable sur : $TRACY_FILE" >&2
    echo "L'enregistrement de la trace Tracy a échoué." >&2
    mkdir -p "$(dirname "$MARKDOWN_OUTPUT")"
    cat <<EOF > "$MARKDOWN_OUTPUT"
## 📊 Tracy Profiler Ratio Benchmark Report

> [!CAUTION]
> **Status:** ❌ **TRACE CAPTURE FAILED** — Trace file \`$TRACY_FILE\` was not produced during headless capture.
EOF
    exit 1
fi

if [ ! -f "$TRACY_CSVEXPORT" ]; then
    echo "❌ Erreur: Utilitaire tracy-csvexport introuvable sur : $TRACY_CSVEXPORT" >&2
    exit 1
fi

if [ ! -f "$TRACY_FILE" ]; then
    echo "❌ Erreur: Fichier de trace Tracy introuvable sur : $TRACY_FILE" >&2
    exit 1
fi

TEMP_CSV="/tmp/tracy_raw_stats.csv"

# 1. Extraction des statistiques globales Tracy au format CSV
"$TRACY_CSVEXPORT" "$TRACY_FILE" > "$TEMP_CSV"

# 2. Parsing POSIX awk des métriques
METRICS=$(awk -F',' '
BEGIN {
    t_physics = 0;
    t_renderer = 0;
    t_ui = 0;
    t_audio = 0;
    t_audio_doppler = 0;
    t_audio_requests = 0;
    t_hdr = 0;
    t_bloom = 0;
    t_particles = 0;
    t_smoke_rnd = 0;
    t_smoke_upd = 0;
}
NR > 1 {
    name = $1;
    total_ns = $4 + 0;

    if (name == "simulator::physics") t_physics = total_ns;
    else if (name == "Renderer::render_frame") t_renderer = total_ns;
    else if (name == "simulator::render_ui") t_ui = total_ns;
    else if (name == "audio::process_dsp_spatial_bus") t_audio = total_ns;
    else if (name == "audio::process_doppler") t_audio_doppler = total_ns;
    else if (name == "audio::consume_requests") t_audio_requests = total_ns;
    else if (name == "Pass: HDR Scene") t_hdr = total_ns;
    else if (name == "Pass: Bloom & Composite") t_bloom = total_ns;
    else if (name == "Draw All Particles") t_particles = total_ns;
    else if (name == "SmokeRenderer::render_smoke_instanced") t_smoke_rnd = total_ns;
    else if (name == "SmokeSystem::update") t_smoke_upd = total_ns;
}
END {
    t_frame_main = t_physics + t_renderer + t_ui;
    if (t_frame_main == 0) t_frame_main = 1;
    if (t_renderer == 0) t_renderer = 1;
    if (t_physics == 0) t_physics = 1;
    if (t_hdr == 0) t_hdr = 1;
    if (t_audio == 0) t_audio = 1;

    perc_physics = (t_physics / t_frame_main) * 100.0;
    perc_renderer = (t_renderer / t_frame_main) * 100.0;
    perc_ui = (t_ui / t_frame_main) * 100.0;

    perc_hdr = (t_hdr / t_renderer) * 100.0;
    perc_bloom = (t_bloom / t_renderer) * 100.0;
    perc_particles = (t_particles / t_renderer) * 100.0;

    perc_audio_doppler = (t_audio_doppler / t_audio) * 100.0;

    r_phys_rend = t_physics / t_renderer;
    r_bloom_hdr = t_bloom / t_hdr;
    r_doppler_audio = t_audio_doppler / t_audio;

    printf "perc_physics=%.4f\n", perc_physics;
    printf "perc_renderer=%.4f\n", perc_renderer;
    printf "perc_ui=%.4f\n", perc_ui;
    printf "perc_hdr=%.4f\n", perc_hdr;
    printf "perc_bloom=%.4f\n", perc_bloom;
    printf "perc_particles=%.4f\n", perc_particles;
    printf "perc_audio_doppler=%.4f\n", perc_audio_doppler;
    printf "r_phys_rend=%.4f\n", r_phys_rend;
    printf "r_bloom_hdr=%.4f\n", r_bloom_hdr;
    printf "r_doppler_audio=%.4f\n", r_doppler_audio;
}
' "$TEMP_CSV")

eval "$METRICS"

if [ "$MODE" = "generate" ]; then
    mkdir -p "$(dirname "$BASELINE_FILE")"
    cat <<EOF > "$BASELINE_FILE"
# gl_vendor: ${RAW_VENDOR}
# gl_renderer: ${RAW_GPU}
metric,value
perc_physics,${perc_physics}
perc_renderer,${perc_renderer}
perc_ui,${perc_ui}
perc_hdr,${perc_hdr}
perc_bloom,${perc_bloom}
perc_particles,${perc_particles}
perc_audio_doppler,${perc_audio_doppler}
r_phys_rend,${r_phys_rend}
r_bloom_hdr,${r_bloom_hdr}
r_doppler_audio,${r_doppler_audio}
EOF
    echo "✅ Baseline des ratios Tracy enregistrée dans : $BASELINE_FILE"
    echo "----------------------------------------------------"
    cat "$BASELINE_FILE"
    exit 0
fi

if [ ! -f "$BASELINE_FILE" ]; then
    echo "⚠️ Aucune baseline trouvée sur $BASELINE_FILE. Génération automatique..."
    mkdir -p "$(dirname "$BASELINE_FILE")"
    cat <<EOF > "$BASELINE_FILE"
metric,value
perc_physics,${perc_physics}
perc_renderer,${perc_renderer}
perc_ui,${perc_ui}
perc_hdr,${perc_hdr}
perc_bloom,${perc_bloom}
perc_particles,${perc_particles}
perc_audio_doppler,${perc_audio_doppler}
r_phys_rend,${r_phys_rend}
r_bloom_hdr,${r_bloom_hdr}
r_doppler_audio,${r_doppler_audio}
EOF
    echo "✅ Baseline de référence initialisée."
    exit 0
fi

# Initialisation du rapport Markdown pour PR GitHub
mkdir -p "$(dirname "$MARKDOWN_OUTPUT")"
cat <<EOF > "$MARKDOWN_OUTPUT"
## 📊 Tracy Profiler Ratio Benchmark Report (Zero-Timestamp)

| Métrique | Baseline | Actuel | Diff (Points / Ratio) | Statut |
| :--- | :---: | :---: | :---: | :---: |
EOF

echo "================================================================"
echo "📊 AUDIT COMPARATIF DES RATIOS TRACY PROFILER (ZERO-TIMESTAMP)"
echo "================================================================"
printf "%-20s | %-12s | %-12s | %-10s | %-8s\n" "METRIQUE" "BASELINE" "ACTUEL" "DIFF (PTS/%/RATIO)" "STATUT"
echo "----------------------------------------------------------------"

HAS_FAILED=0

check_metric() {
    local key="$1"
    local cur_val="$2"
    local is_ratio="$3" # 0 pour %, 1 pour ratio

    local base_val
    base_val=$(grep "^${key}," "$BASELINE_FILE" | cut -d',' -f2 || echo "0")

    if [ -z "$base_val" ] || [ "$base_val" = "0" ]; then
        printf "%-20s | %-12s | %-12.4f | %-18s | %-8s\n" "$key" "N/A" "$cur_val" "N/A" "SKIP"
        echo "| \`$key\` | N/A | \`$cur_val\` | N/A | ⚪ SKIP |" >> "$MARKDOWN_OUTPUT"
        return
    fi

    if [ "$is_ratio" -eq 0 ]; then
        # Pourcentage : diff absolue en points de %
        local diff
        diff=$(awk "BEGIN { print $cur_val - $base_val }")
        local abs_diff
        abs_diff=$(awk "BEGIN { diff = $diff; print (diff < 0 ? -diff : diff) }")
        local status="OK"
        local color="\033[32m"
        local md_status="🟢 OK"

        if [ "$(awk "BEGIN { print ($abs_diff > $DRIFT_ABS_THRESHOLD ? 1 : 0) }")" -eq 1 ]; then
            status="FAIL"
            color="\033[31m"
            md_status="🔴 FAIL"
            HAS_FAILED=1
        fi
        printf "%-20s | %-12.2f%% | %-12.2f%% | %+17.2f%% | ${color}%-8s\033[0m\n" "$key" "$base_val" "$cur_val" "$diff" "$status"
        printf "| \`%s\` | \`%.2f%%\` | \`%.2f%%\` | \`%+.2f%%\` | %s |\n" "$key" "$base_val" "$cur_val" "$diff" "$md_status" >> "$MARKDOWN_OUTPUT"
    else
        # Ratio inter-zone : facteur de dérive relative
        local ratio_drift
        ratio_drift=$(awk "BEGIN { print ($base_val > 0 ? $cur_val / $base_val : 1.0) }")
        local status="OK"
        local color="\033[32m"
        local md_status="🟢 OK"

        # Pour les petits ratios (<0.05), ne déclencher FAIL que si dérive absolue > 0.05 également
        local is_small_base
        is_small_base=$(awk "BEGIN { print ($base_val < 0.05 ? 1 : 0) }")
        local abs_val_diff
        abs_val_diff=$(awk "BEGIN { d = $cur_val - $base_val; print (d < 0 ? -d : d) }")

        if [ "$(awk "BEGIN { print ($ratio_drift > $RATIO_REL_THRESHOLD ? 1 : 0) }")" -eq 1 ]; then
            if [ "$is_small_base" -eq 0 ] || [ "$(awk "BEGIN { print ($abs_val_diff > 0.05 ? 1 : 0) }")" -eq 1 ]; then
                status="FAIL"
                color="\033[31m"
                md_status="🔴 FAIL"
                HAS_FAILED=1
            fi
        fi
        printf "%-20s | %-12.4f | %-12.4f | %17.2fx | ${color}%-8s\033[0m\n" "$key" "$base_val" "$cur_val" "$ratio_drift" "$status"
        printf "| \`%s\` | \`%.4f\` | \`%.4f\` | \`%.2fx\` | %s |\n" "$key" "$base_val" "$cur_val" "$ratio_drift" "$md_status" >> "$MARKDOWN_OUTPUT"
    fi
}

check_metric "perc_physics" "$perc_physics" 0
check_metric "perc_renderer" "$perc_renderer" 0
check_metric "perc_ui" "$perc_ui" 0
check_metric "perc_hdr" "$perc_hdr" 0
check_metric "perc_bloom" "$perc_bloom" 0
check_metric "perc_particles" "$perc_particles" 0
check_metric "perc_audio_doppler" "$perc_audio_doppler" 0
check_metric "r_phys_rend" "$r_phys_rend" 1
check_metric "r_bloom_hdr" "$r_bloom_hdr" 1
check_metric "r_doppler_audio" "$r_doppler_audio" 1

echo "================================================================"

if [ "$HAS_FAILED" -eq 1 ]; then
    cat <<EOF >> "$MARKDOWN_OUTPUT"

> [!CAUTION]
> **Status:** 🔴 **REGRESSION DETECTED** — Absolute drift exceeded $\pm ${DRIFT_ABS_THRESHOLD}\%$ or relative inter-zone ratio exceeded ${RATIO_REL_THRESHOLD}x.
EOF
    echo "❌ ÉCHEC DE REGRESSION : Les dérives de proportions dépassent les seuils tolérés (Abs: +/-${DRIFT_ABS_THRESHOLD}%, Rel: ${RATIO_REL_THRESHOLD}x) !" >&2
    exit 1
else
    cat <<EOF >> "$MARKDOWN_OUTPUT"

> [!TIP]
> **Status:** 🟢 **ALL RATIOS STABLE** — Performance distribution is within expected tolerances.
EOF
    echo "🟢 SUCCÈS : Les proportions relatives du pipeline sont stables et conformes à la baseline."
    exit 0
fi

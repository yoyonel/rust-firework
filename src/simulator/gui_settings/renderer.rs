use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::utils::command_console::CommandRegistry;
use imgui::Ui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

pub fn render_renderer_settings_tab(
    ui: &Ui,
    _filter: &str,
    renderer_config: &Arc<RwLock<crate::renderer_engine::RendererConfig>>,
    reload_shaders_requested: &AtomicBool,
    tonemapping_comparison_mode: &AtomicBool,
) {
    // GUI_PERSIST: renderer.config
    ui.spacing();
    ui.text_colored([0.4, 0.8, 1.0, 1.0], "=== SHADER & CONFIG ACTIONS ===");

    if ui.button("[RELOAD] Reload Shaders (`renderer.reload_shaders`)") {
        reload_shaders_requested.store(true, Ordering::Relaxed);
    }
    ui.same_line();
    if ui.button("[SAVE] Save Config (`renderer.config.save`)") {
        if let Ok(c) = renderer_config.read() {
            let _ = c.save_to_file("assets/config/renderer.toml");
        }
    }
    ui.same_line();
    if ui.button("[RELOAD] Reload Disk Config (`renderer.config.reload`)") {
        if let Ok(new_c) =
            crate::renderer_engine::RendererConfig::from_file("assets/config/renderer.toml")
        {
            if let Ok(mut c) = renderer_config.write() {
                *c = new_c;
            }
        }
    }
    ui.same_line();
    if ui.button("[RESET RENDERER DEFAULTS]") {
        if let Ok(mut c) = renderer_config.write() {
            *c = crate::renderer_engine::RendererConfig::default();
        }
        tonemapping_comparison_mode.store(false, Ordering::Relaxed);
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(
        [0.4, 0.8, 1.0, 1.0],
        "=== GRAPHICAL ELEMENTS VISIBILITY ===",
    );
    ui.same_line();
    if ui.small_button("Reset Visibility Defaults") {
        if let Ok(mut c) = renderer_config.write() {
            c.render_rockets = true;
            c.render_smoke = true;
            c.render_trails = true;
            c.render_explosions = true;
        }
    }

    if let Ok(mut cfg) = renderer_config.write() {
        ui.checkbox(
            "Render Rockets (`renderer.rockets.enable`)",
            &mut cfg.render_rockets,
        );
        ui.checkbox(
            "Render Smoke Trails (`renderer.smoke.enable`)",
            &mut cfg.render_smoke,
        );
        ui.checkbox(
            "Render Rocket Trails (`renderer.trails.enable`)",
            &mut cfg.render_trails,
        );
        ui.checkbox(
            "Render Explosions (`renderer.explosions.enable`)",
            &mut cfg.render_explosions,
        );
    }

    ui.spacing();
    ui.separator();

    // Use a responsive item width (45% of content region, capped 200px..360px) so labels have plenty of space on the right
    let item_width = (ui.content_region_avail()[0] * 0.45).clamp(200.0, 360.0);
    let _item_w_token = ui.push_item_width(item_width);

    ui.text_colored(
        [0.4, 0.8, 1.0, 1.0],
        "=== TONEMAPPING (`renderer.tonemapping`) ===",
    );
    ui.same_line();
    if ui.small_button("Reset Tonemapping") {
        if let Ok(mut c) = renderer_config.write() {
            c.tone_mapping_mode = crate::renderer_engine::config::ToneMappingMode::KhronosPBR;
        }
        tonemapping_comparison_mode.store(false, Ordering::Relaxed);
    }

    // Current Tonemapping mode
    if let Ok(mut cfg) = renderer_config.write() {
        let modes = [
            (
                "Reinhard",
                crate::renderer_engine::config::ToneMappingMode::Reinhard,
            ),
            (
                "Reinhard Extended",
                crate::renderer_engine::config::ToneMappingMode::ReinhardExtended,
            ),
            (
                "ACES",
                crate::renderer_engine::config::ToneMappingMode::ACES,
            ),
            (
                "Uncharted 2",
                crate::renderer_engine::config::ToneMappingMode::Uncharted2,
            ),
            ("AgX", crate::renderer_engine::config::ToneMappingMode::AgX),
            (
                "Khronos PBR",
                crate::renderer_engine::config::ToneMappingMode::KhronosPBR,
            ),
        ];

        let current_idx = modes
            .iter()
            .position(|(_, m)| *m == cfg.tone_mapping_mode)
            .unwrap_or(0);

        let mut selected = current_idx;
        let mode_names: Vec<&str> = modes.iter().map(|(n, _)| *n).collect();

        if ui.combo_simple_string("Tone Mapping Mode", &mut selected, &mode_names) {
            cfg.tone_mapping_mode = modes[selected].1;
        }
    }

    // Comparison mode checkbox
    let mut comparison_active = tonemapping_comparison_mode.load(Ordering::Relaxed);
    if ui.checkbox(
        "Grid Comparison Mode (`renderer.tonemapping.compare`)",
        &mut comparison_active,
    ) {
        tonemapping_comparison_mode.store(comparison_active, Ordering::Relaxed);
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(
        [0.4, 0.8, 1.0, 1.0],
        "=== BLOOM PIPELINE (`renderer.bloom.*`) ===",
    );
    ui.same_line();
    if ui.small_button("Reset Bloom Defaults") {
        if let Ok(mut c) = renderer_config.write() {
            c.bloom_enabled = true;
            c.bloom_intensity = 1.5;
            c.bloom_iterations = 3;
            c.bloom_downsample = 2;
            c.bloom_blur_method = crate::renderer_engine::config::BlurMethod::Gaussian;
        }
    }

    if let Ok(mut cfg) = renderer_config.write() {
        // Enable / Disable Bloom
        ui.checkbox(
            "Enable Bloom (`renderer.bloom.enable` / `disable`)",
            &mut cfg.bloom_enabled,
        );

        // Bloom Intensity
        ui.slider(
            "Intensity (`renderer.bloom.intensity`)",
            0.0,
            10.0,
            &mut cfg.bloom_intensity,
        );

        // Bloom Iterations
        let mut iter = cfg.bloom_iterations as i32;
        if ui.slider("Iterations (`renderer.bloom.iterations`)", 1, 10, &mut iter) {
            cfg.bloom_iterations = iter.max(1) as u32;
        }

        // Downsample Ratio
        let downsample_options = [1u32, 2u32, 4u32];
        let cur_down_idx = downsample_options
            .iter()
            .position(|&v| v == cfg.bloom_downsample)
            .unwrap_or(0);
        let mut sel_down = cur_down_idx;
        let down_labels = ["1x (Native)", "2x (Half)", "4x (Quarter)"];
        if ui.combo_simple_string(
            "Downsample (`renderer.bloom.downsample`)",
            &mut sel_down,
            &down_labels,
        ) {
            cfg.bloom_downsample = downsample_options[sel_down];
        }

        // Blur Method
        let methods = [
            (
                "Gaussian (Classic 5-tap dual)",
                crate::renderer_engine::config::BlurMethod::Gaussian,
            ),
            (
                "Kawase (Optimized pyramid)",
                crate::renderer_engine::config::BlurMethod::Kawase,
            ),
        ];
        let cur_method_idx = methods
            .iter()
            .position(|(_, m)| *m == cfg.bloom_blur_method)
            .unwrap_or(0);
        let mut sel_method = cur_method_idx;
        let method_labels: Vec<&str> = methods.iter().map(|(n, _)| *n).collect();
        if ui.combo_simple_string(
            "Blur Method (`renderer.bloom.method`)",
            &mut sel_method,
            &method_labels,
        ) {
            cfg.bloom_blur_method = methods[sel_method].1;
        }
    }
}

pub fn render_commands_overview_tab<A: AudioEngine, P: PhysicEngineFull>(
    ui: &Ui,
    filter: &str,
    commands_registry: &CommandRegistry,
    audio_engine: &A,
    physic_engine: &P,
) {
    ui.spacing();
    ui.text_colored(
        [0.4, 0.8, 1.0, 1.0],
        "=== ALL REGISTERED CONSOLE COMMANDS ===",
    );
    ui.text("Reference list of commands across Audio, Physics, and Renderer engines:");

    let mut commands = commands_registry.get_commands();
    commands.sort();

    ui.child_window("CommandsChild")
        .size([0.0, 380.0])
        .build(|| {
            for cmd in commands {
                if !filter.is_empty() && !cmd.contains(filter) {
                    continue;
                }

                let val = commands_registry
                    .get_current_value(&cmd, audio_engine, physic_engine)
                    .unwrap_or_else(|| "N/A".to_string());

                let hint = commands_registry
                    .get_hint(&cmd)
                    .cloned()
                    .unwrap_or_default();

                ui.text_colored([0.2, 1.0, 0.6, 1.0], &cmd);
                ui.same_line();
                ui.text_colored([0.8, 0.8, 0.8, 1.0], format!("= {}", val));

                if !hint.is_empty() {
                    ui.same_line();
                    ui.text_colored([0.5, 0.5, 0.5, 1.0], format!("({})", hint));
                }
            }
        });
}

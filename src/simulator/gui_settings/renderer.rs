use super::theme::{COLOR_COMMAND_NAME, COLOR_HEADER, COLOR_TEXT_HINT, COLOR_TEXT_MUTED};
use crate::audio_engine::AudioEngine;
use crate::domain_contracts::{EngineCommand, RendererCommand, RendererStateReader};
use crate::physic_engine::PhysicEngineFull;
use crate::renderer_engine::constants as renderer_constants;
use crate::utils::command_console::CommandRegistry;
use imgui::Ui;
use std::sync::atomic::{AtomicBool, Ordering};

pub fn render_renderer_settings_tab(
    ui: &Ui,
    _filter: &str,
    state: &impl RendererStateReader,
    cmd_queue: &mut Vec<EngineCommand>,
    reload_shaders_requested: &AtomicBool,
    tonemapping_comparison_mode: &AtomicBool,
) {
    let cfg = state.config();

    // GUI_PERSIST: renderer.config
    ui.spacing();
    ui.text_colored(COLOR_HEADER, "=== SHADER & CONFIG ACTIONS ===");

    if ui.button("[RELOAD] Reload Shaders (`renderer.reload_shaders`)") {
        reload_shaders_requested.store(true, Ordering::Relaxed);
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::ReloadShaders));
    }
    ui.same_line();
    if ui.button("[SAVE] Save Config (`renderer.config.save`)") {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::SaveConfig));
    }
    ui.same_line();
    if ui.button("[RELOAD] Reload Disk Config (`renderer.config.reload`)") {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::ReloadConfig));
    }
    ui.same_line();
    if ui.button("[RESET RENDERER DEFAULTS]") {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::ResetDefaults));
        tonemapping_comparison_mode.store(false, Ordering::Relaxed);
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(COLOR_HEADER, "=== GRAPHICAL ELEMENTS VISIBILITY ===");
    ui.same_line();
    if ui.small_button("Reset Visibility Defaults") {
        cmd_queue.push(EngineCommand::Renderer(
            RendererCommand::ResetVisibilityDefaults,
        ));
    }

    let mut render_rockets = cfg.render_rockets;
    if ui.checkbox(
        "Render Rockets (`renderer.rockets.enable`)",
        &mut render_rockets,
    ) {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::SetRenderRockets(
            render_rockets,
        )));
    }

    let mut render_smoke = cfg.render_smoke;
    if ui.checkbox(
        "Render Smoke Trails (`renderer.smoke.enable`)",
        &mut render_smoke,
    ) {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::SetRenderSmoke(
            render_smoke,
        )));
    }

    let mut render_trails = cfg.render_trails;
    if ui.checkbox(
        "Render Rocket Trails (`renderer.trails.enable`)",
        &mut render_trails,
    ) {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::SetRenderTrails(
            render_trails,
        )));
    }

    let mut render_explosions = cfg.render_explosions;
    if ui.checkbox(
        "Render Explosions (`renderer.explosions.enable`)",
        &mut render_explosions,
    ) {
        cmd_queue.push(EngineCommand::Renderer(
            RendererCommand::SetRenderExplosions(render_explosions),
        ));
    }

    ui.spacing();
    ui.separator();

    let font_sz = ui.current_font_size();
    let item_width = (ui.content_region_avail()[0] * 0.45).clamp(font_sz * 14.0, font_sz * 26.0);
    let _item_w_token = ui.push_item_width(item_width);

    ui.text_colored(COLOR_HEADER, "=== TONEMAPPING (`renderer.tonemapping`) ===");
    ui.same_line();
    if ui.small_button("Reset Tonemapping") {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::ResetTonemapping));
        tonemapping_comparison_mode.store(false, Ordering::Relaxed);
    }

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
        cmd_queue.push(EngineCommand::Renderer(
            RendererCommand::SetToneMappingMode(modes[selected].1),
        ));
    }

    let mut comparison_active = tonemapping_comparison_mode.load(Ordering::Relaxed);
    if ui.checkbox(
        "Grid Comparison Mode (`renderer.tonemapping.compare`)",
        &mut comparison_active,
    ) {
        tonemapping_comparison_mode.store(comparison_active, Ordering::Relaxed);
        cmd_queue.push(EngineCommand::Renderer(
            RendererCommand::SetTonemappingComparisonMode(comparison_active),
        ));
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(COLOR_HEADER, "=== BLOOM PIPELINE (`renderer.bloom.*`) ===");
    ui.same_line();
    if ui.small_button("Reset Bloom Defaults") {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::ResetBloomDefaults));
    }

    let mut bloom_enabled = cfg.bloom_enabled;
    if ui.checkbox(
        "Enable Bloom (`renderer.bloom.enable` / `disable`)",
        &mut bloom_enabled,
    ) {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::SetBloomEnabled(
            bloom_enabled,
        )));
    }

    let mut bloom_intensity = cfg.bloom_intensity;
    if ui.slider(
        "Intensity (`renderer.bloom.intensity`)",
        renderer_constants::SLIDER_BLOOM_INTENSITY_MIN,
        renderer_constants::SLIDER_BLOOM_INTENSITY_MAX,
        &mut bloom_intensity,
    ) {
        cmd_queue.push(EngineCommand::Renderer(RendererCommand::SetBloomIntensity(
            bloom_intensity,
        )));
    }

    let mut iter = cfg.bloom_iterations as i32;
    if ui.slider(
        "Iterations (`renderer.bloom.iterations`)",
        renderer_constants::SLIDER_BLOOM_ITERATIONS_MIN as i32,
        renderer_constants::SLIDER_BLOOM_ITERATIONS_MAX as i32,
        &mut iter,
    ) {
        cmd_queue.push(EngineCommand::Renderer(
            RendererCommand::SetBloomIterations(
                iter.max(renderer_constants::SLIDER_BLOOM_ITERATIONS_MIN as i32) as u32,
            ),
        ));
    }

    let cur_down_idx = renderer_constants::BLOOM_DOWNSAMPLE_OPTIONS
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
        cmd_queue.push(EngineCommand::Renderer(
            RendererCommand::SetBloomDownsample(
                renderer_constants::BLOOM_DOWNSAMPLE_OPTIONS[sel_down],
            ),
        ));
    }

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
        cmd_queue.push(EngineCommand::Renderer(
            RendererCommand::SetBloomBlurMethod(methods[sel_method].1),
        ));
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
    ui.text_colored(COLOR_HEADER, "=== ALL REGISTERED CONSOLE COMMANDS ===");
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

                ui.text_colored(COLOR_COMMAND_NAME, &cmd);
                ui.same_line();
                ui.text_colored(COLOR_TEXT_MUTED, format!("= {}", val));

                if !hint.is_empty() {
                    ui.same_line();
                    ui.text_colored(COLOR_TEXT_HINT, format!("({})", hint));
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer_engine::config::RendererConfig;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_render_renderer_settings_tab_pure_function() {
        let config = RendererConfig::default();
        let mut cmd_queue: Vec<EngineCommand> = Vec::with_capacity(16);
        let reload_shaders = AtomicBool::new(false);
        let compare_mode = AtomicBool::new(false);

        let _guard = crate::simulator::gui_settings::IMGUI_TEST_MUTEX
            .lock()
            .unwrap();
        let mut imgui_ctx = imgui::Context::create();
        imgui_ctx.set_ini_filename(None);
        imgui_ctx.fonts().build_rgba32_texture();
        imgui_ctx.io_mut().display_size = [800.0, 600.0];

        let ui = imgui_ctx.frame();

        render_renderer_settings_tab(
            ui,
            "",
            &config,
            &mut cmd_queue,
            &reload_shaders,
            &compare_mode,
        );

        assert!(cmd_queue.capacity() >= 16);
    }
}

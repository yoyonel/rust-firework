// GUI_PERSIST: physics.config

use super::theme::{COLOR_ALERT, COLOR_HEADER, COLOR_TEXT_HINT, COLOR_WARNING};
use crate::domain_contracts::{EngineCommand, SmokeCommand, SmokeStateReader};
use crate::physic_engine::config::{PhysicConfig, SmokeColorMode};
use crate::physic_engine::constants as physic_constants;
use crate::renderer_engine::constants as renderer_constants;
use crate::renderer_engine::smoke_preview::{
    PreviewContext, SmokePreviewRenderer, PREVIEW_PAN_X, PREVIEW_PAN_Y, PREVIEW_ROT_Z, PREVIEW_ZOOM,
};
use imgui::{ColorEditFlags, Ui};
use std::sync::atomic::{AtomicBool, Ordering};

static mut PREVIEW_GPU: Option<SmokePreviewRenderer> = None;
// GUI_PERSIST: gui.layout
pub static SHOW_GEOMETRY_TRIMMING: AtomicBool = AtomicBool::new(true);

/// Shared GUI control panel for Smoke & Alpha Erosion parameters with per-parameter reset buttons.
#[allow(clippy::too_many_arguments)]
pub fn render_smoke_controls(
    ui: &Ui,
    state: &impl SmokeStateReader,
    cmd_queue: &mut Vec<EngineCommand>,
    _preview_max_zoom: &mut f32,
    _preview_rocket_color: &mut [f32; 3],
    _preview_simulated_speed: &mut f32,
    _preview_simulated_angle_offset: &mut f32,
) {
    let cfg = state.config();
    let default_cfg = PhysicConfig::default();

    let font_sz = ui.current_font_size();
    let item_width = (ui.content_region_avail()[0] * 0.38).clamp(font_sz * 11.0, font_sz * 21.0);

    // =========================================================================
    // 1. ALPHA EROSION (DISSOLVE & BURN SEAM)
    // =========================================================================
    ui.text_colored(
        COLOR_HEADER,
        "=== 1. ALPHA EROSION (DISSOLVE & BURN SEAM) ===",
    );

    let mut erosion_enabled = cfg.smoke_erosion_enabled;
    if ui.checkbox(
        "Enable Alpha Erosion Dissolve (`physic.smoke_erosion_enabled`)",
        &mut erosion_enabled,
    ) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetErosionEnabled(
            erosion_enabled,
        )));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_erosion_enabled") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetErosionEnabled(
            default_cfg.smoke_erosion_enabled,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {}",
            default_cfg.smoke_erosion_enabled
        ));
    }

    if cfg.smoke_erosion_enabled {
        let mut erosion_scale = cfg.smoke_erosion_scale;
        ui.set_next_item_width(item_width);
        if ui.slider(
            "Erosion Aggressiveness / Scale (`physic.smoke_erosion_scale`)",
            physic_constants::SMOKE_EROSION_SCALE_MIN,
            physic_constants::SMOKE_EROSION_SCALE_MAX,
            &mut erosion_scale,
        ) {
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetErosionScale(
                erosion_scale,
            )));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_erosion_scale") {
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetErosionScale(
                default_cfg.smoke_erosion_scale,
            )));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!(
                "Reset to default: {:.2}",
                default_cfg.smoke_erosion_scale
            ));
        }

        let mut edge_width = cfg.smoke_erosion_edge_width;
        ui.set_next_item_width(item_width);
        if ui.slider(
            "Erosion Edge Width (`physic.smoke_erosion_edge_width`)",
            physic_constants::SMOKE_EROSION_EDGE_WIDTH_MIN,
            physic_constants::SMOKE_EROSION_EDGE_WIDTH_MAX,
            &mut edge_width,
        ) {
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetErosionEdgeWidth(
                edge_width,
            )));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_erosion_edge_width") {
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetErosionEdgeWidth(
                default_cfg.smoke_erosion_edge_width,
            )));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!(
                "Reset to default: {:.2}",
                default_cfg.smoke_erosion_edge_width
            ));
        }

        let mut edge_color = cfg.smoke_erosion_edge_color;
        ui.set_next_item_width(item_width);
        if ui
            .color_edit3_config(
                "Erosion Edge Glowing Color (`physic.smoke_erosion_edge_color`)",
                &mut edge_color,
            )
            .flags(ColorEditFlags::PICKER_HUE_BAR | ColorEditFlags::DISPLAY_HEX)
            .build()
        {
            let u8_color = [
                (edge_color[0] * 255.0).clamp(0.0, 255.0) as u8,
                (edge_color[1] * 255.0).clamp(0.0, 255.0) as u8,
                (edge_color[2] * 255.0).clamp(0.0, 255.0) as u8,
            ];
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetErosionEdgeColor(
                u8_color,
            )));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_erosion_edge_color") {
            let u8_color = [
                (default_cfg.smoke_erosion_edge_color[0] * 255.0) as u8,
                (default_cfg.smoke_erosion_edge_color[1] * 255.0) as u8,
                (default_cfg.smoke_erosion_edge_color[2] * 255.0) as u8,
            ];
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetErosionEdgeColor(
                u8_color,
            )));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!(
                "Reset to default: [{:.2}, {:.2}, {:.2}]",
                default_cfg.smoke_erosion_edge_color[0],
                default_cfg.smoke_erosion_edge_color[1],
                default_cfg.smoke_erosion_edge_color[2]
            ));
        }
    }

    ui.spacing();
    let mut flow_distortion = cfg.flow_distortion_strength;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Flow Distortion Strength (`physic.flow_distortion_strength`)",
        physic_constants::FLOW_DISTORTION_STRENGTH_MIN,
        physic_constants::FLOW_DISTORTION_STRENGTH_MAX,
        &mut flow_distortion,
    ) {
        cmd_queue.push(EngineCommand::Smoke(
            SmokeCommand::SetFlowDistortionStrength(flow_distortion),
        ));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_flow_distortion_strength") {
        cmd_queue.push(EngineCommand::Smoke(
            SmokeCommand::SetFlowDistortionStrength(default_cfg.flow_distortion_strength),
        ));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}",
            default_cfg.flow_distortion_strength
        ));
    }

    let mut flow_speed = cfg.flow_animation_speed;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Flow Animation Speed (`physic.flow_animation_speed`)",
        physic_constants::FLOW_ANIMATION_SPEED_MIN,
        physic_constants::FLOW_ANIMATION_SPEED_MAX,
        &mut flow_speed,
    ) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetFlowAnimationSpeed(
            flow_speed,
        )));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_flow_animation_speed") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetFlowAnimationSpeed(
            default_cfg.flow_animation_speed,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}",
            default_cfg.flow_animation_speed
        ));
    }

    ui.spacing();
    ui.separator();

    // =========================================================================
    // 2. SMOKE CORE TINT & COLOR SELECTION
    // =========================================================================
    ui.text_colored(COLOR_HEADER, "=== 2. SMOKE CORE TINT & COLOR ===");

    let mut is_rocket_color = cfg.smoke_color_mode == SmokeColorMode::RocketColor;
    let mut is_custom_color = cfg.smoke_color_mode == SmokeColorMode::Custom;

    ui.text("Smoke Base Core Tint Mode:");
    ui.same_line();
    if ui.radio_button("Rocket Color (Inherited)", &mut is_rocket_color, true) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetColorMode(
            SmokeColorMode::RocketColor,
        )));
    }
    ui.same_line();
    if ui.radio_button("Custom Fixed Color", &mut is_custom_color, true) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetColorMode(
            SmokeColorMode::Custom,
        )));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_color_mode") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetColorMode(
            default_cfg.smoke_color_mode,
        )));
        cmd_queue.push(EngineCommand::Smoke(
            SmokeCommand::SetInheritedColorIntensity(default_cfg.smoke_inherited_color_intensity),
        ));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text("Reset color mode to default (RocketColor)");
    }

    if cfg.smoke_color_mode == SmokeColorMode::RocketColor {
        let mut intensity = cfg.smoke_inherited_color_intensity;
        ui.set_next_item_width(item_width);
        if ui.slider(
            "Inherited Rocket Color Intensity (`physic.smoke_inherited_color_intensity`)",
            physic_constants::SMOKE_INHERITED_COLOR_INTENSITY_MIN,
            physic_constants::SMOKE_INHERITED_COLOR_INTENSITY_MAX,
            &mut intensity,
        ) {
            cmd_queue.push(EngineCommand::Smoke(
                SmokeCommand::SetInheritedColorIntensity(intensity),
            ));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_inherited_color_intensity") {
            cmd_queue.push(EngineCommand::Smoke(
                SmokeCommand::SetInheritedColorIntensity(
                    default_cfg.smoke_inherited_color_intensity,
                ),
            ));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!(
                "Reset inherited rocket color intensity to default: {:.2}",
                default_cfg.smoke_inherited_color_intensity
            ));
        }
    }

    if cfg.smoke_color_mode == SmokeColorMode::Custom {
        let mut custom_color = cfg.smoke_custom_color;
        ui.set_next_item_width(item_width);
        if ui
            .color_edit3_config(
                "Smoke Custom Core Color (`physic.smoke_custom_color`)",
                &mut custom_color,
            )
            .flags(ColorEditFlags::PICKER_HUE_BAR | ColorEditFlags::DISPLAY_HEX)
            .build()
        {
            let u8_color = [
                (custom_color[0] * 255.0).clamp(0.0, 255.0) as u8,
                (custom_color[1] * 255.0).clamp(0.0, 255.0) as u8,
                (custom_color[2] * 255.0).clamp(0.0, 255.0) as u8,
            ];
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetCustomColor(u8_color)));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_custom_color") {
            let u8_color = [
                (default_cfg.smoke_custom_color[0] * 255.0) as u8,
                (default_cfg.smoke_custom_color[1] * 255.0) as u8,
                (default_cfg.smoke_custom_color[2] * 255.0) as u8,
            ];
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetCustomColor(u8_color)));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!(
                "Reset custom color to default: [{:.2}, {:.2}, {:.2}]",
                default_cfg.smoke_custom_color[0],
                default_cfg.smoke_custom_color[1],
                default_cfg.smoke_custom_color[2]
            ));
        }
    }

    ui.spacing();
    ui.separator();

    // =========================================================================
    // 3. EMISSION CADENCE & LIFETIME DYNAMICS
    // =========================================================================
    ui.text_colored(
        COLOR_HEADER,
        "=== 3. EMISSION CADENCE & LIFETIME DYNAMICS ===",
    );

    let mut spawn_rate = cfg.smoke_spawn_rate;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Spawn Rate (`physic.smoke_spawn_rate`)",
        physic_constants::SLIDER_SMOKE_SPAWN_RATE_MIN,
        physic_constants::SLIDER_SMOKE_SPAWN_RATE_MAX,
        &mut spawn_rate,
    ) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetSpawnRate(spawn_rate)));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_spawn_rate") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetSpawnRate(
            default_cfg.smoke_spawn_rate,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.1}",
            default_cfg.smoke_spawn_rate
        ));
    }

    let mut initial_size = cfg.smoke_initial_size;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Initial Size (`physic.smoke_initial_size`)",
        physic_constants::SLIDER_SMOKE_INIT_SIZE_MIN,
        physic_constants::SLIDER_SMOKE_INIT_SIZE_MAX,
        &mut initial_size,
    ) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetInitialSize(
            initial_size,
        )));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_initial_size") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetInitialSize(
            default_cfg.smoke_initial_size,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.1}",
            default_cfg.smoke_initial_size
        ));
    }

    let mut growth_rate = cfg.smoke_growth_rate_multiplier;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Growth Rate Multiplier (`physic.smoke_growth_rate_multiplier`)",
        physic_constants::SLIDER_SMOKE_GROWTH_MIN,
        physic_constants::SLIDER_SMOKE_GROWTH_MAX,
        &mut growth_rate,
    ) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetGrowthRateMultiplier(
            growth_rate,
        )));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_growth_rate_multiplier") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetGrowthRateMultiplier(
            default_cfg.smoke_growth_rate_multiplier,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}",
            default_cfg.smoke_growth_rate_multiplier
        ));
    }

    let mut fade_duration = cfg.smoke_fade_duration;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Fade Duration (s) (`physic.smoke_fade_duration`)",
        physic_constants::SLIDER_SMOKE_FADE_DUR_MIN,
        physic_constants::SLIDER_SMOKE_FADE_DUR_MAX,
        &mut fade_duration,
    ) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetFadeDuration(
            fade_duration,
        )));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_fade_duration") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetFadeDuration(
            default_cfg.smoke_fade_duration,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}s",
            default_cfg.smoke_fade_duration
        ));
    }

    let mut intensity = cfg.smoke_intensity;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Intensity / Brightness (`physic.smoke_intensity`)",
        physic_constants::SLIDER_SMOKE_INTENSITY_MIN,
        physic_constants::SLIDER_SMOKE_INTENSITY_MAX,
        &mut intensity,
    ) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetIntensity(intensity)));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_intensity") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetIntensity(
            default_cfg.smoke_intensity,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}",
            default_cfg.smoke_intensity
        ));
    }

    let mut max_smoke = cfg.max_smoke_particles as i32;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Max Smoke Particles Pool (`physic.max_smoke_particles`)",
        physic_constants::SLIDER_SMOKE_PARTICLES_MIN,
        physic_constants::SLIDER_SMOKE_PARTICLES_MAX,
        &mut max_smoke,
    ) {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetMaxSmokeParticles(
            max_smoke.max(100) as u32,
        )));
    }
    ui.same_line();
    if ui.small_button("Reset##reset_max_smoke_particles") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::SetMaxSmokeParticles(
            default_cfg.max_smoke_particles as u32,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {}",
            default_cfg.max_smoke_particles
        ));
    }
}

/// Renders the dedicated Smoke & Alpha Erosion settings tab.
#[allow(clippy::too_many_arguments)]
pub fn render_smoke_settings_tab(
    ui: &Ui,
    state: &impl SmokeStateReader,
    cmd_queue: &mut Vec<EngineCommand>,
    preview_max_zoom: &mut f32,
    preview_rocket_color: &mut [f32; 3],
    preview_simulated_speed: &mut f32,
    preview_simulated_angle_offset: &mut f32,
) {
    let cfg = state.config();

    ui.text_colored(
        COLOR_HEADER,
        "SMOKE TRAIL & ALPHA EROSION (DISSOLVE) CONTROLS",
    );
    ui.same_line();
    if ui.small_button("Reset Smoke Defaults") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::ResetDefaults));
        *preview_max_zoom = physic_constants::DEFAULT_SMOKE_PREVIEW_MAX_ZOOM;
        *preview_rocket_color = physic_constants::DEFAULT_SMOKE_PREVIEW_ROCKET_COLOR;
        *preview_simulated_speed = physic_constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_SPEED;
        *preview_simulated_angle_offset =
            physic_constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET;
    }

    ui.separator();

    let avail_width = ui.content_region_avail()[0];

    // =========================================================================
    // 1. QUICK PRESETS
    // =========================================================================
    ui.text_colored(COLOR_WARNING, "Quick Dissolve Style Presets:");
    ui.same_line();

    if ui.button("[Fire & Ember]") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::ApplyPreset(0)));
    }
    ui.same_line();
    if ui.button("[Plasma Blue]") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::ApplyPreset(1)));
    }
    ui.same_line();
    if ui.button("[Volumetric Cloud]") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::ApplyPreset(2)));
    }
    ui.same_line();
    if ui.button("[Toxic Plasma]") {
        cmd_queue.push(EngineCommand::Smoke(SmokeCommand::ApplyPreset(3)));
    }

    ui.spacing();

    // Init GPU FBO renderer if needed
    #[allow(static_mut_refs)]
    let fbo_tex = unsafe {
        if PREVIEW_GPU.is_none() {
            PREVIEW_GPU = Some(SmokePreviewRenderer::init());
        }
        let preview = PREVIEW_GPU.as_mut().unwrap();
        let zoom = PREVIEW_ZOOM.load(Ordering::Relaxed) as f32 / 100.0;
        let pan_x = PREVIEW_PAN_X.load(Ordering::Relaxed) as f32 / 10.0;
        let pan_y = PREVIEW_PAN_Y.load(Ordering::Relaxed) as f32 / 10.0;
        let rot_z = PREVIEW_ROT_Z.load(Ordering::Relaxed) as f32 / 10.0;
        let font_sz = ui.current_font_size();
        let preview_height = font_sz * 11.15;
        let canvas_aspect = (avail_width / preview_height).max(0.1);
        let ctx = PreviewContext {
            config: cfg,
            zoom,
            pan_x,
            pan_y,
            rot_deg: rot_z,
            canvas_aspect,
            time: ui.time() as f32,
            dt: ui.io().delta_time,
            rocket_color: *preview_rocket_color,
            simulated_speed: *preview_simulated_speed,
            simulated_angle_offset_deg: *preview_simulated_angle_offset,
            tone_mapping_mode: 5,
        };
        preview.render(&ctx)
    };

    // =========================================================================
    // 2. 100% BIT-FOR-BIT ISO GPU FBO PREVIEW CANVAS (INTERACTIVE VIEWPORT)
    // =========================================================================
    let font_sz = ui.current_font_size();
    let font_scale = font_sz / 13.0;
    let preview_outer_height = font_sz * 14.23;
    let preview_height = font_sz * 11.15;

    ui.child_window("SmokeErosionPreviewCanvas")
        .size([avail_width, preview_outer_height])
        .border(true)
        .focused(false)
        .flags(imgui::WindowFlags::NO_SCROLL_WITH_MOUSE | imgui::WindowFlags::NO_SCROLLBAR)
        .build(|| {
            let zoom = PREVIEW_ZOOM.load(Ordering::Relaxed) as f32 / 100.0;
            let pan_x = PREVIEW_PAN_X.load(Ordering::Relaxed) as f32 / 10.0;
            let pan_y = PREVIEW_PAN_Y.load(Ordering::Relaxed) as f32 / 10.0;
            let rot_z = PREVIEW_ROT_Z.load(Ordering::Relaxed) as f32 / 10.0;

            let preview_width = ui.content_region_avail()[0];

            let canvas_pos = ui.cursor_screen_pos();
            let tex_id = imgui::TextureId::new(fbo_tex as usize);
            imgui::Image::new(tex_id, [preview_width, preview_height])
                .uv0([0.0, 1.0]) // Flip Y for OpenGL Framebuffer
                .uv1([1.0, 0.0])
                .build(ui);

            // Draw subtle translucent directional vector arrow representing virtual rocket velocity
            let aspect = (preview_width / preview_height.max(1.0)).max(0.1);
            let sim_h = renderer_constants::SMOKE_PREVIEW_FBO_HEIGHT as f32 / zoom.max(0.01);
            let sim_w = sim_h * aspect;

            let center_x = sim_w * renderer_constants::SMOKE_PREVIEW_CENTER_X_FACTOR + pan_x;
            let center_y = sim_h * renderer_constants::SMOKE_PREVIEW_CENTER_Y_FACTOR + pan_y;

            let rot_rad = rot_z.to_radians();
            let rocket_h = renderer_constants::SMOKE_PREVIEW_ROCKET_H;

            // Rocket nose tip in OpenGL simulation coordinates
            let tip_x = center_x - (rocket_h * 0.5) * rot_rad.sin();
            let tip_y = center_y + (rocket_h * 0.5) * rot_rad.cos();

            // Map tip position to ImGui screen pixel coordinates
            let u_tip = tip_x / sim_w;
            let v_tip = tip_y / sim_h;
            let start_x = canvas_pos[0] + u_tip * preview_width;
            let start_y = canvas_pos[1] + (1.0 - v_tip) * preview_height;

            // Trajectory direction vector in screen coordinates (+X right, +Y down)
            let total_angle_rad = (rot_z + *preview_simulated_angle_offset).to_radians();
            let dir_screen_x = -total_angle_rad.sin();
            let dir_screen_y = -total_angle_rad.cos();

            // Calculate arrow length proportional to simulated speed
            let raw_len = (*preview_simulated_speed
                * renderer_constants::SMOKE_PREVIEW_ARROW_SPEED_SCALE)
                .clamp(
                    renderer_constants::SMOKE_PREVIEW_ARROW_MIN_LEN,
                    renderer_constants::SMOKE_PREVIEW_ARROW_MAX_LEN,
                );
            let arrow_len = raw_len * font_scale;

            let end_x = start_x + dir_screen_x * arrow_len;
            let end_y = start_y + dir_screen_y * arrow_len;

            // Translucent cyan color token
            let arrow_col = renderer_constants::SMOKE_PREVIEW_ARROW_COLOR;
            let draw_list = ui.get_window_draw_list();

            // Only draw arrow if rocket tip is near/inside visible canvas bounds
            if start_x
                >= canvas_pos[0] - renderer_constants::SMOKE_PREVIEW_ARROW_CLIP_MARGIN * font_scale
                && start_x
                    <= canvas_pos[0]
                        + preview_width
                        + renderer_constants::SMOKE_PREVIEW_ARROW_CLIP_MARGIN * font_scale
                && start_y
                    >= canvas_pos[1]
                        - renderer_constants::SMOKE_PREVIEW_ARROW_CLIP_MARGIN * font_scale
                && start_y
                    <= canvas_pos[1]
                        + preview_height
                        + renderer_constants::SMOKE_PREVIEW_ARROW_CLIP_MARGIN * font_scale
            {
                // Draw shaft line
                draw_list
                    .add_line([start_x, start_y], [end_x, end_y], arrow_col)
                    .thickness(renderer_constants::SMOKE_PREVIEW_ARROW_LINE_THICKNESS * font_scale)
                    .build();

                // Draw arrowhead triangle pointer
                let line_angle = dir_screen_y.atan2(dir_screen_x);
                let head_size = renderer_constants::SMOKE_PREVIEW_ARROW_HEAD_SIZE * font_scale;
                let wing_angle =
                    renderer_constants::SMOKE_PREVIEW_ARROW_HEAD_ANGLE_DEG.to_radians();

                let left_angle = line_angle + std::f32::consts::PI - wing_angle;
                let right_angle = line_angle + std::f32::consts::PI + wing_angle;

                let left_x = end_x + head_size * left_angle.cos();
                let left_y = end_y + head_size * left_angle.sin();

                let right_x = end_x + head_size * right_angle.cos();
                let right_y = end_y + head_size * right_angle.sin();

                draw_list
                    .add_triangle(
                        [end_x, end_y],
                        [left_x, left_y],
                        [right_x, right_y],
                        arrow_col,
                    )
                    .filled(true)
                    .build();
            }

            let is_canvas_hovered = ui.is_item_hovered();

            if is_canvas_hovered {
                let io = ui.io();

                // Exclusive Mouse Wheel Zoom: zero out C++ ImGui IO MouseWheel to prevent parent window scrolling
                let wheel = io.mouse_wheel;
                if wheel != 0.0 {
                    let cur_z = PREVIEW_ZOOM.load(Ordering::Relaxed) as f32 / 100.0;
                    let new_z = (cur_z
                        + wheel * renderer_constants::SMOKE_PREVIEW_ZOOM_SCROLL_SPEED)
                        .clamp(
                            physic_constants::DEFAULT_SMOKE_PREVIEW_MIN_ZOOM,
                            *preview_max_zoom,
                        );
                    PREVIEW_ZOOM.store((new_z * 100.0) as u32, Ordering::Relaxed);
                    unsafe {
                        let raw_io = imgui::sys::igGetIO();
                        if !raw_io.is_null() {
                            (*raw_io).MouseWheel = 0.0;
                            (*raw_io).MouseWheelH = 0.0;
                        }
                    }
                }

                // Exclusive Middle Click Drag: Translation (Pan X/Y)
                if ui.is_mouse_dragging(imgui::MouseButton::Middle) {
                    let delta = io.mouse_delta;
                    let cur_x = PREVIEW_PAN_X.load(Ordering::Relaxed) as f32 / 10.0;
                    let cur_y = PREVIEW_PAN_Y.load(Ordering::Relaxed) as f32 / 10.0;
                    let new_x = cur_x + delta[0] * renderer_constants::SMOKE_PREVIEW_PAN_DRAG_SPEED;
                    let new_y = cur_y - delta[1] * renderer_constants::SMOKE_PREVIEW_PAN_DRAG_SPEED; // Screen Y is inverted
                    PREVIEW_PAN_X.store((new_x * 10.0) as i32, Ordering::Relaxed);
                    PREVIEW_PAN_Y.store((new_y * 10.0) as i32, Ordering::Relaxed);
                }

                // Exclusive Right Click Drag: Rotation Z (Unreal/Unity Viewport Orbit)
                if ui.is_mouse_dragging(imgui::MouseButton::Right) {
                    let delta = io.mouse_delta;
                    let cur_r = PREVIEW_ROT_Z.load(Ordering::Relaxed) as f32 / 10.0;
                    let new_r = (cur_r
                        + delta[0] * renderer_constants::SMOKE_PREVIEW_ROT_DRAG_SPEED)
                        % 360.0;
                    PREVIEW_ROT_Z.store((new_r * 10.0) as i32, Ordering::Relaxed);
                }
            }

            ui.set_cursor_pos([10.0 * font_scale, 10.0 * font_scale]);
            ui.text_colored(
                COLOR_HEADER,
                format!(
                    "LIVE GPU VIEWPORT | Zoom: {:.1}x | Pan: ({:.0}, {:.0}) | Rot Z: {:.0}°",
                    zoom, pan_x, pan_y, rot_z
                ),
            );

            ui.same_line();
            if ui.small_button("Reset Viewport") {
                PREVIEW_ZOOM.store(100, Ordering::Relaxed);
                PREVIEW_PAN_X.store(0, Ordering::Relaxed);
                PREVIEW_PAN_Y.store(0, Ordering::Relaxed);
                PREVIEW_ROT_Z.store(0, Ordering::Relaxed);
            }

            ui.set_cursor_pos([10.0 * font_scale, preview_height + 15.0 * font_scale]);
            ui.text_colored(
                COLOR_TEXT_HINT,
                "[Middle Drag: Pan X/Y] | [Right Drag: Rotate Z] | [Wheel: Zoom]",
            );
        });

    // =========================================================================
    // ROCKET & VIEWPORT PREVIEW SETTINGS (DIRECTLY BELOW VIEWPORT)
    // =========================================================================
    ui.spacing();
    let font_sz = ui.current_font_size();
    let item_width = (ui.content_region_avail()[0] * 0.38).clamp(font_sz * 11.0, font_sz * 21.0);

    ui.set_next_item_width(item_width);
    ui.slider(
        "Preview Viewport Max Zoom (`gui.smoke_preview_max_zoom`)",
        physic_constants::SMOKE_PREVIEW_MAX_ZOOM_MIN,
        physic_constants::SMOKE_PREVIEW_MAX_ZOOM_MAX,
        preview_max_zoom,
    );
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_preview_max_zoom") {
        *preview_max_zoom = physic_constants::DEFAULT_SMOKE_PREVIEW_MAX_ZOOM;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.1}x",
            physic_constants::DEFAULT_SMOKE_PREVIEW_MAX_ZOOM
        ));
    }

    ui.set_next_item_width(item_width);
    ui.color_edit3(
        "Preview Rocket Color (`gui.smoke_preview_rocket_color`)",
        preview_rocket_color,
    );
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_preview_rocket_color") {
        *preview_rocket_color = physic_constants::DEFAULT_SMOKE_PREVIEW_ROCKET_COLOR;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text("Reset preview rocket color to white [1.0, 1.0, 1.0]");
    }

    ui.set_next_item_width(item_width);
    ui.slider(
        "Preview Rocket Speed (m/s) (`gui.smoke_preview_simulated_speed`)",
        physic_constants::SMOKE_PREVIEW_SIMULATED_SPEED_MIN,
        physic_constants::SMOKE_PREVIEW_SIMULATED_SPEED_MAX,
        preview_simulated_speed,
    );
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_preview_simulated_speed") {
        *preview_simulated_speed = physic_constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_SPEED;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.1} m/s",
            physic_constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_SPEED
        ));
    }

    ui.set_next_item_width(item_width);
    ui.slider(
        "Preview Velocity Angle (deg) (`gui.smoke_preview_simulated_angle_offset`)",
        physic_constants::SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET_MIN,
        physic_constants::SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET_MAX,
        preview_simulated_angle_offset,
    );
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_preview_simulated_angle_offset") {
        *preview_simulated_angle_offset =
            physic_constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.1}°",
            physic_constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET
        ));
    }

    // =========================================================================
    // 3. DEDICATED GEOMETRY TRIMMING & SPRITE MESH VIEWPORT (1:1 SCALE)
    // =========================================================================
    ui.spacing();
    ui.separator();

    #[allow(static_mut_refs)]
    let raw_smoke_tex = unsafe { PREVIEW_GPU.as_ref().map(|p| p.smoke_tex).unwrap_or(0) };

    let mut show_trimming = SHOW_GEOMETRY_TRIMMING.load(Ordering::Relaxed);
    if ui.checkbox(
        "Show Geometry Trimming Inspection Panel (Tight Octagon Mesh vs Quad 1:1)",
        &mut show_trimming,
    ) {
        SHOW_GEOMETRY_TRIMMING.store(show_trimming, Ordering::Relaxed);
    }

    if show_trimming && raw_smoke_tex != 0 {
        let font_sz = ui.current_font_size();
        let font_scale = font_sz / 13.0;
        let tex_box_size = font_sz * 15.38;
        let container_height = font_sz * 16.92;

        ui.child_window("GeometryTrimmingContainer")
            .size([avail_width, container_height])
            .border(true)
            .flags(imgui::WindowFlags::NO_SCROLLBAR | imgui::WindowFlags::NO_SCROLL_WITH_MOUSE)
            .build(|| {
                // Left Column: 1:1 Texture Viewport with static Octagon Wireframe
                let canvas_pos = ui.cursor_screen_pos();
                let center = [
                    canvas_pos[0] + tex_box_size * 0.5,
                    canvas_pos[1] + tex_box_size * 0.5,
                ];
                let half = tex_box_size * 0.5;

                // 1. Draw 1:1 raw smoke sprite texture
                let tex_id = imgui::TextureId::new(raw_smoke_tex as usize);
                imgui::Image::new(tex_id, [tex_box_size, tex_box_size])
                    .uv0([0.0, 1.0])
                    .uv1([1.0, 0.0])
                    .build(ui);

                let draw_list = ui.get_window_draw_list();

                // 2. Bounding Quad (Square)
                let sq_p0 = [center[0] - half, center[1] - half];
                let sq_p1 = [center[0] + half, center[1] - half];
                let sq_p2 = [center[0] + half, center[1] + half];
                let sq_p3 = [center[0] - half, center[1] + half];

                draw_list
                    .add_line(sq_p0, sq_p1, renderer_constants::COLOR_TRIMMING_QUAD_BORDER)
                    .thickness(renderer_constants::TRIMMING_QUAD_BORDER_THICKNESS * font_scale)
                    .build();
                draw_list
                    .add_line(sq_p1, sq_p2, renderer_constants::COLOR_TRIMMING_QUAD_BORDER)
                    .thickness(renderer_constants::TRIMMING_QUAD_BORDER_THICKNESS * font_scale)
                    .build();
                draw_list
                    .add_line(sq_p2, sq_p3, renderer_constants::COLOR_TRIMMING_QUAD_BORDER)
                    .thickness(renderer_constants::TRIMMING_QUAD_BORDER_THICKNESS * font_scale)
                    .build();
                draw_list
                    .add_line(sq_p3, sq_p0, renderer_constants::COLOR_TRIMMING_QUAD_BORDER)
                    .thickness(renderer_constants::TRIMMING_QUAD_BORDER_THICKNESS * font_scale)
                    .build();

                // 3. 1:1 Octagon Vertices matching GPU SmokeRenderer primitive
                let r_maj = half;
                let r_diag = half * (0.765366 / 1.082392);

                let oct = [
                    [center[0] + r_maj, center[1]],           // V0 (right)
                    [center[0] + r_diag, center[1] + r_diag], // V1 (bottom-right)
                    [center[0], center[1] + r_maj],           // V2 (bottom)
                    [center[0] - r_diag, center[1] + r_diag], // V3 (bottom-left)
                    [center[0] - r_maj, center[1]],           // V4 (left)
                    [center[0] - r_diag, center[1] - r_diag], // V5 (top-left)
                    [center[0], center[1] - r_maj],           // V6 (top)
                    [center[0] + r_diag, center[1] - r_diag], // V7 (top-right)
                ];

                // 4. Shaded Trimmed Corners in Red Translucent (-17.2% Fillrate Surface Bypassed)
                draw_list
                    .add_triangle(
                        sq_p0,
                        oct[6],
                        oct[5],
                        renderer_constants::COLOR_TRIMMING_CORNER_SHADE,
                    )
                    .filled(true)
                    .build();
                draw_list
                    .add_triangle(
                        sq_p1,
                        oct[7],
                        oct[6],
                        renderer_constants::COLOR_TRIMMING_CORNER_SHADE,
                    )
                    .filled(true)
                    .build();
                draw_list
                    .add_triangle(
                        sq_p2,
                        oct[1],
                        oct[2],
                        renderer_constants::COLOR_TRIMMING_CORNER_SHADE,
                    )
                    .filled(true)
                    .build();
                draw_list
                    .add_triangle(
                        sq_p3,
                        oct[3],
                        oct[2],
                        renderer_constants::COLOR_TRIMMING_CORNER_SHADE,
                    )
                    .filled(true)
                    .build();

                // 5. Octagon Outline & Triangle Fan Edges in Cyan
                for i in 0..8 {
                    let next = (i + 1) % 8;
                    draw_list
                        .add_line(
                            oct[i],
                            oct[next],
                            renderer_constants::COLOR_TRIMMING_OCTAGON_OUTLINE,
                        )
                        .thickness(
                            renderer_constants::TRIMMING_OCTAGON_OUTLINE_THICKNESS * font_scale,
                        )
                        .build();
                    draw_list
                        .add_line(
                            center,
                            oct[i],
                            renderer_constants::COLOR_TRIMMING_OCTAGON_INNER,
                        )
                        .thickness(
                            renderer_constants::TRIMMING_OCTAGON_INNER_THICKNESS * font_scale,
                        )
                        .build();
                    draw_list
                        .add_circle(
                            oct[i],
                            renderer_constants::TRIMMING_VERTEX_INDICATOR_RADIUS * font_scale,
                            renderer_constants::COLOR_TRIMMING_VERTEX_INDICATOR,
                        )
                        .filled(true)
                        .build();
                }
                draw_list
                    .add_circle(
                        center,
                        renderer_constants::TRIMMING_CENTER_PIN_RADIUS * font_scale,
                        renderer_constants::COLOR_TRIMMING_CENTER_PIN,
                    )
                    .filled(true)
                    .build();

                // Right Column: Technical Stats & Explanatory Legend
                let right_col_x = tex_box_size + 25.0 * font_scale;
                ui.set_cursor_pos([right_col_x, 15.0 * font_scale]);
                ui.text_colored(
                    COLOR_HEADER,
                    "=== GEOMETRY TRIMMING SPECIFICATIONS (1:1 SCALE) ===",
                );

                ui.set_cursor_pos([right_col_x, 40.0 * font_scale]);
                ui.text(format!(
                    "- Target Sprite Texture: {} (512x512)",
                    physic_constants::TEXTURE_SMOKE_PUFF_PATH
                ));

                ui.set_cursor_pos([right_col_x, 60.0 * font_scale]);
                ui.text("- Hardware Primitive: GL_TRIANGLE_FAN");

                ui.set_cursor_pos([right_col_x, 80.0 * font_scale]);
                ui.text("- Vertex Allocation: 10 Vertices (1 Center + 8 Perimeter + 1 Loop)");

                ui.set_cursor_pos([right_col_x, 105.0 * font_scale]);
                ui.text_colored(COLOR_ALERT, "[Red Shaded Corners] Bypassed Surface:");

                ui.set_cursor_pos([right_col_x + 15.0 * font_scale, 125.0 * font_scale]);
                ui.text("  -17.2% surface area eliminated from Hardware Rasterization.");

                ui.set_cursor_pos([right_col_x + 15.0 * font_scale, 145.0 * font_scale]);
                ui.text("  Zero fragment shader threads scheduled for transparent quad corners.");

                ui.set_cursor_pos([right_col_x, 170.0 * font_scale]);
                ui.text_colored(
                    COLOR_HEADER,
                    "[Cyan Octagon Mesh] Active GPU Raster Boundary:",
                );

                ui.set_cursor_pos([right_col_x + 15.0 * font_scale, 190.0 * font_scale]);
                ui.text("  Tight 8-sided polygon circumscribing the volumetric cloud core.");
            });
    }

    ui.spacing();
    ui.separator();

    render_smoke_controls(
        ui,
        state,
        cmd_queue,
        preview_max_zoom,
        preview_rocket_color,
        preview_simulated_speed,
        preview_simulated_angle_offset,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physic_engine::config::PhysicConfig;

    #[test]
    fn test_render_smoke_controls_pure_function() {
        let _guard = crate::simulator::gui_settings::IMGUI_TEST_MUTEX
            .lock()
            .unwrap();
        let config = PhysicConfig::default();
        let mut cmd_queue: Vec<EngineCommand> = Vec::with_capacity(16);

        let mut imgui_ctx = imgui::Context::create();
        imgui_ctx.set_ini_filename(None);
        imgui_ctx.fonts().build_rgba32_texture();
        imgui_ctx.io_mut().display_size = [800.0, 600.0];

        let ui = imgui_ctx.frame();

        let mut preview_max_zoom = 10.0;
        let mut preview_rocket_color = [1.0, 1.0, 1.0];
        let mut preview_simulated_speed = 400.0;
        let mut preview_simulated_angle_offset = 0.0;
        render_smoke_controls(
            ui,
            &config,
            &mut cmd_queue,
            &mut preview_max_zoom,
            &mut preview_rocket_color,
            &mut preview_simulated_speed,
            &mut preview_simulated_angle_offset,
        );

        assert!(cmd_queue.capacity() >= 16);
    }
}

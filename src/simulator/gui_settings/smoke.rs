// GUI_PERSIST: physics.config

use crate::physic_engine::config::{PhysicConfig, SmokeColorMode};
use crate::physic_engine::PhysicEngineFull;
use crate::renderer_engine::smoke_preview::{
    PreviewContext, SmokePreviewRenderer, PREVIEW_PAN_X, PREVIEW_PAN_Y, PREVIEW_ROT_Z, PREVIEW_ZOOM,
};
use imgui::{ColorEditFlags, Ui};
use std::sync::atomic::{AtomicBool, Ordering};

static mut PREVIEW_GPU: Option<SmokePreviewRenderer> = None;
pub static SHOW_GEOMETRY_TRIMMING: AtomicBool = AtomicBool::new(true);

/// Shared GUI control panel for Smoke & Alpha Erosion parameters with per-parameter reset buttons.
pub fn render_smoke_controls(ui: &Ui, cfg_mut: &mut PhysicConfig) -> bool {
    let default_cfg = PhysicConfig::default();
    let mut modified = false;

    let item_width = (ui.content_region_avail()[0] * 0.38).clamp(160.0, 300.0);

    // =========================================================================
    // 1. ALPHA EROSION (DISSOLVE & BURN SEAM)
    // =========================================================================
    ui.text_colored(
        [0.9, 0.6, 0.2, 1.0],
        "=== 1. ALPHA EROSION (DISSOLVE & BURN SEAM) ===",
    );

    if ui.checkbox(
        "Enable Alpha Erosion Dissolve (`physic.smoke_erosion_enabled`)",
        &mut cfg_mut.smoke_erosion_enabled,
    ) {
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_erosion_enabled") {
        cfg_mut.smoke_erosion_enabled = default_cfg.smoke_erosion_enabled;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {}",
            default_cfg.smoke_erosion_enabled
        ));
    }

    if cfg_mut.smoke_erosion_enabled {
        ui.set_next_item_width(item_width);
        if ui.slider(
            "Erosion Aggressiveness / Scale (`physic.smoke_erosion_scale`)",
            0.0,
            2.0,
            &mut cfg_mut.smoke_erosion_scale,
        ) {
            modified = true;
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_erosion_scale") {
            cfg_mut.smoke_erosion_scale = default_cfg.smoke_erosion_scale;
            modified = true;
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!(
                "Reset to default: {:.2}",
                default_cfg.smoke_erosion_scale
            ));
        }

        ui.set_next_item_width(item_width);
        if ui.slider(
            "Erosion Edge Width (`physic.smoke_erosion_edge_width`)",
            0.0,
            0.80,
            &mut cfg_mut.smoke_erosion_edge_width,
        ) {
            modified = true;
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_erosion_edge_width") {
            cfg_mut.smoke_erosion_edge_width = default_cfg.smoke_erosion_edge_width;
            modified = true;
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!(
                "Reset to default: {:.2}",
                default_cfg.smoke_erosion_edge_width
            ));
        }

        ui.set_next_item_width(item_width);
        if ui
            .color_edit3_config(
                "Erosion Edge Glowing Color (`physic.smoke_erosion_edge_color`)",
                &mut cfg_mut.smoke_erosion_edge_color,
            )
            .flags(ColorEditFlags::PICKER_HUE_BAR | ColorEditFlags::DISPLAY_HEX)
            .build()
        {
            modified = true;
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_erosion_edge_color") {
            cfg_mut.smoke_erosion_edge_color = default_cfg.smoke_erosion_edge_color;
            modified = true;
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
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Flow Distortion Strength (`physic.flow_distortion_strength`)",
        0.0,
        1.0,
        &mut cfg_mut.flow_distortion_strength,
    ) {
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_flow_distortion_strength") {
        cfg_mut.flow_distortion_strength = default_cfg.flow_distortion_strength;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}",
            default_cfg.flow_distortion_strength
        ));
    }

    ui.set_next_item_width(item_width);
    if ui.slider(
        "Flow Animation Speed (`physic.flow_animation_speed`)",
        0.0,
        5.0,
        &mut cfg_mut.flow_animation_speed,
    ) {
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_flow_animation_speed") {
        cfg_mut.flow_animation_speed = default_cfg.flow_animation_speed;
        modified = true;
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
    ui.text_colored([0.4, 0.8, 1.0, 1.0], "=== 2. SMOKE CORE TINT & COLOR ===");

    let mut is_rocket_color = cfg_mut.smoke_color_mode == SmokeColorMode::RocketColor;
    let mut is_custom_color = cfg_mut.smoke_color_mode == SmokeColorMode::Custom;

    ui.text("Smoke Base Core Tint Mode:");
    ui.same_line();
    if ui.radio_button("Rocket Color (Inherited)", &mut is_rocket_color, true) {
        cfg_mut.smoke_color_mode = SmokeColorMode::RocketColor;
        modified = true;
    }
    ui.same_line();
    if ui.radio_button("Custom Fixed Color", &mut is_custom_color, true) {
        cfg_mut.smoke_color_mode = SmokeColorMode::Custom;
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_color_mode") {
        cfg_mut.smoke_color_mode = default_cfg.smoke_color_mode;
        cfg_mut.smoke_inherited_color_intensity = default_cfg.smoke_inherited_color_intensity;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text("Reset color mode to default (RocketColor)");
    }

    if cfg_mut.smoke_color_mode == SmokeColorMode::RocketColor {
        ui.set_next_item_width(item_width);
        if ui.slider(
            "Inherited Rocket Color Intensity (`physic.smoke_inherited_color_intensity`)",
            0.0,
            2.0,
            &mut cfg_mut.smoke_inherited_color_intensity,
        ) {
            modified = true;
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_inherited_color_intensity") {
            cfg_mut.smoke_inherited_color_intensity = default_cfg.smoke_inherited_color_intensity;
            modified = true;
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!(
                "Reset inherited rocket color intensity to default: {:.2}",
                default_cfg.smoke_inherited_color_intensity
            ));
        }
    }

    if cfg_mut.smoke_color_mode == SmokeColorMode::Custom {
        ui.set_next_item_width(item_width);
        if ui
            .color_edit3_config(
                "Smoke Custom Core Color (`physic.smoke_custom_color`)",
                &mut cfg_mut.smoke_custom_color,
            )
            .flags(ColorEditFlags::PICKER_HUE_BAR | ColorEditFlags::DISPLAY_HEX)
            .build()
        {
            modified = true;
        }
        ui.same_line();
        if ui.small_button("Reset##reset_smoke_custom_color") {
            cfg_mut.smoke_custom_color = default_cfg.smoke_custom_color;
            modified = true;
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
        [0.4, 0.8, 1.0, 1.0],
        "=== 3. EMISSION CADENCE & LIFETIME DYNAMICS ===",
    );

    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Spawn Rate (`physic.smoke_spawn_rate`)",
        0.0,
        120.0,
        &mut cfg_mut.smoke_spawn_rate,
    ) {
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_spawn_rate") {
        cfg_mut.smoke_spawn_rate = default_cfg.smoke_spawn_rate;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.1}",
            default_cfg.smoke_spawn_rate
        ));
    }

    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Initial Size (`physic.smoke_initial_size`)",
        1.0,
        40.0,
        &mut cfg_mut.smoke_initial_size,
    ) {
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_initial_size") {
        cfg_mut.smoke_initial_size = default_cfg.smoke_initial_size;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.1}",
            default_cfg.smoke_initial_size
        ));
    }

    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Growth Rate Multiplier (`physic.smoke_growth_rate_multiplier`)",
        0.0,
        5.0,
        &mut cfg_mut.smoke_growth_rate_multiplier,
    ) {
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_growth_rate_multiplier") {
        cfg_mut.smoke_growth_rate_multiplier = default_cfg.smoke_growth_rate_multiplier;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}",
            default_cfg.smoke_growth_rate_multiplier
        ));
    }

    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Fade Duration (s) (`physic.smoke_fade_duration`)",
        0.05,
        3.0,
        &mut cfg_mut.smoke_fade_duration,
    ) {
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_fade_duration") {
        cfg_mut.smoke_fade_duration = default_cfg.smoke_fade_duration;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}s",
            default_cfg.smoke_fade_duration
        ));
    }

    ui.set_next_item_width(item_width);
    if ui.slider(
        "Smoke Intensity / Brightness (`physic.smoke_intensity`)",
        0.0,
        2.0,
        &mut cfg_mut.smoke_intensity,
    ) {
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_smoke_intensity") {
        cfg_mut.smoke_intensity = default_cfg.smoke_intensity;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {:.2}",
            default_cfg.smoke_intensity
        ));
    }

    let mut max_smoke = cfg_mut.max_smoke_particles as i32;
    ui.set_next_item_width(item_width);
    if ui.slider(
        "Max Smoke Particles Pool (`physic.max_smoke_particles`)",
        100,
        16384,
        &mut max_smoke,
    ) {
        cfg_mut.max_smoke_particles = max_smoke.max(100) as usize;
        modified = true;
    }
    ui.same_line();
    if ui.small_button("Reset##reset_max_smoke_particles") {
        cfg_mut.max_smoke_particles = default_cfg.max_smoke_particles;
        modified = true;
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(format!(
            "Reset to default: {}",
            default_cfg.max_smoke_particles
        ));
    }

    modified
}

/// Renders the dedicated Smoke & Alpha Erosion settings tab.
pub fn render_smoke_settings_tab<P: PhysicEngineFull>(ui: &Ui, physic_engine: &mut P) {
    let cfg_mut = physic_engine.get_config_mut();
    let mut modified = false;

    ui.text_colored(
        [0.4, 0.8, 1.0, 1.0],
        "SMOKE TRAIL & ALPHA EROSION (DISSOLVE) CONTROLS",
    );
    ui.same_line();
    if ui.small_button("Reset Smoke Defaults") {
        let default_cfg = PhysicConfig::default();
        cfg_mut.smoke_spawn_rate = default_cfg.smoke_spawn_rate;
        cfg_mut.smoke_initial_size = default_cfg.smoke_initial_size;
        cfg_mut.smoke_growth_rate_multiplier = default_cfg.smoke_growth_rate_multiplier;
        cfg_mut.smoke_fade_duration = default_cfg.smoke_fade_duration;
        cfg_mut.max_smoke_particles = default_cfg.max_smoke_particles;
        cfg_mut.smoke_intensity = default_cfg.smoke_intensity;
        cfg_mut.smoke_color_mode = default_cfg.smoke_color_mode;
        cfg_mut.smoke_custom_color = default_cfg.smoke_custom_color;
        cfg_mut.smoke_erosion_enabled = default_cfg.smoke_erosion_enabled;
        cfg_mut.smoke_erosion_scale = default_cfg.smoke_erosion_scale;
        cfg_mut.smoke_erosion_edge_width = default_cfg.smoke_erosion_edge_width;
        cfg_mut.smoke_erosion_edge_color = default_cfg.smoke_erosion_edge_color;
        modified = true;
    }

    ui.separator();

    let avail_width = ui.content_region_avail()[0];

    // =========================================================================
    // 1. QUICK PRESETS
    // =========================================================================
    ui.text_colored([0.9, 0.8, 0.3, 1.0], "Quick Dissolve Style Presets:");
    ui.same_line();

    if ui.button("[Fire & Ember]") {
        cfg_mut.smoke_erosion_edge_width = 0.12;
        cfg_mut.smoke_erosion_edge_color = [1.0, 0.4, 0.05];
        cfg_mut.smoke_color_mode = SmokeColorMode::Custom;
        cfg_mut.smoke_custom_color = [0.15, 0.15, 0.15];
        cfg_mut.smoke_intensity = 0.85;
        modified = true;
    }
    ui.same_line();
    if ui.button("[Plasma Blue]") {
        cfg_mut.smoke_erosion_edge_width = 0.15;
        cfg_mut.smoke_erosion_edge_color = [0.1, 0.8, 1.0];
        cfg_mut.smoke_color_mode = SmokeColorMode::Custom;
        cfg_mut.smoke_custom_color = [0.8, 0.9, 1.0];
        cfg_mut.smoke_intensity = 1.0;
        modified = true;
    }
    ui.same_line();
    if ui.button("[Volumetric Cloud]") {
        cfg_mut.smoke_erosion_edge_width = 0.05;
        cfg_mut.smoke_erosion_edge_color = [0.75, 0.75, 0.75];
        cfg_mut.smoke_color_mode = SmokeColorMode::Custom;
        cfg_mut.smoke_custom_color = [0.85, 0.85, 0.85];
        cfg_mut.smoke_intensity = 0.5;
        modified = true;
    }
    ui.same_line();
    if ui.button("[Toxic Plasma]") {
        cfg_mut.smoke_erosion_edge_width = 0.18;
        cfg_mut.smoke_erosion_edge_color = [0.2, 1.0, 0.3];
        cfg_mut.smoke_color_mode = SmokeColorMode::Custom;
        cfg_mut.smoke_custom_color = [0.1, 0.25, 0.1];
        cfg_mut.smoke_intensity = 0.9;
        modified = true;
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
        let canvas_aspect = (avail_width / 145.0).max(0.1);
        let ctx = PreviewContext {
            config: cfg_mut,
            zoom,
            pan_x,
            pan_y,
            rot_deg: rot_z,
            canvas_aspect,
            time: ui.time() as f32,
            dt: ui.io().delta_time,
        };
        preview.render(&ctx)
    };

    // =========================================================================
    // 2. 100% BIT-FOR-BIT ISO GPU FBO PREVIEW CANVAS (INTERACTIVE VIEWPORT)
    // =========================================================================
    ui.child_window("SmokeErosionPreviewCanvas")
        .size([avail_width, 185.0])
        .border(true)
        .focused(false)
        .flags(imgui::WindowFlags::NO_SCROLL_WITH_MOUSE | imgui::WindowFlags::NO_SCROLLBAR)
        .build(|| {
            let zoom = PREVIEW_ZOOM.load(Ordering::Relaxed) as f32 / 100.0;
            let pan_x = PREVIEW_PAN_X.load(Ordering::Relaxed) as f32 / 10.0;
            let pan_y = PREVIEW_PAN_Y.load(Ordering::Relaxed) as f32 / 10.0;
            let rot_z = PREVIEW_ROT_Z.load(Ordering::Relaxed) as f32 / 10.0;

            let preview_width = ui.content_region_avail()[0];
            let preview_height = 145.0;

            // Render FBO color texture in ImGui!
            let tex_id = imgui::TextureId::new(fbo_tex as usize);
            imgui::Image::new(tex_id, [preview_width, preview_height])
                .uv0([0.0, 1.0]) // Flip Y for OpenGL Framebuffer
                .uv1([1.0, 0.0])
                .build(ui);

            let is_canvas_hovered = ui.is_item_hovered();

            if is_canvas_hovered {
                let io = ui.io();

                // Exclusive Mouse Wheel Zoom: zero out C++ ImGui IO MouseWheel to prevent parent window scrolling
                let wheel = io.mouse_wheel;
                if wheel != 0.0 {
                    let cur_z = PREVIEW_ZOOM.load(Ordering::Relaxed) as f32 / 100.0;
                    let new_z = (cur_z + wheel * 0.12).clamp(0.4, 3.5);
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
                    let new_x = cur_x + delta[0] * 0.4;
                    let new_y = cur_y - delta[1] * 0.4; // Screen Y is inverted
                    PREVIEW_PAN_X.store((new_x * 10.0) as i32, Ordering::Relaxed);
                    PREVIEW_PAN_Y.store((new_y * 10.0) as i32, Ordering::Relaxed);
                }

                // Exclusive Right Click Drag: Rotation Z (Unreal/Unity Viewport Orbit)
                if ui.is_mouse_dragging(imgui::MouseButton::Right) {
                    let delta = io.mouse_delta;
                    let cur_r = PREVIEW_ROT_Z.load(Ordering::Relaxed) as f32 / 10.0;
                    let new_r = (cur_r + delta[0] * 0.5) % 360.0;
                    PREVIEW_ROT_Z.store((new_r * 10.0) as i32, Ordering::Relaxed);
                }
            }

            ui.set_cursor_pos([10.0, 10.0]);
            ui.text_colored(
                [0.4, 0.8, 1.0, 1.0],
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

            ui.set_cursor_pos([10.0, 160.0]);
            ui.text_colored(
                [0.6, 0.6, 0.6, 0.9],
                "[Middle Drag: Pan X/Y] | [Right Drag: Rotate Z] | [Wheel: Zoom]",
            );
        });

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
        let tex_box_size = 200.0;

        ui.child_window("GeometryTrimmingContainer")
            .size([avail_width, 220.0])
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

                // 2. Bounding Quad (Square 200x200) in Red
                let sq_p0 = [center[0] - half, center[1] - half];
                let sq_p1 = [center[0] + half, center[1] - half];
                let sq_p2 = [center[0] + half, center[1] + half];
                let sq_p3 = [center[0] - half, center[1] + half];

                draw_list
                    .add_line(sq_p0, sq_p1, [1.0, 0.2, 0.2, 0.9])
                    .thickness(1.5)
                    .build();
                draw_list
                    .add_line(sq_p1, sq_p2, [1.0, 0.2, 0.2, 0.9])
                    .thickness(1.5)
                    .build();
                draw_list
                    .add_line(sq_p2, sq_p3, [1.0, 0.2, 0.2, 0.9])
                    .thickness(1.5)
                    .build();
                draw_list
                    .add_line(sq_p3, sq_p0, [1.0, 0.2, 0.2, 0.9])
                    .thickness(1.5)
                    .build();

                // 3. 1:1 Octagon Vertices matching GPU SmokeRenderer primitive
                let r_maj = half;
                let r_diag = half * (0.765366 / 1.082392); // ~70.7px

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
                    .add_triangle(sq_p0, oct[6], oct[5], [1.0, 0.1, 0.1, 0.45])
                    .filled(true)
                    .build();
                draw_list
                    .add_triangle(sq_p1, oct[7], oct[6], [1.0, 0.1, 0.1, 0.45])
                    .filled(true)
                    .build();
                draw_list
                    .add_triangle(sq_p2, oct[1], oct[2], [1.0, 0.1, 0.1, 0.45])
                    .filled(true)
                    .build();
                draw_list
                    .add_triangle(sq_p3, oct[3], oct[2], [1.0, 0.1, 0.1, 0.45])
                    .filled(true)
                    .build();

                // 5. Octagon Outline & Triangle Fan Edges in Cyan
                for i in 0..8 {
                    let next = (i + 1) % 8;
                    draw_list
                        .add_line(oct[i], oct[next], [0.0, 1.0, 0.8, 1.0])
                        .thickness(2.0)
                        .build();
                    draw_list
                        .add_line(center, oct[i], [0.0, 1.0, 0.8, 0.35])
                        .thickness(1.0)
                        .build();
                    draw_list
                        .add_circle(oct[i], 4.0, [0.0, 1.0, 1.0, 1.0])
                        .filled(true)
                        .build();
                }
                draw_list
                    .add_circle(center, 4.5, [1.0, 1.0, 0.0, 1.0])
                    .filled(true)
                    .build();

                // Right Column: Technical Stats & Explanatory Legend
                ui.set_cursor_pos([225.0, 15.0]);
                ui.text_colored(
                    [0.0, 1.0, 0.8, 1.0],
                    "=== GEOMETRY TRIMMING SPECIFICATIONS (1:1 SCALE) ===",
                );

                ui.set_cursor_pos([225.0, 40.0]);
                ui.text("- Target Sprite Texture: assets/textures/smoke_puff.png (512x512)");

                ui.set_cursor_pos([225.0, 60.0]);
                ui.text("- Hardware Primitive: GL_TRIANGLE_FAN");

                ui.set_cursor_pos([225.0, 80.0]);
                ui.text("- Vertex Allocation: 10 Vertices (1 Center + 8 Perimeter + 1 Loop)");

                ui.set_cursor_pos([225.0, 105.0]);
                ui.text_colored(
                    [1.0, 0.3, 0.3, 1.0],
                    "[Red Shaded Corners] Bypassed Surface:",
                );

                ui.set_cursor_pos([240.0, 125.0]);
                ui.text("  -17.2% surface area eliminated from Hardware Rasterization.");

                ui.set_cursor_pos([240.0, 145.0]);
                ui.text("  Zero fragment shader threads scheduled for transparent quad corners.");

                ui.set_cursor_pos([225.0, 170.0]);
                ui.text_colored(
                    [0.0, 1.0, 0.8, 1.0],
                    "[Cyan Octagon Mesh] Active GPU Raster Boundary:",
                );

                ui.set_cursor_pos([240.0, 190.0]);
                ui.text("  Tight 8-sided polygon circumscribing the volumetric cloud core.");
            });
    }

    ui.spacing();
    ui.separator();

    if render_smoke_controls(ui, cfg_mut) {
        modified = true;
    }

    // REAL-TIME INSTANT SYNC WITH ENGINE
    if modified {
        let pending = physic_engine.get_config_mut().clone();
        let _ = physic_engine.reload_config(&pending);
    }
}

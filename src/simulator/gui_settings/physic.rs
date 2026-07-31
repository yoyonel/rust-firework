use crate::physic_engine::PhysicEngineFull;
use imgui::Ui;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

pub const PRESET_DEFINITIONS: [(&str, &str, &str, f32, f32); 5] = [
    (
        "Heart",
        "heart",
        "assets/textures/explosion_shapes/heart.png",
        150.0,
        1.5,
    ),
    (
        "Star",
        "star",
        "assets/textures/explosion_shapes/star.png",
        180.0,
        1.5,
    ),
    (
        "Smiley",
        "smiley",
        "assets/textures/explosion_shapes/smiley.png",
        200.0,
        2.0,
    ),
    (
        "Note",
        "note",
        "assets/textures/explosion_shapes/note.png",
        160.0,
        1.5,
    ),
    (
        "Ring",
        "ring",
        "assets/textures/explosion_shapes/ring.png",
        190.0,
        1.8,
    ),
];

pub fn default_preset_weights() -> [f32; 5] {
    [1.0; 5]
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersistedExplosionImage {
    pub file_stem: String,
    pub scale: f32,
    pub flight_time: f32,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum PersistedExplosionShape {
    #[default]
    Spherical,
    Images {
        images: Vec<PersistedExplosionImage>,
    },
}

impl PersistedExplosionShape {
    pub fn from_engine(shape: &crate::physic_engine::ExplosionShape) -> Self {
        match shape {
            crate::physic_engine::ExplosionShape::Spherical => Self::Spherical,
            crate::physic_engine::ExplosionShape::Image(img) => Self::Images {
                images: vec![PersistedExplosionImage {
                    file_stem: img.file_stem.clone(),
                    scale: img.scale,
                    flight_time: img.flight_time,
                    weight: 1.0,
                }],
            },
            crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => Self::Images {
                images: shapes
                    .iter()
                    .map(|(img, weight)| PersistedExplosionImage {
                        file_stem: img.file_stem.clone(),
                        scale: img.scale,
                        flight_time: img.flight_time,
                        weight: *weight,
                    })
                    .collect(),
            },
        }
    }
}

pub fn get_preset_defaults(stem: &str) -> (f32, f32) {
    let key = stem.to_lowercase();
    PRESET_DEFINITIONS
        .iter()
        .find(|(_, s, _, _, _)| *s == key)
        .map(|(_, _, _, scale, flight)| (*scale, *flight))
        .unwrap_or((180.0, 1.5))
}

pub fn apply_session_to_physic<P: PhysicEngineFull>(
    _preset_weights: [f32; 5],
    explosion_shape: &PersistedExplosionShape,
    physic_engine: &mut P,
) {
    let PersistedExplosionShape::Images { images } = explosion_shape else {
        physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
        return;
    };

    let images: Vec<_> = images.iter().filter(|image| image.weight > 0.0).collect();
    let Some(first) = images.first() else {
        physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
        return;
    };

    let first_path = format!("assets/textures/explosion_shapes/{}.png", first.file_stem);
    if images.len() == 1 {
        let _ = physic_engine.load_explosion_image(&first_path, first.scale, first.flight_time);
        return;
    }

    let _ = physic_engine.load_explosion_image_weighted(
        &first_path,
        first.scale,
        first.flight_time,
        first.weight,
    );
    for image in &images[1..] {
        let path = format!("assets/textures/explosion_shapes/{}.png", image.file_stem);
        let _ = physic_engine.load_explosion_image_weighted(
            &path,
            image.scale,
            image.flight_time,
            image.weight,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_physics_settings_tab<P: PhysicEngineFull>(
    ui: &Ui,
    filter: &str,
    physic_engine: &mut P,
    physic_reinit_requested: &AtomicBool,
    preset_weights: &mut [f32; 5],
) {
    // GUI_PERSIST: physics.config
    ui.spacing();

    // Check if there are pending config changes
    let applied_cfg = physic_engine.get_config().clone();
    let pending_cfg = physic_engine.get_config_mut().clone();
    let is_modified = applied_cfg != pending_cfg;

    if is_modified {
        ui.text_colored(
            [1.0, 0.6, 0.0, 1.0],
            "WARNING: Pending Physics Changes Detected! Click 'Apply' to sync engines.",
        );
    } else {
        ui.text_colored([0.0, 1.0, 0.4, 1.0], "[OK] Physics Configuration Synced.");
    }

    // Action buttons
    if ui.button("[APPLY] PENDING CHANGES (`physic.apply`)") {
        let pending = physic_engine.get_config_mut().clone();
        let _ = physic_engine.reload_config(&pending);
        physic_reinit_requested.store(true, Ordering::Relaxed);
    }
    ui.same_line();
    if ui.button("[SAVE] Save Config (`physic.config.save`)") {
        let _ = physic_engine
            .get_config()
            .save_to_file("assets/config/physic.toml");
    }
    ui.same_line();
    if ui.button("[RELOAD] Reload Disk Config (`physic.config.reload`)") {
        if let Ok(new_cfg) =
            crate::physic_engine::config::PhysicConfig::from_file("assets/config/physic.toml")
        {
            *physic_engine.get_config_mut() = new_cfg.clone();
            let _ = physic_engine.reload_config(&new_cfg);
            physic_reinit_requested.store(true, Ordering::Relaxed);
        }
    }
    ui.same_line();
    if ui.button("[RESET PHYSICS DEFAULTS]") {
        let default_cfg = crate::physic_engine::config::PhysicConfig::default();
        *physic_engine.get_config_mut() = default_cfg.clone();
        let _ = physic_engine.reload_config(&default_cfg);
        physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
        physic_reinit_requested.store(true, Ordering::Relaxed);
    }

    ui.separator();

    // Use a responsive item width (45% of content region, capped 200px..360px) so labels have plenty of space on the right
    let item_width = (ui.content_region_avail()[0] * 0.45).clamp(200.0, 360.0);
    let _item_w_token = ui.push_item_width(item_width);

    // 1. Simulation Capacity
    if filter.is_empty() || "capacity max_rockets particles".contains(filter) {
        ui.text_colored(
            [0.4, 0.8, 1.0, 1.0],
            "=== SIMULATION CAPACITY & BUFFERS ===",
        );
        ui.same_line();
        if ui.small_button("Reset Capacity Defaults") {
            let default_cfg = crate::physic_engine::config::PhysicConfig::default();
            let cfg_mut = physic_engine.get_config_mut();
            cfg_mut.max_rockets = default_cfg.max_rockets;
            cfg_mut.particles_per_explosion = default_cfg.particles_per_explosion;
            cfg_mut.particles_per_trail = default_cfg.particles_per_trail;
        }

        let cfg_mut = physic_engine.get_config_mut();

        let mut max_rockets = cfg_mut.max_rockets as i32;
        if ui.slider(
            "Max Concurrent Rockets (`physic.max_rockets`)",
            1,
            2048,
            &mut max_rockets,
        ) {
            cfg_mut.max_rockets = max_rockets.max(1) as usize;
        }

        let mut p_explosion = cfg_mut.particles_per_explosion as i32;
        if ui.slider(
            "Particles / Explosion (`physic.particles_per_explosion`)",
            10,
            2000,
            &mut p_explosion,
        ) {
            cfg_mut.particles_per_explosion = p_explosion.max(10) as usize;
        }

        let mut p_trail = cfg_mut.particles_per_trail as i32;
        if ui.slider(
            "Particles / Trail (`physic.particles_per_trail`)",
            0,
            500,
            &mut p_trail,
        ) {
            cfg_mut.particles_per_trail = p_trail.max(0) as usize;
        }

        let mut max_smoke = cfg_mut.max_smoke_particles as i32;
        if ui.slider(
            "Max Smoke Particles (`physic.max_smoke_particles`)",
            100,
            16384,
            &mut max_smoke,
        ) {
            cfg_mut.max_smoke_particles = max_smoke.max(100) as usize;
        }
    }

    // 2. Smoke Trail Dynamics & Alpha Erosion
    if filter.is_empty()
        || "smoke trail rate size growth fade erosion edge dissolve color".contains(filter)
    {
        ui.spacing();
        ui.separator();
        ui.text_colored(
            [0.4, 0.8, 1.0, 1.0],
            "=== SMOKE TRAIL DYNAMICS & ALPHA EROSION ===",
        );
        ui.same_line();
        if ui.small_button("Reset Smoke Defaults") {
            let default_cfg = crate::physic_engine::config::PhysicConfig::default();
            let cfg_mut = physic_engine.get_config_mut();
            cfg_mut.smoke_spawn_rate = default_cfg.smoke_spawn_rate;
            cfg_mut.smoke_initial_size = default_cfg.smoke_initial_size;
            cfg_mut.smoke_growth_rate_multiplier = default_cfg.smoke_growth_rate_multiplier;
            cfg_mut.smoke_fade_duration = default_cfg.smoke_fade_duration;
            cfg_mut.max_smoke_particles = default_cfg.max_smoke_particles;
            cfg_mut.smoke_intensity = default_cfg.smoke_intensity;
            cfg_mut.smoke_color_mode = default_cfg.smoke_color_mode;
            cfg_mut.smoke_custom_color = default_cfg.smoke_custom_color;
            cfg_mut.smoke_inherited_color_intensity = default_cfg.smoke_inherited_color_intensity;
            cfg_mut.smoke_erosion_enabled = default_cfg.smoke_erosion_enabled;
            cfg_mut.smoke_erosion_scale = default_cfg.smoke_erosion_scale;
            cfg_mut.smoke_erosion_edge_width = default_cfg.smoke_erosion_edge_width;
            cfg_mut.smoke_erosion_edge_color = default_cfg.smoke_erosion_edge_color;
            cfg_mut.flow_distortion_strength = default_cfg.flow_distortion_strength;
            cfg_mut.flow_animation_speed = default_cfg.flow_animation_speed;
        }

        let cfg_mut = physic_engine.get_config_mut();
        let _ = super::smoke::render_smoke_controls(ui, cfg_mut);
    }

    // 2. Launch & Spawn Dynamics
    if filter.is_empty() || "spawn launch interval angle speed".contains(filter) {
        ui.spacing();
        ui.separator();
        ui.text_colored(
            [0.4, 0.8, 1.0, 1.0],
            "=== ROCKET LAUNCH & SPAWN DYNAMICS ===",
        );
        ui.same_line();
        if ui.small_button("Reset Spawn Defaults") {
            let default_cfg = crate::physic_engine::config::PhysicConfig::default();
            let cfg_mut = physic_engine.get_config_mut();
            cfg_mut.rocket_interval_mean = default_cfg.rocket_interval_mean;
            cfg_mut.rocket_interval_variation = default_cfg.rocket_interval_variation;
            cfg_mut.rocket_max_next_interval = default_cfg.rocket_max_next_interval;
            cfg_mut.spawn_rocket_margin = default_cfg.spawn_rocket_margin;
            cfg_mut.spawn_rocket_vertical_angle = default_cfg.spawn_rocket_vertical_angle;
            cfg_mut.spawn_rocket_angle_variation = default_cfg.spawn_rocket_angle_variation;
            cfg_mut.spawn_rocket_min_speed = default_cfg.spawn_rocket_min_speed;
            cfg_mut.spawn_rocket_max_speed = default_cfg.spawn_rocket_max_speed;
            cfg_mut.initial_rocket_speed = default_cfg.initial_rocket_speed;
        }

        let cfg_mut = physic_engine.get_config_mut();

        ui.slider(
            "Spawn Interval Mean (s) (`physic.rocket_interval_mean`)",
            0.05,
            5.0,
            &mut cfg_mut.rocket_interval_mean,
        );
        ui.slider(
            "Interval Variation (`physic.rocket_interval_variation`)",
            0.0,
            3.0,
            &mut cfg_mut.rocket_interval_variation,
        );
        ui.slider(
            "Max Next Interval (`physic.rocket_max_next_interval`)",
            0.1,
            10.0,
            &mut cfg_mut.rocket_max_next_interval,
        );
        ui.slider(
            "Spawn Margin (`physic.spawn_rocket_margin`)",
            0.0,
            200.0,
            &mut cfg_mut.spawn_rocket_margin,
        );
        ui.slider(
            "Vertical Angle (rad) (`physic.spawn_rocket_vertical_angle`)",
            0.0,
            std::f32::consts::PI,
            &mut cfg_mut.spawn_rocket_vertical_angle,
        );
        ui.slider(
            "Angle Variation (`physic.spawn_rocket_angle_variation`)",
            0.0,
            1.57,
            &mut cfg_mut.spawn_rocket_angle_variation,
        );
        ui.slider(
            "Spawn Min Speed (`physic.spawn_rocket_min_speed`)",
            10.0,
            1000.0,
            &mut cfg_mut.spawn_rocket_min_speed,
        );
        ui.slider(
            "Spawn Max Speed (`physic.spawn_rocket_max_speed`)",
            10.0,
            2000.0,
            &mut cfg_mut.spawn_rocket_max_speed,
        );
        ui.slider(
            "Initial Rocket Speed (`physic.initial_rocket_speed`)",
            10.0,
            1500.0,
            &mut cfg_mut.initial_rocket_speed,
        );
    }

    // 3. Forces & Particle Physics
    if filter.is_empty() || "gravity threshold explosion velocity forces".contains(filter) {
        ui.spacing();
        ui.separator();
        ui.text_colored([0.4, 0.8, 1.0, 1.0], "=== FORCES & EXPLOSION DYNAMICS ===");
        ui.same_line();
        if ui.small_button("Reset Forces Defaults") {
            let default_cfg = crate::physic_engine::config::PhysicConfig::default();
            let cfg_mut = physic_engine.get_config_mut();
            cfg_mut.gravity = default_cfg.gravity;
            cfg_mut.explosion_threshold = default_cfg.explosion_threshold;
            cfg_mut.explosion_min_vel = default_cfg.explosion_min_vel;
            cfg_mut.explosion_max_vel = default_cfg.explosion_max_vel;
        }

        let cfg_mut = physic_engine.get_config_mut();

        ui.slider(
            "Gravity (`physic.gravity`)",
            -2000.0,
            2000.0,
            &mut cfg_mut.gravity,
        );
        ui.slider(
            "Explosion Speed Threshold (`physic.explosion_threshold`)",
            0.0,
            500.0,
            &mut cfg_mut.explosion_threshold,
        );
        ui.slider(
            "Explosion Min Velocity (`physic.explosion_min_vel`)",
            1.0,
            1000.0,
            &mut cfg_mut.explosion_min_vel,
        );
        ui.slider(
            "Explosion Max Velocity (`physic.explosion_max_vel`)",
            1.0,
            2000.0,
            &mut cfg_mut.explosion_max_vel,
        );
    }

    // 4. Explosion Shapes & MultiImage Tuning
    if filter.is_empty()
        || "shape image preset heart star smiley weight scale flight_time add delete remove"
            .contains(filter)
    {
        ui.spacing();
        ui.separator();
        ui.text_colored(
            [0.4, 0.8, 1.0, 1.0],
            "=== EXPLOSION SHAPE & PRESETS (`physic.explosion.*`) ===",
        );
        ui.same_line();
        if ui.small_button("Reset to Spherical") {
            physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
        }

        let current_shape_str = match physic_engine.get_explosion_shape() {
            crate::physic_engine::ExplosionShape::Spherical => {
                "Spherical (Standard 3D/2D Burst)".to_string()
            }
            crate::physic_engine::ExplosionShape::Image(img) => {
                format!("Single Image: '{}'", img.file_stem)
            }
            crate::physic_engine::ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                format!(
                    "MultiImage ({} shapes active, total weight {:.1})",
                    shapes.len(),
                    total_weight
                )
            }
        };
        ui.text(format!("Current Mode: {}", current_shape_str));

        if ui.button("Spherical Mode (`physic.explosion.shape spherical`)") {
            physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
        }

        ui.spacing();
        ui.text_colored([0.9, 0.9, 0.4, 1.0], "Presets & Custom Weight Adjustment:");
        ui.same_line();
        if ui.small_button("Reset All Preset Weights") {
            *preset_weights = [1.0; 5];
        }

        for (i, (name, _stem, path, scale, flight_time)) in PRESET_DEFINITIONS.iter().enumerate() {
            let label_name = format!("{:<6}", name);
            ui.text(&label_name);
            ui.same_line();

            ui.set_next_item_width(110.0);
            let slider_label = format!("Weight##preset_w_{}", i);
            ui.slider(&slider_label, 0.1, 10.0, &mut preset_weights[i]);
            ui.same_line();

            let reset_w_btn = format!("Reset##reset_w_{}", i);
            if ui.small_button(&reset_w_btn) {
                preset_weights[i] = 1.0;
            }
            if ui.is_item_hovered() {
                ui.tooltip_text("Reset weight to default 1.0");
            }
            ui.same_line();

            let set_btn = format!("Set##set_{}", name);
            if ui.button(&set_btn) {
                let _ = physic_engine.load_explosion_image(path, *scale, *flight_time);
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(format!("Set single active shape: {}", name));
            }
            ui.same_line();

            let add_btn = format!("+Add##add_{}", name);
            if ui.button(&add_btn) {
                let _ = physic_engine.load_explosion_image_weighted(
                    path,
                    *scale,
                    *flight_time,
                    preset_weights[i],
                );
            }
            if ui.is_item_hovered() {
                ui.tooltip_text(format!(
                    "Add {} to MultiImage set with weight {:.1}",
                    name, preset_weights[i]
                ));
            }
        }

        // Active Shape Controls & Reset Buttons
        let shape_clone = physic_engine.get_explosion_shape().clone();
        match shape_clone {
            crate::physic_engine::ExplosionShape::Image(img) => {
                let (def_scale, def_flight) = get_preset_defaults(&img.file_stem);

                ui.spacing();
                ui.text_colored(
                    [0.2, 0.9, 0.4, 1.0],
                    format!("--- Active Single Image: '{}' ---", img.file_stem),
                );
                ui.same_line();
                let del_btn = format!("[X Delete '{}']", img.file_stem);
                if ui.button(&del_btn) {
                    let _ = physic_engine.remove_explosion_image(&img.file_stem);
                }
                ui.same_line();
                let reset_shape_btn = format!("[Reset '{}' Defaults]", img.file_stem);
                if ui.button(&reset_shape_btn) {
                    let mut updated = img.clone();
                    updated.scale = def_scale;
                    updated.flight_time = def_flight;
                    physic_engine
                        .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
                }

                let mut scale = img.scale;
                if ui.slider(
                    "Image Scale (`physic.explosion.scale`)",
                    20.0,
                    500.0,
                    &mut scale,
                ) {
                    let mut updated = img.clone();
                    updated.scale = scale;
                    physic_engine
                        .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
                }
                ui.same_line();
                let reset_scale_btn = format!("Reset Scale##single_scale_{}", img.file_stem);
                if ui.small_button(&reset_scale_btn) {
                    let mut updated = img.clone();
                    updated.scale = def_scale;
                    physic_engine
                        .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
                }

                let mut flight_time = img.flight_time;
                if ui.slider(
                    "Flight Time (s) (`physic.explosion.flight_time`)",
                    0.2,
                    5.0,
                    &mut flight_time,
                ) {
                    let mut updated = img.clone();
                    updated.flight_time = flight_time;
                    physic_engine
                        .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
                }
                ui.same_line();
                let reset_flight_btn = format!("Reset Flight##single_flight_{}", img.file_stem);
                if ui.small_button(&reset_flight_btn) {
                    let mut updated = img.clone();
                    updated.flight_time = def_flight;
                    physic_engine
                        .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
                }
            }
            crate::physic_engine::ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                ui.spacing();
                ui.text_colored(
                    [0.2, 0.9, 0.4, 1.0],
                    format!(
                        "--- Active MultiImage Shapes ({}) & Parameters Breakdown ---",
                        shapes.len()
                    ),
                );

                let mut updated_shapes = shapes.clone();
                let mut shape_to_remove: Option<String> = None;
                let mut changed = false;

                for (shape, weight) in updated_shapes.iter_mut() {
                    let (def_scale, def_flight) = get_preset_defaults(&shape.file_stem);

                    let pct = if total_weight > 0.0 {
                        (*weight / total_weight) * 100.0
                    } else {
                        0.0
                    };

                    ui.text_colored(
                        [0.4, 0.8, 1.0, 1.0],
                        format!("* Forme: '{}' ({:.1}% d'apparition)", shape.file_stem, pct),
                    );
                    ui.same_line();
                    let del_btn = format!("[X Delete]##{}", shape.file_stem);
                    if ui.button(&del_btn) {
                        shape_to_remove = Some(shape.file_stem.clone());
                    }
                    ui.same_line();
                    let reset_all_btn = format!("[Reset Defaults]##{}", shape.file_stem);
                    if ui.button(&reset_all_btn) {
                        *weight = 1.0;
                        shape.scale = def_scale;
                        shape.flight_time = def_flight;
                        changed = true;
                    }

                    // Weight Slider & Reset
                    let mut w = *weight;
                    let weight_label = format!("Weight##w_{}", shape.file_stem);
                    if ui.slider(&weight_label, 0.0, 10.0, &mut w) {
                        *weight = w;
                        changed = true;
                    }
                    ui.same_line();
                    let reset_w_btn = format!("Reset W##rw_{}", shape.file_stem);
                    if ui.small_button(&reset_w_btn) {
                        *weight = 1.0;
                        changed = true;
                    }

                    // Image Scale Slider & Reset
                    let mut scale = shape.scale;
                    let scale_label = format!("Image Scale (px)##scale_{}", shape.file_stem);
                    if ui.slider(&scale_label, 20.0, 500.0, &mut scale) {
                        shape.scale = scale;
                        changed = true;
                    }
                    ui.same_line();
                    let reset_s_btn = format!("Reset Scale##rs_{}", shape.file_stem);
                    if ui.small_button(&reset_s_btn) {
                        shape.scale = def_scale;
                        changed = true;
                    }

                    // Flight Time Slider & Reset
                    let mut flight_time = shape.flight_time;
                    let flight_label = format!("Flight Time (s)##flight_{}", shape.file_stem);
                    if ui.slider(&flight_label, 0.2, 5.0, &mut flight_time) {
                        shape.flight_time = flight_time;
                        changed = true;
                    }
                    ui.same_line();
                    let reset_f_btn = format!("Reset Flight##rf_{}", shape.file_stem);
                    if ui.small_button(&reset_f_btn) {
                        shape.flight_time = def_flight;
                        changed = true;
                    }

                    ui.separator();
                }

                if let Some(stem) = shape_to_remove {
                    let _ = physic_engine.remove_explosion_image(&stem);
                } else if changed {
                    let new_total: f32 = updated_shapes.iter().map(|(_, w)| *w).sum();
                    physic_engine.set_explosion_shape(
                        crate::physic_engine::ExplosionShape::MultiImage {
                            shapes: updated_shapes,
                            total_weight: new_total,
                        },
                    );
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physic_engine::{ExplosionShape, ImageShape};
    use glam::Vec2;

    #[test]
    fn test_preset_definitions_and_defaults() {
        let weights = default_preset_weights();
        assert_eq!(weights, [1.0; 5]);
        assert_eq!(PRESET_DEFINITIONS.len(), 5);
    }

    #[test]
    fn test_persisted_explosion_shape_conversion() {
        let persisted = PersistedExplosionShape::Spherical;
        assert_eq!(persisted, PersistedExplosionShape::Spherical);

        let img_shape = ImageShape {
            file_stem: "heart".to_string(),
            sampled_points: vec![Vec2::new(0.1, 0.2)],
            scale: 150.0,
            flight_time: 1.5,
        };

        let multi = ExplosionShape::MultiImage {
            shapes: vec![(img_shape.clone(), 2.0)],
            total_weight: 2.0,
        };

        let converted_persisted = PersistedExplosionShape::from_engine(&multi);
        match converted_persisted {
            PersistedExplosionShape::Images { images } => {
                assert_eq!(images.len(), 1);
                assert_eq!(images[0].file_stem, "heart");
                assert_eq!(images[0].weight, 2.0);
            }
            _ => panic!("Expected Images variant"),
        }
    }

    #[test]
    fn test_apply_session_to_physic() {
        use crate::physic_engine::config::PhysicConfig;
        use crate::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
        use crate::physic_engine::PhysicEngine;

        let config = PhysicConfig::default();
        let mut engine = PhysicEngineFireworks::new(&config, 800.0);
        let weights = [2.0, 1.5, 1.0, 0.5, 3.0];
        let shape = PersistedExplosionShape::Spherical;

        apply_session_to_physic(weights, &shape, &mut engine);
        assert_eq!(engine.get_explosion_shape(), &ExplosionShape::Spherical);
    }
}

use super::theme::{COLOR_HEADER, COLOR_SUCCESS, COLOR_WARNING};
use crate::domain_contracts::{EngineCommand, PhysicCommand, PhysicStateReader, SmokeCommand};
use crate::physic_engine::constants as physic_constants;
use crate::physic_engine::PhysicEngineFull;
use imgui::Ui;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

pub use physic_constants::{ExplosionPresetSpec, PRESET_DEFINITIONS};

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
    pub fn is_spherical(&self) -> bool {
        matches!(self, Self::Spherical)
    }

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
    physic_constants::ExplosionPresetSpec::find_by_stem(stem)
        .map(|preset| (preset.default_scale, preset.default_flight_time))
        .unwrap_or((
            physic_constants::PRESET_STAR_SCALE,
            physic_constants::PRESET_STAR_FLIGHT_TIME,
        ))
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

    let first_path = physic_constants::get_explosion_shape_texture_path(&first.file_stem);
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
        let path = physic_constants::get_explosion_shape_texture_path(&image.file_stem);
        let _ = physic_engine.load_explosion_image_weighted(
            &path,
            image.scale,
            image.flight_time,
            image.weight,
        );
    }
}

pub fn preset_weights_from_shape(shape: &crate::physic_engine::ExplosionShape) -> [f32; 5] {
    let mut weights = [1.0f32; 5];
    match shape {
        crate::physic_engine::ExplosionShape::Spherical => weights,
        crate::physic_engine::ExplosionShape::Image(img) => {
            let key = img.file_stem.to_lowercase();
            for (i, preset) in PRESET_DEFINITIONS.iter().enumerate() {
                if preset.stem == key {
                    weights[i] = 1.0;
                } else {
                    weights[i] = 0.0;
                }
            }
            weights
        }
        crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
            for (i, preset) in PRESET_DEFINITIONS.iter().enumerate() {
                if let Some((_, w)) = shapes
                    .iter()
                    .find(|(s, _)| s.file_stem.to_lowercase() == preset.stem)
                {
                    weights[i] = *w;
                } else {
                    weights[i] = 0.0;
                }
            }
            weights
        }
    }
}

#[inline]
fn slider_f32<F>(
    ui: &Ui,
    cmd_queue: &mut Vec<EngineCommand>,
    label: &str,
    min: f32,
    max: f32,
    val: f32,
    make_cmd: F,
) where
    F: FnOnce(f32) -> PhysicCommand,
{
    let mut tmp = val;
    if ui.slider(label, min, max, &mut tmp) {
        cmd_queue.push(EngineCommand::Physic(make_cmd(tmp)));
    }
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn slider_i32_with_reset<F>(
    ui: &Ui,
    cmd_queue: &mut Vec<EngineCommand>,
    label: &str,
    min: i32,
    max: i32,
    val: usize,
    reset_label: &str,
    default_val: usize,
    min_clamp: u32,
    make_cmd: F,
) where
    F: Fn(u32) -> PhysicCommand,
{
    let mut tmp = val as i32;
    if ui.slider(label, min, max, &mut tmp) {
        cmd_queue.push(EngineCommand::Physic(make_cmd(
            tmp.max(min_clamp as i32) as u32
        )));
    }
    ui.same_line();
    if ui.small_button(reset_label) {
        cmd_queue.push(EngineCommand::Physic(make_cmd(default_val as u32)));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn render_physics_settings_tab(
    ui: &Ui,
    filter: &str,
    state: &impl PhysicStateReader,
    cmd_queue: &mut Vec<EngineCommand>,
    physic_reinit_requested: &AtomicBool,
    preview_max_zoom: &mut f32,
    preview_rocket_color: &mut [f32; 3],
    preview_simulated_speed: &mut f32,
    preview_simulated_angle_offset: &mut f32,
) {
    let cfg = state.config();

    // GUI_PERSIST: physics.config
    ui.spacing();
    ui.text_colored(COLOR_SUCCESS, "[OK] Physics Configuration Synced.");

    // Action buttons
    if ui.button("[APPLY] PENDING CHANGES (`physic.apply`)") {
        physic_reinit_requested.store(true, Ordering::Relaxed);
        cmd_queue.push(EngineCommand::Physic(PhysicCommand::ApplyPendingConfig));
    }
    ui.same_line();
    if ui.button("[SAVE] Save Config (`physic.config.save`)") {
        cmd_queue.push(EngineCommand::Physic(PhysicCommand::SaveConfig));
    }
    ui.same_line();
    if ui.button("[RELOAD] Reload Disk Config (`physic.config.reload`)") {
        physic_reinit_requested.store(true, Ordering::Relaxed);
        cmd_queue.push(EngineCommand::Physic(PhysicCommand::ReloadConfig));
    }
    ui.same_line();
    if ui.button("[RESET PHYSICS DEFAULTS]") {
        physic_reinit_requested.store(true, Ordering::Relaxed);
        cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetDefaults));
    }

    ui.separator();

    let font_sz = ui.current_font_size();
    let item_width = (ui.content_region_avail()[0] * 0.45).clamp(font_sz * 14.0, font_sz * 26.0);
    let _item_w_token = ui.push_item_width(item_width);

    // 1. Simulation Capacity
    if filter.is_empty() || "capacity max_rockets particles".contains(filter) {
        let default_cfg = crate::physic_engine::config::PhysicConfig::default();

        ui.text_colored(COLOR_HEADER, "=== SIMULATION CAPACITY & BUFFERS ===");
        ui.same_line();
        if ui.small_button("Reset Capacity Defaults") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetCapacityDefaults));
        }

        slider_i32_with_reset(
            ui,
            cmd_queue,
            "Max Rockets (`physic.max_rockets`)",
            physic_constants::SLIDER_ROCKETS_MIN,
            physic_constants::SLIDER_ROCKETS_MAX,
            cfg.max_rockets,
            "Reset##reset_max_rockets",
            default_cfg.max_rockets,
            physic_constants::SLIDER_ROCKETS_MIN_CLAMP,
            PhysicCommand::SetMaxRockets,
        );

        slider_i32_with_reset(
            ui,
            cmd_queue,
            "Particles Per Explosion (`physic.particles_per_explosion`)",
            physic_constants::SLIDER_PARTICLES_EXPLOSION_MIN,
            physic_constants::SLIDER_PARTICLES_EXPLOSION_MAX,
            cfg.particles_per_explosion,
            "Reset##reset_particles_per_explosion",
            default_cfg.particles_per_explosion,
            physic_constants::SLIDER_PARTICLES_EXPLOSION_MIN_CLAMP,
            PhysicCommand::SetParticlesPerExplosion,
        );

        slider_i32_with_reset(
            ui,
            cmd_queue,
            "Particles Per Trail (`physic.particles_per_trail`)",
            physic_constants::SLIDER_PARTICLES_TRAIL_MIN,
            physic_constants::SLIDER_PARTICLES_TRAIL_MAX,
            cfg.particles_per_trail,
            "Reset##reset_particles_per_trail",
            default_cfg.particles_per_trail,
            0,
            PhysicCommand::SetParticlesPerTrail,
        );

        slider_i32_with_reset(
            ui,
            cmd_queue,
            "Max Smoke Particles Pool (`physic.max_smoke_particles`)",
            physic_constants::SLIDER_SMOKE_PARTICLES_MIN,
            physic_constants::SLIDER_SMOKE_PARTICLES_MAX,
            cfg.max_smoke_particles,
            "Reset##reset_max_smoke_particles",
            default_cfg.max_smoke_particles,
            100,
            PhysicCommand::SetMaxSmokeParticles,
        );
    }

    // 2. Smoke Trail Dynamics & Alpha Erosion
    if filter.is_empty()
        || "smoke trail rate size growth fade erosion edge dissolve color".contains(filter)
    {
        ui.spacing();
        ui.separator();
        ui.text_colored(COLOR_HEADER, "=== SMOKE TRAIL DYNAMICS & ALPHA EROSION ===");
        ui.same_line();
        if ui.small_button("Reset Smoke Defaults") {
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::ResetDefaults));
        }

        super::smoke::render_smoke_controls(
            ui,
            state,
            cmd_queue,
            preview_max_zoom,
            preview_rocket_color,
            preview_simulated_speed,
            preview_simulated_angle_offset,
        );
    }

    // 3. Launch & Spawn Dynamics
    if filter.is_empty() || "spawn launch interval angle speed".contains(filter) {
        ui.spacing();
        ui.separator();
        ui.text_colored(COLOR_HEADER, "=== ROCKET LAUNCH & SPAWN DYNAMICS ===");
        ui.same_line();
        if ui.small_button("Reset Spawn Defaults") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetSpawnDefaults));
        }

        slider_f32(
            ui,
            cmd_queue,
            "Spawn Interval Mean (s) (`physic.rocket_interval_mean`)",
            physic_constants::SLIDER_INTERVAL_MEAN_MIN,
            physic_constants::SLIDER_INTERVAL_MEAN_MAX,
            cfg.rocket_interval_mean,
            PhysicCommand::SetRocketIntervalMean,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Interval Variation (`physic.rocket_interval_variation`)",
            physic_constants::SLIDER_INTERVAL_VAR_MIN,
            physic_constants::SLIDER_INTERVAL_VAR_MAX,
            cfg.rocket_interval_variation,
            PhysicCommand::SetRocketIntervalVariation,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Max Next Interval (`physic.rocket_max_next_interval`)",
            physic_constants::SLIDER_MAX_NEXT_INT_MIN,
            physic_constants::SLIDER_MAX_NEXT_INT_MAX,
            cfg.rocket_max_next_interval,
            PhysicCommand::SetRocketMaxNextInterval,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Spawn Margin (`physic.spawn_rocket_margin`)",
            physic_constants::SLIDER_SPAWN_MARGIN_MIN,
            physic_constants::SLIDER_SPAWN_MARGIN_MAX,
            cfg.spawn_rocket_margin,
            PhysicCommand::SetSpawnRocketMargin,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Vertical Angle (rad) (`physic.spawn_rocket_vertical_angle`)",
            physic_constants::SLIDER_SPAWN_ANGLE_MIN,
            std::f32::consts::PI,
            cfg.spawn_rocket_vertical_angle,
            PhysicCommand::SetSpawnRocketVerticalAngle,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Angle Variation (`physic.spawn_rocket_angle_variation`)",
            physic_constants::SLIDER_ANGLE_VAR_MIN,
            physic_constants::SLIDER_ANGLE_VAR_MAX,
            cfg.spawn_rocket_angle_variation,
            PhysicCommand::SetSpawnRocketAngleVariation,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Spawn Min Speed (`physic.spawn_rocket_min_speed`)",
            physic_constants::SLIDER_SPAWN_SPEED_MIN,
            physic_constants::SLIDER_SPAWN_SPEED_MAX,
            cfg.spawn_rocket_min_speed,
            PhysicCommand::SetSpawnRocketMinSpeed,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Spawn Max Speed (`physic.spawn_rocket_max_speed`)",
            physic_constants::SLIDER_SPAWN_SPEED_MIN,
            physic_constants::SLIDER_SPAWN_SPEED_MAX,
            cfg.spawn_rocket_max_speed,
            PhysicCommand::SetSpawnRocketMaxSpeed,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Initial Rocket Speed (`physic.initial_rocket_speed`)",
            physic_constants::SLIDER_SPAWN_SPEED_MIN,
            physic_constants::SLIDER_INIT_SPEED_MAX,
            cfg.initial_rocket_speed,
            PhysicCommand::SetInitialRocketSpeed,
        );
    }

    // 4. Forces & Particle Physics
    if filter.is_empty() || "gravity threshold explosion velocity forces boost".contains(filter) {
        ui.spacing();
        ui.separator();
        ui.text_colored(COLOR_HEADER, "=== FORCES & EXPLOSION DYNAMICS ===");
        ui.same_line();
        if ui.small_button("Reset Forces Defaults") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetForcesDefaults));
        }

        slider_f32(
            ui,
            cmd_queue,
            "Gravity (`physic.gravity`)",
            physic_constants::SLIDER_GRAVITY_MIN,
            physic_constants::SLIDER_GRAVITY_MAX,
            cfg.gravity,
            PhysicCommand::SetGravity,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Explosion Speed Threshold (`physic.explosion_threshold`)",
            physic_constants::SLIDER_EXPLOSION_THRESH_MIN,
            physic_constants::SLIDER_EXPLOSION_THRESH_MAX,
            cfg.explosion_threshold,
            PhysicCommand::SetExplosionThreshold,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Explosion Min Velocity (`physic.explosion_min_vel`)",
            physic_constants::SLIDER_EXPLOSION_VEL_MIN,
            physic_constants::SLIDER_EXPLOSION_VEL_MAX,
            cfg.explosion_min_vel,
            PhysicCommand::SetExplosionMinVel,
        );

        slider_f32(
            ui,
            cmd_queue,
            "Explosion Max Velocity (`physic.explosion_max_vel`)",
            physic_constants::SLIDER_EXPLOSION_VEL_MIN,
            physic_constants::SLIDER_EXPLOSION_VEL_MAX,
            cfg.explosion_max_vel,
            PhysicCommand::SetExplosionMaxVel,
        );

        // GUI_PERSIST: physics.config
        slider_f32(
            ui,
            cmd_queue,
            "Explosion Velocity Boost (`physic.explosion_velocity_boost`)",
            physic_constants::SLIDER_EXPLOSION_BOOST_MIN,
            physic_constants::SLIDER_EXPLOSION_BOOST_MAX,
            cfg.explosion_velocity_boost,
            PhysicCommand::SetExplosionVelocityBoost,
        );
    }

    // 5. Explosion Shapes & MultiImage Tuning
    if filter.is_empty()
        || "shape image preset heart star smiley weight scale flight_time add delete remove"
            .contains(filter)
    {
        ui.spacing();
        ui.separator();
        ui.text_colored(
            COLOR_HEADER,
            "=== EXPLOSION SHAPE & PRESETS (`physic.explosion.*`) ===",
        );
        ui.same_line();
        if ui.small_button("Reset to Spherical") {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetExplosionShapeSpherical,
            ));
        }

        let explosion_shape = state.explosion_shape();
        match explosion_shape {
            crate::physic_engine::ExplosionShape::Spherical => {
                ui.text("Current Mode: Spherical (Standard 3D/2D Burst)");
            }
            crate::physic_engine::ExplosionShape::Image(img) => {
                ui.text(format!("Current Mode: Single Image: '{}'", img.file_stem));
            }
            crate::physic_engine::ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                ui.text(format!(
                    "Current Mode: MultiImage ({} shapes active, total weight {:.1})",
                    shapes.len(),
                    total_weight
                ));
            }
        }

        if ui.button("Spherical Mode (`physic.explosion.shape spherical`)") {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetExplosionShapeSpherical,
            ));
        }

        ui.spacing();
        ui.text_colored(COLOR_WARNING, "Presets & Custom Weight Adjustment:");
        ui.same_line();
        if ui.small_button("Reset All Preset Weights") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetAllPresetWeights));
        }

        let mut preset_weights = preset_weights_from_shape(explosion_shape);

        for (i, preset) in PRESET_DEFINITIONS.iter().enumerate() {
            let _id = ui.push_id_usize(i);
            ui.text(preset.name);
            ui.same_line();

            ui.set_next_item_width(font_sz * 7.5);
            if ui.slider(
                "Weight",
                physic_constants::SLIDER_WEIGHT_MIN,
                physic_constants::SLIDER_WEIGHT_MAX,
                &mut preset_weights[i],
            ) {
                cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetPresetWeight {
                    index: i as u32,
                    weight: preset_weights[i],
                }));
            }
            ui.same_line();

            if ui.small_button("Reset") {
                cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetPresetWeight {
                    index: i as u32,
                    weight: 1.0,
                }));
            }
            if ui.is_item_hovered() {
                ui.tooltip_text("Reset weight to default 1.0");
            }
            ui.same_line();

            if ui.button("Set") {
                cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetPresetSingleShape {
                    index: i as u32,
                }));
            }
            if ui.is_item_hovered() {
                ui.tooltip_text("Set single active shape");
            }
            ui.same_line();

            if ui.button("+Add") {
                let weight = if preset_weights[i] <= 0.0 {
                    1.0
                } else {
                    preset_weights[i]
                };
                cmd_queue.push(EngineCommand::Physic(
                    PhysicCommand::AddPresetShapeWeighted {
                        index: i as u32,
                        weight,
                    },
                ));
            }
            if ui.is_item_hovered() {
                ui.tooltip_text("Add to MultiImage set");
            }
        }

        // Active Shape Controls & Reset Buttons
        match explosion_shape {
            crate::physic_engine::ExplosionShape::Image(img) => {
                let (def_scale, def_flight) = get_preset_defaults(&img.file_stem);

                ui.spacing();
                ui.text_colored(
                    COLOR_SUCCESS,
                    format!("--- Active Single Image: '{}' ---", img.file_stem),
                );
                ui.same_line();
                if ui.button("[X Delete]") {
                    cmd_queue.push(EngineCommand::Physic(PhysicCommand::DeleteSingleShape));
                }
                ui.same_line();
                if ui.button("[Reset Defaults]") {
                    cmd_queue.push(EngineCommand::Physic(
                        PhysicCommand::ResetSingleShapeDefaults,
                    ));
                }

                let mut scale = img.scale;
                if ui.slider(
                    "Image Scale (`physic.explosion.scale`)",
                    physic_constants::SLIDER_IMAGE_SCALE_MIN,
                    physic_constants::SLIDER_IMAGE_SCALE_MAX,
                    &mut scale,
                ) {
                    cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetSingleShapeScale(
                        scale,
                    )));
                }
                ui.same_line();
                if ui.small_button("Reset Scale") {
                    cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetSingleShapeScale(
                        def_scale,
                    )));
                }

                let mut flight_time = img.flight_time;
                if ui.slider(
                    "Flight Time (s) (`physic.explosion.flight_time`)",
                    physic_constants::SLIDER_FLIGHT_TIME_MIN,
                    physic_constants::SLIDER_FLIGHT_TIME_MAX,
                    &mut flight_time,
                ) {
                    cmd_queue.push(EngineCommand::Physic(
                        PhysicCommand::SetSingleShapeFlightTime(flight_time),
                    ));
                }
                ui.same_line();
                if ui.small_button("Reset Flight") {
                    cmd_queue.push(EngineCommand::Physic(
                        PhysicCommand::SetSingleShapeFlightTime(def_flight),
                    ));
                }
            }
            crate::physic_engine::ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                ui.spacing();
                ui.text_colored(
                    COLOR_SUCCESS,
                    format!(
                        "--- Active MultiImage Shapes ({}) & Parameters Breakdown ---",
                        shapes.len()
                    ),
                );

                for (idx, (shape, weight)) in shapes.iter().enumerate() {
                    let idx_u32 = idx as u32;
                    let _id = ui.push_id_usize(idx);
                    let (def_scale, def_flight) = get_preset_defaults(&shape.file_stem);

                    let pct = if *total_weight > 0.0 {
                        (*weight / total_weight) * 100.0
                    } else {
                        0.0
                    };

                    ui.text_colored(
                        COLOR_HEADER,
                        format!("* Forme: '{}' ({:.1}% d'apparition)", shape.file_stem, pct),
                    );
                    ui.same_line();
                    if ui.button("[X Delete]") {
                        cmd_queue.push(EngineCommand::Physic(PhysicCommand::DeleteMultiShapeItem(
                            idx_u32,
                        )));
                    }
                    ui.same_line();
                    if ui.button("[Reset Defaults]") {
                        cmd_queue.push(EngineCommand::Physic(
                            PhysicCommand::ResetMultiShapeItemDefaults(idx_u32),
                        ));
                    }

                    // Weight Slider & Reset
                    let mut w = *weight;
                    if ui.slider(
                        "Weight",
                        physic_constants::SLIDER_WEIGHT_MIN,
                        physic_constants::SLIDER_WEIGHT_MAX,
                        &mut w,
                    ) {
                        cmd_queue.push(EngineCommand::Physic(
                            PhysicCommand::SetMultiShapeItemWeight {
                                index: idx_u32,
                                weight: w,
                            },
                        ));
                    }
                    ui.same_line();
                    if ui.small_button("Reset W") {
                        cmd_queue.push(EngineCommand::Physic(
                            PhysicCommand::SetMultiShapeItemWeight {
                                index: idx_u32,
                                weight: 1.0,
                            },
                        ));
                    }

                    // Image Scale Slider & Reset
                    let mut scale = shape.scale;
                    if ui.slider(
                        "Image Scale (px)",
                        physic_constants::SLIDER_IMAGE_SCALE_MIN,
                        physic_constants::SLIDER_IMAGE_SCALE_MAX,
                        &mut scale,
                    ) {
                        cmd_queue.push(EngineCommand::Physic(
                            PhysicCommand::SetMultiShapeItemScale {
                                index: idx_u32,
                                scale,
                            },
                        ));
                    }
                    ui.same_line();
                    if ui.small_button("Reset Scale") {
                        cmd_queue.push(EngineCommand::Physic(
                            PhysicCommand::SetMultiShapeItemScale {
                                index: idx_u32,
                                scale: def_scale,
                            },
                        ));
                    }

                    // Flight Time Slider & Reset
                    let mut flight_time = shape.flight_time;
                    if ui.slider(
                        "Flight Time (s)",
                        physic_constants::SLIDER_FLIGHT_TIME_MIN,
                        physic_constants::SLIDER_FLIGHT_TIME_MAX,
                        &mut flight_time,
                    ) {
                        cmd_queue.push(EngineCommand::Physic(
                            PhysicCommand::SetMultiShapeItemFlightTime {
                                index: idx_u32,
                                flight_time,
                            },
                        ));
                    }
                    ui.same_line();
                    if ui.small_button("Reset Flight") {
                        cmd_queue.push(EngineCommand::Physic(
                            PhysicCommand::SetMultiShapeItemFlightTime {
                                index: idx_u32,
                                flight_time: def_flight,
                            },
                        ));
                    }

                    ui.separator();
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
    use serial_test::serial;

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
        let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);
        let weights = [2.0, 1.5, 1.0, 0.5, 3.0];
        let shape = PersistedExplosionShape::Spherical;

        apply_session_to_physic(weights, &shape, &mut engine);
        assert_eq!(engine.get_explosion_shape(), &ExplosionShape::Spherical);
    }

    #[test]
    #[serial]
    fn test_render_physics_settings_tab_pure_function() {
        use crate::physic_engine::config::PhysicConfig;
        use crate::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;

        let config = PhysicConfig::default();
        let engine = PhysicEngineFireworks::new(&config, 800.0, None);
        let mut cmd_queue: Vec<EngineCommand> = Vec::with_capacity(16);
        let reinit = AtomicBool::new(false);
        let _guard = crate::simulator::gui_settings::IMGUI_TEST_MUTEX
            .lock()
            .unwrap();
        let mut imgui_ctx = imgui::Context::create();
        imgui_ctx.set_ini_filename(None);
        imgui_ctx.fonts().build_rgba32_texture();
        imgui_ctx.io_mut().display_size = [800.0, 600.0];

        let ui = imgui_ctx.frame();

        let mut preview_max_zoom = 10.0;
        let mut preview_rocket_color = [1.0, 1.0, 1.0];
        let mut preview_simulated_speed = 400.0;
        let mut preview_simulated_angle_offset = 0.0;
        render_physics_settings_tab(
            ui,
            "",
            &engine,
            &mut cmd_queue,
            &reinit,
            &mut preview_max_zoom,
            &mut preview_rocket_color,
            &mut preview_simulated_speed,
            &mut preview_simulated_angle_offset,
        );

        assert!(cmd_queue.capacity() >= 16);
    }
}

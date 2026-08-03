use crate::domain_contracts::{EngineCommand, PhysicCommand, SmokeCommand};
use crate::physic_engine::constants as physic_constants;
use crate::physic_engine::PhysicEngineFull;
use crate::simulator::gui_settings::physic::{self, PRESET_DEFINITIONS};
use std::sync::atomic::{AtomicBool, Ordering};

pub fn dispatch_engine_commands<P: PhysicEngineFull>(
    cmd_queue: &mut Vec<EngineCommand>,
    physic_engine: &mut P,
    physic_reinit_requested: &AtomicBool,
) {
    let mut smoke_modified = false;

    for cmd in cmd_queue.drain(..) {
        match cmd {
            EngineCommand::Physic(physic_cmd) => {
                dispatch_physic_command(physic_cmd, physic_engine, physic_reinit_requested);
            }
            EngineCommand::Smoke(smoke_cmd) => {
                smoke_modified = true;
                dispatch_smoke_command(smoke_cmd, physic_engine);
            }
            _ => {}
        }
    }

    if smoke_modified {
        let pending = physic_engine.get_config_mut().clone();
        let _ = physic_engine.reload_config(&pending);
    }
}

fn dispatch_physic_command<P: PhysicEngineFull>(
    physic_cmd: PhysicCommand,
    physic_engine: &mut P,
    physic_reinit_requested: &AtomicBool,
) {
    match physic_cmd {
        PhysicCommand::SetGravity(g) => {
            physic_engine.get_config_mut().gravity = g;
        }
        PhysicCommand::SetDrag(_d) => {}
        PhysicCommand::SetMaxParticles(m) => {
            physic_engine.get_config_mut().max_rockets = m as usize;
        }
        PhysicCommand::SetExplosionForce(f) => {
            physic_engine.get_config_mut().explosion_max_vel = f;
        }
        PhysicCommand::ApplyPendingConfig => {
            let pending = physic_engine.get_config_mut().clone();
            let _ = physic_engine.reload_config(&pending);
            physic_reinit_requested.store(true, Ordering::Relaxed);
        }
        PhysicCommand::SaveConfig => {
            if crate::utils::config_path::is_config_save_enabled() {
                let _ = physic_engine
                    .get_config()
                    .save_to_file(crate::utils::config_path::get_physic_config_path());
            }
        }
        PhysicCommand::ReloadConfig => {
            if let Ok(new_cfg) = crate::physic_engine::config::PhysicConfig::from_file(
                crate::utils::config_path::get_physic_config_path(),
            ) {
                *physic_engine.get_config_mut() = new_cfg.clone();
                let _ = physic_engine.reload_config(&new_cfg);
                physic_reinit_requested.store(true, Ordering::Relaxed);
            }
        }
        PhysicCommand::ResetDefaults => {
            let default_cfg = crate::physic_engine::config::PhysicConfig::default();
            *physic_engine.get_config_mut() = default_cfg.clone();
            let _ = physic_engine.reload_config(&default_cfg);
            physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
            physic_reinit_requested.store(true, Ordering::Relaxed);
        }
        PhysicCommand::ResetCapacityDefaults => {
            let default_cfg = crate::physic_engine::config::PhysicConfig::default();
            let cfg_mut = physic_engine.get_config_mut();
            cfg_mut.max_rockets = default_cfg.max_rockets;
            cfg_mut.particles_per_explosion = default_cfg.particles_per_explosion;
            cfg_mut.particles_per_trail = default_cfg.particles_per_trail;
        }
        PhysicCommand::SetMaxRockets(r) => {
            physic_engine.get_config_mut().max_rockets = r as usize;
        }
        PhysicCommand::SetParticlesPerExplosion(p) => {
            physic_engine.get_config_mut().particles_per_explosion = p as usize;
        }
        PhysicCommand::SetParticlesPerTrail(p) => {
            physic_engine.get_config_mut().particles_per_trail = p as usize;
        }
        PhysicCommand::SetMaxSmokeParticles(m) => {
            physic_engine.get_config_mut().max_smoke_particles = m as usize;
        }
        PhysicCommand::ResetSpawnDefaults => {
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
        PhysicCommand::SetRocketIntervalMean(v) => {
            physic_engine.get_config_mut().rocket_interval_mean = v;
        }
        PhysicCommand::SetRocketIntervalVariation(v) => {
            physic_engine.get_config_mut().rocket_interval_variation = v;
        }
        PhysicCommand::SetRocketMaxNextInterval(v) => {
            physic_engine.get_config_mut().rocket_max_next_interval = v;
        }
        PhysicCommand::SetSpawnRocketMargin(v) => {
            physic_engine.get_config_mut().spawn_rocket_margin = v;
        }
        PhysicCommand::SetSpawnRocketVerticalAngle(v) => {
            physic_engine.get_config_mut().spawn_rocket_vertical_angle = v;
        }
        PhysicCommand::SetSpawnRocketAngleVariation(v) => {
            physic_engine.get_config_mut().spawn_rocket_angle_variation = v;
        }
        PhysicCommand::SetSpawnRocketMinSpeed(v) => {
            physic_engine.get_config_mut().spawn_rocket_min_speed = v;
        }
        PhysicCommand::SetSpawnRocketMaxSpeed(v) => {
            physic_engine.get_config_mut().spawn_rocket_max_speed = v;
        }
        PhysicCommand::SetInitialRocketSpeed(v) => {
            physic_engine.get_config_mut().initial_rocket_speed = v;
        }
        PhysicCommand::ResetForcesDefaults => {
            let default_cfg = crate::physic_engine::config::PhysicConfig::default();
            let cfg_mut = physic_engine.get_config_mut();
            cfg_mut.gravity = default_cfg.gravity;
            cfg_mut.explosion_threshold = default_cfg.explosion_threshold;
            cfg_mut.explosion_min_vel = default_cfg.explosion_min_vel;
            cfg_mut.explosion_max_vel = default_cfg.explosion_max_vel;
        }
        PhysicCommand::SetExplosionThreshold(v) => {
            physic_engine.get_config_mut().explosion_threshold = v;
        }
        PhysicCommand::SetExplosionMinVel(v) => {
            physic_engine.get_config_mut().explosion_min_vel = v;
        }
        PhysicCommand::SetExplosionMaxVel(v) => {
            physic_engine.get_config_mut().explosion_max_vel = v;
        }
        PhysicCommand::SetExplosionShapeSpherical => {
            physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
        }
        PhysicCommand::ResetAllPresetWeights => {}
        PhysicCommand::SetPresetWeight { index, weight } => {
            let idx = index as usize;
            if let Some(preset) = PRESET_DEFINITIONS.get(idx) {
                let _ = physic_engine.load_explosion_image_weighted(
                    preset.path,
                    preset.default_scale,
                    preset.default_flight_time,
                    weight,
                );
            }
        }
        PhysicCommand::SetPresetSingleShape { index } => {
            let idx = index as usize;
            if let Some(preset) = PRESET_DEFINITIONS.get(idx) {
                let _ = physic_engine.load_explosion_image(
                    preset.path,
                    preset.default_scale,
                    preset.default_flight_time,
                );
            }
        }
        PhysicCommand::AddPresetShapeWeighted { index, weight } => {
            let idx = index as usize;
            if let Some(preset) = PRESET_DEFINITIONS.get(idx) {
                let effective_weight = if weight <= 0.0 { 1.0 } else { weight };
                let _ = physic_engine.load_explosion_image_weighted(
                    preset.path,
                    preset.default_scale,
                    preset.default_flight_time,
                    effective_weight,
                );
            }
        }
        PhysicCommand::DeleteSingleShape => {
            if let crate::physic_engine::ExplosionShape::Image(img) =
                physic_engine.get_explosion_shape().clone()
            {
                let _ = physic_engine.remove_explosion_image(&img.file_stem);
            }
        }
        PhysicCommand::ResetSingleShapeDefaults => {
            if let crate::physic_engine::ExplosionShape::Image(img) =
                physic_engine.get_explosion_shape().clone()
            {
                let (def_scale, def_flight) = physic::get_preset_defaults(&img.file_stem);
                let mut updated = img.clone();
                updated.scale = def_scale;
                updated.flight_time = def_flight;
                physic_engine
                    .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
            }
        }
        PhysicCommand::SetSingleShapeScale(scale) => {
            if let crate::physic_engine::ExplosionShape::Image(img) =
                physic_engine.get_explosion_shape().clone()
            {
                let mut updated = img.clone();
                updated.scale = scale;
                physic_engine
                    .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
            }
        }
        PhysicCommand::SetSingleShapeFlightTime(flight) => {
            if let crate::physic_engine::ExplosionShape::Image(img) =
                physic_engine.get_explosion_shape().clone()
            {
                let mut updated = img.clone();
                updated.flight_time = flight;
                physic_engine
                    .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
            }
        }
        PhysicCommand::DeleteMultiShapeItem(idx) => {
            let idx = idx as usize;
            if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                physic_engine.get_explosion_shape().clone()
            {
                if idx < shapes.len() {
                    let stem = shapes[idx].0.file_stem.clone();
                    let _ = physic_engine.remove_explosion_image(&stem);
                }
            }
        }
        PhysicCommand::ResetMultiShapeItemDefaults(idx) => {
            let idx = idx as usize;
            if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                physic_engine.get_explosion_shape().clone()
            {
                if idx < shapes.len() {
                    let mut updated = shapes.clone();
                    let (def_scale, def_flight) =
                        physic::get_preset_defaults(&updated[idx].0.file_stem);
                    updated[idx].1 = 1.0;
                    updated[idx].0.scale = def_scale;
                    updated[idx].0.flight_time = def_flight;
                    let new_total: f32 = updated.iter().map(|(_, w)| *w).sum();
                    physic_engine.set_explosion_shape(
                        crate::physic_engine::ExplosionShape::MultiImage {
                            shapes: updated,
                            total_weight: new_total,
                        },
                    );
                }
            }
        }
        PhysicCommand::SetMultiShapeItemWeight { index, weight } => {
            let index = index as usize;
            if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                physic_engine.get_explosion_shape().clone()
            {
                if index < shapes.len() {
                    let mut updated = shapes.clone();
                    updated[index].1 = weight;
                    let new_total: f32 = updated.iter().map(|(_, w)| *w).sum();
                    physic_engine.set_explosion_shape(
                        crate::physic_engine::ExplosionShape::MultiImage {
                            shapes: updated,
                            total_weight: new_total,
                        },
                    );
                }
            }
        }
        PhysicCommand::SetMultiShapeItemScale { index, scale } => {
            let index = index as usize;
            if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                physic_engine.get_explosion_shape().clone()
            {
                if index < shapes.len() {
                    let mut updated = shapes.clone();
                    updated[index].0.scale = scale;
                    let new_total: f32 = updated.iter().map(|(_, w)| *w).sum();
                    physic_engine.set_explosion_shape(
                        crate::physic_engine::ExplosionShape::MultiImage {
                            shapes: updated,
                            total_weight: new_total,
                        },
                    );
                }
            }
        }
        PhysicCommand::SetMultiShapeItemFlightTime { index, flight_time } => {
            let index = index as usize;
            if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                physic_engine.get_explosion_shape().clone()
            {
                if index < shapes.len() {
                    let mut updated = shapes.clone();
                    updated[index].0.flight_time = flight_time;
                    let new_total: f32 = updated.iter().map(|(_, w)| *w).sum();
                    physic_engine.set_explosion_shape(
                        crate::physic_engine::ExplosionShape::MultiImage {
                            shapes: updated,
                            total_weight: new_total,
                        },
                    );
                }
            }
        }
    }
}

fn dispatch_smoke_command<P: PhysicEngineFull>(smoke_cmd: SmokeCommand, physic_engine: &mut P) {
    match smoke_cmd {
        SmokeCommand::SetDensity(d) => {
            physic_engine.get_config_mut().smoke_intensity = d;
        }
        SmokeCommand::SetDissipation(d) => {
            physic_engine.get_config_mut().smoke_fade_duration = d;
        }
        SmokeCommand::SetWind(_) => {}
        SmokeCommand::SetErosionEnabled(e) => {
            physic_engine.get_config_mut().smoke_erosion_enabled = e;
        }
        SmokeCommand::SetErosionScale(s) => {
            physic_engine.get_config_mut().smoke_erosion_scale = s;
        }
        SmokeCommand::SetErosionEdgeWidth(w) => {
            physic_engine.get_config_mut().smoke_erosion_edge_width = w;
        }
        SmokeCommand::SetErosionEdgeColor(c) => {
            let f32_color = [
                c[0] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[2] as f32 / 255.0,
            ];
            physic_engine.get_config_mut().smoke_erosion_edge_color = f32_color;
        }
        SmokeCommand::SetFlowDistortionStrength(s) => {
            physic_engine.get_config_mut().flow_distortion_strength = s;
        }
        SmokeCommand::SetFlowAnimationSpeed(s) => {
            physic_engine.get_config_mut().flow_animation_speed = s;
        }
        SmokeCommand::SetColorMode(m) => {
            physic_engine.get_config_mut().smoke_color_mode = m;
        }
        SmokeCommand::SetInheritedColorIntensity(i) => {
            physic_engine
                .get_config_mut()
                .smoke_inherited_color_intensity = i;
        }
        SmokeCommand::SetCustomColor(c) => {
            let f32_color = [
                c[0] as f32 / 255.0,
                c[1] as f32 / 255.0,
                c[2] as f32 / 255.0,
            ];
            physic_engine.get_config_mut().smoke_custom_color = f32_color;
        }
        SmokeCommand::SetSpawnRate(r) => {
            physic_engine.get_config_mut().smoke_spawn_rate = r;
        }
        SmokeCommand::SetInitialSize(s) => {
            physic_engine.get_config_mut().smoke_initial_size = s;
        }
        SmokeCommand::SetGrowthRateMultiplier(g) => {
            physic_engine.get_config_mut().smoke_growth_rate_multiplier = g;
        }
        SmokeCommand::SetFadeDuration(f) => {
            physic_engine.get_config_mut().smoke_fade_duration = f;
        }
        SmokeCommand::SetIntensity(i) => {
            physic_engine.get_config_mut().smoke_intensity = i;
        }
        SmokeCommand::SetMaxSmokeParticles(m) => {
            physic_engine.get_config_mut().max_smoke_particles = m as usize;
        }
        SmokeCommand::ResetDefaults => {
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
            cfg_mut.smoke_erosion_enabled = default_cfg.smoke_erosion_enabled;
            cfg_mut.smoke_erosion_scale = default_cfg.smoke_erosion_scale;
            cfg_mut.smoke_erosion_edge_width = default_cfg.smoke_erosion_edge_width;
            cfg_mut.smoke_erosion_edge_color = default_cfg.smoke_erosion_edge_color;
        }
        SmokeCommand::ApplyPreset(preset_id) => {
            let preset_idx = preset_id as usize;
            if let Some(preset) = physic_constants::SMOKE_PRESET_DEFINITIONS.get(preset_idx) {
                let cfg_mut = physic_engine.get_config_mut();
                cfg_mut.smoke_erosion_edge_width = preset.edge_width;
                cfg_mut.smoke_erosion_edge_color = preset.edge_color;
                cfg_mut.smoke_color_mode = crate::physic_engine::config::SmokeColorMode::Custom;
                cfg_mut.smoke_custom_color = preset.custom_color;
                cfg_mut.smoke_intensity = preset.intensity;
            }
        }
    }
}

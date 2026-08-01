use crate::domain_contracts::{EngineCommand, PhysicCommand, PhysicStateReader, SmokeCommand};
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

pub fn preset_weights_from_shape(shape: &crate::physic_engine::ExplosionShape) -> [f32; 5] {
    let mut weights = [1.0f32; 5];
    match shape {
        crate::physic_engine::ExplosionShape::Spherical => weights,
        crate::physic_engine::ExplosionShape::Image(img) => {
            let key = img.file_stem.to_lowercase();
            for (i, (_, stem, _, _, _)) in PRESET_DEFINITIONS.iter().enumerate() {
                if *stem == key {
                    weights[i] = 1.0;
                } else {
                    weights[i] = 0.0;
                }
            }
            weights
        }
        crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
            for (i, (_, stem, _, _, _)) in PRESET_DEFINITIONS.iter().enumerate() {
                if let Some((_, w)) = shapes
                    .iter()
                    .find(|(s, _)| s.file_stem.to_lowercase() == *stem)
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

pub fn render_physics_settings_tab(
    ui: &Ui,
    filter: &str,
    state: &impl PhysicStateReader,
    cmd_queue: &mut Vec<EngineCommand>,
    physic_reinit_requested: &AtomicBool,
) {
    let cfg = state.config();

    // GUI_PERSIST: physics.config
    ui.spacing();

    ui.text_colored([0.0, 1.0, 0.4, 1.0], "[OK] Physics Configuration Synced.");

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

    // Use a responsive item width (45% of content region, capped 200px..360px) so labels have plenty of space on the right
    let item_width = (ui.content_region_avail()[0] * 0.45).clamp(200.0, 360.0);
    let _item_w_token = ui.push_item_width(item_width);

    // 1. Simulation Capacity
    if filter.is_empty() || "capacity max_rockets particles".contains(filter) {
        let default_cfg = crate::physic_engine::config::PhysicConfig::default();

        ui.text_colored(
            [0.4, 0.8, 1.0, 1.0],
            "=== SIMULATION CAPACITY & BUFFERS ===",
        );
        ui.same_line();
        if ui.small_button("Reset Capacity Defaults") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetCapacityDefaults));
        }

        let mut max_rockets = cfg.max_rockets as i32;
        if ui.slider(
            "Max Rockets (`physic.max_rockets`)",
            1,
            100,
            &mut max_rockets,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetMaxRockets(
                max_rockets.max(1) as u32,
            )));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_max_rockets") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetMaxRockets(
                default_cfg.max_rockets as u32,
            )));
        }

        let mut p_explosion = cfg.particles_per_explosion as i32;
        if ui.slider(
            "Particles per Explosion (`physic.particles_per_explosion`)",
            10,
            1000,
            &mut p_explosion,
        ) {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetParticlesPerExplosion(p_explosion.max(10) as u32),
            ));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_particles_per_explosion") {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetParticlesPerExplosion(default_cfg.particles_per_explosion as u32),
            ));
        }

        let mut p_trail = cfg.particles_per_trail as i32;
        if ui.slider(
            "Particles per Trail (`physic.particles_per_trail`)",
            0,
            200,
            &mut p_trail,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetParticlesPerTrail(
                p_trail.max(0) as u32,
            )));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_particles_per_trail") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetParticlesPerTrail(
                default_cfg.particles_per_trail as u32,
            )));
        }

        let mut max_smoke = cfg.max_smoke_particles as i32;
        if ui.slider(
            "Max Smoke Particles Pool (`physic.max_smoke_particles`)",
            100,
            16384,
            &mut max_smoke,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetMaxSmokeParticles(
                max_smoke.max(100) as u32,
            )));
        }
        ui.same_line();
        if ui.small_button("Reset##reset_max_smoke_particles") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetMaxSmokeParticles(
                default_cfg.max_smoke_particles as u32,
            )));
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
            cmd_queue.push(EngineCommand::Smoke(SmokeCommand::ResetDefaults));
        }

        super::smoke::render_smoke_controls(ui, state, cmd_queue);
    }

    // 3. Launch & Spawn Dynamics
    if filter.is_empty() || "spawn launch interval angle speed".contains(filter) {
        ui.spacing();
        ui.separator();
        ui.text_colored(
            [0.4, 0.8, 1.0, 1.0],
            "=== ROCKET LAUNCH & SPAWN DYNAMICS ===",
        );
        ui.same_line();
        if ui.small_button("Reset Spawn Defaults") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetSpawnDefaults));
        }

        let mut interval_mean = cfg.rocket_interval_mean;
        if ui.slider(
            "Spawn Interval Mean (s) (`physic.rocket_interval_mean`)",
            0.05,
            5.0,
            &mut interval_mean,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetRocketIntervalMean(
                interval_mean,
            )));
        }

        let mut interval_var = cfg.rocket_interval_variation;
        if ui.slider(
            "Interval Variation (`physic.rocket_interval_variation`)",
            0.0,
            3.0,
            &mut interval_var,
        ) {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetRocketIntervalVariation(interval_var),
            ));
        }

        let mut max_next = cfg.rocket_max_next_interval;
        if ui.slider(
            "Max Next Interval (`physic.rocket_max_next_interval`)",
            0.1,
            10.0,
            &mut max_next,
        ) {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetRocketMaxNextInterval(max_next),
            ));
        }

        let mut margin = cfg.spawn_rocket_margin;
        if ui.slider(
            "Spawn Margin (`physic.spawn_rocket_margin`)",
            0.0,
            200.0,
            &mut margin,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetSpawnRocketMargin(
                margin,
            )));
        }

        let mut vert_angle = cfg.spawn_rocket_vertical_angle;
        if ui.slider(
            "Vertical Angle (rad) (`physic.spawn_rocket_vertical_angle`)",
            0.0,
            std::f32::consts::PI,
            &mut vert_angle,
        ) {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetSpawnRocketVerticalAngle(vert_angle),
            ));
        }

        let mut angle_var = cfg.spawn_rocket_angle_variation;
        if ui.slider(
            "Angle Variation (`physic.spawn_rocket_angle_variation`)",
            0.0,
            1.57,
            &mut angle_var,
        ) {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetSpawnRocketAngleVariation(angle_var),
            ));
        }

        let mut min_speed = cfg.spawn_rocket_min_speed;
        if ui.slider(
            "Spawn Min Speed (`physic.spawn_rocket_min_speed`)",
            10.0,
            1000.0,
            &mut min_speed,
        ) {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetSpawnRocketMinSpeed(min_speed),
            ));
        }

        let mut max_speed = cfg.spawn_rocket_max_speed;
        if ui.slider(
            "Spawn Max Speed (`physic.spawn_rocket_max_speed`)",
            10.0,
            2000.0,
            &mut max_speed,
        ) {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetSpawnRocketMaxSpeed(max_speed),
            ));
        }

        let mut init_speed = cfg.initial_rocket_speed;
        if ui.slider(
            "Initial Rocket Speed (`physic.initial_rocket_speed`)",
            10.0,
            1500.0,
            &mut init_speed,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetInitialRocketSpeed(
                init_speed,
            )));
        }
    }

    // 4. Forces & Particle Physics
    if filter.is_empty() || "gravity threshold explosion velocity forces".contains(filter) {
        ui.spacing();
        ui.separator();
        ui.text_colored([0.4, 0.8, 1.0, 1.0], "=== FORCES & EXPLOSION DYNAMICS ===");
        ui.same_line();
        if ui.small_button("Reset Forces Defaults") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetForcesDefaults));
        }

        let mut gravity = cfg.gravity;
        if ui.slider("Gravity (`physic.gravity`)", -2000.0, 2000.0, &mut gravity) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetGravity(gravity)));
        }

        let mut threshold = cfg.explosion_threshold;
        if ui.slider(
            "Explosion Speed Threshold (`physic.explosion_threshold`)",
            0.0,
            500.0,
            &mut threshold,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetExplosionThreshold(
                threshold,
            )));
        }

        let mut min_vel = cfg.explosion_min_vel;
        if ui.slider(
            "Explosion Min Velocity (`physic.explosion_min_vel`)",
            1.0,
            1000.0,
            &mut min_vel,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetExplosionMinVel(
                min_vel,
            )));
        }

        let mut max_vel = cfg.explosion_max_vel;
        if ui.slider(
            "Explosion Max Velocity (`physic.explosion_max_vel`)",
            1.0,
            2000.0,
            &mut max_vel,
        ) {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::SetExplosionMaxVel(
                max_vel,
            )));
        }
    }

    // 5. Explosion Shapes & MultiImage Tuning
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
                let mut buf = [0u8; 128];
                let mut cursor = std::io::Cursor::new(&mut buf[..]);
                if std::io::Write::write_fmt(
                    &mut cursor,
                    format_args!("Current Mode: Single Image: '{}'", img.file_stem),
                )
                .is_ok()
                {
                    let pos = cursor.position() as usize;
                    if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                        ui.text(s);
                    }
                }
            }
            crate::physic_engine::ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                let mut buf = [0u8; 128];
                let mut cursor = std::io::Cursor::new(&mut buf[..]);
                if std::io::Write::write_fmt(
                    &mut cursor,
                    format_args!(
                        "Current Mode: MultiImage ({} shapes active, total weight {:.1})",
                        shapes.len(),
                        total_weight
                    ),
                )
                .is_ok()
                {
                    let pos = cursor.position() as usize;
                    if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                        ui.text(s);
                    }
                }
            }
        }

        if ui.button("Spherical Mode (`physic.explosion.shape spherical`)") {
            cmd_queue.push(EngineCommand::Physic(
                PhysicCommand::SetExplosionShapeSpherical,
            ));
        }

        ui.spacing();
        ui.text_colored([0.9, 0.9, 0.4, 1.0], "Presets & Custom Weight Adjustment:");
        ui.same_line();
        if ui.small_button("Reset All Preset Weights") {
            cmd_queue.push(EngineCommand::Physic(PhysicCommand::ResetAllPresetWeights));
        }

        let mut preset_weights = preset_weights_from_shape(explosion_shape);

        for (i, (name, _stem, _path, _scale, _flight_time)) in PRESET_DEFINITIONS.iter().enumerate()
        {
            let _id = ui.push_id_usize(i);
            ui.text(*name);
            ui.same_line();

            ui.set_next_item_width(110.0);
            if ui.slider("Weight", 0.1, 10.0, &mut preset_weights[i]) {
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
                cmd_queue.push(EngineCommand::Physic(
                    PhysicCommand::AddPresetShapeWeighted {
                        index: i as u32,
                        weight: preset_weights[i],
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
                {
                    let mut buf = [0u8; 128];
                    let mut cursor = std::io::Cursor::new(&mut buf[..]);
                    if std::io::Write::write_fmt(
                        &mut cursor,
                        format_args!("--- Active Single Image: '{}' ---", img.file_stem),
                    )
                    .is_ok()
                    {
                        let pos = cursor.position() as usize;
                        if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                            ui.text_colored([0.2, 0.9, 0.4, 1.0], s);
                        }
                    }
                }
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
                    20.0,
                    500.0,
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
                    0.2,
                    5.0,
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
                {
                    let mut buf = [0u8; 128];
                    let mut cursor = std::io::Cursor::new(&mut buf[..]);
                    if std::io::Write::write_fmt(
                        &mut cursor,
                        format_args!(
                            "--- Active MultiImage Shapes ({}) & Parameters Breakdown ---",
                            shapes.len()
                        ),
                    )
                    .is_ok()
                    {
                        let pos = cursor.position() as usize;
                        if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                            ui.text_colored([0.2, 0.9, 0.4, 1.0], s);
                        }
                    }
                }

                for (idx, (shape, weight)) in shapes.iter().enumerate() {
                    let idx_u32 = idx as u32;
                    let _id = ui.push_id_usize(idx);
                    let (def_scale, def_flight) = get_preset_defaults(&shape.file_stem);

                    let pct = if *total_weight > 0.0 {
                        (*weight / total_weight) * 100.0
                    } else {
                        0.0
                    };

                    {
                        let mut buf = [0u8; 128];
                        let mut cursor = std::io::Cursor::new(&mut buf[..]);
                        if std::io::Write::write_fmt(
                            &mut cursor,
                            format_args!(
                                "* Forme: '{}' ({:.1}% d'apparition)",
                                shape.file_stem, pct
                            ),
                        )
                        .is_ok()
                        {
                            let pos = cursor.position() as usize;
                            if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                                ui.text_colored([0.4, 0.8, 1.0, 1.0], s);
                            }
                        }
                    }
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
                    if ui.slider("Weight", 0.0, 10.0, &mut w) {
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
                    if ui.slider("Image Scale (px)", 20.0, 500.0, &mut scale) {
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
                    if ui.slider("Flight Time (s)", 0.2, 5.0, &mut flight_time) {
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

    #[test]
    fn test_render_physics_settings_tab_pure_function() {
        use crate::physic_engine::config::PhysicConfig;
        use crate::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;

        let config = PhysicConfig::default();
        let engine = PhysicEngineFireworks::new(&config, 800.0);
        let mut cmd_queue: Vec<EngineCommand> = Vec::with_capacity(16);
        let reinit = AtomicBool::new(false);
        let mut imgui_ctx = imgui::Context::create();
        imgui_ctx.set_ini_filename(None);
        imgui_ctx.fonts().build_rgba32_texture();
        imgui_ctx.io_mut().display_size = [800.0, 600.0];

        let ui = imgui_ctx.frame();

        render_physics_settings_tab(ui, "", &engine, &mut cmd_queue, &reinit);

        assert!(cmd_queue.capacity() >= 16);
    }
}

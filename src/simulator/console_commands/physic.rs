#![allow(clippy::manual_is_multiple_of)]

use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::renderer_engine::RendererEngine;
use crate::window_engine::WindowEngine;
use crate::Simulator;

impl<R, P, A, W> Simulator<R, P, A, W>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
    W: WindowEngine,
{
    pub(crate) fn register_physic_commands(&mut self) {
        self.commands_registry
            .register_for_physic("physic.config", |engine, _, _| {
                let current = engine.get_config().clone();
                let pending = engine.get_pending_config().clone();
                if current == pending {
                    format!("Applied Configuration:\n{:#?}", current)
                } else {
                    format!(
                        "Applied Configuration:\n{:#?}\n\n[PENDING CHANGES] (run 'physic.apply' to apply):\n{:#?}",
                        current, pending
                    )
                }
            });
        self.commands_registry.register_hint(
            "physic.config",
            "Display current applied and pending physics configurations",
        );

        macro_rules! reg_usize_param {
            ($registry:expr, $name:expr, $variant:ident, $field:ident, $hint:expr) => {
                $registry.register_for_physic($name, |engine, args, cmd_queue| {
                    let val_str = args.split_whitespace().nth(1).unwrap_or("");
                    if val_str.is_empty() {
                        let applied = engine.get_config().$field;
                        let pending = engine.get_pending_config().$field;
                        return format!(
                            "Usage: {} <value> (applied: {}, pending: {})",
                            $name, applied, pending
                        );
                    }
                    match val_str.parse::<usize>() {
                        Ok(val) => {
                            cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                                crate::domain_contracts::PhysicCommand::$variant(val as u32),
                            ));
                            format!(
                                "-> Set {} = {} (pending, run 'physic.apply' to apply)",
                                $name, val
                            )
                        }
                        Err(_) => "x Invalid unsigned integer value".to_string(),
                    }
                });
                $registry.register_hint($name, $hint);
                $registry.register_current_value($name, |_, physic| {
                    physic.get_config().$field.to_string()
                });
            };
        }

        macro_rules! reg_f32_param {
            ($registry:expr, $name:expr, $variant:ident, $field:ident, $hint:expr) => {
                $registry.register_for_physic($name, |engine, args, cmd_queue| {
                    let val_str = args.split_whitespace().nth(1).unwrap_or("");
                    if val_str.is_empty() {
                        let applied = engine.get_config().$field;
                        let pending = engine.get_pending_config().$field;
                        return format!(
                            "Usage: {} <value> (applied: {}, pending: {})",
                            $name, applied, pending
                        );
                    }
                    match val_str.parse::<f32>() {
                        Ok(val) => {
                            cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                                crate::domain_contracts::PhysicCommand::$variant(val),
                            ));
                            format!(
                                "-> Set {} = {} (pending, run 'physic.apply' to apply)",
                                $name, val
                            )
                        }
                        Err(_) => "x Invalid float value".to_string(),
                    }
                });
                $registry.register_hint($name, $hint);
                $registry.register_current_value($name, |_, physic| {
                    physic.get_config().$field.to_string()
                });
            };
        }

        reg_usize_param!(
            self.commands_registry,
            "physic.max_rockets",
            SetMaxRockets,
            max_rockets,
            "Set maximum concurrent rockets"
        );
        reg_usize_param!(
            self.commands_registry,
            "physic.particles_per_explosion",
            SetParticlesPerExplosion,
            particles_per_explosion,
            "Set particles per explosion"
        );
        reg_usize_param!(
            self.commands_registry,
            "physic.particles_per_trail",
            SetParticlesPerTrail,
            particles_per_trail,
            "Set particles per trail"
        );

        reg_f32_param!(
            self.commands_registry,
            "physic.rocket_interval_mean",
            SetRocketIntervalMean,
            rocket_interval_mean,
            "Set mean time interval between rocket spawns (seconds)"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.rocket_interval_variation",
            SetRocketIntervalVariation,
            rocket_interval_variation,
            "Set variation of interval between rocket spawns"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.rocket_max_next_interval",
            SetRocketMaxNextInterval,
            rocket_max_next_interval,
            "Set maximum interval constraint between rocket spawns"
        );

        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_margin",
            SetSpawnRocketMargin,
            spawn_rocket_margin,
            "Set screen margin for rocket spawns"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_vertical_angle",
            SetSpawnRocketVerticalAngle,
            spawn_rocket_vertical_angle,
            "Set vertical spawn angle of rockets (radians)"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_angle_variation",
            SetSpawnRocketAngleVariation,
            spawn_rocket_angle_variation,
            "Set random angle variation of spawned rockets"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_min_speed",
            SetSpawnRocketMinSpeed,
            spawn_rocket_min_speed,
            "Set minimum initial speed of spawned rockets"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_max_speed",
            SetSpawnRocketMaxSpeed,
            spawn_rocket_max_speed,
            "Set maximum initial speed of spawned rockets"
        );

        reg_f32_param!(
            self.commands_registry,
            "physic.explosion_threshold",
            SetExplosionThreshold,
            explosion_threshold,
            "Set speed threshold under which rockets explode"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.gravity",
            SetGravity,
            gravity,
            "Set gravity value affecting rockets and particles"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.initial_rocket_speed",
            SetInitialRocketSpeed,
            initial_rocket_speed,
            "Set target initial speed (metadata)"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.explosion_min_vel",
            SetExplosionMinVel,
            explosion_min_vel,
            "Set minimum velocity of explosion particles"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.explosion_max_vel",
            SetExplosionMaxVel,
            explosion_max_vel,
            "Set maximum velocity of explosion particles"
        );

        // Apply config changes / reinit engines
        self.commands_registry
            .register_for_physic("physic.apply", move |_, _, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                    crate::domain_contracts::PhysicCommand::ApplyPendingConfig,
                ));
                "-> Physics configuration applied and engines re-synchronized.".to_string()
            });
        self.commands_registry.register_hint(
            "physic.apply",
            "Apply all pending configuration changes and re-synchronize engines (physics & renderer)",
        );

        // Save current configuration to disk
        self.commands_registry
            .register_for_physic("physic.config.save", |_, _, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                    crate::domain_contracts::PhysicCommand::SaveConfig,
                ));
                format!(
                    "-> Physics configuration saved to {}",
                    crate::utils::config_path::get_physic_config_path().display()
                )
            });
        self.commands_registry.register_hint(
            "physic.config.save",
            &format!(
                "Save current applied physics configuration to {}",
                crate::utils::config_path::get_physic_config_path().display()
            ),
        );

        // Reload configuration from disk
        self.commands_registry.register_for_physic(
            "physic.config.reload",
            move |_, _, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                    crate::domain_contracts::PhysicCommand::ReloadConfig,
                ));
                "-> Physics configuration reloaded from disk and engines re-synchronized"
                    .to_string()
            },
        );
        self.commands_registry.register_hint(
            "physic.config.reload",
            &format!(
                "Reload physics configuration from {} and re-synchronize engines",
                crate::utils::config_path::get_physic_config_path().display()
            ),
        );

        // --- Explosion Shape Commands ---

        // Display current explosion shape
        self.commands_registry
            .register_for_physic("physic.explosion.shape", |engine, args, cmd_queue| {
                let arg = args.split_whitespace().nth(1).unwrap_or("").to_lowercase();

                if arg.is_empty() {
                    // Show current shape info
                    match engine.get_explosion_shape() {
                        crate::physic_engine::ExplosionShape::Spherical => {
                            "Current explosion shape: spherical".to_string()
                        }
                        crate::physic_engine::ExplosionShape::Image(img) => {
                            format!(
                                "Current explosion shape: image - {}\n  Points: {}\n  Scale: {:.1}\n  Flight time: {:.2}s",
                                img.file_stem,
                                img.sampled_points.len(),
                                img.scale,
                                img.flight_time
                            )
                        }
                        crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
                            format!(
                                "Current explosion shape: MultiImage ({} images)\n{}",
                                shapes.len(),
                                shapes
                                    .iter()
                                    .map(|(s, w)| format!(
                                        "  - {} (w={:.1}, scale={:.1}, t={:.2}s)",
                                        s.file_stem, w, s.scale, s.flight_time
                                    ))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            )
                        }
                    }
                } else {
                    match arg.as_str() {
                        "spherical" => {
                            cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                                crate::domain_contracts::PhysicCommand::SetExplosionShapeSpherical,
                            ));
                            "-> Explosion shape: spherical".to_string()
                        }
                        _ => "Usage: physic.explosion.shape [spherical]\nUse physic.explosion.preset <name> to load a preset shape".to_string()
                    }
                }
            });
        self.commands_registry
            .register_args("physic.explosion.shape", vec!["spherical"]);
        self.commands_registry
            .register_hint("physic.explosion.shape", "Usage: [spherical]");

        // Load explosion image with parameters
        self.commands_registry
            .register_for_physic("physic.explosion.image", |_, args, cmd_queue| {
                let parts: Vec<&str> = args.split_whitespace().collect();
                let params = &parts[1..];

                if params.is_empty() {
                    return "Usage: physic.explosion.image <path> [scale] [flight_time] (Single)\n\
                            Usage: physic.explosion.image <path> <weight> [scale] [time] (Add)\n\
                            Usage: physic.explosion.image <path> <weight> <path> <weight> ... (Batch)".to_string();
                }

                let resolve_preset = |path: &str| -> Option<u32> {
                    let stem = std::path::Path::new(path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(path)
                        .to_lowercase();
                    match stem.as_str() {
                        "heart" => Some(0),
                        "star" => Some(1),
                        "smiley" => Some(2),
                        "note" => Some(3),
                        "ring" => Some(4),
                        _ => None,
                    }
                };

                // Batch Mode
                if params.len() >= 4 && params.len() % 2 == 0 {
                    let looks_like_batch = params.chunks(2).all(|chunk| {
                        chunk[1].parse::<f32>().map(|w| w < 20.0).unwrap_or(false)
                    });

                    if looks_like_batch {
                        let mut results = Vec::new();
                        for chunk in params.chunks(2) {
                            let path = chunk[0];
                            let weight = chunk[1].parse::<f32>().unwrap_or(1.0);
                            if let Some(idx) = resolve_preset(path) {
                                cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                                    crate::domain_contracts::PhysicCommand::AddPresetShapeWeighted {
                                        index: idx,
                                        weight,
                                    },
                                ));
                                results.push(format!("{} ({:.1})", path, weight));
                            } else {
                                results.push(format!("x Unknown preset '{}'", path));
                            }
                        }
                        return format!("-> Batch Loaded:\n   {}", results.join("\n   "));
                    }
                }

                let path = params[0];
                let arg2 = params.get(1).and_then(|s| s.parse::<f32>().ok());
                if let Some(idx) = resolve_preset(path) {
                    if let Some(val) = arg2 {
                        if val < 20.0 {
                            let weight = val;
                            cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                                crate::domain_contracts::PhysicCommand::AddPresetShapeWeighted {
                                    index: idx,
                                    weight,
                                },
                            ));
                            format!("-> Added: {} (w={:.1})", path, weight)
                        } else {
                            cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                                crate::domain_contracts::PhysicCommand::SetPresetSingleShape { index: idx },
                            ));
                            format!("-> Loaded: {}", path)
                        }
                    } else {
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                            crate::domain_contracts::PhysicCommand::SetPresetSingleShape { index: idx },
                        ));
                        format!("-> Loaded: {}", path)
                    }
                } else {
                    format!("x Preset for image '{}' not found", path)
                }
            });
        self.commands_registry
            .register_hint("physic.explosion.image", "Usage: <path> [weight|scale] ...");

        // Add weighted explosion image
        self.commands_registry
            .register_for_physic("physic.explosion.add", |_, args, cmd_queue| {
                let parts: Vec<&str> = args.split_whitespace().collect();

                if parts.len() < 3 {
                    return "Usage: physic.explosion.add <path> <weight> [scale] [flight_time]\n\
                            Defaults: scale=150.0, flight_time=1.5\n\
                            Example: physic.explosion.add assets/textures/explosion_shapes/heart.png 5.0".to_string();
                }

                let path = parts[1];
                let weight = parts.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);

                let resolve_preset = |path: &str| -> Option<u32> {
                    let stem = std::path::Path::new(path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(path)
                        .to_lowercase();
                    match stem.as_str() {
                        "heart" => Some(0),
                        "star" => Some(1),
                        "smiley" => Some(2),
                        "note" => Some(3),
                        "ring" => Some(4),
                        _ => None,
                    }
                };

                if let Some(idx) = resolve_preset(path) {
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                        crate::domain_contracts::PhysicCommand::AddPresetShapeWeighted {
                            index: idx,
                            weight,
                        },
                    ));
                    format!("-> Added: {} (weight={:.1})", path, weight)
                } else {
                    format!("x Preset for image '{}' not found", path)
                }
            });
        self.commands_registry.register_hint(
            "physic.explosion.add",
            "Usage: <path> <weight> [scale] [flight_time]",
        );

        // Show statistics for weighted images
        self.commands_registry
            .register_for_physic("physic.explosion.stats", |engine, _, _| {
                match engine.get_explosion_shape() {
                    crate::physic_engine::ExplosionShape::Spherical => {
                        "Explosion Mode: Spherical (100%)".to_string()
                    }
                    crate::physic_engine::ExplosionShape::Image(img) => {
                        format!("Explosion Mode: Single Image (100%)\n  - {}", img.file_stem)
                    }
                    crate::physic_engine::ExplosionShape::MultiImage {
                        shapes,
                        total_weight,
                    } => {
                        if *total_weight <= 0.0 {
                            return "Explosion Mode: MultiImage (Error: Total weight <= 0)"
                                .to_string();
                        }

                        let mut output = format!(
                            "Explosion Mode: MultiImage (Total Weight: {:.2})\n",
                            total_weight
                        );
                        output.push_str("Probability Distribution:\n");

                        // Sort by weight/probability descending for better readability
                        let mut stats: Vec<_> = shapes.iter().collect();
                        stats.sort_by(|a, b| {
                            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        for (shape, weight) in stats {
                            let percentage = (weight / total_weight) * 100.0;
                            output.push_str(&format!(
                                "  - {:<20} : {:>6.2}% (Weight: {:.2})\n",
                                shape.file_stem, percentage, weight
                            ));
                        }
                        output
                    }
                }
            });

        // Register dynamic arguments for weight command to suggest loaded image names
        self.commands_registry
            .register_dynamic_args("physic.explosion.weight", |_, physic| {
                if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                    physic.get_explosion_shape()
                {
                    shapes.iter().map(|(s, _)| s.file_stem.clone()).collect()
                } else {
                    vec![]
                }
            });

        // Set weight for specific image in MultiImage
        self.commands_registry.register_for_physic(
            "physic.explosion.weight",
            |engine, args, cmd_queue| {
                let parts: Vec<&str> = args.split_whitespace().collect();
                if parts.len() < 3 {
                    return "Usage: physic.explosion.weight <name> <new_weight>\n\
                        Example: physic.explosion.weight heart 2.5"
                        .to_string();
                }

                let name = parts[1];
                let weight = match parts[2].parse::<f32>() {
                    Ok(v) if v >= 0.0 => v,
                    _ => return "Weight must be a positive number".to_string(),
                };

                if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                    engine.get_explosion_shape()
                {
                    if let Some((idx, _)) = shapes
                        .iter()
                        .enumerate()
                        .find(|(_, (s, _))| s.file_stem.eq_ignore_ascii_case(name))
                    {
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                            crate::domain_contracts::PhysicCommand::SetMultiShapeItemWeight {
                                index: idx as u32,
                                weight,
                            },
                        ));
                        return format!("-> Updated weight for '{}' to {:.2}", name, weight);
                    }
                }

                let resolve_preset = |name: &str| -> Option<u32> {
                    match name.to_lowercase().as_str() {
                        "heart" => Some(0),
                        "star" => Some(1),
                        "smiley" => Some(2),
                        "note" => Some(3),
                        "ring" => Some(4),
                        _ => None,
                    }
                };

                if let Some(idx) = resolve_preset(name) {
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                        crate::domain_contracts::PhysicCommand::SetPresetWeight {
                            index: idx,
                            weight,
                        },
                    ));
                    format!("-> Updated weight for '{}' to {:.2}", name, weight)
                } else {
                    format!("x Image or preset '{}' not found", name)
                }
            },
        );
        self.commands_registry
            .register_hint("physic.explosion.weight", "Usage: <name> <weight>");

        self.commands_registry
            .register_current_value("physic.explosion.weight", |_, physic| {
                match physic.get_explosion_shape() {
                    crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
                        let s = shapes
                            .iter()
                            .map(|(img, w)| format!("{}: {:.1}", img.file_stem, w))
                            .collect::<Vec<_>>()
                            .join(", ");
                        if s.len() > 60 {
                            format!("{}...", &s[..57])
                        } else {
                            s
                        }
                    }
                    _ => "N/A".to_string(),
                }
            });

        // Set scale for current image explosion
        self.commands_registry.register_for_physic(
            "physic.explosion.scale",
            |engine, args, cmd_queue| {
                let scale_str = args.split_whitespace().nth(1).unwrap_or("");

                if scale_str.is_empty() {
                    // Show current scale
                    return match engine.get_explosion_shape() {
                        crate::physic_engine::ExplosionShape::Image(img) => {
                            format!("Current scale: {:.1}", img.scale)
                        }
                        crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
                            let scales: Vec<String> = shapes
                                .iter()
                                .map(|(s, _)| format!("{:.1}", s.scale))
                                .collect();
                            format!("Current scales: [{}]", scales.join(", "))
                        }
                        _ => "No image explosion loaded.".to_string(),
                    };
                }

                let scale = match scale_str.parse::<f32>() {
                    Ok(v) if v > 0.0 => v,
                    _ => {
                        return "Usage: physic.explosion.scale <value> (positive number)"
                            .to_string()
                    }
                };

                cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                    crate::domain_contracts::PhysicCommand::SetSingleShapeScale(scale),
                ));
                format!("-> Scale: {:.1}", scale)
            },
        );
        self.commands_registry
            .register_hint("physic.explosion.scale", "Usage: <50-500>");

        // Set flight_time for current image explosion
        self.commands_registry.register_for_physic(
            "physic.explosion.flight_time",
            |engine, args, cmd_queue| {
                let time_str = args.split_whitespace().nth(1).unwrap_or("");

                if time_str.is_empty() {
                    // Show current flight_time
                    return match engine.get_explosion_shape() {
                        crate::physic_engine::ExplosionShape::Image(img) => {
                            format!("Current flight_time: {:.2}s", img.flight_time)
                        }
                        crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
                            let times: Vec<String> = shapes
                                .iter()
                                .map(|(s, _)| format!("{:.2}s", s.flight_time))
                                .collect();
                            format!("Current flight_times: [{}]", times.join(", "))
                        }
                        _ => "No image explosion loaded.".to_string(),
                    };
                }

                let flight_time = match time_str.parse::<f32>() {
                    Ok(v) if v > 0.0 => v,
                    _ => {
                        return "Usage: physic.explosion.flight_time <seconds> (positive number)"
                            .to_string()
                    }
                };

                cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                    crate::domain_contracts::PhysicCommand::SetSingleShapeFlightTime(flight_time),
                ));
                format!("-> Flight time: {:.2}s", flight_time)
            },
        );
        self.commands_registry
            .register_hint("physic.explosion.flight_time", "Usage: <0.5-5.0>");

        self.commands_registry
            .register_current_value("physic.explosion.shape", |_, physic| {
                match physic.get_explosion_shape() {
                    crate::physic_engine::ExplosionShape::Spherical => "Spherical".to_string(),
                    crate::physic_engine::ExplosionShape::Image(img) => {
                        format!("Image ({})", img.file_stem)
                    }
                    crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
                        format!("MultiImage ({} shapes)", shapes.len())
                    }
                }
            });

        // Use same logic for image/preset commands to give context
        self.commands_registry
            .register_current_value("physic.explosion.image", |_, physic| {
                match physic.get_explosion_shape() {
                    crate::physic_engine::ExplosionShape::Image(img) => img.file_stem.clone(),
                    crate::physic_engine::ExplosionShape::MultiImage { .. } => {
                        "MultiImage Mode".to_string()
                    }
                    _ => "None".to_string(),
                }
            });

        self.commands_registry
            .register_current_value("physic.explosion.preset", |_, physic| {
                match physic.get_explosion_shape() {
                    crate::physic_engine::ExplosionShape::Image(img) => img.file_stem.clone(),
                    crate::physic_engine::ExplosionShape::MultiImage { .. } => {
                        "MultiImage Mode".to_string()
                    }
                    _ => "None".to_string(),
                }
            });

        // --- Current Value Getters for Explosion ---
        self.commands_registry
            .register_current_value("physic.explosion.scale", |_, physic| {
                match physic.get_explosion_shape() {
                    crate::physic_engine::ExplosionShape::Image(img) => format!("{:.1}", img.scale),
                    crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
                        if shapes.is_empty() {
                            "N/A".to_string()
                        } else {
                            format!("{:.1}...", shapes[0].0.scale)
                        }
                    }
                    _ => "N/A".to_string(),
                }
            });

        self.commands_registry.register_current_value(
            "physic.explosion.flight_time",
            |_, physic| match physic.get_explosion_shape() {
                crate::physic_engine::ExplosionShape::Image(img) => {
                    format!("{:.2}s", img.flight_time)
                }
                crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } => {
                    if shapes.is_empty() {
                        "N/A".to_string()
                    } else {
                        format!("{:.2}s...", shapes[0].0.flight_time)
                    }
                }
                _ => "N/A".to_string(),
            },
        );

        // Presets for common shapes
        self.commands_registry.register_for_physic(
            "physic.explosion.preset",
            |_, args, cmd_queue| {
                let parts: Vec<&str> = args.split_whitespace().collect();
                let params = &parts[1..];
                if params.is_empty() {
                    return "Available presets: heart, star, smiley, note, ring\n\
                             Usage: preset <name> [weight] [<name> <weight> ...]"
                        .to_string();
                }

                let resolve_preset = |name: &str| -> Option<u32> {
                    match name.to_lowercase().as_str() {
                        "heart" => Some(0),
                        "star" => Some(1),
                        "smiley" => Some(2),
                        "note" => Some(3),
                        "ring" => Some(4),
                        _ => None,
                    }
                };

                if params.len() == 1 {
                    let name = params[0];
                    if let Some(idx) = resolve_preset(name) {
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                            crate::domain_contracts::PhysicCommand::SetPresetSingleShape {
                                index: idx,
                            },
                        ));
                        format!("-> Preset '{}' loaded", name)
                    } else {
                        format!("x Unknown preset '{}'", name)
                    }
                } else if params.len() >= 2 && params.len() % 2 == 0 {
                    let mut results = Vec::new();
                    for chunk in params.chunks(2) {
                        let name = chunk[0];
                        let weight_str = chunk[1];

                        if let (Some(idx), Ok(weight)) =
                            (resolve_preset(name), weight_str.parse::<f32>())
                        {
                            cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                                crate::domain_contracts::PhysicCommand::AddPresetShapeWeighted {
                                    index: idx,
                                    weight,
                                },
                            ));
                            results.push(format!("{} ({:.1})", name, weight));
                        } else {
                            results.push(format!("x Unknown preset or invalid weight '{}'", name));
                        }
                    }
                    if results.is_empty() {
                        "x No valid presets processed".to_string()
                    } else {
                        format!("-> Multi-Preset Added:\n   {}", results.join("\n   "))
                    }
                } else {
                    "Usage: preset <name> (Replace) OR preset <name> <weight> ... (Add)".to_string()
                }
            },
        );
        self.commands_registry.register_args(
            "physic.explosion.preset",
            vec!["heart", "star", "smiley", "note", "ring"],
        );
        self.commands_registry
            .register_hint("physic.explosion.preset", "Usage: <preset> [weight] ...");
    }
}

#[cfg(test)]
mod tests {
    use crate::simulator::gui_settings::PRESET_DEFINITIONS;

    #[test]
    fn test_physic_preset_definitions_available() {
        assert_eq!(PRESET_DEFINITIONS.len(), 5);
        let names: Vec<&str> = PRESET_DEFINITIONS
            .iter()
            .map(|preset| preset.name)
            .collect();
        assert!(names.contains(&"Heart"));
        assert!(names.contains(&"Star"));
    }
}

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
            .register_for_physic("physic.config", |engine, _| {
                let current = engine.get_config().clone();
                let pending = engine.get_config_mut().clone();
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
            ($registry:expr, $name:expr, $field:ident, $hint:expr) => {
                $registry.register_for_physic($name, |engine, args| {
                    let val_str = args.split_whitespace().nth(1).unwrap_or("");
                    if val_str.is_empty() {
                        let applied = engine.get_config().$field;
                        let pending = engine.get_config_mut().$field;
                        return format!(
                            "Usage: {} <value> (applied: {}, pending: {})",
                            $name, applied, pending
                        );
                    }
                    match val_str.parse::<usize>() {
                        Ok(val) => {
                            engine.get_config_mut().$field = val;
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
            ($registry:expr, $name:expr, $field:ident, $hint:expr) => {
                $registry.register_for_physic($name, |engine, args| {
                    let val_str = args.split_whitespace().nth(1).unwrap_or("");
                    if val_str.is_empty() {
                        let applied = engine.get_config().$field;
                        let pending = engine.get_config_mut().$field;
                        return format!(
                            "Usage: {} <value> (applied: {}, pending: {})",
                            $name, applied, pending
                        );
                    }
                    match val_str.parse::<f32>() {
                        Ok(val) => {
                            engine.get_config_mut().$field = val;
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
            max_rockets,
            "Set maximum concurrent rockets"
        );
        reg_usize_param!(
            self.commands_registry,
            "physic.particles_per_explosion",
            particles_per_explosion,
            "Set particles per explosion"
        );
        reg_usize_param!(
            self.commands_registry,
            "physic.particles_per_trail",
            particles_per_trail,
            "Set particles per trail"
        );

        reg_f32_param!(
            self.commands_registry,
            "physic.rocket_interval_mean",
            rocket_interval_mean,
            "Set mean time interval between rocket spawns (seconds)"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.rocket_interval_variation",
            rocket_interval_variation,
            "Set variation of interval between rocket spawns"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.rocket_max_next_interval",
            rocket_max_next_interval,
            "Set maximum interval constraint between rocket spawns"
        );

        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_margin",
            spawn_rocket_margin,
            "Set screen margin for rocket spawns"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_vertical_angle",
            spawn_rocket_vertical_angle,
            "Set vertical spawn angle of rockets (radians)"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_angle_variation",
            spawn_rocket_angle_variation,
            "Set random angle variation of spawned rockets"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_min_speed",
            spawn_rocket_min_speed,
            "Set minimum initial speed of spawned rockets"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.spawn_rocket_max_speed",
            spawn_rocket_max_speed,
            "Set maximum initial speed of spawned rockets"
        );

        reg_f32_param!(
            self.commands_registry,
            "physic.explosion_threshold",
            explosion_threshold,
            "Set speed threshold under which rockets explode"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.gravity",
            gravity,
            "Set gravity value affecting rockets and particles"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.initial_rocket_speed",
            initial_rocket_speed,
            "Set target initial speed (metadata)"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.explosion_min_vel",
            explosion_min_vel,
            "Set minimum velocity of explosion particles"
        );
        reg_f32_param!(
            self.commands_registry,
            "physic.explosion_max_vel",
            explosion_max_vel,
            "Set maximum velocity of explosion particles"
        );

        // Apply config changes / reinit engines
        let physic_reinit_flag = self.physic_reinit_requested.clone();
        self.commands_registry
            .register_for_physic("physic.apply", move |engine, _| {
                let pending = engine.get_config_mut().clone();
                let _updated = engine.reload_config(&pending);
                physic_reinit_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                "-> Physics configuration applied and engines re-synchronized.".to_string()
            });
        self.commands_registry.register_hint(
            "physic.apply",
            "Apply all pending configuration changes and re-synchronize engines (physics & renderer)",
        );

        // Save current configuration to disk
        self.commands_registry
            .register_for_physic("physic.config.save", |engine, _| {
                let current = engine.get_config();
                match current.save_to_file("assets/config/physic.toml") {
                    Ok(_) => {
                        "-> Physics configuration saved to assets/config/physic.toml".to_string()
                    }
                    Err(e) => format!("x Failed to save physics configuration: {}", e),
                }
            });
        self.commands_registry.register_hint(
            "physic.config.save",
            "Save current applied physics configuration to assets/config/physic.toml",
        );

        // Reload configuration from disk
        let reinit_flag_reload = self.physic_reinit_requested.clone();
        self.commands_registry
            .register_for_physic("physic.config.reload", move |engine, _| {
                match crate::physic_engine::config::PhysicConfig::from_file(
                    "assets/config/physic.toml",
                ) {
                    Ok(new_cfg) => {
                        *engine.get_config_mut() = new_cfg.clone();
                        let _updated = engine.reload_config(&new_cfg);
                        reinit_flag_reload.store(true, std::sync::atomic::Ordering::Relaxed);
                        "-> Physics configuration reloaded from disk and engines re-synchronized"
                            .to_string()
                    }
                    Err(e) => format!("x Failed to load physics configuration: {}", e),
                }
            });
        self.commands_registry.register_hint(
            "physic.config.reload",
            "Reload physics configuration from assets/config/physic.toml and re-synchronize engines",
        );

        // --- Explosion Shape Commands ---

        // Display current explosion shape
        self.commands_registry
            .register_for_physic("physic.explosion.shape", |engine, args| {
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
                            engine
                                .set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
                            "-> Explosion shape: spherical".to_string()
                        }
                        _ => "Usage: physic.explosion.shape [spherical]\nUse physic.explosion.image <path> <scale> <flight_time> to load an image".to_string()
                    }
                }
            });
        self.commands_registry
            .register_args("physic.explosion.shape", vec!["spherical"]);
        self.commands_registry
            .register_hint("physic.explosion.shape", "Usage: [spherical]");

        // Load explosion image with parameters
        // Usage: physic.explosion.image <path> [scale] [flight_time]
        // Load explosion image with parameters (Now supports weighted MultiImage)
        // Usage: physic.explosion.image <path> [scale] [flight_time]   -> Single (Replace)
        // Usage: physic.explosion.image <path> <weight> [scale] [time] -> Multi (Add/Upgrade)
        // Usage: physic.explosion.image <path> <weight> <path> <weight> ... -> Batch (Replace)
        self.commands_registry
            .register_for_physic("physic.explosion.image", |engine, args| {
                let parts: Vec<&str> = args.split_whitespace().collect();
                let params = &parts[1..];

                if params.is_empty() {
                    return "Usage: physic.explosion.image <path> [scale] [flight_time] (Single)\n\
                            Usage: physic.explosion.image <path> <weight> [scale] [time] (Add)\n\
                            Usage: physic.explosion.image <path> <weight> <path> <weight> ... (Batch)".to_string();
                }

                // --- 1. Batch Mode (Multiple pairs) ---
                if params.len() >= 4 && params.len() % 2 == 0 {
                    // Check if every odd argument is a small float (weight)
                    let looks_like_batch = params.chunks(2).all(|chunk| {
                        chunk[1].parse::<f32>().map(|w| w < 20.0).unwrap_or(false)
                    });

                    if looks_like_batch {
                        engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
                        let mut results = Vec::new();
                        for chunk in params.chunks(2) {
                            let path = chunk[0];
                            let weight = chunk[1].parse::<f32>().unwrap_or(1.0);
                            match engine.load_explosion_image_weighted(path, 150.0, 1.5, weight) {
                                Ok(()) => results.push(format!("{} ({:.1})", path, weight)),
                                Err(e) => results.push(format!("x {} (Err: {})", path, e)),
                            }
                        }
                        return format!("-> Batch Loaded:\n   {}", results.join("\n   "));
                    }
                }

                // --- 2. Single or Add Mode ---
                let path = params[0];
                let arg2 = params.get(1).and_then(|s| s.parse::<f32>().ok());

                // Heuristic: If arg2 exists and is < 20.0, we treat it as WEIGHT -> "ADD Mode"
                if let Some(val) = arg2 {
                    if val < 20.0 {
                        // ADD MODE: path weight [scale] [time]
                        let weight = val;
                        let scale = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(150.0);
                        let time = params.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.5);

                        match engine.load_explosion_image_weighted(path, scale, time, weight) {
                            Ok(()) => format!("-> Added: {} (w={:.1}, s={:.1}, t={:.2}s)", path, weight, scale, time),
                            Err(e) => format!("x Failed to add: {}", e)
                        }
                    } else {
                        // LEGACY REPLACE MODE: path scale [time]
                        // val is scale >= 20.0
                        let scale = val;
                        let time = params.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.5);
                        match engine.load_explosion_image(path, scale, time) {
                            Ok(()) => format!("-> Loaded: {} (s={:.1}, t={:.2}s)", path, scale, time),
                            Err(e) => format!("x Failed to load: {}", e)
                        }
                    }
                } else {
                    // LEGACY REPLACE MODE: path (default scale/time)
                    match engine.load_explosion_image(path, 150.0, 1.5) {
                        Ok(()) => format!("-> Loaded: {} (default)", path),
                        Err(e) => format!("x Failed to load: {}", e)
                    }
                }
            });
        self.commands_registry
            .register_hint("physic.explosion.image", "Usage: <path> [weight|scale] ...");

        // Add weighted explosion image (Deprecated wrapper around image smart-add)
        // Usage: physic.explosion.add <path> <weight> [scale] [flight_time]
        self.commands_registry
            .register_for_physic("physic.explosion.add", |engine, args| {
                let parts: Vec<&str> = args.split_whitespace().collect();

                if parts.len() < 3 {
                    return "Usage: physic.explosion.add <path> <weight> [scale] [flight_time]\n\
                            Defaults: scale=150.0, flight_time=1.5\n\
                            Example: physic.explosion.add assets/textures/explosion_shapes/heart.png 5.0".to_string();
                }

                let path = parts[1];
                let weight = parts.get(2).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.0);
                let scale = parts.get(3).and_then(|s| s.parse::<f32>().ok()).unwrap_or(150.0);
                let flight_time = parts.get(4).and_then(|s| s.parse::<f32>().ok()).unwrap_or(1.5);

                match engine.load_explosion_image_weighted(path, scale, flight_time, weight) {
                    Ok(()) => format!("-> Added: {} (weight={:.1}, scale={:.1}, flight_time={:.2}s)", path, weight, scale, flight_time),
                    Err(e) => format!("x Failed to add image: {}", e)
                }
            });
        self.commands_registry.register_hint(
            "physic.explosion.add",
            "Usage: <path> <weight> [scale] [flight_time]",
        );

        // Show statistics for weighted images
        self.commands_registry
            .register_for_physic("physic.explosion.stats", |engine, _| {
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
        self.commands_registry
            .register_for_physic("physic.explosion.weight", |engine, args| {
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

                match engine.set_explosion_image_weight(name, weight) {
                    Ok(()) => format!("-> Updated weight for '{}' to {:.2}", name, weight),
                    Err(e) => format!("x Failed: {}", e),
                }
            });
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
        self.commands_registry
            .register_for_physic("physic.explosion.scale", |engine, args| {
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

                // Modify scale of current image shape
                match engine.get_explosion_shape().clone() {
                    crate::physic_engine::ExplosionShape::Image(mut img) => {
                        img.scale = scale;
                        engine
                            .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(img));
                        format!("-> Scale: {:.1}", scale)
                    }
                    crate::physic_engine::ExplosionShape::MultiImage {
                        mut shapes,
                        total_weight,
                    } => {
                        for (s, _) in shapes.iter_mut() {
                            s.scale = scale;
                        }
                        engine.set_explosion_shape(
                            crate::physic_engine::ExplosionShape::MultiImage {
                                shapes,
                                total_weight,
                            },
                        );
                        format!("-> Scale set to {:.1} for all images", scale)
                    }
                    _ => "No image explosion loaded.".to_string(),
                }
            });
        self.commands_registry
            .register_hint("physic.explosion.scale", "Usage: <50-500>");

        // Set flight_time for current image explosion
        self.commands_registry.register_for_physic(
            "physic.explosion.flight_time",
            |engine, args| {
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

                // Modify flight_time of current image shape
                match engine.get_explosion_shape().clone() {
                    crate::physic_engine::ExplosionShape::Image(mut img) => {
                        img.flight_time = flight_time;
                        engine
                            .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(img));
                        format!("-> Flight time: {:.2}s", flight_time)
                    }
                    crate::physic_engine::ExplosionShape::MultiImage {
                        mut shapes,
                        total_weight,
                    } => {
                        for (s, _) in shapes.iter_mut() {
                            s.flight_time = flight_time;
                        }
                        engine.set_explosion_shape(
                            crate::physic_engine::ExplosionShape::MultiImage {
                                shapes,
                                total_weight,
                            },
                        );
                        format!("-> Flight time set to {:.2}s for all images", flight_time)
                    }
                    _ => "No image explosion loaded.".to_string(),
                }
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
                        // Just show range or first
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
        self.commands_registry
            .register_for_physic("physic.explosion.preset", |engine, args| {
                let parts: Vec<&str> = args.split_whitespace().collect();
                // parts[0] is command name

                let params = &parts[1..];
                if params.is_empty() {
                    return "Available presets: heart, star, smiley, note, ring\n\
                             Usage: preset <name> [weight] [<name> <weight> ...]"
                        .to_string();
                }

                // Helper to resolve preset data
                let resolve_preset = |name: &str| -> Option<(&str, f32, f32)> {
                    match name.to_lowercase().as_str() {
                        "heart" => Some(("assets/textures/explosion_shapes/heart.png", 150.0, 1.5)),
                        "star" => Some(("assets/textures/explosion_shapes/star.png", 180.0, 1.5)),
                        "smiley" => {
                            Some(("assets/textures/explosion_shapes/smiley.png", 200.0, 2.0))
                        }
                        "note" => Some(("assets/textures/explosion_shapes/note.png", 160.0, 1.5)),
                        "ring" => Some(("assets/textures/explosion_shapes/ring.png", 190.0, 1.8)),
                        _ => None,
                    }
                };

                // CASE 1: Single Preset (No weight) -> Exact Replace (Single Image)
                if params.len() == 1 {
                    let name = params[0];
                    if let Some((path, scale, flight_time)) = resolve_preset(name) {
                        match engine.load_explosion_image(path, scale, flight_time) {
                            Ok(()) => format!(
                                "-> Preset '{}' loaded (scale={:.1}, time={:.2}s)",
                                name, scale, flight_time
                            ),
                            Err(e) => format!("x Failed to load preset '{}': {}", name, e),
                        }
                    } else {
                        format!("x Unknown preset '{}'", name)
                    }
                }
                // CASE 2: Weighted Presets (One or Multiple pairs) -> Add to MultiImage (Batch Add)
                else if params.len() >= 2 && params.len() % 2 == 0 {
                    // Note: We do NOT reset to Spherical here anymore.
                    // This allows mixing presets and images cumulatively.
                    // To clear, user must run `physic.explosion.shape spherical` or use single-preset replace mode.

                    let mut results = Vec::new();

                    // Iterate pairs
                    for chunk in params.chunks(2) {
                        let name = chunk[0];
                        let weight_str = chunk[1];

                        if let Some((path, scale, flight_time)) = resolve_preset(name) {
                            if let Ok(weight) = weight_str.parse::<f32>() {
                                match engine.load_explosion_image_weighted(
                                    path,
                                    scale,
                                    flight_time,
                                    weight,
                                ) {
                                    Ok(()) => results.push(format!("{} ({:.1})", name, weight)),
                                    Err(e) => {
                                        results.push(format!("x {} (Err: {})", name, e));
                                    }
                                }
                            } else {
                                results
                                    .push(format!("x {} (Invalid weight: {})", name, weight_str));
                            }
                        } else {
                            results.push(format!("x Unknown preset '{}'", name));
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
            });
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
            .map(|(name, _, _, _, _)| *name)
            .collect();
        assert!(names.contains(&"Heart"));
        assert!(names.contains(&"Star"));
    }
}

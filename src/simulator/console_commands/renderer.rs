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
    pub(crate) fn register_renderer_base_commands(&mut self) {
        // Reload Shaders
        self.commands_registry.register_for_renderer(
            "renderer.reload_shaders",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::ReloadShaders,
                ));
                "-> Shader reload requested".to_string()
            },
        );

        // Config View
        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.config", move |_, _| {
                cfg.read()
                    .map(|c| format!("{:#?}", *c))
                    .unwrap_or_else(|_| "x Lock fail".into())
            });

        // Config Save
        self.commands_registry.register_for_renderer(
            "renderer.config.save",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SaveConfig,
                ));
                "-> Config saved".into()
            },
        );

        // Config Reload
        self.commands_registry.register_for_renderer(
            "renderer.config.reload",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::ReloadConfig,
                ));
                "-> Config reloaded".into()
            },
        );

        // Rocket Cursor command
        self.commands_registry.register_for_renderer(
            "renderer.rocket_cursor",
            move |args, cmd_queue| {
                let trimmed = args.trim().to_lowercase();
                if trimmed.is_empty() {
                    return "Usage: renderer.rocket_cursor [true|false|1|0]".into();
                }
                let enable = match trimmed.as_str() {
                    "true" | "1" | "on" => true,
                    "false" | "0" | "off" => false,
                    other => {
                        return format!("Unknown argument '{}', expected true or false", other);
                    }
                };
                cmd_queue.push(crate::domain_contracts::EngineCommand::Gui(
                    crate::domain_contracts::GuiCommand::SetRocketCursor(enable),
                ));
                if enable {
                    "-> Rocket cursor enabled".into()
                } else {
                    "-> Rocket cursor disabled".into()
                }
            },
        );
    }

    pub(crate) fn register_bloom_commands(&mut self) {
        // Enable/Disable
        self.commands_registry.register_for_renderer(
            "renderer.bloom.enable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetBloomEnabled(true),
                ));
                "-> Bloom enabled".into()
            },
        );
        self.commands_registry.register_for_renderer(
            "renderer.bloom.disable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetBloomEnabled(false),
                ));
                "-> Bloom disabled".into()
            },
        );

        // Intensity
        self.commands_registry.register_for_renderer(
            "renderer.bloom.intensity",
            move |args, cmd_queue| {
                let val = args
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok());
                match val {
                    Some(v) if (0.0..=10.0).contains(&v) => {
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetBloomIntensity(v),
                        ));
                        format!("-> Intensity: {:.2}", v)
                    }
                    _ => "Usage: bloom.intensity <0.0-10.0>".into(),
                }
            },
        );
        self.commands_registry
            .register_hint("renderer.bloom.intensity", "Usage: <0.0-10.0>");

        // Iterations
        self.commands_registry.register_for_renderer(
            "renderer.bloom.iterations",
            move |args, cmd_queue| {
                let val = args
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u32>().ok());
                match val {
                    Some(v) if (1..=10).contains(&v) => {
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetBloomIterations(v),
                        ));
                        format!("-> Iterations: {}", v)
                    }
                    _ => "Usage: bloom.iterations <1-10>".into(),
                }
            },
        );
        self.commands_registry
            .register_hint("renderer.bloom.iterations", "Usage: <1-10>");

        // Downsample
        self.commands_registry.register_for_renderer(
            "renderer.bloom.downsample",
            move |args, cmd_queue| match args
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u32>().ok())
            {
                Some(v) if [1, 2, 4].contains(&v) => {
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                        crate::domain_contracts::RendererCommand::SetBloomDownsample(v),
                    ));
                    format!("-> Downsample: {}x", v)
                }
                _ => "Usage: bloom.downsample <1|2|4>".into(),
            },
        );
        self.commands_registry
            .register_args("renderer.bloom.downsample", vec!["1", "2", "4"]);
        self.commands_registry
            .register_hint("renderer.bloom.downsample", "Usage: <1|2|4>");

        // Method
        self.commands_registry.register_for_renderer(
            "renderer.bloom.method",
            move |args, cmd_queue| {
                let method = args.split_whitespace().nth(1).unwrap_or("").to_lowercase();
                match method.as_str() {
                    "gaussian" => {
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetBloomBlurMethod(
                                crate::renderer_engine::config::BlurMethod::Gaussian,
                            ),
                        ));
                        "-> Method: Gaussian".into()
                    }
                    "kawase" => {
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetBloomBlurMethod(
                                crate::renderer_engine::config::BlurMethod::Kawase,
                            ),
                        ));
                        "-> Method: Kawase".into()
                    }
                    _ => "Usage: bloom.method <gaussian|kawase>".into(),
                }
            },
        );
        self.commands_registry
            .register_args("renderer.bloom.method", vec!["gaussian", "kawase"]);
        self.commands_registry
            .register_hint("renderer.bloom.method", "Usage: <gaussian|kawase>");

        // --- Current Value Getters for Bloom ---
        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_current_value("renderer.bloom.intensity", move |_, _| {
                cfg.read()
                    .map(|c| format!("{:.2}", c.bloom_intensity))
                    .unwrap_or("?".to_string())
            });

        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_current_value("renderer.bloom.iterations", move |_, _| {
                cfg.read()
                    .map(|c| format!("{}", c.bloom_iterations))
                    .unwrap_or("?".to_string())
            });

        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_current_value("renderer.bloom.downsample", move |_, _| {
                cfg.read()
                    .map(|c| format!("{}x", c.bloom_downsample))
                    .unwrap_or("?".to_string())
            });

        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_current_value("renderer.bloom.method", move |_, _| {
                cfg.read()
                    .map(|c| format!("{:?}", c.bloom_blur_method))
                    .unwrap_or("?".to_string())
            });
    }

    pub(crate) fn register_tonemapping_commands(&mut self) {
        self.commands_registry.register_for_renderer(
            "renderer.tonemapping",
            move |args, cmd_queue| {
                let mode_str = args.split_whitespace().nth(1).unwrap_or("").to_lowercase();
                let mode = Self::parse_tonemap_mode(&mode_str);

                if let Some(m) = mode {
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                        crate::domain_contracts::RendererCommand::SetToneMappingMode(m),
                    ));
                    return format!("-> Tone mapping: {:?}", m);
                }
                "Available: reinhard, reinhard_extended, aces, uncharted2, khronos".to_string()
            },
        );
        self.commands_registry.register_args(
            "renderer.tonemapping",
            vec![
                "reinhard",
                "reinhard_extended",
                "aces",
                "uncharted2",
                "agx",
                "khronos",
            ],
        );

        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_current_value("renderer.tonemapping", move |_, _| {
                cfg.read()
                    .map(|c| format!("{:?}", c.tone_mapping_mode))
                    .unwrap_or("?".to_string())
            });

        // Comparison Toggle
        let comparison_mode = self.tonemapping_comparison_mode.clone();
        self.commands_registry.register_for_renderer(
            "renderer.tonemapping.compare",
            move |_, cmd_queue| {
                let old = comparison_mode.load(std::sync::atomic::Ordering::Relaxed);
                let new_val = !old;
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetTonemappingComparisonMode(new_val),
                ));
                if new_val {
                    "-> Comparison enabled"
                } else {
                    "-> Comparison disabled"
                }
                .to_string()
            },
        );

        let comparison_mode = self.tonemapping_comparison_mode.clone();
        self.commands_registry.register_current_value(
            "renderer.tonemapping.compare",
            move |_, _| {
                if comparison_mode.load(std::sync::atomic::Ordering::Relaxed) {
                    "Enabled".to_string()
                } else {
                    "Disabled".to_string()
                }
            },
        );

        // Rockets visibility
        self.commands_registry.register_for_renderer(
            "renderer.rockets.enable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetRenderRockets(true),
                ));
                "-> Rockets rendering enabled".into()
            },
        );
        self.commands_registry.register_for_renderer(
            "renderer.rockets.disable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetRenderRockets(false),
                ));
                "-> Rockets rendering disabled".into()
            },
        );

        // Smoke visibility
        self.commands_registry.register_for_renderer(
            "renderer.smoke.enable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetRenderSmoke(true),
                ));
                "-> Smoke rendering enabled".into()
            },
        );
        self.commands_registry.register_for_renderer(
            "renderer.smoke.disable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetRenderSmoke(false),
                ));
                "-> Smoke rendering disabled".into()
            },
        );

        // Trails visibility
        self.commands_registry.register_for_renderer(
            "renderer.trails.enable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetRenderTrails(true),
                ));
                "-> Rocket trails rendering enabled".into()
            },
        );
        self.commands_registry.register_for_renderer(
            "renderer.trails.disable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetRenderTrails(false),
                ));
                "-> Rocket trails rendering disabled".into()
            },
        );

        // Explosions visibility
        self.commands_registry.register_for_renderer(
            "renderer.explosions.enable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetRenderExplosions(true),
                ));
                "-> Explosions rendering enabled".into()
            },
        );
        self.commands_registry.register_for_renderer(
            "renderer.explosions.disable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetRenderExplosions(false),
                ));
                "-> Explosions rendering disabled".into()
            },
        );

        // Master volumetric lighting toggle
        for name in ["renderer.lighting", "renderer.volumetric_lighting"] {
            self.commands_registry
                .register_for_renderer(name, move |args, cmd_queue| {
                    let trimmed = args.trim().to_lowercase();
                    if trimmed.is_empty() {
                        return "Usage: renderer.lighting [true|false|1|0|on|off]".into();
                    }
                    let enable = match trimmed.as_str() {
                        "true" | "1" | "on" => true,
                        "false" | "0" | "off" => false,
                        other => {
                            return format!("Unknown argument '{}', expected true or false", other);
                        }
                    };
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                        crate::domain_contracts::RendererCommand::SetVolumetricLightingEnabled(
                            enable,
                        ),
                    ));
                    format!("-> Volumetric lighting (master): {}", enable)
                });
        }

        // Volumetric smoke lighting
        self.commands_registry.register_for_renderer(
            "renderer.smoke_lighting",
            move |args, cmd_queue| {
                let trimmed = args.trim().to_lowercase();
                if trimmed.is_empty() {
                    return "Usage: renderer.smoke_lighting [true|false|1|0]".into();
                }
                let enable = match trimmed.as_str() {
                    "true" | "1" | "on" => true,
                    "false" | "0" | "off" => false,
                    other => {
                        return format!("Unknown argument '{}', expected true or false", other);
                    }
                };
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetSmokeLightingEnabled(enable),
                ));
                format!("-> Volumetric smoke lighting: {}", enable)
            },
        );

        // Volumetric smoke scattering intensity
        self.commands_registry.register_for_renderer(
            "renderer.smoke_scattering",
            move |args, cmd_queue| {
                let trimmed = args.trim();
                if trimmed.is_empty() {
                    return "Usage: renderer.smoke_scattering [float]".into();
                }
                match trimmed.parse::<f32>() {
                    Ok(val) => {
                        let clamped = val.clamp(0.0, 10.0);
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetSmokeScatteringIntensity(
                                clamped,
                            ),
                        ));
                        format!("-> Smoke scattering intensity set to: {}", clamped)
                    }
                    Err(_) => format!("Invalid float value: '{}'", trimmed),
                }
            },
        );

        // Volumetric smoke ambient flash
        self.commands_registry.register_for_renderer(
            "renderer.smoke_ambient_flash",
            move |args, cmd_queue| {
                let trimmed = args.trim();
                if trimmed.is_empty() {
                    return "Usage: renderer.smoke_ambient_flash [float]".into();
                }
                match trimmed.parse::<f32>() {
                    Ok(val) => {
                        let clamped = val.clamp(0.0, 2.0);
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetSmokeAmbientFlash(clamped),
                        ));
                        format!("-> Smoke ambient flash intensity set to: {}", clamped)
                    }
                    Err(_) => format!("Invalid float value: '{}'", trimmed),
                }
            },
        );

        // Atmospheric sky haze enabled
        self.commands_registry.register_for_renderer(
            "renderer.sky_haze",
            move |args, cmd_queue| {
                let trimmed = args.trim();
                let enable = match trimmed {
                    "" => true,
                    "1" | "true" | "on" => true,
                    "0" | "false" | "off" => false,
                    other => {
                        return format!("Unknown argument '{}', expected true or false", other);
                    }
                };
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetSkyHazeEnabled(enable),
                ));
                format!("-> Atmospheric sky haze: {}", enable)
            },
        );

        // Atmospheric sky haze intensity
        self.commands_registry.register_for_renderer(
            "renderer.sky_haze_intensity",
            move |args, cmd_queue| {
                let trimmed = args.trim();
                if trimmed.is_empty() {
                    return "Usage: renderer.sky_haze_intensity [float]".into();
                }
                match trimmed.parse::<f32>() {
                    Ok(val) => {
                        let clamped = val.clamp(0.0, 5.0);
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetSkyHazeIntensity(clamped),
                        ));
                        format!("-> Sky haze intensity set to: {}", clamped)
                    }
                    Err(_) => format!("Invalid float value: '{}'", trimmed),
                }
            },
        );

        // Atmospheric sky haze ambient flash
        self.commands_registry.register_for_renderer(
            "renderer.sky_haze_ambient_flash",
            move |args, cmd_queue| {
                let trimmed = args.trim();
                if trimmed.is_empty() {
                    return "Usage: renderer.sky_haze_ambient_flash [float]".into();
                }
                match trimmed.parse::<f32>() {
                    Ok(val) => {
                        let clamped = val.clamp(0.0, 2.0);
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetSkyHazeAmbientFlash(
                                clamped,
                            ),
                        ));
                        format!("-> Sky haze ambient flash set to: {}", clamped)
                    }
                    Err(_) => format!("Invalid float value: '{}'", trimmed),
                }
            },
        );

        // Volumetric lighting hysteresis
        self.commands_registry.register_for_renderer(
            "renderer.lighting.hysteresis",
            move |args, cmd_queue| {
                let trimmed = args.trim();
                let enable = match trimmed {
                    "" => true,
                    "1" | "true" | "on" => true,
                    "0" | "false" | "off" => false,
                    other => {
                        return format!("Unknown argument '{}', expected true or false", other);
                    }
                };
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetVolumetricLightingHysteresisEnabled(
                        enable,
                    ),
                ));
                format!("-> Volumetric lighting hysteresis: {}", enable)
            },
        );
        self.commands_registry
            .register_args("renderer.lighting.hysteresis", vec!["true", "false"]);
        self.commands_registry.register_hint(
            "renderer.lighting.hysteresis",
            "Usage: [true|false|1|0|on|off]",
        );

        // Volumetric lighting fade-in duration
        self.commands_registry.register_for_renderer(
            "renderer.lighting.fade_in",
            move |args, cmd_queue| {
                let trimmed = args.trim();
                if trimmed.is_empty() {
                    return "Usage: renderer.lighting.fade_in [float ms]".into();
                }
                match trimmed.parse::<f32>() {
                    Ok(val) => {
                        let clamped = val.clamp(
                            crate::renderer_engine::constants::SLIDER_VOLUMETRIC_FADE_IN_MS_MIN,
                            crate::renderer_engine::constants::SLIDER_VOLUMETRIC_FADE_IN_MS_MAX,
                        );
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetVolumetricLightingFadeInMs(
                                clamped,
                            ),
                        ));
                        format!(
                            "-> Volumetric lighting fade-in duration set to: {:.1} ms",
                            clamped
                        )
                    }
                    Err(_) => format!("Invalid float value: '{}'", trimmed),
                }
            },
        );
        self.commands_registry.register_hint(
            "renderer.lighting.fade_in",
            &format!(
                "Usage: <{:.0}-{:.0} ms>",
                crate::renderer_engine::constants::SLIDER_VOLUMETRIC_FADE_IN_MS_MIN,
                crate::renderer_engine::constants::SLIDER_VOLUMETRIC_FADE_IN_MS_MAX,
            ),
        );
        self.commands_registry.register_args(
            "renderer.lighting.fade_in",
            vec!["0.0", "50.0", "100.0", "200.0"],
        );

        // Current value getters
        let cfg = self.renderer_config.clone();
        self.commands_registry.register_current_value(
            "renderer.lighting.hysteresis",
            move |_, _| {
                cfg.read()
                    .map(|c| format!("{}", c.volumetric_lighting_hysteresis_enabled))
                    .unwrap_or("?".to_string())
            },
        );

        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_current_value("renderer.lighting.fade_in", move |_, _| {
                cfg.read()
                    .map(|c| format!("{:.1}", c.volumetric_lighting_fade_in_ms))
                    .unwrap_or("?".to_string())
            });
    }

    pub(crate) fn register_dither_commands(&mut self) {
        // Enable
        self.commands_registry.register_for_renderer(
            "renderer.dither.enable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetDitherEnabled(true),
                ));
                "-> Dither anti-banding enabled".into()
            },
        );

        // Disable
        self.commands_registry.register_for_renderer(
            "renderer.dither.disable",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetDitherEnabled(false),
                ));
                "-> Dither anti-banding disabled".into()
            },
        );

        // Toggle
        let cfg = self.renderer_config.clone();
        self.commands_registry.register_for_renderer(
            "renderer.dither.toggle",
            move |_, cmd_queue| {
                let current = cfg.read().map(|c| c.dither_enabled).unwrap_or(false);
                let next = !current;
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::SetDitherEnabled(next),
                ));
                if next {
                    "-> Dither anti-banding toggled ON".into()
                } else {
                    "-> Dither anti-banding toggled OFF".into()
                }
            },
        );

        // Strength
        self.commands_registry.register_for_renderer(
            "renderer.dither.strength",
            move |args, cmd_queue| {
                let val = args
                    .split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<f32>().ok());
                match val {
                    Some(v)
                        if (crate::renderer_engine::constants::SLIDER_DITHER_STRENGTH_MIN
                            ..=crate::renderer_engine::constants::SLIDER_DITHER_STRENGTH_MAX)
                            .contains(&v) =>
                    {
                        cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                            crate::domain_contracts::RendererCommand::SetDitherStrength(v),
                        ));
                        format!("-> Dither strength: {:.2}", v)
                    }
                    _ => format!(
                        "Usage: renderer.dither.strength <{:.1}-{:.1}>",
                        crate::renderer_engine::constants::SLIDER_DITHER_STRENGTH_MIN,
                        crate::renderer_engine::constants::SLIDER_DITHER_STRENGTH_MAX
                    ),
                }
            },
        );
        self.commands_registry.register_hint(
            "renderer.dither.strength",
            &format!(
                "Usage: <{:.1}-{:.1}>",
                crate::renderer_engine::constants::SLIDER_DITHER_STRENGTH_MIN,
                crate::renderer_engine::constants::SLIDER_DITHER_STRENGTH_MAX
            ),
        );
        self.commands_registry
            .register_args("renderer.dither.strength", vec!["0.6", "1.5", "3.0"]);

        // Reset
        self.commands_registry.register_for_renderer(
            "renderer.dither.reset",
            move |_, cmd_queue| {
                cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                    crate::domain_contracts::RendererCommand::ResetDitherDefaults,
                ));
                "-> Dither reset to defaults".into()
            },
        );

        // Current value providers
        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_current_value("renderer.dither.enable", move |_, _| {
                cfg.read()
                    .map(|c| format!("{}", c.dither_enabled))
                    .unwrap_or("?".to_string())
            });

        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_current_value("renderer.dither.strength", move |_, _| {
                cfg.read()
                    .map(|c| format!("{:.2}", c.dither_strength))
                    .unwrap_or("?".to_string())
            });
    }

    // Helper pur pour le parsing (peut être statique ou hors de la classe)
    fn parse_tonemap_mode(s: &str) -> Option<crate::renderer_engine::config::ToneMappingMode> {
        use crate::renderer_engine::config::ToneMappingMode::*;
        match s {
            "reinhard" => Some(Reinhard),
            "reinhard_extended" => Some(ReinhardExtended),
            "aces" => Some(ACES),
            "uncharted2" => Some(Uncharted2),
            "agx" => Some(AgX),
            "khronos" => Some(KhronosPBR),
            _ => None,
        }
    }
}

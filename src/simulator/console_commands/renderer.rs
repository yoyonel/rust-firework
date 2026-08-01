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

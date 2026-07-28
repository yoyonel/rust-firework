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
        let reload_flag = self.reload_shaders_requested.clone();
        self.commands_registry
            .register_for_renderer("renderer.reload_shaders", move |_| {
                reload_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                "-> Shader reload requested".to_string()
            });

        // Config View
        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.config", move |_| {
                cfg.read()
                    .map(|c| format!("{:#?}", *c))
                    .unwrap_or_else(|_| "x Lock fail".into())
            });

        // Config Save
        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.config.save", move |_| {
                if let Ok(c) = cfg.read() {
                    match c.save_to_file("assets/config/renderer.toml") {
                        Ok(_) => "-> Config saved".into(),
                        Err(e) => format!("x Save failed: {}", e),
                    }
                } else {
                    "x Lock fail".into()
                }
            });

        // Config Reload
        let cfg = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.config.reload", move |_| {
                match crate::renderer_engine::RendererConfig::from_file(
                    "assets/config/renderer.toml",
                ) {
                    Ok(new_c) => {
                        if let Ok(mut c) = cfg.write() {
                            *c = new_c;
                            "-> Config reloaded".into()
                        } else {
                            "x Lock fail".into()
                        }
                    }
                    Err(e) => format!("x Load failed: {}", e),
                }
            });
    }

    pub(crate) fn register_bloom_commands(&mut self) {
        // Macro pour éviter de répéter le config.clone() + write lock check partout
        macro_rules! update_config {
            ($self:expr, $name:expr, $logic:expr) => {
                let cfg = $self.renderer_config.clone();
                $self
                    .commands_registry
                    .register_for_renderer($name, move |args| {
                        if let Ok(mut config) = cfg.write() {
                            let f: &dyn Fn(
                                &mut crate::renderer_engine::RendererConfig,
                                &str,
                            ) -> String = &$logic;
                            f(&mut *config, args)
                        } else {
                            "x Failed to lock config".to_string()
                        }
                    });
            };
        }

        // Enable/Disable simplifiés
        update_config!(self, "renderer.bloom.enable", |c, _| {
            c.bloom_enabled = true;
            "-> Bloom enabled".into()
        });
        update_config!(self, "renderer.bloom.disable", |c, _| {
            c.bloom_enabled = false;
            "-> Bloom disabled".into()
        });

        // Intensity
        update_config!(self, "renderer.bloom.intensity", |c, args| {
            let val = args
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<f32>().ok());
            match val {
                Some(v) if (0.0..=10.0).contains(&v) => {
                    c.bloom_intensity = v;
                    format!("-> Intensity: {:.2}", v)
                }
                _ => "Usage: bloom.intensity <0.0-10.0>".into(),
            }
        });
        self.commands_registry
            .register_hint("renderer.bloom.intensity", "Usage: <0.0-10.0>");

        // Iterations
        update_config!(self, "renderer.bloom.iterations", |c, args| {
            let val = args
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u32>().ok());
            match val {
                Some(v) if (1..=10).contains(&v) => {
                    c.bloom_iterations = v;
                    format!("-> Iterations: {}", v)
                }
                _ => "Usage: bloom.iterations <1-10>".into(),
            }
        });
        self.commands_registry
            .register_hint("renderer.bloom.iterations", "Usage: <1-10>");

        // Downsample
        update_config!(self, "renderer.bloom.downsample", |c, args| {
            match args
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u32>().ok())
            {
                Some(v) if [1, 2, 4].contains(&v) => {
                    c.bloom_downsample = v;
                    format!("-> Downsample: {}x", v)
                }
                _ => "Usage: bloom.downsample <1|2|4>".into(),
            }
        });
        self.commands_registry
            .register_args("renderer.bloom.downsample", vec!["1", "2", "4"]);
        self.commands_registry
            .register_hint("renderer.bloom.downsample", "Usage: <1|2|4>");

        // Method
        update_config!(self, "renderer.bloom.method", |c, args| {
            let method = args.split_whitespace().nth(1).unwrap_or("").to_lowercase();
            match method.as_str() {
                "gaussian" => {
                    c.bloom_blur_method = crate::renderer_engine::config::BlurMethod::Gaussian;
                    "-> Method: Gaussian".into()
                }
                "kawase" => {
                    c.bloom_blur_method = crate::renderer_engine::config::BlurMethod::Kawase;
                    "-> Method: Kawase".into()
                }
                _ => "Usage: bloom.method <gaussian|kawase>".into(),
            }
        });
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
        let cfg = self.renderer_config.clone();

        self.commands_registry
            .register_for_renderer("renderer.tonemapping", move |args| {
                let mode_str = args.split_whitespace().nth(1).unwrap_or("").to_lowercase();
                // J'utilise Self::parse_tonemap_mode pour garder le code propre
                let mode = Self::parse_tonemap_mode(&mode_str);

                if let Some(m) = mode {
                    if let Ok(mut config) = cfg.write() {
                        config.tone_mapping_mode = m;
                        return format!("-> Tone mapping: {:?}", m);
                    }
                    return "x Lock fail".to_string();
                }
                "Available: reinhard, reinhard_extended, aces, uncharted2, khronos".to_string()
            });
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
        self.commands_registry
            .register_for_renderer("renderer.tonemapping.compare", move |_| {
                let old = comparison_mode.fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
                // fetch_xor retourne l'ancienne valeur. Si c'était false, c'était devenu true (Enabled).
                if !old {
                    "-> Comparison enabled"
                } else {
                    "-> Comparison disabled"
                }
                .to_string()
            });

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

use crate::audio_engine::AudioEngine;
use crate::physic_engine::{config::PhysicConfig, PhysicEngine, PhysicEngineFull, UpdateResult};
use crate::renderer_engine::utils::adaptative_sampler::{ascii_sample_timeline, AdaptiveSampler};
use crate::renderer_engine::RendererEngine;
use crate::utils::Fullscreen;
use crate::window_engine::WindowEngine;
use crate::{log_metrics_and_fps, profiler::Profiler};
use crate::{CommandRegistry, Console};
use glfw::{Action, Key, WindowMode};
use imgui_glfw_rs::glfw;
use log::{debug, info};
use std::time::Instant;

pub struct Simulator<R, P, A, W>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
    W: WindowEngine,
{
    renderer_engine: R,
    physic_engine: P,
    pub audio_engine: A,
    pub commands_registry: CommandRegistry,

    // Window & Loop management
    window_engine: W,
    pub console: Console,

    // Flags for console commands
    reload_shaders_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,

    // Renderer configuration
    renderer_config: std::sync::Arc<std::sync::RwLock<crate::renderer_engine::RendererConfig>>,

    frames: u64,
    last_time: Instant,

    // Window state
    window_size: (i32, i32),
    window_size_f32: (f32, f32),
    window_last_pos: (i32, i32),
    window_last_size: (i32, i32),

    // Loop state
    profiler: Profiler,
    sampler: AdaptiveSampler,
    sampled_fps: Vec<f32>,
    fps_avg: f32,
    fps_avg_iter: f32,
    last_log: Instant,
    first_frame: bool,
}

impl<R, P, A, W> Simulator<R, P, A, W>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
    W: WindowEngine,
{
    pub fn new(renderer_engine: R, physic_engine: P, audio_engine: A, window_engine: W) -> Self {
        let window_size = window_engine.get_size();
        let window_pos = window_engine.get_pos();

        Self {
            renderer_engine,
            physic_engine,
            audio_engine,
            commands_registry: CommandRegistry::new(),
            window_engine,
            console: Console::new(),
            reload_shaders_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                false,
            )),

            // Initialize renderer config
            renderer_config: std::sync::Arc::new(std::sync::RwLock::new(
                crate::renderer_engine::RendererConfig::from_file("assets/config/renderer.toml")
                    .unwrap_or_default(),
            )),

            frames: 0,
            last_time: Instant::now(),
            window_size,
            window_size_f32: (window_size.0 as f32, window_size.1 as f32),
            window_last_pos: window_pos,
            window_last_size: window_size,

            profiler: Profiler::new(200),
            sampler: AdaptiveSampler::new(std::time::Duration::from_secs(5), 200, 60.0),
            sampled_fps: Vec::with_capacity(200),
            fps_avg: 0.0,
            fps_avg_iter: 0.0,
            last_log: Instant::now(),
            first_frame: true,
        }
    }

    pub fn run(&mut self, export_path: Option<String>) -> anyhow::Result<()> {
        self.audio_engine.start_audio_thread(export_path.as_deref());
        self.audio_engine
            .set_listener_position((self.window_size_f32.0 / 2.0, 0.0));

        while self.step() {}

        Ok(())
    }

    pub fn step(&mut self) -> bool {
        if self.window_engine.should_close() {
            return false;
        }

        let mut reload_config = false;
        let mut reload_shaders = false;

        // Window events
        self.window_engine.poll_events();

        // Collect events into a Vec to avoid borrow checker issues
        let events: Vec<_> = glfw::flush_messages(self.window_engine.get_events()).collect();

        for (_, event) in events {
            match event {
                glfw::WindowEvent::FramebufferSize(w, h) => {
                    self.renderer_engine.set_window_size(w, h);
                    self.window_size_f32 = (w as f32, h as f32);
                    self.physic_engine.set_window_width(w as f32);
                    self.audio_engine
                        .set_listener_position(((w / 2) as f32, 0.0));
                }
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    self.window_engine.set_should_close(true);
                }
                glfw::WindowEvent::Key(Key::R, _, Action::Press, _) => {
                    if !self.console.open {
                        reload_config = true;
                    }
                }
                glfw::WindowEvent::Key(Key::S, _, Action::Press, _) => {
                    if !self.console.open {
                        reload_shaders = true;
                    }
                }
                glfw::WindowEvent::Key(Key::F11, _, Action::Press, _) => {
                    if self.window_engine.is_fullscreen() {
                        self.window_engine.set_monitor(
                            WindowMode::Windowed,
                            self.window_last_pos.0,
                            self.window_last_pos.1,
                            self.window_last_size.0 as u32,
                            self.window_last_size.1 as u32,
                            None,
                        );
                        self.window_size = self.window_last_size;
                        self.window_size_f32 = (
                            self.window_last_size.0 as f32,
                            self.window_last_size.1 as f32,
                        );
                        info!(
                            "🖥️ Window resized: {} x {}",
                            self.window_size.0, self.window_size.1
                        );
                    } else {
                        self.window_last_pos = self.window_engine.get_pos();
                        self.window_last_size = self.window_engine.get_size();

                        let mut glfw = self.window_engine.get_glfw().clone();
                        let window = self.window_engine.get_window_mut();
                        glfw.with_primary_monitor(|_, primary_monitor| {
                            if let Some(mon) = primary_monitor {
                                if let Some(video_mode) = mon.get_video_mode() {
                                    window.set_fullscreen(mon);
                                    self.window_size =
                                        (video_mode.width as i32, video_mode.height as i32);
                                    self.window_size_f32 =
                                        (self.window_size.0 as f32, self.window_size.1 as f32);
                                    info!(
                                        "🖥️ Fullscreen: {} x {}",
                                        self.window_size.0, self.window_size.1
                                    );
                                } else {
                                    info!("⚠️ Could not get monitor video mode, staying windowed");
                                }
                            }
                        });
                    }
                }
                glfw::WindowEvent::Key(Key::GraveAccent, _, Action::Press, _) => {
                    self.console.open = !self.console.open;
                    self.window_engine.set_cursor_mode(if self.console.open {
                        self.console.focus_previous_widget = true;
                        glfw::CursorMode::Normal
                    } else {
                        glfw::CursorMode::Disabled
                    });
                }
                _ => {}
            }
            // Pas besoin de helper externe, on peut le faire "inline"
            let imgui_system = self.window_engine.get_imgui_system_mut();
            imgui_system
                .glfw
                .handle_event(&mut imgui_system.context, &event);
        }
        if reload_config {
            self.reload_config();
        }
        if reload_shaders {
            self.reload_shaders();
        }

        // Check console command flags
        if self
            .reload_shaders_requested
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            self.reload_shaders_requested
                .store(false, std::sync::atomic::Ordering::Relaxed);
            self.reload_shaders();
        }

        // Update bloom intensity
        // --- Apply Bloom Parameters from Config ---
        {
            if let Ok(config) = self.renderer_config.read() {
                let bloom_pass = self.renderer_engine.bloom_pass_mut();

                bloom_pass.enabled = config.bloom_enabled;
                bloom_pass.intensity = config.bloom_intensity;
                bloom_pass.blur_iterations = config.bloom_iterations;
                bloom_pass.blur_method = match config.bloom_blur_method {
                    crate::renderer_engine::config::BlurMethod::Gaussian => {
                        crate::renderer_engine::bloom::BlurMethod::Gaussian
                    }
                    crate::renderer_engine::config::BlurMethod::Kawase => {
                        crate::renderer_engine::bloom::BlurMethod::Kawase
                    }
                };

                // Check for downsample change
                if bloom_pass.downsample_factor != config.bloom_downsample {
                    bloom_pass.downsample_factor = config.bloom_downsample;
                    unsafe {
                        bloom_pass.recreate_blur_buffers();
                    }
                }
            }
        }

        // 🔹 start global frame
        let _frame_guard = self.profiler.frame(); // RAII: mesure totale de la frame

        let now = Instant::now();
        let delta = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        self.frames += 1;

        // 🔹 Calcul FPS instantané
        let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };

        // 🔹 On demande à l’échantillonneur s’il faut enregistrer ce FPS
        if self.sampler.should_sample(delta) {
            self.sampled_fps.push(fps);
        }

        let update_result = self
            .profiler
            .profile_block("physic - update", || self.physic_engine.update(delta));
        Self::synch_audio_with_physic(&mut self.audio_engine, &update_result);

        // Render frame with all renderers
        self.profiler.profile_block("render frame", || {
            self.profiler.record_metric(
                "total particles drawn",
                self.renderer_engine.render_frame(&self.physic_engine),
            );
        });

        // moyenne pondérée EMA
        let alpha = 0.15;
        self.fps_avg = alpha * fps + (1.0 - alpha) * self.fps_avg;
        // moyenne simple itérative
        let n_frames = 100;
        self.fps_avg_iter = (self.fps_avg_iter * (n_frames - 1) as f32 + fps) / n_frames as f32;

        let log_interval = std::time::Duration::from_secs(5);
        // affichage périodique
        if self.last_log.elapsed() >= log_interval {
            log_metrics_and_fps!(&self.profiler);

            if !self.sampler.samples.is_empty() {
                // Moyenne des FPS mesurés
                let avg_fps: f32 = self
                    .sampler
                    .samples
                    .iter()
                    .map(|(_, sample_fps)| *sample_fps)
                    .sum::<f32>()
                    / self.sampler.samples.len() as f32;

                // 🔹 Graph ASCII coloré selon FPS
                let graph = ascii_sample_timeline(
                    &self.sampler.samples,
                    log_interval.as_secs_f32(),
                    50,
                    avg_fps,
                );
                info!("Graphe - Sample Timeline");
                // [Trait Iterator - for_each - Calls a closure on each element of an iterator.](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.for_each)
                graph.lines().for_each(|line| info!("{}", line));

                info!(
                    "Samples: {} / {} | Moyenne FPS: {:.2}",
                    self.sampler.samples.len(),
                    self.sampler.target_samples,
                    avg_fps
                );

                self.sampler.reset();

                info!("FPS moyen (EMA): {:.2}", self.fps_avg);
                info!("FPS moyen (iter): {:.2}", self.fps_avg_iter);
            }

            self.last_log = Instant::now();
        }

        if self.console.open {
            let (window, imgui_system) = self.window_engine.get_window_and_imgui_mut();
            let ui = imgui_system.glfw.frame(window, &mut imgui_system.context);
            self.console.draw(
                ui,
                &mut self.audio_engine,
                &mut self.physic_engine,
                &self.commands_registry,
            );
            // Get references again after draw
            let (win, sys) = self.window_engine.get_window_and_imgui_mut();
            sys.glfw.draw(&mut sys.context, win);
        }

        self.window_engine.swap_buffers();

        if self.first_frame {
            info!("🚀 First frame rendered");
            self.first_frame = false;
        }

        true
    }

    fn synch_audio_with_physic(audio_engine: &mut A, update_result: &UpdateResult) {
        if let Some(rocket) = &update_result.new_rocket {
            debug!("🚀 Rocket spawned at ({}, {})", rocket.pos.x, rocket.pos.y);
            audio_engine.play_rocket((rocket.pos.x, rocket.pos.y), 0.6);
        }

        for (i, expl) in update_result.triggered_explosions.iter().enumerate() {
            debug!(
                "💥 Explosion triggered: {} at ({}, {})",
                i, expl.pos.x, expl.pos.y
            );
            audio_engine.play_explosion((expl.pos.x, expl.pos.y), 1.0);
        }
    }

    pub fn reload_config(&mut self) {
        let physic_config =
            PhysicConfig::from_file("assets/config/physic.toml").unwrap_or_default();
        info!("Physic config loaded:\n{:#?}", physic_config);

        self.physic_engine.reload_config(&physic_config);

        let new_max = physic_config.max_rockets * physic_config.particles_per_explosion; // ou autre logique

        // TODO: This logic was in Renderer, now Simulator orchestrates it.
        // But RendererEngine trait needs to expose a way to check current max or just force recreate.
        // For now, we just call recreate_buffers.
        self.renderer_engine.recreate_buffers(new_max);
    }

    pub fn reload_shaders(&mut self) {
        info!("🔄 Reloading shaders...");
        match self.renderer_engine.reload_shaders() {
            Ok(_) => {
                self.console.log("✅ Shaders reloaded successfully");
            }
            Err(e) => {
                self.console.log(format!("❌ Shader reload failed:\n{}", e));
            }
        }
    }

    pub fn close(&mut self) {
        self.renderer_engine.close();
        self.physic_engine.close();
        self.audio_engine.stop_audio_thread();
        // Window engine cleanup happens automatically when dropped
    }

    pub fn renderer_engine(&self) -> &R {
        &self.renderer_engine
    }

    pub fn physic_engine(&self) -> &P {
        &self.physic_engine
    }

    pub fn audio_engine(&self) -> &A {
        &self.audio_engine
    }
}

impl<R, P, A, W> Simulator<R, P, A, W>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
    W: WindowEngine,
{
    pub fn init_console_commands(&mut self) {
        // Commande "mute"
        self.commands_registry.register_for_audio(
            "audio.mute",
            |engine: &mut dyn AudioEngine, _args| {
                engine.mute();
                "Audio muted".to_string()
            },
        );

        // Tu pourrais ajouter d'autres commandes ici (unmute, volume, etc.)
        self.commands_registry.register_for_audio(
            "audio.unmute",
            |engine: &mut dyn AudioEngine, _args| {
                engine.unmute();
                "Audio unmuted".to_string()
            },
        );

        self.commands_registry
            // register_physic est ici une méthode qui stocke la closure pour
            // exécution future.
            .register_for_physic("physic.config", |engine: &mut dyn PhysicEngine, _args| {
                // <-- LE CAST À L'INTÉRIEUR DE LA CLOSURE
                // Le moteur passé ici n'est que la partie Dyn Compatible.
                // Or, get_config() est bien dans PhysicEngine (maintenant Dyn Compatible).
                format!("{:#?}", engine.get_config())
            });

        // Register renderer commands
        let reload_flag = self.reload_shaders_requested.clone();
        self.commands_registry
            .register_for_renderer("renderer.reload_shaders", move |_args| {
                reload_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                "✅ Shader reload requested".to_string()
            });

        // --- Bloom Commands ---
        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.bloom.enable", move |_args| {
                if let Ok(mut config) = config_clone.write() {
                    config.bloom_enabled = true;
                    "✅ Bloom enabled".to_string()
                } else {
                    "❌ Failed to lock config".to_string()
                }
            });

        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.bloom.disable", move |_args| {
                if let Ok(mut config) = config_clone.write() {
                    config.bloom_enabled = false;
                    "✅ Bloom disabled".to_string()
                } else {
                    "❌ Failed to lock config".to_string()
                }
            });

        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.bloom.intensity", move |args| {
                if args.trim().is_empty() {
                    return "Usage: bloom.intensity <value> (0.0-2.0)".to_string();
                }
                let value_str = args.split_whitespace().nth(1).unwrap_or("");
                match value_str.parse::<f32>() {
                    Ok(val) if (0.0..=2.0).contains(&val) => {
                        if let Ok(mut config) = config_clone.write() {
                            config.bloom_intensity = val;
                            format!("✅ Bloom intensity set to {:.2}", val)
                        } else {
                            "❌ Failed to lock config".to_string()
                        }
                    }
                    Ok(val) => format!("❌ Value {:.2} out of range [0.0, 2.0]", val),
                    Err(_) => "❌ Invalid number".to_string(),
                }
            });

        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.bloom.iterations", move |args| {
                if args.trim().is_empty() {
                    return "Usage: bloom.iterations <count> (1-10)".to_string();
                }
                let value_str = args.split_whitespace().nth(1).unwrap_or("");
                match value_str.parse::<u32>() {
                    Ok(val) if (1..=10).contains(&val) => {
                        if let Ok(mut config) = config_clone.write() {
                            config.bloom_iterations = val;
                            format!("✅ Bloom iterations set to {}", val)
                        } else {
                            "❌ Failed to lock config".to_string()
                        }
                    }
                    Ok(val) => format!("❌ Value {} out of range [1, 10]", val),
                    Err(_) => "❌ Invalid number".to_string(),
                }
            });

        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.bloom.downsample", move |args| {
                if args.trim().is_empty() {
                    return "Usage: bloom.downsample <factor> (1=full, 2=half, 4=quarter)"
                        .to_string();
                }
                let value_str = args.split_whitespace().nth(1).unwrap_or("");
                match value_str.parse::<u32>() {
                    Ok(1) | Ok(2) | Ok(4) => {
                        let val = value_str.parse::<u32>().unwrap();
                        if let Ok(mut config) = config_clone.write() {
                            config.bloom_downsample = val;
                            format!("✅ Bloom downsample set to {}x", val)
                        } else {
                            "❌ Failed to lock config".to_string()
                        }
                    }
                    Ok(val) => format!("❌ Value {} invalid. Use 1, 2, or 4", val),
                    Err(_) => "❌ Invalid number".to_string(),
                }
            });

        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.bloom.method", move |args| {
                if args.trim().is_empty() {
                    return "Usage: bloom.method <gaussian|kawase>".to_string();
                }
                let method_str = args.split_whitespace().nth(1).unwrap_or("").to_lowercase();
                match method_str.as_str() {
                    "gaussian" => {
                        if let Ok(mut config) = config_clone.write() {
                            config.bloom_blur_method =
                                crate::renderer_engine::config::BlurMethod::Gaussian;
                            "✅ Bloom method set to Gaussian".to_string()
                        } else {
                            "❌ Failed to lock config".to_string()
                        }
                    }
                    "kawase" => {
                        if let Ok(mut config) = config_clone.write() {
                            config.bloom_blur_method =
                                crate::renderer_engine::config::BlurMethod::Kawase;
                            "✅ Bloom method set to Kawase (Dual Filtering)".to_string()
                        } else {
                            "❌ Failed to lock config".to_string()
                        }
                    }
                    _ => format!(
                        "❌ Unknown method '{}'. Use 'gaussian' or 'kawase'",
                        method_str
                    ),
                }
            });

        // New command: renderer.config
        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.config", move |_args| {
                if let Ok(config) = config_clone.read() {
                    format!("{:#?}", *config)
                } else {
                    "❌ Failed to read config".to_string()
                }
            });

        // New command: renderer.config.save
        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.config.save", move |_args| {
                if let Ok(config) = config_clone.read() {
                    match config.save_to_file("assets/config/renderer.toml") {
                        Ok(_) => "✅ Config saved to assets/config/renderer.toml".to_string(),
                        Err(e) => format!("❌ Failed to save config: {}", e),
                    }
                } else {
                    "❌ Failed to read config".to_string()
                }
            });

        // New command: renderer.config.reload
        let config_clone = self.renderer_config.clone();
        self.commands_registry
            .register_for_renderer("renderer.config.reload", move |_args| {
                match crate::renderer_engine::RendererConfig::from_file(
                    "assets/config/renderer.toml",
                ) {
                    Ok(new_config) => {
                        if let Ok(mut config) = config_clone.write() {
                            *config = new_config;
                            "✅ Config reloaded from assets/config/renderer.toml".to_string()
                        } else {
                            "❌ Failed to lock config for writing".to_string()
                        }
                    }
                    Err(e) => format!("❌ Failed to load config: {}", e),
                }
            });
    }
}

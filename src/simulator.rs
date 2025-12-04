use crate::audio_engine::AudioEngine;
use crate::physic_engine::{config::PhysicConfig, PhysicEngineFull, UpdateResult};
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

    // Tone mapping comparison
    pub tonemapping_comparison_mode: std::sync::Arc<std::sync::atomic::AtomicBool>,
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
            tonemapping_comparison_mode: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                false,
            )),
        }
    }

    pub fn run(&mut self, export_path: Option<String>) -> anyhow::Result<()> {
        self.audio_engine.start_audio_thread(export_path.as_deref());
        self.audio_engine
            .set_listener_position((self.window_size_f32.0 / 2.0, 0.0));

        while self.step() {}

        Ok(())
    }

    /// Main Loop Step
    pub fn step(&mut self) -> bool {
        // Early exit check
        if self.window_engine.should_close() {
            return false;
        }

        // 1. Gestion des événements
        let (reload_config, reload_shaders) = self.handle_window_events();

        // 2. Application des rechargements
        self.apply_reload_requests(reload_config, reload_shaders);

        // 3. Synchronisation config renderer
        self.sync_renderer_config();

        // 4. Timing
        let _frame_guard = self.profiler.frame(); // RAII timing
        let delta = self.update_frame_timing();

        // 5. Simulation physique + audio
        self.update_simulation(delta);

        // 6. Rendu
        self.render_frame();

        // 7. Logs périodiques
        self.log_metrics_periodically(delta);

        // 8. UI (console + labels)
        self.render_ui();

        // 9. Finalisation
        self.finalize_frame();

        true
    }

    // --- Helper Methods ---

    fn handle_window_events(&mut self) -> (bool, bool) {
        let mut reload_config = false;
        let mut reload_shaders = false;

        self.window_engine.poll_events();
        let events: Vec<_> = glfw::flush_messages(self.window_engine.get_events()).collect();

        for (_, event) in events {
            match event {
                glfw::WindowEvent::FramebufferSize(w, h) => self.handle_resize(w, h),
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    self.window_engine.set_should_close(true);
                }
                glfw::WindowEvent::Key(Key::R, _, Action::Press, _) if !self.console.open => {
                    reload_config = true;
                }
                glfw::WindowEvent::Key(Key::S, _, Action::Press, _) if !self.console.open => {
                    reload_shaders = true;
                }
                glfw::WindowEvent::Key(Key::F11, _, Action::Press, _) => {
                    self.toggle_fullscreen();
                }
                glfw::WindowEvent::Key(Key::GraveAccent, _, Action::Press, _) => {
                    self.toggle_console();
                }
                _ => {}
            }

            // ImGui Input Handling
            // ImGui Input Handling
            let is_key_event = matches!(
                event,
                glfw::WindowEvent::Key(_, _, _, _) | glfw::WindowEvent::Char(_)
            );

            if self.console.open || !is_key_event {
                let imgui_system = self.window_engine.get_imgui_system_mut();
                imgui_system
                    .glfw
                    .handle_event(&mut imgui_system.context, &event);
            }
        }

        (reload_config, reload_shaders)
    }

    fn handle_resize(&mut self, w: i32, h: i32) {
        self.renderer_engine.set_window_size(w, h);
        self.window_size_f32 = (w as f32, h as f32);
        self.physic_engine.set_window_width(w as f32);
        self.audio_engine
            .set_listener_position(((w / 2) as f32, 0.0));
    }

    fn toggle_fullscreen(&mut self) {
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
                        self.window_size = (video_mode.width as i32, video_mode.height as i32);
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

    fn toggle_console(&mut self) {
        self.console.open = !self.console.open;
        self.window_engine.set_cursor_mode(if self.console.open {
            self.console.focus_previous_widget = true;
            glfw::CursorMode::Normal
        } else {
            glfw::CursorMode::Disabled
        });
    }

    fn apply_reload_requests(&mut self, reload_config: bool, reload_shaders: bool) {
        if reload_config {
            self.reload_config();
        }

        let atomic_reload = self
            .reload_shaders_requested
            .load(std::sync::atomic::Ordering::Relaxed);

        if reload_shaders || atomic_reload {
            if atomic_reload {
                self.reload_shaders_requested
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            self.reload_shaders();
        }
    }

    fn sync_renderer_config(&mut self) {
        // Apply Bloom Parameters from Config
        if let Ok(config) = self.renderer_config.read() {
            self.renderer_engine.sync_bloom_config(&config);
        }

        // Sync comparison mode with BloomPass
        let comparison_active = self
            .tonemapping_comparison_mode
            .load(std::sync::atomic::Ordering::Relaxed);
        self.renderer_engine.bloom_pass_mut().comparison_mode = comparison_active;
    }

    fn update_frame_timing(&mut self) -> f32 {
        let now = Instant::now();
        let delta = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        self.frames += 1;

        // Instant FPS for sampling
        let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };

        if self.sampler.should_sample(delta) {
            self.sampled_fps.push(fps);
        }

        // Calculate averages
        let alpha = 0.15;
        self.fps_avg = alpha * fps + (1.0 - alpha) * self.fps_avg;

        let n_frames = 100;
        self.fps_avg_iter = (self.fps_avg_iter * (n_frames - 1) as f32 + fps) / n_frames as f32;

        delta
    }

    fn update_simulation(&mut self, delta: f32) {
        let update_result = self
            .profiler
            .profile_block("physic - update", || self.physic_engine.update(delta));
        Self::synch_audio_with_physic(&mut self.audio_engine, &update_result);
    }

    fn render_frame(&mut self) {
        self.profiler.profile_block("render frame", || {
            self.profiler.record_metric(
                "total particles drawn",
                self.renderer_engine.render_frame(&self.physic_engine),
            );
        });

        // Render comparison textures if mode is active
        let comparison_active = self
            .tonemapping_comparison_mode
            .load(std::sync::atomic::Ordering::Relaxed);

        if comparison_active {
            unsafe {
                self.renderer_engine.bloom_pass_mut().render_comparison();
            }
        }
    }

    fn log_metrics_periodically(&mut self, _delta: f32) {
        let log_interval = std::time::Duration::from_secs(5);

        if self.last_log.elapsed() < log_interval {
            return;
        }

        log_metrics_and_fps!(&self.profiler);

        if !self.sampler.samples.is_empty() {
            let avg_fps: f32 = self
                .sampler
                .samples
                .iter()
                .map(|(_, fps)| *fps)
                .sum::<f32>()
                / self.sampler.samples.len() as f32;

            let graph = ascii_sample_timeline(
                &self.sampler.samples,
                log_interval.as_secs_f32(),
                50,
                avg_fps,
            );

            info!("Graphe - Sample Timeline");
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

    fn render_ui(&mut self) {
        let comparison_active = self
            .tonemapping_comparison_mode
            .load(std::sync::atomic::Ordering::Relaxed);

        if !self.console.open && !comparison_active {
            return;
        }

        let (window, imgui_system) = self.window_engine.get_window_and_imgui_mut();
        let ui = imgui_system.glfw.frame(window, &mut imgui_system.context);

        // Draw comparison labels (background)
        if comparison_active {
            let (positions, labels) = self
                .renderer_engine
                .bloom_pass_mut()
                .get_comparison_grid_info();
            let draw_list = ui.get_background_draw_list();

            for ((x, y, _w, _h), &label) in positions.iter().zip(labels.iter()) {
                let text_x = x + 10.0;
                let text_y = y + 10.0;
                let text_size = ui.calc_text_size(label);
                let padding = 5.0;

                draw_list
                    .add_rect(
                        [text_x - padding, text_y - padding],
                        [
                            text_x + text_size[0] + padding,
                            text_y + text_size[1] + padding,
                        ],
                        [0.0, 0.0, 0.0, 0.8],
                    )
                    .filled(true)
                    .build();
                draw_list.add_text([text_x, text_y], [1.0, 1.0, 1.0, 1.0], label);
            }
        }

        // Draw console (foreground)
        if self.console.open {
            self.console.draw(
                ui,
                &mut self.audio_engine,
                &mut self.physic_engine,
                &self.commands_registry,
            );
        }

        // Finalize ImGui Draw
        let (win, sys) = self.window_engine.get_window_and_imgui_mut();
        sys.glfw.draw(&mut sys.context, win);
    }

    fn finalize_frame(&mut self) {
        self.window_engine.swap_buffers();

        if self.first_frame {
            info!("🚀 First frame rendered");
            self.first_frame = false;
        }
    }

    fn synch_audio_with_physic(audio_engine: &mut A, update_result: &UpdateResult) {
        if let Some(rocket) = &update_result.new_rocket {
            debug!("🚀 Rocket spawned at ({}, {})", rocket.pos.x, rocket.pos.y);
            audio_engine.play_rocket((rocket.pos.x, rocket.pos.y), 0.8);
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
        let new_max = physic_config.max_rockets * physic_config.particles_per_explosion;
        self.renderer_engine.recreate_buffers(new_max);
    }

    pub fn reload_shaders(&mut self) {
        info!("🔄 Reloading shaders...");
        match self.renderer_engine.reload_shaders() {
            Ok(_) => {
                self.console.log("-> Shaders reloaded successfully");
            }
            Err(e) => {
                self.console.log(format!("x Shader reload failed:\n{}", e));
            }
        }
    }

    pub fn close(&mut self) {
        self.renderer_engine.close();
        self.physic_engine.close();
        self.audio_engine.stop_audio_thread();
    }

    // Command registry init omitted for brevity, logic remains identical to original...
    pub fn init_console_commands(&mut self) {
        self.register_audio_commands();
        self.register_physic_commands();
        self.register_renderer_base_commands();
        self.register_bloom_commands();
        self.register_tonemapping_commands();
    }

    fn register_audio_commands(&mut self) {
        self.commands_registry
            .register_for_audio("audio.mute", |engine, _| {
                engine.mute();
                "Audio muted".to_string()
            });

        self.commands_registry
            .register_for_audio("audio.unmute", |engine, _| {
                engine.unmute();
                "Audio unmuted".to_string()
            });
    }

    fn register_physic_commands(&mut self) {
        self.commands_registry
            .register_for_physic("physic.config", |engine, _| {
                format!("{:#?}", engine.get_config())
            });
    }

    fn register_renderer_base_commands(&mut self) {
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

    fn register_bloom_commands(&mut self) {
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
    }

    fn register_tonemapping_commands(&mut self) {
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

        // Comparison Toggle
        let comparison_mode = self.tonemapping_comparison_mode.clone();
        self.commands_registry
            .register_for_renderer("renderer.tonemapping.compare", move |_| {
                let old = comparison_mode.fetch_xor(true, std::sync::atomic::Ordering::Relaxed);
                // fetch_xor retourne l'ancienne valeur. Si c'était false, c'est devenu true (Enabled).
                if !old {
                    "-> Comparison enabled"
                } else {
                    "-> Comparison disabled"
                }
                .to_string()
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

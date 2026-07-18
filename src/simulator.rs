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

/// Macro pour créer une zone Tracy **sans conditionner l'exécution du code**.
/// Utilisation : tracy_zone!("simulator::physics", 0xFF5500, self.update_simulation(delta));
#[cfg(feature = "tracy")]
macro_rules! tracy_zone {
    ($name:expr, $color:expr, $block:expr) => {{
        let _span = tracy_client::span!($name);
        _span.emit_color($color);
        $block
    }};
}

/// Macro vide si Tracy n'est pas activé
#[cfg(not(feature = "tracy"))]
macro_rules! tracy_zone {
    ($name:expr, $color:expr, $block:expr) => {
        $block
    };
}

/// Crée une zone Tracy + émet une valeur (pour les métriques).
#[cfg(feature = "tracy")]
macro_rules! tracy_zone_with_value {
    ($name:expr, $color:expr, $value:expr) => {
        let _span = tracy_client::span!($name);
        _span.emit_color($color);
        _span.emit_value($value as u64);
    };
}

#[cfg(not(feature = "tracy"))]
macro_rules! tracy_zone_with_value {
    ($name:expr, $color:expr, $value:expr) => {};
}

macro_rules! ui_text {
    ($ui:expr, $($arg:tt)*) => {
        let mut buf = [0u8; 256];
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        if std::io::Write::write_fmt(&mut cursor, format_args!($($arg)*)).is_ok() {
            let pos = cursor.position() as usize;
            if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                $ui.text(s);
            }
        }
    };
}

macro_rules! ui_text_colored {
    ($ui:expr, $color:expr, $($arg:tt)*) => {
        let mut buf = [0u8; 256];
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        if std::io::Write::write_fmt(&mut cursor, format_args!($($arg)*)).is_ok() {
            let pos = cursor.position() as usize;
            if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                $ui.text_colored($color, s);
            }
        }
    };
}

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
    pub last_audio_debug_update: Instant,
    pub show_audio_diagnostic: bool,

    // Tone mapping comparison
    pub tonemapping_comparison_mode: std::sync::Arc<std::sync::atomic::AtomicBool>,

    // NOUVEAU: Statistiques et tracking des requêtes audio
    pub audio_debug_records:
        std::collections::VecDeque<crate::audio_engine::types::AudioDebugRecord>,
    pub audio_events_buf: Vec<crate::audio_engine::types::AudioDebugEvent>,
    pub audio_sent_rocket: u64,
    pub audio_received_rocket: u64,
    pub audio_played_rocket: u64,
    pub audio_dropped_rocket: u64,
    pub audio_completed_rocket: u64,
    pub audio_sent_explosion: u64,
    pub audio_received_explosion: u64,
    pub audio_played_explosion: u64,
    pub audio_dropped_explosion: u64,
    pub audio_completed_explosion: u64,

    pub latency_dispatch_sum: std::time::Duration,
    pub latency_dispatch_count: u64,
    pub latency_play_sum: std::time::Duration,
    pub latency_play_count: u64,
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
            last_audio_debug_update: Instant::now(),
            show_audio_diagnostic: false,
            tonemapping_comparison_mode: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                false,
            )),
            audio_debug_records: std::collections::VecDeque::with_capacity(128),
            audio_events_buf: Vec::with_capacity(2048),
            audio_sent_rocket: 0,
            audio_received_rocket: 0,
            audio_played_rocket: 0,
            audio_dropped_rocket: 0,
            audio_completed_rocket: 0,
            audio_sent_explosion: 0,
            audio_received_explosion: 0,
            audio_played_explosion: 0,
            audio_dropped_explosion: 0,
            audio_completed_explosion: 0,
            latency_dispatch_sum: std::time::Duration::ZERO,
            latency_dispatch_count: 0,
            latency_play_sum: std::time::Duration::ZERO,
            latency_play_count: 0,
        }
    }

    pub fn run(&mut self, export_path: Option<String>) -> anyhow::Result<()> {
        self.audio_engine.start_audio_thread(export_path.as_deref());
        self.audio_engine
            .set_listener_position(glam::Vec2::new(self.window_size_f32.0 / 2.0, 0.0));

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
        tracy_zone!(
            "simulator::physics",
            0xFF5500, // Orange
            {
                self.update_simulation(delta);
                if (self.console.open || self.show_audio_diagnostic)
                    && self.last_audio_debug_update.elapsed()
                        >= std::time::Duration::from_millis(16)
                {
                    self.process_audio_debug_events();
                    self.last_audio_debug_update = std::time::Instant::now();
                }
            }
        );

        // 6. Rendu
        tracy_zone!(
            "simulator::render",
            0x00FF00, // Vert
            self.render_frame()
        );

        // 7. Logs périodiques
        tracy_zone!(
            "simulator::log_metrics",
            0xFFFFFF, // Vert
            self.log_metrics_periodically(delta)
        );

        // 8. UI (console + labels)
        tracy_zone!("simulator::render_ui", 0x00FF55, self.render_ui());

        // 9. Finalisation
        tracy_zone!(
            "simulator::finalize_frame(swap_buffer)",
            0xFF0055,
            self.finalize_frame()
        );

        true
    }

    // --- Helper Methods ---

    fn process_audio_debug_events(&mut self) {
        self.audio_events_buf.clear();
        self.audio_engine
            .pop_debug_events(&mut self.audio_events_buf);
        for evt in self.audio_events_buf.drain(..) {
            match evt {
                crate::audio_engine::types::AudioDebugEvent::Sent {
                    request_id,
                    sound_type,
                    entity_id,
                    sent_at,
                } => {
                    self.audio_debug_records.push_back(
                        crate::audio_engine::types::AudioDebugRecord {
                            request_id,
                            sound_type,
                            entity_id,
                            sent_at,
                            received_at: None,
                            started_at: None,
                            dropped_at: None,
                            completed_at: None,
                            status: crate::audio_engine::types::AudioPlayStatus::Sent,
                            voice_index: None,
                            drop_reason: None,
                        },
                    );
                    if self.audio_debug_records.len() > 100 {
                        self.audio_debug_records.pop_front();
                    }
                    match sound_type {
                        crate::audio_engine::types::AudioSoundType::Rocket => {
                            self.audio_sent_rocket += 1;
                        }
                        crate::audio_engine::types::AudioSoundType::Explosion => {
                            self.audio_sent_explosion += 1;
                        }
                    }
                }
                crate::audio_engine::types::AudioDebugEvent::Received {
                    request_id,
                    received_at,
                } => {
                    if let Some(rec) = self
                        .audio_debug_records
                        .iter_mut()
                        .find(|r| r.request_id == request_id)
                    {
                        rec.received_at = Some(received_at);
                        rec.status = crate::audio_engine::types::AudioPlayStatus::Received;
                        let latency = received_at.duration_since(rec.sent_at);
                        self.latency_dispatch_sum += latency;
                        self.latency_dispatch_count += 1;
                        match rec.sound_type {
                            crate::audio_engine::types::AudioSoundType::Rocket => {
                                self.audio_received_rocket += 1;
                            }
                            crate::audio_engine::types::AudioSoundType::Explosion => {
                                self.audio_received_explosion += 1;
                            }
                        }
                    }
                }
                crate::audio_engine::types::AudioDebugEvent::Started {
                    request_id,
                    started_at,
                    voice_index,
                } => {
                    if let Some(rec) = self
                        .audio_debug_records
                        .iter_mut()
                        .find(|r| r.request_id == request_id)
                    {
                        rec.started_at = Some(started_at);
                        rec.voice_index = Some(voice_index);
                        rec.status = crate::audio_engine::types::AudioPlayStatus::Playing;
                        let latency = started_at.duration_since(rec.sent_at);
                        self.latency_play_sum += latency;
                        self.latency_play_count += 1;
                        match rec.sound_type {
                            crate::audio_engine::types::AudioSoundType::Rocket => {
                                self.audio_played_rocket += 1;
                            }
                            crate::audio_engine::types::AudioSoundType::Explosion => {
                                self.audio_played_explosion += 1;
                            }
                        }
                    }
                }
                crate::audio_engine::types::AudioDebugEvent::Dropped {
                    request_id,
                    dropped_at,
                    reason,
                } => {
                    if let Some(rec) = self
                        .audio_debug_records
                        .iter_mut()
                        .find(|r| r.request_id == request_id)
                    {
                        rec.dropped_at = Some(dropped_at);
                        rec.drop_reason = Some(reason);
                        rec.status = crate::audio_engine::types::AudioPlayStatus::Dropped;
                        match rec.sound_type {
                            crate::audio_engine::types::AudioSoundType::Rocket => {
                                self.audio_dropped_rocket += 1;
                            }
                            crate::audio_engine::types::AudioSoundType::Explosion => {
                                self.audio_dropped_explosion += 1;
                            }
                        }
                        log::warn!(
                            "⚠️ AUDIO DROPPED: request #{} ({:?}) for entity {} was dropped: {}",
                            request_id,
                            rec.sound_type,
                            rec.entity_id,
                            reason
                        );
                    }
                }
                crate::audio_engine::types::AudioDebugEvent::Completed {
                    request_id,
                    completed_at,
                } => {
                    if let Some(rec) = self
                        .audio_debug_records
                        .iter_mut()
                        .find(|r| r.request_id == request_id)
                    {
                        rec.completed_at = Some(completed_at);
                        rec.status = crate::audio_engine::types::AudioPlayStatus::Completed;
                        match rec.sound_type {
                            crate::audio_engine::types::AudioSoundType::Rocket => {
                                self.audio_completed_rocket += 1;
                            }
                            crate::audio_engine::types::AudioSoundType::Explosion => {
                                self.audio_completed_explosion += 1;
                            }
                        }
                    }
                }
            }
        }
    }

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
                glfw::WindowEvent::Key(Key::F3, _, Action::Press, _) => {
                    self.show_audio_diagnostic = !self.show_audio_diagnostic;
                    self.update_cursor_mode();
                }
                glfw::WindowEvent::Key(Key::GraveAccent, _, Action::Press, _) => {
                    self.toggle_console();
                }
                _ => {}
            }

            // ImGui Input Handling
            let is_key_event = matches!(
                event,
                glfw::WindowEvent::Key(_, _, _, _) | glfw::WindowEvent::Char(_)
            );

            if self.console.open || self.show_audio_diagnostic || !is_key_event {
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
            .set_listener_position(glam::Vec2::new((w / 2) as f32, 0.0));
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
        if self.console.open {
            self.console.focus_previous_widget = true;
        }
        self.update_cursor_mode();
    }

    fn update_cursor_mode(&mut self) {
        let cursor_mode = if self.console.open || self.show_audio_diagnostic {
            glfw::CursorMode::Normal
        } else {
            glfw::CursorMode::Disabled
        };
        self.window_engine.set_cursor_mode(cursor_mode);
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

        tracy_zone_with_value!(
            "physics::update",
            0xAA00FF, // Violet
            update_result.new_rocket.as_ref().map_or(0, |_| 1)
        );
    }

    fn render_frame(&mut self) {
        let particles_drawn = self.renderer_engine.render_frame(&self.physic_engine);
        self.profiler
            .record_metric("total particles drawn", particles_drawn);

        tracy_zone_with_value!("render_frame::main_pass", 0xFF00FF, particles_drawn); // Magenta

        // Render comparison textures if mode is active
        let comparison_active = self
            .tonemapping_comparison_mode
            .load(std::sync::atomic::Ordering::Relaxed);

        if comparison_active {
            unsafe {
                tracy_zone!(
                    "render_frame::comparison",
                    0x00FFFF, // Cyan
                    self.renderer_engine.bloom_pass_mut().render_comparison()
                );
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

        // NOUVEAU: Fenêtre ImGui de diagnostic Audio (indépendante de la console, toggle via F3)
        if self.show_audio_diagnostic {
            {
                // NOUVEAU : Dessiner l'indicateur graphique de l'auditeur (Listener) en arrière-plan
                let draw_list = ui.get_background_draw_list();
                let window_width = ui.io().display_size[0];
                let window_height = ui.io().display_size[1];

                let listener_x = window_width * 0.5;
                let listener_y = window_height;

                // Icône de casque (dessin vectoriel)
                draw_list
                    .add_circle([listener_x, listener_y - 20.0], 12.0, [0.0, 1.0, 0.0, 0.8])
                    .thickness(2.0)
                    .build();
                draw_list
                    .add_circle([listener_x, listener_y - 20.0], 4.0, [0.0, 1.0, 0.0, 1.0])
                    .filled(true)
                    .build();

                let label = "🎧 Listener (Sol / Centre)";
                let text_size = ui.calc_text_size(label);
                draw_list.add_text(
                    [listener_x - text_size[0] * 0.5, listener_y - 45.0],
                    [0.0, 1.0, 0.0, 1.0],
                    label,
                );

                // Cercle indicatif de la zone de volume max (ref_distance = 50px)
                draw_list
                    .add_circle([listener_x, listener_y], 50.0, [0.0, 0.8, 1.0, 0.35])
                    .thickness(1.5)
                    .build();
                draw_list.add_text(
                    [listener_x + 55.0, listener_y - 20.0],
                    [0.0, 0.8, 1.0, 0.7],
                    "Volume Max (50px)",
                );

                // Cercle de la zone d'atténuation (max_distance réelle récupérée dynamiquement)
                let max_dist = self.audio_engine.get_max_distance();
                draw_list
                    .add_circle([listener_x, listener_y], max_dist, [1.0, 0.5, 0.0, 0.15])
                    .thickness(1.5)
                    .build();

                let text_y = (listener_y - max_dist).max(10.0);
                let mut buf = [0u8; 64];
                let mut cursor = std::io::Cursor::new(&mut buf[..]);
                use std::io::Write;
                let _ = write!(cursor, "Zone d'attenuation (max {}px)", max_dist as u32);
                let pos = cursor.position() as usize;
                if let Ok(label_text) = std::str::from_utf8(&buf[..pos]) {
                    draw_list.add_text(
                        [listener_x + 10.0, text_y],
                        [1.0, 0.5, 0.0, 0.5],
                        label_text,
                    );
                }
            }

            let window_width = ui.io().display_size[0];
            let window_height = ui.io().display_size[1];
            ui.window("Audio Diagnostic Monitor")
                .size([window_width * 0.45, window_height * 0.45], imgui::Condition::FirstUseEver)
                .position([window_width * 0.53, window_height * 0.52], imgui::Condition::FirstUseEver)
                .resizable(true)
                .collapsible(true)
                .build(|| {
                    ui.text("=== AUDIO ENGINE REAL-TIME DIAGNOSTIC ===");
                    ui.separator();

                    // Statistiques globales
                    ui_text!(
                        ui,
                        "Rockets: Sent: {}, Received: {}, Played: {}, Dropped: {}, Completed: {}",
                        self.audio_sent_rocket,
                        self.audio_received_rocket,
                        self.audio_played_rocket,
                        self.audio_dropped_rocket,
                        self.audio_completed_rocket
                    );

                    ui_text!(
                        ui,
                        "Explosions: Sent: {}, Received: {}, Played: {}, Dropped: {}, Completed: {}",
                        self.audio_sent_explosion,
                        self.audio_received_explosion,
                        self.audio_played_explosion,
                        self.audio_dropped_explosion,
                        self.audio_completed_explosion
                    );

                    ui.separator();
                    ui.text("=== LATENCY QUANTIFICATION ===");

                    let avg_dispatch = if self.latency_dispatch_count > 0 {
                        self.latency_dispatch_sum.as_secs_f64() * 1000.0 / self.latency_dispatch_count as f64
                    } else {
                        0.0
                    };

                    let avg_play = if self.latency_play_count > 0 {
                        self.latency_play_sum.as_secs_f64() * 1000.0 / self.latency_play_count as f64
                    } else {
                        0.0
                    };

                    ui_text!(ui, "Avg thread transit latency: {:.3} ms", avg_dispatch);
                    ui_text!(ui, "Avg render-to-audio-start latency: {:.3} ms", avg_play);

                    // Warning indicator if anything dropped
                    let total_dropped = self.audio_dropped_rocket + self.audio_dropped_explosion;
                    if total_dropped > 0 {
                        ui_text_colored!(ui, [1.0, 0.0, 0.0, 1.0], "⚠️ CRITICAL: {} SOUNDS DROPPED!", total_dropped);
                    } else {
                        ui.text_colored([0.0, 1.0, 0.0, 1.0], "   All sounds successfully dispatched and mixed");
                    }

                    ui.separator();
                    ui.text("=== RECENT AUDIO EVENT LOG ===");

                    ui.child_window("RecentLogChild")
                        .size([0.0, 180.0])
                        .build(|| {
                            for r in self.audio_debug_records.iter().rev().take(15) {
                                let type_str = match r.sound_type {
                                    crate::audio_engine::types::AudioSoundType::Rocket => "ROCKET",
                                    crate::audio_engine::types::AudioSoundType::Explosion => "EXPLOSION",
                                };
                                let status_color = match r.status {
                                    crate::audio_engine::types::AudioPlayStatus::Sent => [0.7, 0.7, 0.7, 1.0],
                                    crate::audio_engine::types::AudioPlayStatus::Received => [0.2, 0.6, 1.0, 1.0],
                                    crate::audio_engine::types::AudioPlayStatus::Playing => [0.0, 1.0, 0.0, 1.0],
                                    crate::audio_engine::types::AudioPlayStatus::Dropped => [1.0, 0.0, 0.0, 1.0],
                                    crate::audio_engine::types::AudioPlayStatus::Completed => [0.5, 0.5, 0.5, 1.0],
                                };

                                let transit_ms = r.received_at.map(|t| t.duration_since(r.sent_at).as_secs_f64() * 1000.0);
                                let start_ms = r.started_at.map(|t| t.duration_since(r.sent_at).as_secs_f64() * 1000.0);

                                ui_text!(
                                    ui,
                                    "#{:<3} {:<9} (id:{:<2}) - ",
                                    r.request_id, type_str, r.entity_id
                                );
                                ui.same_line();
                                ui_text_colored!(ui, status_color, "{:?}", r.status);

                                if r.status == crate::audio_engine::types::AudioPlayStatus::Dropped {
                                    if let Some(reason) = r.drop_reason {
                                        ui.same_line();
                                        ui_text!(ui, "({})", reason);
                                    }
                                } else {
                                    if let Some(t_ms) = transit_ms {
                                        ui.same_line();
                                        ui_text!(ui, " | Transit: {:.2}ms", t_ms);
                                    }
                                    if let Some(s_ms) = start_ms {
                                        ui.same_line();
                                        ui_text!(ui, " | Render-to-start: {:.2}ms", s_ms);
                                    }
                                }
                            }
                        });
                });
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
            // MODIFIÉ : On utilise play_rocket_with_id en transmettant rocket.id !
            audio_engine.play_rocket_with_id(rocket.id, rocket.pos, 0.8);
        }

        for (i, expl) in update_result.triggered_explosions.iter().enumerate() {
            debug!(
                "💥 Explosion triggered: {} at ({}, {})",
                i, expl.pos.x, expl.pos.y
            );
            audio_engine.play_explosion(expl.pos, 1.0);
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

        // --- Commandes de contrôle des effets DSP ---

        // audio.fx <effect_name> <on|off>
        // Toggle un effet DSP à chaud. Lock-free, sans overhead sur le thread CPAL.
        self.commands_registry
            .register_for_audio("audio.fx", |engine, input| {
                use crate::audio_engine::effect_flags::AudioEffect;
                let parts: Vec<&str> = input.split_whitespace().collect();
                match parts.as_slice() {
                    [_, effect_name, state] => {
                        if let Ok(fx) = effect_name.parse::<AudioEffect>() {
                            let enabled = matches!(*state, "on" | "1" | "true");
                            engine.set_effect_enabled(fx, enabled);
                            format!(
                                "Effect '{}' -> {}",
                                effect_name,
                                if enabled { "ON ✅" } else { "OFF ❌" }
                            )
                        } else {
                            let names: Vec<&str> =
                                AudioEffect::all_names().iter().map(|(n, _)| *n).collect();
                            format!(
                                "Unknown effect '{}'. Available: {}",
                                effect_name,
                                names.join(", ")
                            )
                        }
                    }
                    _ => format!(
                        "Usage: audio.fx <effect_name> <on|off>\nAvailable effects: {}",
                        AudioEffect::all_names()
                            .iter()
                            .map(|(n, _)| *n)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                }
            });

        // Autocomplétion statique : noms des effets disponibles
        self.commands_registry.register_args(
            "audio.fx",
            crate::audio_engine::effect_flags::AudioEffect::all_names()
                .iter()
                .map(|(n, _)| *n)
                .collect(),
        );
        self.commands_registry.register_hint(
            "audio.fx",
            "<effect_name> <on|off> — Toggle a DSP effect at runtime",
        );

        // Valeur courante affichée en bleu dans la console lors de l'autocomplétion
        self.commands_registry
            .register_current_value("audio.fx", |audio, _| audio.get_effects_status());

        // audio.fx_all — Active ou désactive tous les effets DSP en même temps
        self.commands_registry
            .register_for_audio("audio.fx_all", |engine, input| {
                let parts: Vec<&str> = input.split_whitespace().collect();
                match parts.as_slice() {
                    [_, state] => {
                        let enabled = matches!(*state, "on" | "1" | "true");
                        engine.set_all_effects_enabled(enabled);
                        format!(
                            "All DSP effects -> {}",
                            if enabled { "ON ✅" } else { "OFF ❌" }
                        )
                    }
                    _ => "Usage: audio.fx_all <on|off>".to_string(),
                }
            });
        self.commands_registry
            .register_args("audio.fx_all", vec!["on", "off"]);
        self.commands_registry.register_hint(
            "audio.fx_all",
            "<on|off> — Enable or disable all DSP effects at runtime",
        );

        // audio.fx_status — Affiche l'état de tous les effets DSP
        self.commands_registry
            .register_for_audio("audio.fx_status", |engine, _| {
                format!("DSP Effects:\n  {}", engine.get_effects_status())
            });
        self.commands_registry.register_hint(
            "audio.fx_status",
            "List all DSP effects and their current state",
        );
    }

    fn register_physic_commands(&mut self) {
        self.commands_registry
            .register_for_physic("physic.config", |engine, _| {
                format!("{:#?}", engine.get_config())
            });

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
                if params.len() >= 4 && params.len().is_multiple_of(2) {
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
                else if params.len() >= 2 && params.len().is_multiple_of(2) {
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
                // fetch_xor retourne l'ancienne valeur. Si c'était false, c'est devenu true (Enabled).
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

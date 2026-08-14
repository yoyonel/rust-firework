use crate::audio_engine::AudioEngine;
use crate::physic_engine::{config::PhysicConfig, PhysicEngineFull};
use crate::renderer_engine::utils::adaptative_sampler::{ascii_sample_timeline, AdaptiveSampler};
use crate::renderer_engine::RendererEngine;
use crate::window_engine::WindowEngine;
use crate::{log_metrics_and_fps, profiler::Profiler};
use crate::{CommandRegistry, Console};
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

pub mod audio_stress_scene;
pub mod console_commands;
pub mod events;
pub mod gui_settings;
pub mod ui;
pub use audio_stress_scene::{AudioStressScene, VirtualSource};

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
    physic_reinit_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,

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
    pub dt_accumulator: f32,
    pub render_alpha: f32,
    profiler: Profiler,
    sampler: AdaptiveSampler,
    sampled_fps: Vec<f32>,
    fps_avg: f32,
    fps_avg_iter: f32,
    last_log: Instant,
    first_frame: bool,
    pub last_audio_debug_update: Instant,
    pub show_audio_diagnostic: bool,
    pub gui_settings: crate::simulator::gui_settings::GuiSettings,
    pub engine_commands: Vec<crate::domain_contracts::EngineCommand>,

    // Tone mapping comparison
    pub tonemapping_comparison_mode: std::sync::Arc<std::sync::atomic::AtomicBool>,

    // NOUVEAU: Métriques de synchronisation physique-audio
    pub sync_launch_sum: f64,
    pub sync_launch_count: u64,
    pub sync_explosion_sum: f64,
    pub sync_explosion_count: u64,
    pub phys_launch_times: std::collections::HashMap<u64, std::time::Instant>,
    pub phys_explosion_times: std::collections::HashMap<u64, std::time::Instant>,
    pub audio_start_launch_times: std::collections::HashMap<u64, std::time::Instant>,
    pub audio_start_explosion_times: std::collections::HashMap<u64, std::time::Instant>,

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

    pub audio_stress_scene: AudioStressScene,

    pub launch_trend_dir: i32, // 1: augmentation, -1: diminution, 0: stable
    pub explosion_trend_dir: i32, // 1: augmentation, -1: diminution, 0: stable

    // NOUVEAU: Indicateurs visuels GPU des évènements audio (mode debug F3)
    pub audio_event_renderer: Option<crate::renderer_engine::AudioEventRenderer>,
    pub audio_event_pool: Vec<crate::renderer_engine::AudioEvent>,
    pub show_audio_visual_overlay: bool,

    // Scratch persistent buffers for multi-substep event accumulation (0 heap allocs in update_simulation)
    accumulated_new_rocket_ids: Vec<u64>,
    accumulated_exploded_ids: Vec<u64>,
    accumulated_anticipated_launches: Vec<(u64, glam::Vec2)>,
    pub accumulated_anticipated_explosions: Vec<(u64, glam::Vec2)>,

    pub max_frames: Option<u64>,
    pub fixed_dt: Option<f32>,
    pub timeout_secs: Option<u64>,
    pub start_time: Instant,
    pub disable_audio: bool,
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
        let session_path = crate::utils::config_path::get_gui_session_path();
        let gui_session =
            crate::simulator::gui_settings::GuiSessionState::load_from_file(&session_path);

        let renderer_path = crate::utils::config_path::get_renderer_config_path();
        let event_cap = crate::physic_engine::constants::INITIAL_EVENT_BUFFER_CAPACITY;
        let mut sim = Self {
            renderer_engine,
            physic_engine,
            audio_engine,
            commands_registry: CommandRegistry::new(),
            window_engine,
            console: Console::new(),
            reload_shaders_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                false,
            )),
            physic_reinit_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            renderer_config: std::sync::Arc::new(std::sync::RwLock::new(
                crate::renderer_engine::RendererConfig::from_file(&renderer_path)
                    .unwrap_or_default(),
            )),
            frames: 0,
            last_time: Instant::now(),
            window_size,
            window_size_f32: (window_size.0 as f32, window_size.1 as f32),
            window_last_pos: window_pos,
            window_last_size: window_size,
            dt_accumulator: crate::physic_engine::constants::FIXED_TIMESTEP_DELTA,
            render_alpha: 0.0,
            profiler: Profiler::new(200),
            sampler: AdaptiveSampler::new(std::time::Duration::from_secs(5), 200, 60.0),
            sampled_fps: Vec::with_capacity(200),
            fps_avg: 0.0,
            fps_avg_iter: 0.0,
            last_log: Instant::now(),
            first_frame: true,
            last_audio_debug_update: Instant::now(),
            show_audio_diagnostic: false,
            gui_settings: crate::simulator::gui_settings::GuiSettings::new(),
            engine_commands: Vec::with_capacity(64),
            tonemapping_comparison_mode: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                gui_session.tonemapping_comparison_mode,
            )),
            sync_launch_sum: 0.0,
            sync_launch_count: 0,
            sync_explosion_sum: 0.0,
            sync_explosion_count: 0,
            phys_launch_times: std::collections::HashMap::new(),
            phys_explosion_times: std::collections::HashMap::new(),
            audio_start_launch_times: std::collections::HashMap::new(),
            audio_start_explosion_times: std::collections::HashMap::new(),
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
            audio_stress_scene: AudioStressScene::new(),
            launch_trend_dir: 0,
            explosion_trend_dir: 0,
            audio_event_renderer: None,
            audio_event_pool: Vec::with_capacity(32),
            show_audio_visual_overlay: true,
            accumulated_new_rocket_ids: Vec::with_capacity(event_cap),
            accumulated_exploded_ids: Vec::with_capacity(event_cap),
            accumulated_anticipated_launches: Vec::with_capacity(event_cap),
            accumulated_anticipated_explosions: Vec::with_capacity(event_cap),
            max_frames: None,
            fixed_dt: None,
            timeout_secs: None,
            start_time: Instant::now(),
            disable_audio: false,
        };

        sim.gui_settings.apply_session_to_audio(
            &mut sim.audio_engine,
            &mut sim.show_audio_diagnostic,
            &mut sim.show_audio_visual_overlay,
        );
        sim.gui_settings
            .apply_session_to_physic(&mut sim.physic_engine);

        if gui_session.fullscreen {
            sim.toggle_fullscreen();
        }

        sim
    }

    pub fn save_gui_session(&self) {
        if !crate::utils::config_path::is_config_save_enabled() {
            return;
        }
        self.gui_settings.save_session_state(
            &self.audio_engine,
            &self.physic_engine,
            self.show_audio_diagnostic,
            self.show_audio_visual_overlay,
            self.tonemapping_comparison_mode
                .load(std::sync::atomic::Ordering::Relaxed),
            self.window_engine.is_fullscreen(),
        );
        let physic_path = crate::utils::config_path::get_physic_config_path();
        let _ = self.physic_engine.get_config().save_to_file(&physic_path);
        if let Ok(renderer) = self.renderer_config.read() {
            let renderer_path = crate::utils::config_path::get_renderer_config_path();
            let _ = renderer.save_to_file(&renderer_path);
        }
    }

    pub fn set_doppler_sender(
        &mut self,
        sender: crossbeam_channel::Sender<crate::audio_engine::DopplerEvent>,
    ) {
        self.audio_stress_scene.set_doppler_sender(sender);
    }

    pub fn enable_audio_stress_scene(&mut self, num_sources: usize, randomize_positions: bool) {
        self.show_audio_diagnostic = true;
        self.audio_stress_scene.enable(
            num_sources,
            randomize_positions,
            self.window_size_f32,
            &mut self.audio_engine,
        );
    }

    pub fn run(&mut self, export_path: Option<String>) -> anyhow::Result<()> {
        if !self.disable_audio {
            self.audio_engine.start_audio_thread(export_path.as_deref());
        }
        let listener_pos = if self.audio_stress_scene.enabled {
            glam::Vec2::new(self.window_size_f32.0 / 2.0, self.window_size_f32.1 / 2.0)
        } else {
            glam::Vec2::new(self.window_size_f32.0 / 2.0, 0.0)
        };
        self.audio_engine.set_listener_position(listener_pos);

        while self.step() {}

        // Libérer explicitement le renderer GPU avant la destruction du contexte OpenGL.
        // Sans cela, le Drop de AudioEventRenderer appellerait gl::Delete* après que
        // GLFW ait détruit le contexte → segfault garanti.
        self.audio_event_renderer = None;

        Ok(())
    }

    /// Main Loop Step
    pub fn step(&mut self) -> bool {
        // Early exit check
        if self.window_engine.should_close() {
            self.save_gui_session();
            return false;
        }

        if let Some(max_f) = self.max_frames {
            if self.frames >= max_f {
                info!("🏁 Max frames reached ({}), exiting.", max_f);
                self.save_gui_session();
                return false;
            }
        }

        if let Some(t) = self.timeout_secs {
            if self.start_time.elapsed().as_secs() >= t {
                eprintln!(
                    "\n=== [METRIC] TOTAL FRAMES GENERATED: {} ===\n",
                    self.frames
                );
                self.save_gui_session();
                return false;
            }
        }

        // 1. Gestion des événements
        let (reload_config, reload_shaders) = self.handle_window_events();

        // 2. Application des rechargements
        self.apply_reload_requests(reload_config, reload_shaders);

        // 3. Synchronisation config renderer
        self.sync_renderer_config();

        // 4. Timing
        let _frame_guard = self.profiler.frame(); // RAII timing
        let delta = if let Some(dt) = self.fixed_dt {
            // still need to call update_frame_timing to update last_time etc, but we discard its delta
            let _ = self.update_frame_timing();
            dt
        } else {
            self.update_frame_timing()
        };

        // 5. Simulation physique + audio
        tracy_zone!(
            "simulator::physics",
            0xFF5500, // Orange
            {
                self.update_simulation(delta);
                if !self.audio_stress_scene.enabled {
                    if (self.console.open || self.show_audio_diagnostic)
                        && self.last_audio_debug_update.elapsed()
                            >= std::time::Duration::from_millis(16)
                    {
                        self.process_audio_debug_events();
                        self.last_audio_debug_update = std::time::Instant::now();
                    } else if self.last_audio_debug_update.elapsed()
                        >= std::time::Duration::from_millis(100)
                    {
                        self.audio_events_buf.clear();
                        self.audio_engine
                            .pop_debug_events(&mut self.audio_events_buf);
                        self.last_audio_debug_update = std::time::Instant::now();
                    }
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

    fn update_simulation(&mut self, delta: f32) {
        if self.audio_stress_scene.enabled {
            self.audio_stress_scene.update(
                delta,
                self.window_size_f32,
                &mut self.audio_engine,
                &mut self.audio_events_buf,
            );
            return;
        }

        // Clamp large frame deltas to avoid spiral of death / sudden accumulator surge
        let clamped_delta = delta.min(crate::physic_engine::constants::MAX_ACCUMULATOR_DELTA_CLAMP);
        self.dt_accumulator += clamped_delta;

        let fixed_dt = crate::physic_engine::constants::FIXED_TIMESTEP_DELTA;
        let max_sub_steps = crate::physic_engine::constants::MAX_SUB_STEPS;

        self.accumulated_new_rocket_ids.clear();
        self.accumulated_exploded_ids.clear();
        self.accumulated_anticipated_launches.clear();
        self.accumulated_anticipated_explosions.clear();

        let mut sub_steps = 0;
        let mut ran_any_substep = false;

        while self.dt_accumulator >= fixed_dt && sub_steps < max_sub_steps {
            let update_result = self
                .profiler
                .profile_block("physic - update", || self.physic_engine.update(fixed_dt));

            if let Some(r) = &update_result.new_rocket {
                if !self.accumulated_new_rocket_ids.contains(&r.id) {
                    self.accumulated_new_rocket_ids.push(r.id);
                }
            }
            for &id in update_result.triggered_explosion_ids {
                if !self.accumulated_exploded_ids.contains(&id) {
                    self.accumulated_exploded_ids.push(id);
                }
            }
            if let Some(launch) = update_result.anticipated_rocket_launch {
                if !self
                    .accumulated_anticipated_launches
                    .iter()
                    .any(|&(id, _)| id == launch.0)
                {
                    self.accumulated_anticipated_launches.push(launch);
                }
            }
            for &item in update_result.anticipated_explosions {
                if !self
                    .accumulated_anticipated_explosions
                    .iter()
                    .any(|&(id, _)| id == item.0)
                {
                    self.accumulated_anticipated_explosions.push(item);
                }
            }

            self.dt_accumulator -= fixed_dt;
            sub_steps += 1;
            ran_any_substep = true;
        }

        // Clamp safety guard against spiral of death
        if sub_steps >= max_sub_steps {
            self.dt_accumulator = 0.0;
        }

        self.render_alpha = (self.dt_accumulator / fixed_dt).clamp(0.0, 1.0);

        if !ran_any_substep {
            return;
        }

        // Copy event IDs to stack buffers (max 16 IDs) to release immutable borrow on self
        let mut new_ids_buf = [0u64; 16];
        let new_count = self.accumulated_new_rocket_ids.len().min(16);
        new_ids_buf[..new_count].copy_from_slice(&self.accumulated_new_rocket_ids[..new_count]);

        let mut exploded_ids_buf = [0u64; 16];
        let exploded_count = self.accumulated_exploded_ids.len().min(16);
        exploded_ids_buf[..exploded_count]
            .copy_from_slice(&self.accumulated_exploded_ids[..exploded_count]);

        // Dispatch events to tracking and audio engine
        self.track_physical_events(
            &new_ids_buf[..new_count],
            &exploded_ids_buf[..exploded_count],
        );
        if !self.disable_audio {
            Self::synch_audio_with_physic_extracted(
                &mut self.audio_engine,
                &self.accumulated_anticipated_launches[..],
                &self.accumulated_anticipated_explosions[..],
            );
        }

        // NOUVEAU: Alimenter le pool d'indicateurs visuels audio (mode debug F3)
        if self.show_audio_diagnostic && self.show_audio_visual_overlay {
            // Injection des évènements anticipés dans le pool d'animation
            for &(_id, pos) in &self.accumulated_anticipated_launches {
                self.audio_event_pool
                    .push(crate::renderer_engine::AudioEvent::new(
                        pos,
                        crate::renderer_engine::AudioEventKind::Launch,
                    ));
            }
            for &(_id, pos) in &self.accumulated_anticipated_explosions {
                self.audio_event_pool
                    .push(crate::renderer_engine::AudioEvent::new(
                        pos,
                        crate::renderer_engine::AudioEventKind::Explosion,
                    ));
            }
            // Vieillissement + élagage des évènements expirés
            self.audio_event_pool.retain_mut(|evt| {
                evt.age += delta;
                !evt.is_expired()
            });

            // Limiter le nombre maximum d'événements pour éviter les baisses de FPS liées au fillrate
            const MAX_AUDIO_EVENTS: usize = 48;
            if self.audio_event_pool.len() > MAX_AUDIO_EVENTS {
                let to_remove = self.audio_event_pool.len() - MAX_AUDIO_EVENTS;
                self.audio_event_pool.drain(0..to_remove);
            }
        } else if !self.audio_event_pool.is_empty() {
            self.audio_event_pool.clear();
        }

        tracy_zone_with_value!(
            "physics::update",
            0xAA00FF, // Violet
            if !self.accumulated_new_rocket_ids.is_empty() {
                1
            } else {
                0
            }
        );
    }

    fn adjust_launch_anticipation_ms(&mut self, error_ms: f32) {
        let gain = 0.05;
        let old_val = self.physic_engine.get_config().audio_launch_anticipation_ms;
        let mut new_val = old_val + error_ms * gain;
        new_val = new_val.clamp(0.0, 150.0);

        if (new_val - old_val).abs() > 0.001 {
            self.launch_trend_dir = if new_val > old_val { 1 } else { -1 };
            let current_explosion = self
                .physic_engine
                .get_config()
                .audio_explosion_anticipation_ms;
            self.physic_engine
                .update_anticipation_times(new_val, current_explosion);
        }
    }

    fn adjust_explosion_anticipation_ms(&mut self, error_ms: f32) {
        let gain = 0.05;
        let old_val = self
            .physic_engine
            .get_config()
            .audio_explosion_anticipation_ms;
        let mut new_val = old_val + error_ms * gain;
        new_val = new_val.clamp(0.0, 150.0);

        if (new_val - old_val).abs() > 0.001 {
            self.explosion_trend_dir = if new_val > old_val { 1 } else { -1 };
            let current_launch = self.physic_engine.get_config().audio_launch_anticipation_ms;
            self.physic_engine
                .update_anticipation_times(current_launch, new_val);
        }
    }

    fn track_physical_events(&mut self, new_rocket_ids: &[u64], triggered_explosion_ids: &[u64]) {
        let now = std::time::Instant::now();

        // 1. Lancement physique/visuel
        for &id in new_rocket_ids {
            if let Some(audio_start) = self.audio_start_launch_times.remove(&id) {
                let diff_ms = if audio_start >= now {
                    audio_start.duration_since(now).as_secs_f32() * 1000.0
                } else {
                    now.duration_since(audio_start).as_secs_f32() * -1000.0
                };
                self.sync_launch_sum += diff_ms as f64;
                self.sync_launch_count += 1;
                self.profiler.record_metric("sync_launch_ms", diff_ms);

                // Ajustement dynamique basé sur l'erreur mesurée
                self.adjust_launch_anticipation_ms(diff_ms);
            } else {
                self.phys_launch_times.insert(id, now);
            }
        }

        // 2. Explosion physique/visuelle
        for &id in triggered_explosion_ids {
            if let Some(audio_start) = self.audio_start_explosion_times.remove(&id) {
                let diff_ms = if audio_start >= now {
                    audio_start.duration_since(now).as_secs_f32() * 1000.0
                } else {
                    now.duration_since(audio_start).as_secs_f32() * -1000.0
                };
                self.sync_explosion_sum += diff_ms as f64;
                self.sync_explosion_count += 1;
                self.profiler.record_metric("sync_explosion_ms", diff_ms);

                // Ajustement dynamique basé sur l'erreur mesurée
                self.adjust_explosion_anticipation_ms(diff_ms);
            } else {
                self.phys_explosion_times.insert(id, now);
            }
        }
    }

    fn render_frame(&mut self) {
        let particles_drawn = self
            .renderer_engine
            .render_frame(&self.physic_engine, self.render_alpha);
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

        if self.audio_stress_scene.enabled {
            self.audio_stress_scene
                .draw(self.window_size_f32, &self.audio_engine);
        }

        // NOUVEAU: Indicateurs visuels GPU des évènements audio (anneau de propagation + beam)
        if self.show_audio_diagnostic
            && self.show_audio_visual_overlay
            && !self.audio_event_pool.is_empty()
        {
            // Lazy-init du renderer (crée les VBO/VAO/shaders la première fois)
            if self.audio_event_renderer.is_none() {
                self.audio_event_renderer = Some(crate::renderer_engine::AudioEventRenderer::new());
            }
            if let Some(renderer) = &mut self.audio_event_renderer {
                // Position du listener (bas-centre de l'écran en mode normal)
                let listener = glam::Vec2::new(
                    self.window_size_f32.0 * 0.5,
                    self.audio_engine.get_listener_position().y,
                );
                // Construire le buffer GPU depuis le pool CPU (pile temporaire, zéro allocation)
                let mut gpu_buf: Vec<crate::renderer_engine::AudioEventGPUData> =
                    Vec::with_capacity(self.audio_event_pool.len());
                for evt in &self.audio_event_pool {
                    gpu_buf.push(evt.to_gpu(listener));
                }
                unsafe {
                    renderer.draw(&gpu_buf);
                }
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

    fn finalize_frame(&mut self) {
        self.window_engine.swap_buffers();

        if self.first_frame {
            info!("🚀 First frame rendered");
            self.first_frame = false;
        }
    }

    fn synch_audio_with_physic_extracted(
        audio_engine: &mut A,
        anticipated_rocket_launches: &[(u64, glam::Vec2)],
        anticipated_explosions: &[(u64, glam::Vec2)],
    ) {
        for &(id, pos) in anticipated_rocket_launches {
            debug!(
                "🚀 [Anticipated] Rocket launch audio triggered for ID {} at ({}, {})",
                id, pos.x, pos.y
            );
            audio_engine.play_rocket_with_id(id, pos, 0.8);
        }

        for (i, &(id, pos)) in anticipated_explosions.iter().enumerate() {
            debug!(
                "💥 [Anticipated] Explosion audio triggered: {} for ID {} at ({}, {})",
                i, id, pos.x, pos.y
            );
            audio_engine.play_explosion_with_id(id, pos, 1.0);
        }
    }

    pub fn reload_config(&mut self) {
        let physic_config =
            PhysicConfig::from_file(crate::utils::config_path::get_physic_config_path())
                .unwrap_or_default();
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
        self.save_gui_session();
        if let Some(mut renderer) = self.audio_stress_scene.circle_renderer.take() {
            renderer.destroy();
        }
        self.renderer_engine.close();
        self.physic_engine.close();
        self.audio_engine.stop_audio_thread();
    }

    /// Helper pour avancer la simulation d'un pas de temps fixe (uniquement pour les tests)
    pub fn step_custom_dt(&mut self, dt: f32) {
        self.update_simulation(dt);
        self.process_audio_debug_events();
    }

    /// Helper pour avancer la simulation ET le rendu GPU d'un pas de temps fixe (uniquement pour les tests d'intégration visuels)
    pub fn step_full_frame_with_dt(&mut self, dt: f32) {
        self.update_simulation(dt);
        self.render_frame();
        self.finalize_frame();
    }

    /// Helper pour obtenir la configuration physique actuelle (uniquement pour les tests)
    pub fn get_physic_config(&self) -> &PhysicConfig {
        self.physic_engine.get_config()
    }

    /// Helper pour obtenir le renderer (réservé exclusivement aux tests d'intégration visuels)
    #[cfg(any(test, feature = "interactive_tests"))]
    pub fn get_renderer_engine(&self) -> &R {
        &self.renderer_engine
    }

    /// Helper pour obtenir les moyennes de synchronisation de debug (uniquement pour les tests)
    pub fn get_average_syncs_test_helper(&self) -> (f64, f64) {
        let avg_launch_sync = if self.sync_launch_count > 0 {
            self.sync_launch_sum / self.sync_launch_count as f64
        } else {
            0.0
        };

        let avg_explosion_sync = if self.sync_explosion_count > 0 {
            self.sync_explosion_sum / self.sync_explosion_count as f64
        } else {
            0.0
        };

        (avg_launch_sync, avg_explosion_sync)
    }
}

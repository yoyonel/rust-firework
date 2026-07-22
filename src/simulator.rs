use crate::audio_engine::AudioEngine;
use crate::physic_engine::{config::PhysicConfig, PhysicEngineFull, UpdateResult};
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

    pub audio_stress_scene: AudioStressScene,
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
            physic_reinit_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            audio_stress_scene: AudioStressScene::new(),
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
        self.audio_engine.start_audio_thread(export_path.as_deref());
        let listener_pos = if self.audio_stress_scene.enabled {
            glam::Vec2::new(self.window_size_f32.0 / 2.0, self.window_size_f32.1 / 2.0)
        } else {
            glam::Vec2::new(self.window_size_f32.0 / 2.0, 0.0)
        };
        self.audio_engine.set_listener_position(listener_pos);

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

        if self.audio_stress_scene.enabled {
            self.audio_stress_scene
                .draw(self.window_size_f32, &self.audio_engine);
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
        if let Some(mut renderer) = self.audio_stress_scene.circle_renderer.take() {
            renderer.destroy();
        }
        self.renderer_engine.close();
        self.physic_engine.close();
        self.audio_engine.stop_audio_thread();
    }
}

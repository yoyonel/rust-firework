use crate::audio_engine::AudioEngine;
use crate::physic_engine::{config::PhysicConfig, PhysicEngine, PhysicEngineFull, UpdateResult};
use crate::renderer_engine::RendererEngine;
use crate::renderer_engine::{
    tools::{setup_opengl_debug, show_opengl_context_info},
    utils::{
        adaptative_sampler::{ascii_sample_timeline, AdaptiveSampler},
        glfw_window::Fullscreen,
    },
};
use crate::{log_metrics_and_fps, profiler::Profiler};
use crate::{CommandRegistry, Console};
use anyhow::{anyhow, Result};
use glfw::{Action, Context, Key, WindowMode};
use imgui::Context as ImContext;
use imgui_glfw_rs::glfw;
use imgui_glfw_rs::imgui;
use imgui_glfw_rs::ImguiGLFW;
use log::{debug, info};
use std::time::Instant;

pub struct ImguiSystem {
    pub context: imgui::Context,
    pub glfw: ImguiGLFW,
}

pub type WindowEvents = glfw::GlfwReceiver<(f64, glfw::WindowEvent)>;

pub struct Simulator<R, P, A>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
{
    renderer_engine: R,
    physic_engine: P,
    pub audio_engine: A,
    pub commands_registry: CommandRegistry,

    // Window & Loop management
    pub glfw: glfw::Glfw,
    pub window: Option<glfw::PWindow>,
    pub events: Option<WindowEvents>,
    pub imgui_system: Option<ImguiSystem>,
    console: Console,

    frames: u32,
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

impl<R, P, A> Simulator<R, P, A>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
{
    pub fn init_window(
        width: i32,
        height: i32,
        title: &str,
    ) -> Result<(glfw::Glfw, glfw::PWindow, WindowEvents, ImguiSystem)> {
        let _ = env_logger::builder().is_test(true).try_init();

        let mut glfw = glfw::init(glfw::fail_on_errors)
            .map_err(|_| anyhow!("Impossible d’initialiser GLFW"))?;

        glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
        glfw.window_hint(glfw::WindowHint::ContextVersionMinor(3));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(
            glfw::OpenGlProfileHint::Core,
        ));

        let (mut window, events) = glfw
            .create_window(
                width as u32,
                height as u32,
                title,
                glfw::WindowMode::Windowed,
            )
            .expect("Erreur création fenêtre GLFW");

        window.make_current();
        window.set_key_polling(true);
        window.set_char_polling(true);
        window.set_framebuffer_size_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_mouse_button_polling(true);
        window.set_scroll_polling(true);

        info!("✅ OpenGL context ready for '{}'", title);

        // load OpenGL function pointers
        gl::load_with(|s| window.get_proc_address(s) as *const _);

        unsafe {
            show_opengl_context_info();
            setup_opengl_debug();
            gl::Enable(gl::PROGRAM_POINT_SIZE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let mut imgui = ImContext::create();
        let font_data =
            std::fs::read("assets/fonts/PerfectDOSVGA437.ttf").expect("Failed to read font file");
        imgui.fonts().add_font(&[imgui::FontSource::TtfData {
            data: &font_data,
            size_pixels: 18.0,
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                oversample_v: 1,
                rasterizer_multiply: 1.0,
                ..Default::default()
            }),
        }]);

        imgui.fonts().build_rgba32_texture();
        imgui.style_mut().use_dark_colors();

        let imgui_glfw = ImguiGLFW::new(&mut imgui, &mut window);

        Ok((
            glfw,
            window,
            events,
            ImguiSystem {
                context: imgui,
                glfw: imgui_glfw,
            },
        ))
    }

    pub fn new(
        renderer_engine: R,
        physic_engine: P,
        audio_engine: A,
        glfw: glfw::Glfw,
        window: glfw::PWindow,
        events: WindowEvents,
        imgui_system: ImguiSystem,
    ) -> Self {
        let window_size = window.get_size();
        let window_pos = window.get_pos();

        Self {
            renderer_engine,
            physic_engine,
            audio_engine,
            commands_registry: CommandRegistry::new(),
            glfw,
            window: Some(window),
            events: Some(events),
            imgui_system: Some(imgui_system),
            console: Console::new(),
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
        if let Some(window) = &mut self.window {
            if window.should_close() {
                return false;
            }
        } else {
            return false;
        }

        let mut reload_config = false;

        // Window events
        if let Some(window) = &mut self.window {
            self.glfw.poll_events();

            if let Some(events) = &self.events {
                for (_, event) in glfw::flush_messages(events) {
                    match event {
                        glfw::WindowEvent::FramebufferSize(w, h) => {
                            self.renderer_engine.set_window_size(w, h);
                            self.window_size_f32 = (w as f32, h as f32);
                            self.physic_engine.set_window_width(w as f32);
                            self.audio_engine
                                .set_listener_position(((w / 2) as f32, 0.0));
                        }
                        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                            window.set_should_close(true);
                        }
                        glfw::WindowEvent::Key(Key::R, _, Action::Press, _) => {
                            reload_config = true;
                        }
                        glfw::WindowEvent::Key(Key::F11, _, Action::Press, _) => {
                            if window.is_fullscreen() {
                                window.set_monitor(
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
                                self.window_last_pos = window.get_pos();
                                self.window_last_size = window.get_size();

                                let mut glfw = window.glfw.clone();
                                glfw.with_primary_monitor(|_, monitor| {
                                    if let Some(monitor) = monitor {
                                        if let Some(video_mode) = monitor.get_video_mode() {
                                            window.set_fullscreen(monitor);
                                            self.window_size = (
                                                video_mode.width as i32,
                                                video_mode.height as i32,
                                            );
                                            self.window_size_f32 = (
                                                self.window_size.0 as f32,
                                                self.window_size.1 as f32,
                                            );
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
                            window.set_cursor_mode(if self.console.open {
                                self.console.focus_previous_widget = true;
                                glfw::CursorMode::Normal
                            } else {
                                glfw::CursorMode::Disabled
                            });
                        }
                        _ => {}
                    }
                    // Pas besoin de helper externe, on peut le faire "inline"
                    if let Some(system) = &mut self.imgui_system {
                        system.glfw.handle_event(&mut system.context, &event);
                    }
                }
            }
        }
        if reload_config {
            self.reload_config();
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
                    .map(|(_, fps)| *fps)
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

        if let Some(window) = &mut self.window {
            if self.console.open {
                if let Some(system) = &mut self.imgui_system {
                    let ui = system.glfw.frame(window, &mut system.context);
                    self.console.draw(
                        ui,
                        &mut self.audio_engine,
                        &mut self.physic_engine,
                        &self.commands_registry,
                    );
                    system.glfw.draw(&mut system.context, window);
                }
            }

            window.swap_buffers();

            if self.first_frame {
                info!("🚀 First frame rendered");
                self.first_frame = false;
            }
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

    pub fn close(&mut self) {
        self.renderer_engine.close();
        self.physic_engine.close();
        self.audio_engine.stop_audio_thread();

        // Important de drop la ressource imgui pour glfw avant de drop la window glfw
        self.imgui_system = None;
        self.window = None;
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

impl<R, P, A> Simulator<R, P, A>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
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
    }
}

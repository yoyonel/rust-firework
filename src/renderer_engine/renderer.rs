use crate::physic_engine::{PhysicEngineFull, PhysicEngineIterator};
use crate::RendererEngine;
use crate::{log_metrics_and_fps, profiler::Profiler};
use anyhow::{anyhow, Result};
use glfw::{Action, Context, Key, WindowMode};
use imgui::Context as ImContext;
use imgui_glfw_rs::glfw;
use imgui_glfw_rs::imgui;
use imgui_glfw_rs::ImguiGLFW;
use log::{debug, info, warn};
use std::time::Instant;

use crate::audio_engine::AudioEngine;
use crate::physic_engine::{config::PhysicConfig, PhysicEngine, UpdateResult};
use crate::renderer_engine::particle_renderer::ParticleGraphicsRenderer;
use crate::renderer_engine::RendererGraphics;
use crate::renderer_engine::RendererGraphicsInstanced;
use crate::renderer_engine::{
    command_console::{CommandRegistry, Console},
    tools::{setup_opengl_debug, show_opengl_context_info},
    utils::{
        adaptative_sampler::{ascii_sample_timeline, AdaptiveSampler},
        glfw_window::Fullscreen,
    },
};

//
pub struct ImguiSystem {
    pub context: imgui::Context,
    pub glfw: ImguiGLFW,
}

// ---------------------------------------------------------
pub struct Renderer {
    pub glfw: glfw::Glfw,
    pub window: Option<glfw::PWindow>,
    pub events: Option<glfw::GlfwReceiver<(f64, glfw::WindowEvent)>>,

    pub imgui_system: Option<ImguiSystem>,
    console: Console,

    max_particles_on_gpu: usize,

    frames: u32,
    last_time: Instant,

    // Window management
    window_size: (i32, i32),
    window_size_f32: (f32, f32),
    window_last_pos: (i32, i32),
    window_last_size: (i32, i32),

    renderers: Vec<Box<dyn ParticleGraphicsRenderer>>,
}

// ---------------------------------------------------------
// Implémentation générique du Renderer pour tout type A
// qui implémente le trait AudioEngine.
//
// Signification exacte :
// - `impl<A: crate::audio_engine::AudioEngine> Renderer<A>`
//   signifie que toutes les méthodes définies ici sont disponibles
//   pour un Renderer dont le type `A` satisfait le trait AudioEngine.
// - `pub fn new(..., audio: A) -> Result<Self>`
//   prend **ownership** d'un objet `audio` de type `A`.
//   Comme le Renderer possède cet objet, il n'y a pas besoin de
//   références mutables externes ou de lifetimes (`&mut`) pour l'audio.
// Conséquences / avantages :
// 1. Typage statique et monomorphisation : pas de dispatch dynamique,
//    ce qui permet des appels plus rapides.
// 2. Flexibilité : on peut injecter un moteur audio réel ou un mock
//    pour les tests, simplement en changeant le type `A`.
// 3. Sécurité mémoire : le Renderer est propriétaire de l'audio et
//    gère sa durée de vie, pas de risque de référence suspendue.
//
// Limitation :
// - Chaque type `A` utilisé génère une version spécifique du Renderer
//   dans le binaire, ce qui peut augmenter légèrement la taille du code.
impl Renderer {
    pub fn new(width: i32, height: i32, title: &str, physic_config: &PhysicConfig) -> Result<Self> {
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

        let window_last_pos = window.get_pos();
        let window_last_size = window.get_size();

        info!("✅ OpenGL context ready for '{}'", title);

        // load OpenGL function pointers
        gl::load_with(|s| window.get_proc_address(s) as *const _);

        unsafe {
            show_opengl_context_info();

            // activate OpenGL debug output
            setup_opengl_debug();

            // set OpenGL states for the rendering
            // but it's link to the renderer graphics
            gl::Enable(gl::PROGRAM_POINT_SIZE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let mut imgui = ImContext::create();

        // Charge la font TTF “Quake style”
        let font_data =
            std::fs::read("assets/fonts/PerfectDOSVGA437.ttf").expect("Failed to read font file");
        imgui.fonts().add_font(&[imgui::FontSource::TtfData {
            data: &font_data,
            size_pixels: 18.0, // ajuste la taille selon le rendu
            config: Some(imgui::FontConfig {
                oversample_h: 1,          // ne pas lisser horizontalement
                oversample_v: 1,          // ne pas lisser verticalement
                rasterizer_multiply: 1.0, // contraste des glyphes
                ..Default::default()
            }),
        }]);

        imgui.fonts().build_rgba32_texture();
        if !imgui.fonts().is_built() {
            warn!("No ImGui fonts built");
        } else {
            info!("✅ ImGui fonts BUILD");
        }

        imgui.style_mut().use_dark_colors();

        let imgui_glfw = ImguiGLFW::new(&mut imgui, &mut window);

        let max_particles_on_gpu: usize =
            physic_config.max_rockets * physic_config.particles_per_explosion;

        let renderers: Vec<Box<dyn ParticleGraphicsRenderer>> = vec![
            Box::new(RendererGraphics::new(max_particles_on_gpu)),
            Box::new(RendererGraphicsInstanced::new(physic_config.max_rockets)),
        ];

        let console = Console::new();

        Ok(Self {
            glfw,
            window: Some(window),
            events: Some(events),
            imgui_system: Some(ImguiSystem {
                context: imgui,
                glfw: imgui_glfw,
            }),
            console,
            frames: 0,
            last_time: Instant::now(),
            window_size: (width, height),
            window_size_f32: (width as f32, height as f32),
            window_last_pos,
            window_last_size,
            renderers,
            max_particles_on_gpu,
        })
    }

    pub fn reload_config<P: PhysicEngine>(&mut self, physic: &mut P) {
        let physic_config =
            PhysicConfig::from_file("assets/config/physic.toml").unwrap_or_default();
        info!("Physic config loaded:\n{:#?}", physic_config);

        physic.reload_config(&physic_config);

        let new_max = physic_config.max_rockets * physic_config.particles_per_explosion; // ou autre logique

        if new_max != self.max_particles_on_gpu {
            info!(
                "🔁 GPU buffer reallocation required ({} → {})",
                self.max_particles_on_gpu, new_max
            );
            unsafe {
                for renderer in &mut self.renderers {
                    renderer.recreate_buffers(new_max);
                }
            }
        }
    }

    /// Exécute une seule frame (update + rendu)
    /// # Safety
    /// Cette fonction est unsafe car elle effectue des appels OpenGL non sécurisés.
    pub unsafe fn render_frame<P: PhysicEngineIterator>(&mut self, physic: &P) -> usize {
        let mut total_particles = 0;
        for renderer in &mut self.renderers {
            // Remplit le buffer GPU
            let nb = renderer.fill_particle_data_direct(physic);
            // Dessine les particules
            renderer.render_particles_with_persistent_buffer(nb, self.window_size_f32);
            total_particles += nb;
        }
        total_particles
    }

    /// Boucle infinie (production) qui appelle `step_frame`
    pub fn run_loop<P: PhysicEngineFull, A: AudioEngine>(
        &mut self,
        physic: &mut P,
        audio: &mut A,
        commands_registry: &CommandRegistry,
    ) -> Result<()> {
        // Partagé entre moteurs
        let profiler = Profiler::new(200);
        let mut last_log = Instant::now();
        let log_interval = std::time::Duration::from_secs(5);

        // 🔹 Initialisation de l’échantillonneur adaptatif
        let target_samples = 200;
        let mut sampler = AdaptiveSampler::new(log_interval, target_samples, 60.0);
        let mut sampled_fps: Vec<f32> = Vec::with_capacity(target_samples);

        audio.set_listener_position((self.window_size_f32.0 / 2.0, 0.0));

        // moyenne pondérée EMA
        let alpha = 0.15;
        let mut fps_avg = 0.0;
        // moyenne simple itérative
        let n_frames = 100;
        let mut fps_avg_iter = 0.0;

        let mut first_frame = true;

        while let Some(window) = &mut self.window {
            if window.should_close() {
                break;
            }
            let mut reload_config = false;

            // Window events
            if let Some(window) = &mut self.window {
                self.glfw.poll_events();

                if let Some(events) = &self.events {
                    for (_, event) in glfw::flush_messages(events) {
                        match event {
                            glfw::WindowEvent::FramebufferSize(w, h) => unsafe {
                                gl::Viewport(0, 0, w, h);
                                self.window_size_f32 = (w as f32, h as f32);
                                physic.set_window_width(w as f32);
                                audio.set_listener_position(((w / 2) as f32, 0.0));
                            },
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
                                            window.set_fullscreen(monitor);
                                            self.window_size = (
                                                monitor.get_video_mode().unwrap().width as i32,
                                                monitor.get_video_mode().unwrap().height as i32,
                                            );
                                            self.window_size_f32 = (
                                                self.window_last_size.0 as f32,
                                                self.window_last_size.1 as f32,
                                            );
                                            info!(
                                                "🖥️ Fullscreen: {} x {}",
                                                self.window_size.0, self.window_size.1
                                            );
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
                self.reload_config(physic);
            }

            // 🔹 start global frame
            let _frame_guard = profiler.frame(); // RAII: mesure totale de la frame

            let now = Instant::now();
            let delta = now.duration_since(self.last_time).as_secs_f32();
            self.last_time = now;
            self.frames += 1;

            // 🔹 Calcul FPS instantané
            let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };

            // 🔹 On demande à l’échantillonneur s’il faut enregistrer ce FPS
            if sampler.should_sample(delta) {
                sampled_fps.push(fps);
            }

            let update_result = profiler.profile_block("physic - update", || physic.update(delta));
            self.synch_audio_with_physic(&update_result, audio);

            // Clear screen before rendering
            unsafe {
                // Efface l’écran (fond noir)
                gl::ClearColor(0.0, 0.0, 0.0, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT);
            }

            // Render frame with all renderers
            profiler.profile_block("render frame", || {
                profiler.record_metric("total particles drawn", unsafe {
                    self.render_frame(physic)
                });
            });

            // FPSmoyenne​ ← α⋅FPSinstant ​+ (1 − α)⋅FPSmoyenne​
            fps_avg = alpha * fps + (1.0 - alpha) * fps_avg;
            // xˉn−1 ​= FPS moyenne des frames 1 aˋ n-1
            // xˉn​ = n(n − 1)⋅xˉn−1​ + xn​​
            fps_avg_iter = (fps_avg_iter * (n_frames - 1) as f32 + fps) / n_frames as f32;

            // affichage périodique
            if last_log.elapsed() >= log_interval {
                log_metrics_and_fps!(&profiler);

                if !sampler.samples.is_empty() {
                    // Moyenne des FPS mesurés
                    let avg_fps: f32 = sampler.samples.iter().map(|(_, fps)| *fps).sum::<f32>()
                        / sampler.samples.len() as f32;

                    // 🔹 Graph ASCII coloré selon FPS
                    let graph = ascii_sample_timeline(
                        &sampler.samples,
                        log_interval.as_secs_f32(),
                        50,
                        avg_fps,
                    );
                    info!("Graphe - Sample Timeline");
                    // [Trait Iterator - for_each - Calls a closure on each element of an iterator.](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.for_each)
                    graph.lines().for_each(|line| info!("{}", line));

                    info!(
                        "Samples: {} / {} | Moyenne FPS: {:.2}",
                        sampler.samples.len(),
                        sampler.target_samples,
                        avg_fps
                    );

                    sampler.reset();

                    info!("FPS moyen (EMA): {:.2}", fps_avg);
                    info!("FPS moyen (iter): {:.2}", fps_avg_iter);
                }

                last_log = Instant::now();
            }

            if let Some(window) = &mut self.window {
                if self.console.open {
                    if let Some(system) = &mut self.imgui_system {
                        let ui = system.glfw.frame(window, &mut system.context);
                        self.console.draw(ui, audio, physic, commands_registry);
                        system.glfw.draw(&mut system.context, window);
                    }
                }

                window.swap_buffers();

                if first_frame {
                    info!("🚀 First frame rendered");
                    first_frame = false;
                }
            }
        }

        Ok(())
    }

    fn synch_audio_with_physic<A: AudioEngine>(
        &mut self,
        update_result: &UpdateResult,
        audio: &mut A,
    ) {
        if let Some(rocket) = &update_result.new_rocket {
            debug!("🚀 Rocket spawned at ({}, {})", rocket.pos.x, rocket.pos.y);
            audio.play_rocket((rocket.pos.x, rocket.pos.y), 0.6);
        }

        for (i, expl) in update_result.triggered_explosions.iter().enumerate() {
            debug!(
                "💥 Explosion triggered: {} at ({}, {})",
                i, expl.pos.x, expl.pos.y
            );
            audio.play_explosion((expl.pos.x, expl.pos.y), 1.0);
        }
    }

    pub fn close(&mut self) {
        info!("🧹 Fermeture du Renderer");

        unsafe {
            for renderer in &mut self.renderers {
                renderer.close();
            }
        }

        // Important de drop la ressource imgui pour glfw avant de drop la window glfw
        // car la window porte le contexte OpenGL et au drop de imgui_glfw il sera alors
        // impossible (sans crash) de libérer les ressources GL du crate.
        // L'assignation déclenche le drop de l'ancienne valeur immédiatement
        self.imgui_system = None;

        self.window = None;
    }
}

// Trait implementation
impl RendererEngine for Renderer {
    fn run_loop<P: PhysicEngineFull, A: AudioEngine>(
        &mut self,
        physic: &mut P,
        audio: &mut A,
        commands_registry: &CommandRegistry,
    ) -> Result<()> {
        self.run_loop(physic, audio, commands_registry)
    }

    fn close(&mut self) {
        self.close();
    }
}

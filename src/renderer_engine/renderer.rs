use crate::RendererEngine;
use crate::{log_metrics_and_fps, profiler::Profiler};
use anyhow::{anyhow, Result};
use glfw::{Action, Context, Key, WindowMode};
use log::{debug, info};
use std::time::Instant;

use crate::audio_engine::AudioEngine;
use crate::physic_engine::{config::PhysicConfig, PhysicEngine};
use crate::renderer_engine::RendererGraphics;
use crate::renderer_engine::{
    tools::{setup_opengl_debug, show_opengl_context_info},
    utils::{
        adaptative_sampler::{ascii_sample_timeline, AdaptiveSampler},
        glfw_window::Fullscreen,
    },
};

pub struct Renderer {
    pub glfw: glfw::Glfw,
    pub window: Option<glfw::PWindow>,
    pub events: Option<glfw::GlfwReceiver<(f64, glfw::WindowEvent)>>,

    max_particles_on_gpu: usize,

    frames: u32,
    last_time: Instant,

    // Window management
    window_size: (i32, i32),
    window_size_f32: (f32, f32),
    window_last_pos: (i32, i32),
    window_last_size: (i32, i32),

    graphics: RendererGraphics,
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
    pub fn new(
        width: i32,
        height: i32,
        title: &str,
        max_particles_on_gpu: usize,
        _use_instanced_quads: bool,
    ) -> Result<Self> {
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

        // TODO: il faut utiliser un trait ici sur les renderer graphics pour
        // pouvoir choisir l'implémentation qu'on souhaite.
        // Par exemple, on peut vouloir choisir entre RendererGraphics et RendererGraphicsInstanced
        // et du point de vue de l'interface, ils devraient être interchangeables.
        //
        // À terme, on pourrait vouloir utiliser plusieurs renderering graphics pendant la génération d'une frame.
        // Par exemple, on pourrait vouloir conserver le renderer graphics à base de point rendering pour
        // les rockets (trainées + explosions), et on pourrait vouloir ajouter des particules en quads instanciés,
        // pour rendre des effets comme de la poussière/fumée/etc. au départ (de la trainée) et arrivée (explosion de la rocket).
        let graphics = RendererGraphics::new(max_particles_on_gpu);

        Ok(Self {
            glfw,
            window: Some(window),
            events: Some(events),
            frames: 0,
            last_time: Instant::now(),
            window_size: (width, height),
            window_size_f32: (width as f32, height as f32),
            window_last_pos,
            window_last_size,
            graphics,
            max_particles_on_gpu,
        })
    }

    fn reload_config<P: PhysicEngine>(&mut self, physic: &mut P) {
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
                self.graphics.recreate_buffers(new_max);
            }
        }
    }

    /// Exécute une seule frame (update + rendu)
    pub fn render_frame<P: PhysicEngine>(&mut self, physic: &P) -> usize {
        // Early-out si la fenêtre est fermée
        if let Some(w) = &self.window {
            if w.should_close() {
                return 0;
            }
        }

        unsafe {
            // Efface l’écran (fond noir)
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            // Remplit le buffer GPU
            let nb_particles_rendered = self.graphics.fill_particle_data_direct(physic);

            // // Dessine les particules
            self.graphics.render_particles_with_persistent_buffer(
                nb_particles_rendered,
                self.window_size_f32,
            );

            nb_particles_rendered
        }
    }

    /// Boucle infinie (production) qui appelle `step_frame`
    pub fn run_loop<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        physic: &mut P,
        audio: &mut A,
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

            self.update_physic_and_sync_with_audio(physic, audio, delta, &profiler);

            let _render_guard = profiler.measure("render frame");
            let particles_rendered = self.render_frame(physic);
            profiler.record_metric("total particles drawn", particles_rendered);

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

            let mut reload_config = false;

            // Swap buffers + events
            if let Some(window) = &mut self.window {
                window.swap_buffers();
                drop(_render_guard);

                if first_frame {
                    info!("🚀 First frame rendered");
                    first_frame = false;
                }

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
                            _ => {}
                        }
                    }
                }
            }
            if reload_config {
                self.reload_config(physic);
            }
        }

        Ok(())
    }

    fn update_physic_and_sync_with_audio<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        physic: &mut P,
        audio: &mut A,
        delta: f32,
        profiler: &Profiler,
    ) {
        let update_result = profiler.profile_block("physic - update", || physic.update(delta));

        if let Some(rocket) = update_result.new_rocket {
            debug!("🚀 Rocket spawned at ({}, {})", rocket.pos.x, rocket.pos.y);
            audio.play_rocket((rocket.pos.x, rocket.pos.y), 0.6);
        }

        for (i, expl) in update_result.explosions.iter().enumerate() {
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
            self.graphics.close();
        }

        if let Some(window) = self.window.take() {
            drop(window);
        }
    }
}

// Trait implementation
impl RendererEngine for Renderer {
    fn run_loop<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        physic: &mut P,
        audio: &mut A,
    ) -> Result<()> {
        self.run_loop(physic, audio)
    }

    fn close(&mut self) {
        self.close();
    }
}

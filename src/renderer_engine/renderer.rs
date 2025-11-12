use crate::RendererEngine;
use crate::{log_metrics_and_fps, profiler::Profiler};
use anyhow::{anyhow, Result};
use glfw::{Action, Context, Key, WindowMode};
use log::{debug, info};
use std::time::Instant;

use crate::audio_engine::AudioEngine;
use crate::physic_engine::{config::PhysicConfig, PhysicEngine};
use crate::renderer_engine::{
    tools::{compile_shader_program, print_context_info, setup_opengl_debug},
    types::ParticleGPU,
    utils::{
        adaptative_sampler::{ascii_sample_timeline, AdaptiveSampler},
        glfw_window::Fullscreen,
        texture::load_texture,
    },
};

use crate::utils::human_bytes::HumanBytes;

macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const i8
    };
}

pub struct Renderer {
    pub glfw: glfw::Glfw,
    pub window: Option<glfw::PWindow>,
    pub events: Option<glfw::GlfwReceiver<(f64, glfw::WindowEvent)>>,

    vao: u32,
    vbo_particles: u32,
    vbo_quad: u32,
    mapped_ptr: *mut ParticleGPU,
    shader_program: u32,

    max_particles_on_gpu: usize,
    buffer_size: isize,

    frames: u32,
    last_time: Instant,

    // Window management
    window_size: (i32, i32),
    window_size_f32: (f32, f32),
    window_last_pos: (i32, i32),
    window_last_size: (i32, i32),

    // Shader
    loc_size: i32,
    loc_tex: i32,
    loc_use: i32,
    texture_id: u32,

    // [POC] Instanced Quads
    use_instanced_quads: bool,
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
        use_instanced_quads: bool,
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

        gl::load_with(|s| window.get_proc_address(s) as *const _);
        print_context_info();

        unsafe {
            setup_opengl_debug();

            gl::Enable(gl::PROGRAM_POINT_SIZE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        let (vertex_src, fragment_src) = if !use_instanced_quads {
            src_shaders_particles()
        } else {
            src_shaders_instanced_quads()
        };
        let shader_program = unsafe { compile_shader_program(vertex_src, fragment_src) };

        let loc_size = unsafe { gl::GetUniformLocation(shader_program, cstr!("uSize")) };
        let loc_tex = unsafe { gl::GetUniformLocation(shader_program, cstr!("uTexture")) };
        let loc_use = unsafe { gl::GetUniformLocation(shader_program, cstr!("uUseTexture")) };
        let texture_id = load_texture("assets/textures/toppng.com-realistic-smoke-texture-with-soft-particle-edges-png-399x385.png");

        // VAO/VBO setup

        unsafe {
            let (vao, vbo_quad, vbo_particles, mapped_ptr, buffer_size) =
                setup_gpu_buffers(max_particles_on_gpu, use_instanced_quads);

            // comme on stocke le pointeur mappé GPU,
            // on doit renvoyer le résultat du constructeur de structure dans la partie unsafe
            Ok(Self {
                glfw,
                window: Some(window),
                events: Some(events),
                vao,
                vbo_particles,
                vbo_quad,
                mapped_ptr,
                shader_program,
                max_particles_on_gpu,
                buffer_size,
                frames: 0,
                last_time: Instant::now(),
                window_size: (width, height),
                window_size_f32: (width as f32, height as f32),
                window_last_pos,
                window_last_size,
                loc_size,
                loc_tex,
                loc_use,
                texture_id,
                use_instanced_quads,
            })
        }
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
                self.recreate_buffers(new_max);
            }
        }
    }

    unsafe fn recreate_buffers(&mut self, new_max: usize) {
        // 1. Libérer les anciens buffers
        gl::DeleteVertexArrays(1, &self.vao);
        gl::DeleteBuffers(1, &self.vbo_particles);

        // 2. Recréer avec la nouvelle taille
        let (vao, vbo_quad, vbo_particles, mapped_ptr, buffer_size) =
            setup_gpu_buffers(new_max, self.use_instanced_quads);

        // 3. Mettre à jour les champs
        self.vao = vao;
        self.vbo_particles = vbo_particles;
        self.vbo_quad = vbo_quad;
        self.mapped_ptr = mapped_ptr;
        self.buffer_size = buffer_size;
        self.max_particles_on_gpu = new_max;
    }

    /// Remplit directement le buffer GPU avec les données des particules actives.
    ///
    /// Cette méthode copie les données de toutes les particules actives fournies par
    /// le moteur physique `physic` dans le buffer GPU mappé en mémoire CPU (`self.mapped_ptr`),
    /// jusqu'à un maximum défini par `self.max_particles_on_gpu`.
    ///
    /// # Fonctionnalité
    /// - Chaque particule physique active est convertie en `ParticleGPU` via `Particle::to_particle_gpu()`.
    /// - Les données sont écrites directement dans le slice mappé `gpu_slice`, garantissant que
    ///   la mémoire GPU est correctement mise à jour.
    /// - La méthode renvoie le nombre de particules copiées dans le buffer GPU (`count`),
    ///   ce qui peut être utilisé pour des opérations ultérieures (par exemple le rendu).
    /// - Après avoir rempli le slice, un flush explicite est effectué via
    ///   `gl::FlushMappedBufferRange` pour que le GPU prenne en compte les modifications.
    ///
    /// # Sécurité et `unsafe`
    /// - La méthode utilise un bloc `unsafe` car elle crée un slice Rust mutable (`gpu_slice`)
    ///   à partir d'un pointeur brut mappé sur la mémoire GPU (`self.mapped_ptr`).
    /// - Les garanties suivantes sont respectées pour que cette opération soit sûre :
    ///   1. `self.mapped_ptr` pointe vers une mémoire valide et correctement alignée
    ///      pour `ParticleGPU`.
    ///   2. Le slice a une longueur exacte de `self.max_particles_on_gpu`, garantissant
    ///      que l’on n’accède jamais hors limites.
    ///   3. La boucle `for` itère en parallèle sur le slice GPU et les particules actives via `zip`,
    ///      donc aucune écriture ne dépasse la capacité du slice.
    /// - Chaque élément du slice est écrit **en place** (`*dst = …`), et le flush est effectué
    ///   après toutes les écritures pour synchroniser le GPU.
    ///
    /// # Remarques
    /// - Il est important de **ne pas utiliser de `map()` ou `collect()` ici**, car la mémoire
    ///   mappée GPU requiert des écritures en place. Les transformations fonctionnelles pourraient
    ///   entraîner des écritures incorrectes ou hors ordre, rendant le GPU incapable de lire les
    ///   données correctement.
    /// - Cette méthode est conçue pour être rapide et sûre, tout en restant compatible avec
    ///   des milliers de particules dans un buffer mappé CPU ↔ GPU.
    pub fn fill_particle_data_direct<P: PhysicEngine>(&mut self, physic: &P) -> usize {
        let mut count = 0;

        unsafe {
            // Crée un slice Rust sûr sur le buffer GPU
            let gpu_slice =
                std::slice::from_raw_parts_mut(self.mapped_ptr, self.max_particles_on_gpu);

            // Itère en parallèle sur les particules physiques actives et les slots GPU disponibles
            // le zip se fait (dans l'ordre) du slice gpu vers les particules actives,
            // donc la taille max du slice gpu ne pourra (implicitement) jamais être dépassée.
            for (i, (dst, src)) in gpu_slice
                .iter_mut()
                .zip(physic.active_particles())
                .enumerate()
            {
                *dst = src.to_particle_gpu();
                count = i + 1;
            }

            // Flush explicite de la zone modifiée pour que le GPU voit les changements
            let written_bytes = (count * std::mem::size_of::<ParticleGPU>()) as isize;
            gl::FlushMappedBufferRange(gl::ARRAY_BUFFER, 0, written_bytes);
        }

        count
    }

    /// Envoie le slice de ParticleGPU au GPU et dessine.
    /// Cette fonction est stateless vis-à-vis de `self` (sauf pour uniforms), et accepte le slice brut.
    /// Rendu des particules via un buffer OpenGL persistant.
    ///
    /// Cette méthode lie les ressources GPU nécessaires, et dessine
    /// les particules à l’écran sous forme de points (`GL_POINTS`).
    ///
    /// # Paramètres
    /// - `count`: nombre de particules à afficher. Si `count` vaut 0, aucun rendu n’est effectué.
    ///
    /// # Détails techniques
    /// - **Persistent Mapping** : Le VBO (Vertex Buffer Object) est mappé de manière
    ///   persistante en mémoire GPU. Cela signifie que les données peuvent être modifiées
    ///   directement via un pointeur mémoire (obtenu avec `glMapBufferRange`), sans devoir
    ///   réappeler `glBufferSubData` à chaque frame.
    /// - Le shader utilisé (`self.shader_program`) est supposé gérer le rendu de chaque
    ///   particule via les attributs du VBO et les uniformes `width` et `height`.
    ///
    /// # Sécurité
    /// Cette fonction utilise des appels `unsafe` à l’API OpenGL, car ces fonctions
    /// manipulent directement des pointeurs mémoire GPU et des ressources système.
    /// Il est de la responsabilité de l’appelant de garantir que le contexte OpenGL
    /// est valide et que les ressources (`VAO`, `VBO`, shader, etc.) sont correctement initialisées.
    fn render_particles_with_persistent_buffer(&self, count: usize) {
        // Si aucune particule, on ne fait rien
        if count == 0 {
            return;
        }

        unsafe {
            // Active le shader de rendu des particules
            gl::UseProgram(self.shader_program);

            // Envoie les dimensions de la fenêtre au shader (uniforms)
            gl::Uniform2f(
                self.loc_size,
                self.window_size_f32.0,
                self.window_size_f32.1,
            );

            // Lie le VAO et VBO correspondant aux particules
            gl::BindVertexArray(self.vao);

            if !self.use_instanced_quads {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles);
                // Dessine les particules sous forme de points
                gl::DrawArrays(gl::POINTS, 0, count as i32);
            } else {
                gl::ActiveTexture(gl::TEXTURE0);
                gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
                gl::Uniform1i(self.loc_tex, 0);
                gl::Uniform1i(self.loc_use, 1); // 1 = activer la texture
                                                //
                gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_quad);
                gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, count as i32);
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
        }

        // Remplit le buffer GPU
        let nb_particles_rendered = self.fill_particle_data_direct(physic);

        // // Dessine les particules
        self.render_particles_with_persistent_buffer(nb_particles_rendered);

        nb_particles_rendered
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
            if self.vbo_particles != 0 {
                gl::DeleteBuffers(1, &self.vbo_particles);
                self.vbo_particles = 0;
            }
            if self.vao != 0 {
                gl::DeleteVertexArrays(1, &self.vao);
                self.vao = 0;
            }
            if self.shader_program != 0 {
                gl::DeleteProgram(self.shader_program);
                self.shader_program = 0;
            }
        }

        if let Some(window) = self.window.take() {
            drop(window);
        }
    }
}

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

unsafe fn setup_gpu_buffers(
    max_particles_on_gpu: usize,
    use_instanced_quads: bool,
) -> (u32, u32, u32, *mut ParticleGPU, isize) {
    let (mut vao, mut vbo_quad, mut vbo_particles) = (0u32, 0u32, 0u32);

    // === VAO ===
    gl::GenVertexArrays(1, &mut vao);
    gl::BindVertexArray(vao);

    if use_instanced_quads {
        // === 1️⃣ QuadVertexAttribPointer unité statique ===
        const QUAD_VERTICES: [f32; 8] = [
            -1.0, -1.0, // bottom-left
            1.0, -1.0, // bottom-right
            -1.0, 1.0, // top-left
            1.0, 1.0, // top-right
        ];

        gl::GenBuffers(1, &mut vbo_quad);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (QUAD_VERTICES.len() * std::mem::size_of::<f32>()) as isize,
            QUAD_VERTICES.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        // layout(location = 0): sommets du quad
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            2,
            gl::FLOAT,
            gl::FALSE,
            2 * std::mem::size_of::<f32>() as i32,
            std::ptr::null(),
        );
        gl::VertexAttribDivisor(0, 0); // par sommet
    }

    // === 2️⃣ Particules persistantes ===
    gl::GenBuffers(1, &mut vbo_particles);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo_particles);

    let buffer_size = (max_particles_on_gpu * std::mem::size_of::<ParticleGPU>()) as isize;
    info!(
        "🎮 Allocating instanced particle buffer: {} particles → {}",
        max_particles_on_gpu,
        buffer_size.human_bytes()
    );

    // Allocation persistante
    gl::BufferStorage(
        gl::ARRAY_BUFFER,
        buffer_size,
        std::ptr::null(),
        gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
    );

    // Mapping CPU → GPU
    let mapped_ptr = gl::MapBufferRange(
        gl::ARRAY_BUFFER,
        0,
        buffer_size,
        gl::MAP_WRITE_BIT
            | gl::MAP_PERSISTENT_BIT
            | gl::MAP_COHERENT_BIT
            | gl::MAP_FLUSH_EXPLICIT_BIT,
    ) as *mut ParticleGPU;

    // === Définition des attributs instanciés ===
    if !use_instanced_quads {
        ParticleGPU::setup_vertex_attribs();
    } else {
        ParticleGPU::setup_vertex_attribs_for_instanced_quad();
    }
    // === Nettoyage ===
    gl::BindVertexArray(0);

    (vao, vbo_quad, vbo_particles, mapped_ptr, buffer_size)
}

pub fn src_shaders_particles() -> (&'static str, &'static str) {
    let vertex_src = r#"
        #version 330 core
        layout(location = 0) in vec2 aPos;
        layout(location = 1) in vec3 aColor;
        layout(location = 2) in float aLife;
        layout(location = 3) in float aMaxLife;
        layout(location = 4) in float aSize;

        out vec3 vertexColor;
        out float alpha;

        uniform vec2 uSize;

        void main() {
            float a = clamp(aLife / max(aMaxLife, 0.0001), 0.0, 1.0);
            alpha = a;
            vertexColor = aColor;

            float x = aPos.x / uSize.x * 2.0 - 1.0;
            float y = aPos.y / uSize.y * 2.0 - 1.0;
            gl_Position = vec4(x, y, 0.0, 1.0);

            gl_PointSize = 2.0 + 5.0 * a;
        }
        "#;

    let fragment_src = r#"
        #version 330 core
        in vec3 vertexColor;
        in float alpha;
        out vec4 FragColor;

        void main() {
            vec2 uv = gl_PointCoord - vec2(0.5);
            float dist = dot(uv, uv);
            if(dist > 0.25) discard;
            float falloff = smoothstep(0.25, 0.0, dist);
            FragColor = vec4(vertexColor, alpha * falloff);
        }
        "#;
    (vertex_src, fragment_src)
}

fn src_shaders_instanced_quads() -> (&'static str, &'static str) {
    let vertex_src = r#"
        #version 330 core

        // === Quad unité (4 sommets pour TRIANGLE_STRIP)
        layout(location = 0) in vec2 aQuad;

        // === Attributs instanciés (1 par particule)
        layout(location = 1) in vec2 aPos;
        layout(location = 2) in vec3 aColor;
        layout(location = 3) in float aLife;
        layout(location = 4) in float aMaxLife;
        layout(location = 5) in float aSize;

        out vec3 vColor;
        out float vAlpha;
        out vec2 vUV;

        uniform vec2 uSize;

        void main() {
            // Ratio de vie (comme avant)
            vAlpha = clamp(aLife / max(aMaxLife, 0.0001), 0.0, 1.0);
            vColor = aColor;

            // On reconstruit les coordonnées UV du quad (-0.5 → +0.5)
            vUV = aQuad * 0.5 + 0.5;

            // Position du sommet quad dans l’espace clip (avec taille)
            vec2 world = aPos + aQuad * aSize * (2.0 + 5.0 * vAlpha);

            float x = world.x / uSize.x * 2.0 - 1.0;
            float y = world.y / uSize.y * 2.0 - 1.0;
            gl_Position = vec4(x, y, 0.0, 1.0);
        }
        "#;

    let fragment_src = r#"
        #version 330 core

        in vec3 vColor;
        in float vAlpha;
        in vec2 vUV;
        out vec4 FragColor;

        uniform sampler2D uTexture;
        uniform bool uUseTexture;

        void main() {
            // Recrée le disque (comme avant)
            vec2 uv = vUV - vec2(0.5);
            float dist = dot(uv, uv);
            if (dist > 0.25)
                discard;

            float falloff = smoothstep(0.25, 0.0, dist);

            // Heat-color fade (identique)
            vec3 heatColor;
            if (vAlpha > 0.66) {
                heatColor = mix(vec3(1.0, 1.0, 1.0), vec3(1.0, 0.5, 0.0), (1.0 - vAlpha) / 0.34);
            } else if (vAlpha > 0.33) {
                heatColor = mix(vec3(1.0, 0.5, 0.0), vec3(1.0, 0.0, 0.0), (0.66 - vAlpha) / 0.33);
            } else {
                heatColor = mix(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0), (0.33 - vAlpha) / 0.33);
            }

            vec4 baseColor = vec4(vColor * heatColor, vAlpha) * falloff;

            // Si une texture est utilisée → multiplie le résultat
            if (uUseTexture) {
                vec4 texColor = texture(uTexture, vUV);
                baseColor *= texColor;
            }

            FragColor = baseColor;
        }
        "#;
    (vertex_src, fragment_src)
}

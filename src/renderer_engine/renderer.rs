use crate::RendererEngine;
use crate::{log_metrics_and_fps, profiler::Profiler};
use anyhow::{anyhow, Result};
use glfw::{Action, Context, Key};
use log::{debug, info};
use std::ffi::CString;
use std::time::Instant;

use crate::audio_engine::AudioEngine;
use crate::physic_engine::config::PhysicConfig;
use crate::physic_engine::PhysicEngine;
use crate::renderer_engine::tools::{
    compile_shader_program, print_context_info, setup_opengl_debug,
};
use crate::renderer_engine::types::ParticleGPU;

use crate::audio_engine::DopplerEvent;
use crate::utils::human_bytes::HumanBytes;
use crossbeam::channel::Sender;

pub struct Renderer {
    pub glfw: glfw::Glfw,
    pub window: Option<glfw::PWindow>,
    pub events: Option<glfw::GlfwReceiver<(f64, glfw::WindowEvent)>>,

    vao: u32,
    vbo: u32,
    mapped_ptr: *mut ParticleGPU,
    shader_program: u32,

    max_particles_on_gpu: usize,
    buffer_size: isize,

    frames: u32,
    last_time: Instant,
    width: f32,
    height: f32,

    loc_w: i32,
    loc_h: i32,
    _doppler_sender: Option<Sender<DopplerEvent>>, // option pour compatibilité si non fourni
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
//doppler_queue
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
        width: u32,
        height: u32,
        title: &str,
        max_particles_on_gpu: usize,
        doppler_sender: Option<Sender<DopplerEvent>>,
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
            .create_window(width, height, title, glfw::WindowMode::Windowed)
            .ok_or_else(|| anyhow!("Erreur: impossible de créer la fenêtre GLFW"))?;

        window.make_current();
        window.set_key_polling(true);
        window.set_framebuffer_size_polling(true);
        window.set_cursor_pos_polling(true);
        window.set_mouse_button_polling(true);
        window.set_scroll_polling(true);

        info!("✅ OpenGL context ready for '{}'", title);

        gl::load_with(|s| window.get_proc_address(s) as *const _);
        print_context_info();

        unsafe {
            setup_opengl_debug();

            gl::Enable(gl::PROGRAM_POINT_SIZE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        // Vertex / Fragment shaders (GPU fait normalisation, alpha, size)
        let vertex_src = r#"
            #version 330 core
            layout(location = 0) in vec2 aPos;       // pixel position
            layout(location = 1) in vec3 aColor;     // rgb
            layout(location = 2) in float aLife;     // life
            layout(location = 3) in float aMaxLife;  // max_life
            layout(location = 4) in float aSize;     // base size

            out vec3 vertexColor;
            out float alpha;

            uniform float uWidth;
            uniform float uHeight;

            void main() {
                float x = aPos.x / uWidth * 2.0 - 1.0;
                float y = aPos.y / uHeight * 2.0 - 1.0;
                gl_Position = vec4(x, y, 0.0, 1.0);

                alpha = clamp(aLife / max(aMaxLife, 0.0001), 0.0, 1.0);
                gl_PointSize = 2.0 + 5.0 * alpha;
                vertexColor = aColor;
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
                if (dist > 0.25) discard;
                float falloff = smoothstep(0.25, 0.0, dist);
    
                // heat color fade: life fraction = alpha
                vec3 heatColor;
                if (alpha > 0.66) {
                    // white → yellow
                    heatColor = mix(vec3(1.0, 1.0, 1.0), vec3(1.0, 0.5, 0.0), (1.0 - alpha) / 0.34);
                } else if (alpha > 0.33) {
                    // yellow → red
                    heatColor = mix(vec3(1.0, 0.5, 0.0), vec3(1.0, 0.0, 0.0), (0.66 - alpha) / 0.33);
                } else {
                    // red → black
                    heatColor = mix(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0), (0.33 - alpha) / 0.33);
                }
                // mix finale avec la couleur de vertex/particule et la heat color fade
                FragColor = vec4(vertexColor * falloff * heatColor, alpha * falloff);
            }
        "#;

        let shader_program = unsafe { compile_shader_program(vertex_src, fragment_src) };
        let (loc_w, loc_h) = unsafe {
            (
                gl::GetUniformLocation(shader_program, CString::new("uWidth").unwrap().as_ptr()),
                gl::GetUniformLocation(shader_program, CString::new("uHeight").unwrap().as_ptr()),
            )
        };

        // // VAO/VBO setup
        // let (mut vao, mut vbo) = (0u32, 0u32);

        unsafe {
            let (vao, vbo, mapped_ptr, buffer_size) = setup_gpu_buffers(max_particles_on_gpu);

            // comme on stocke le pointeur mappé GPU,
            // on doit renvoyer le résultat du constructeur de structure dans la partie unsafe
            Ok(Self {
                glfw,
                window: Some(window),
                events: Some(events),
                vao,
                vbo,
                mapped_ptr,
                shader_program,
                max_particles_on_gpu,
                buffer_size,
                frames: 0,
                last_time: Instant::now(),
                width: width as f32,
                height: height as f32,
                loc_w,
                loc_h,
                _doppler_sender: doppler_sender,
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
        gl::DeleteBuffers(1, &self.vbo);

        // 2. Recréer avec la nouvelle taille
        let (vao, vbo, mapped_ptr, buffer_size) = setup_gpu_buffers(new_max);

        // 3. Mettre à jour les champs
        self.vao = vao;
        self.vbo = vbo;
        self.mapped_ptr = mapped_ptr;
        self.buffer_size = buffer_size;
        self.max_particles_on_gpu = new_max;
    }

    /// Remplit le buffer GPU directement
    pub fn fill_particle_data_direct<P: PhysicEngine>(&mut self, physic: &P) -> usize {
        let mut count = 0;

        unsafe {
            // Crée un slice Rust sûr sur le buffer GPU
            let gpu_slice =
                std::slice::from_raw_parts_mut(self.mapped_ptr, self.max_particles_on_gpu);

            for (i, p) in physic.active_particles().enumerate() {
                if i >= self.max_particles_on_gpu {
                    break;
                }
                gpu_slice[i] = ParticleGPU {
                    pos_x: p.pos.x,
                    pos_y: p.pos.y,
                    col_r: p.color.x,
                    col_g: p.color.y,
                    col_b: p.color.z,
                    life: p.life,
                    max_life: p.max_life,
                    size: p.size,
                };
                count += 1;
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
            // Lie le VAO et VBO correspondant aux particules
            gl::BindVertexArray(self.vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            // Active le shader de rendu des particules
            gl::UseProgram(self.shader_program);

            // Envoie les dimensions de la fenêtre au shader (uniforms)
            gl::Uniform1f(self.loc_w, self.width);
            gl::Uniform1f(self.loc_h, self.height);

            // Dessine les particules sous forme de points
            gl::DrawArrays(gl::POINTS, 0, count as i32);
        }
    }

    /// Exécute une seule frame (update + rendu)
    pub fn render_frame<P: PhysicEngine>(&mut self, physic: &mut P) -> usize {
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
        let log_interval = std::time::Duration::from_secs(5); // toutes les 5 secondes
                                                              // let mut next_doppler_update = Instant::now();

        audio.set_listener_position((self.width / 2.0, 0.0));

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

            let _render_guard = profiler.measure("render frame");
            let particles_rendered = self.render_frame(physic);
            profiler.record_metric("total particles drawn", particles_rendered);

            // affichage périodique
            if last_log.elapsed() >= log_interval {
                log_metrics_and_fps!(&profiler);
                last_log = Instant::now();
            }

            let mut reload_config = false;

            // Swap buffers + events
            if let Some(window) = &mut self.window {
                window.swap_buffers();
                drop(_render_guard);

                self.glfw.poll_events();

                if let Some(events) = &self.events {
                    for (_, event) in glfw::flush_messages(events) {
                        match event {
                            glfw::WindowEvent::FramebufferSize(w, h) => unsafe {
                                gl::Viewport(0, 0, w, h);
                                self.width = w as f32;
                                self.height = h as f32;
                                physic.set_window_width(w as f32);
                                audio.set_listener_position((w as f32 / 2.0, 0.0));
                            },
                            glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                                window.set_should_close(true);
                            }
                            glfw::WindowEvent::Key(Key::R, _, Action::Press, _) => {
                                reload_config = true;
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

    pub fn close(&mut self) {
        info!("🧹 Fermeture du Renderer");

        unsafe {
            if self.vbo != 0 {
                gl::DeleteBuffers(1, &self.vbo);
                self.vbo = 0;
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

unsafe fn setup_gpu_buffers(max_particles_on_gpu: usize) -> (u32, u32, *mut ParticleGPU, isize) {
    let (mut vao, mut vbo) = (0u32, 0u32);
    gl::GenVertexArrays(1, &mut vao);
    gl::GenBuffers(1, &mut vbo);

    gl::BindVertexArray(vao);
    gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

    let buffer_size = (max_particles_on_gpu * std::mem::size_of::<ParticleGPU>()) as isize;
    info!(
        "🎮 Reallocating GPU buffer: {} particles → {}",
        max_particles_on_gpu,
        buffer_size.human_bytes()
    );

    gl::BufferStorage(
        gl::ARRAY_BUFFER,
        buffer_size,
        std::ptr::null(),
        gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT,
    );

    let mapped_ptr = gl::MapBufferRange(
        gl::ARRAY_BUFFER,
        0,
        buffer_size,
        gl::MAP_WRITE_BIT
            | gl::MAP_PERSISTENT_BIT
            | gl::MAP_COHERENT_BIT
            | gl::MAP_FLUSH_EXPLICIT_BIT,
    ) as *mut ParticleGPU;

    // Appel unique, auto-configuré
    ParticleGPU::setup_vertex_attribs();

    (vao, vbo, mapped_ptr, buffer_size)
}

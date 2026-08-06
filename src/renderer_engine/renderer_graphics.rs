use log::{debug, info};

use crate::physic_engine::PhysicEngineIterator;
use crate::renderer_engine::shader::compile_shader_program_from_files;
use crate::renderer_engine::types::ParticleGPU;
use crate::utils::human_bytes::HumanBytes;
use crate::{label_gl_object, pop_debug_group, push_debug_group};

macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const i8
    };
}

use crate::renderer_engine::constants;

const VERTEX_SHADER_PATH: &str = constants::SHADER_POINT_VERTEX_PATH;
const FRAGMENT_SHADER_PATH: &str = constants::SHADER_POINT_FRAGMENT_PATH;

pub struct RendererGraphics {
    pub vao: u32,
    pub vbo_particles: u32,

    pub mapped_ptr: *mut ParticleGPU,

    // Shader
    pub shader_program: u32,

    pub max_particles_on_gpu: usize,

    pub render_trails: bool,
    pub render_explosions: bool,

    // Triple buffering
    pub current_frame: usize,
    pub fences: [Option<gl::types::GLsync>; 3],
}

impl RendererGraphics {
    pub fn new(max_particles_on_gpu: usize) -> Self {
        let shader_program =
            unsafe { compile_shader_program_from_files(VERTEX_SHADER_PATH, FRAGMENT_SHADER_PATH) };

        // Bind uniform block "GlobalData" to binding point 0
        unsafe {
            let block_idx = gl::GetUniformBlockIndex(shader_program, cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(shader_program, block_idx, 0);
            }
        }

        // VAO/VBO setup
        unsafe {
            let (vao, vbo_particles, mapped_ptr, _buffer_size) =
                RendererGraphics::setup_gpu_buffers(max_particles_on_gpu);

            // 🏷️ Rendre tes ressources visibles dans RenderDoc
            label_gl_object!(gl::PROGRAM, shader_program, "Shader_PointRendering");
            label_gl_object!(gl::VERTEX_ARRAY, vao, "VAO_Particules_Base");
            label_gl_object!(gl::BUFFER, vbo_particles, "VBO_Particules_Data");

            Self {
                vao,
                vbo_particles,
                mapped_ptr,
                shader_program,
                max_particles_on_gpu,
                render_trails: true,
                render_explosions: true,
                current_frame: 0,
                fences: [None, None, None],
            }
        }
    }

    unsafe fn setup_gpu_buffers(
        max_particles_on_gpu: usize,
    ) -> (u32, u32, *mut ParticleGPU, isize) {
        let (mut vao, mut vbo_particles) = (0u32, 0u32);

        // === VAO ===
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // === 2️⃣ Particules persistantes ===
        gl::GenBuffers(1, &mut vbo_particles);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo_particles);

        let buffer_size = (3 * max_particles_on_gpu * std::mem::size_of::<ParticleGPU>()) as isize;
        info!(
            "🎮 Allocating instanced particle buffer: {} particles → {}",
            max_particles_on_gpu,
            buffer_size.human_bytes()
        );

        // Allocation persistante (sans MAP_COHERENT_BIT)
        gl::BufferStorage(
            gl::ARRAY_BUFFER,
            buffer_size,
            std::ptr::null(),
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT,
        );

        // Mapping CPU → GPU (sans MAP_COHERENT_BIT, avec MAP_FLUSH_EXPLICIT_BIT)
        let mapped_ptr = gl::MapBufferRange(
            gl::ARRAY_BUFFER,
            0,
            buffer_size,
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_FLUSH_EXPLICIT_BIT,
        ) as *mut ParticleGPU;

        // === Définition des attributs instanciés ===
        ParticleGPU::setup_vertex_attribs();
        // === Nettoyage ===
        gl::BindVertexArray(0);

        label_gl_object!(gl::VERTEX_ARRAY, vao, "VAO_Points");
        label_gl_object!(gl::BUFFER, vbo_particles, "VBO_Points_Data");

        (vao, vbo_particles, mapped_ptr, buffer_size)
    }

    /// Libère le mapping persistant du VBO particules, puis supprime le VAO et le VBO.
    ///
    /// Cette fonction centralise la séquence critique :
    ///   1. `UnmapBuffer` (obligatoire avant tout `DeleteBuffers` sur un buffer persistently-mapped)
    ///   2. `DeleteVertexArrays`
    ///   3. `DeleteBuffers`
    ///
    /// Elle est appelée par [`recreate_buffers`] ET [`close`] pour garantir que les deux
    /// chemins appliquent exactement le même protocole — évitant toute divergence future.
    ///
    /// # Safety
    /// Le contexte OpenGL doit être valide et actif.
    unsafe fn release_particle_buffers(&mut self) {
        // Unmap the persistent buffer BEFORE deleting it (required by OpenGL spec /
        // ARB_buffer_storage): deleting a mapped buffer is undefined behavior.
        if !self.mapped_ptr.is_null() && self.vbo_particles != 0 {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles);
            gl::UnmapBuffer(gl::ARRAY_BUFFER);
            self.mapped_ptr = std::ptr::null_mut();
        }
        if self.vao != 0 {
            gl::DeleteVertexArrays(1, &self.vao);
            self.vao = 0;
        }
        if self.vbo_particles != 0 {
            gl::DeleteBuffers(1, &self.vbo_particles);
            self.vbo_particles = 0;
        }
    }

    /// Recrée les buffers GPU avec une nouvelle taille maximale.
    /// Cette opération libère les anciens buffers via [`release_particle_buffers`]
    /// et en crée de nouveaux, puis met à jour les champs internes.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn recreate_buffers(&mut self, new_max: usize) {
        // 1. Libérer les fences et reset l'index de frame
        for fence in self.fences.iter_mut() {
            if let Some(sync) = fence.take() {
                gl::DeleteSync(sync);
            }
        }
        self.current_frame = 0;

        // 2. Libérer les anciens buffers (unmap + delete) via la fonction partagée
        self.release_particle_buffers();

        // 3. Recréer avec la nouvelle taille
        let (vao, vbo_particles, mapped_ptr, _buffer_size) =
            RendererGraphics::setup_gpu_buffers(new_max);

        // 4. Mettre à jour les champs
        self.vao = vao;
        self.vbo_particles = vbo_particles;
        self.mapped_ptr = mapped_ptr;
        self.max_particles_on_gpu = new_max;
    }

    /// Remplit directement le buffer GPU mappé avec les particules "têtes"
    /// renvoyées par le moteur physique.
    ///
    /// Cette fonction :
    /// - itère sur un pipeline paresseux (aucune allocation CPU)
    /// - écrit séquentiellement dans la mémoire GPU persistently-mapped (optimal)
    /// - flush uniquement la zone écrite
    ///
    /// C’est un pattern AZDO performant : aucune écriture sparse, aucun saut mémoire,
    /// seulement du contigu cpu → gpu.
    /// # Safety
    /// This function is unsafe because it directly manipulates GPU resources.
    /// The caller must ensure that the OpenGL context is valid.
    pub unsafe fn fill_particle_data_direct(
        &mut self,
        physic: &dyn PhysicEngineIterator,
        alpha: f32,
    ) -> usize {
        // Synchroniser : Attendre que le GPU ait fini de lire cette section
        if let Some(sync) = self.fences[self.current_frame] {
            gl::ClientWaitSync(sync, gl::SYNC_FLUSH_COMMANDS_BIT, 10_000_000_000); // 10s
            gl::DeleteSync(sync);
            self.fences[self.current_frame] = None;
        }

        let mut count = 0;

        // Slice Rust mutable mappé sur la section courante de la mémoire GPU.
        let offset = self.current_frame * self.max_particles_on_gpu;
        let gpu_slice =
            std::slice::from_raw_parts_mut(self.mapped_ptr.add(offset), self.max_particles_on_gpu);

        let factor = (1.0 - alpha) * crate::physic_engine::constants::FIXED_TIMESTEP_DELTA;
        let all_visible = self.render_trails && self.render_explosions;
        physic.for_each_active_particle(&mut |p| {
            let visible = if all_visible {
                true
            } else {
                match p.particle_type {
                    crate::physic_engine::ParticleType::Trail => self.render_trails,
                    crate::physic_engine::ParticleType::Explosion => self.render_explosions,
                    _ => true,
                }
            };
            if visible && count < self.max_particles_on_gpu {
                // ⏱️ Piste 3 : Fast Cast-Copy (Layout parfait)
                let src_ptr =
                    p as *const crate::physic_engine::particle::Particle as *const ParticleGPU;
                let mut gpu_p = *src_ptr;

                if factor > crate::renderer_engine::constants::RENDER_INTERPOLATION_EPSILON {
                    gpu_p.pos_x -= p.vel.x * factor;
                    gpu_p.pos_y -= p.vel.y * factor;
                }

                // Assigne la luminosité calculée (x^4 via multiplication rapide sans libm powi)
                let l = p.life / p.max_life.max(0.0001);
                let l2 = l * l;
                gpu_p.brightness = l2 * l2;

                gpu_slice[count] = gpu_p;
                count += 1;
            }
        });

        // ⏱️ Piste 1 : Explicit Flush manquant de gl::MAP_COHERENT_BIT
        if count > 0 {
            let write_size = (count * std::mem::size_of::<ParticleGPU>()) as isize;
            let offset_bytes = (offset * std::mem::size_of::<ParticleGPU>()) as isize;
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles);
            gl::FlushMappedBufferRange(gl::ARRAY_BUFFER, offset_bytes, write_size);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
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
    /// # Safety
    /// Cette fonction utilise des appels `unsafe` à l’API OpenGL, car ces fonctions
    /// manipulent directement des pointeurs mémoire GPU et des ressources système.
    /// Il est de la responsabilité de l’appelant de garantir que le contexte OpenGL
    /// est valide et que les ressources (`VAO`, `VBO`, shader, etc.) sont correctement initialisées.
    pub unsafe fn render_particles_with_persistent_buffer(
        &mut self,
        count: usize,
        active_shader: &mut u32,
        _active_texture: &mut u32,
    ) {
        // Si aucune particule, on ne fait rien
        if count == 0 {
            return;
        }

        push_debug_group!(20, "Draw Points");

        // Active l'Additive Blending pour les particules de traînée et d'explosion
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE);
        gl::DepthMask(gl::FALSE);

        // Active le shader de rendu des particules (seulement s'il n'est pas déjà actif)
        if *active_shader != self.shader_program {
            gl::UseProgram(self.shader_program);
            *active_shader = self.shader_program;
        }

        // Lie le VAO correspondant aux particules
        gl::BindVertexArray(self.vao);

        // Dessine les particules sous forme de points en décalant l'index de départ selon la section courante
        let first_vertex = (self.current_frame * self.max_particles_on_gpu) as i32;
        gl::DrawArrays(gl::POINTS, first_vertex, count as i32);

        // Restore default alpha blending and depth mask
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::DepthMask(gl::TRUE);

        pop_debug_group!();

        // Placer une barrière de synchronisation après le draw call
        if let Some(old_sync) = self.fences[self.current_frame] {
            gl::DeleteSync(old_sync);
        }
        let sync = gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0);
        self.fences[self.current_frame] = Some(sync);

        // Passer à la frame suivante
        self.current_frame = (self.current_frame + 1) % 3;
    }

    /// Libère les ressources GPU associées à ce RendererGraphics.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn close(&mut self) {
        // Libérer les fences
        for fence in self.fences.iter_mut() {
            if let Some(sync) = fence.take() {
                gl::DeleteSync(sync);
            }
        }

        // Libérer les buffers particules (unmap + delete) via la fonction partagée
        self.release_particle_buffers();

        if self.shader_program != 0 {
            gl::DeleteProgram(self.shader_program);
            self.shader_program = 0;
        }
        debug!("Graphic Engine for Points Rendering closed and reset.");
    }

    /// Recharge les shaders depuis les fichiers et recompile le programme shader.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn reload_shaders(&mut self) -> Result<(), String> {
        use crate::renderer_engine::shader::try_compile_shader_program_from_files;
        use log::error;

        match try_compile_shader_program_from_files(VERTEX_SHADER_PATH, FRAGMENT_SHADER_PATH) {
            Ok(new_program) => {
                // Supprimer l'ancien programme shader
                if self.shader_program != 0 {
                    gl::DeleteProgram(self.shader_program);
                }

                // Utiliser le nouveau programme
                self.shader_program = new_program;

                // Lier le block uniform "GlobalData" au binding point 0
                let block_idx = gl::GetUniformBlockIndex(self.shader_program, cstr!("GlobalData"));
                if block_idx != gl::INVALID_INDEX {
                    gl::UniformBlockBinding(self.shader_program, block_idx, 0);
                }

                info!("Point rendering shaders reloaded successfully");
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to reload point rendering shaders:\n{}", e);
                Err(e)
            }
        }
    }
}
use crate::renderer_engine::particle_renderer::ParticleGraphicsRenderer;

impl ParticleGraphicsRenderer for RendererGraphics {
    unsafe fn recreate_buffers(&mut self, new_max: usize) {
        self.recreate_buffers(new_max);
    }

    unsafe fn fill_particle_data_direct(
        &mut self,
        physic: &dyn PhysicEngineIterator,
        alpha: f32,
    ) -> usize {
        self.fill_particle_data_direct(physic, alpha)
    }

    unsafe fn render_particles_with_persistent_buffer(
        &mut self,
        count: usize,
        active_shader: &mut u32,
        active_texture: &mut u32,
    ) {
        self.render_particles_with_persistent_buffer(count, active_shader, active_texture);
    }

    fn get_shader_program(&self) -> u32 {
        self.shader_program
    }

    fn get_texture_id(&self) -> u32 {
        0 // Pas de texture pour le rendu par points
    }

    fn get_tex_ratio(&self) -> f32 {
        1.0
    }

    fn set_visibility(&mut self, render_trails: bool, render_explosions: bool) {
        self.render_trails = render_trails;
        self.render_explosions = render_explosions;
    }

    fn render_order(&self) -> u32 {
        30
    }

    unsafe fn reload_shaders(&mut self) -> Result<(), String> {
        self.reload_shaders()
    }

    unsafe fn close(&mut self) {
        self.close();
    }
}

impl Drop for RendererGraphics {
    fn drop(&mut self) {
        unsafe {
            self.close();
        }
    }
}

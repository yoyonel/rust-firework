use log::{debug, info};

use crate::cstr;
use crate::physic_engine::{ParticleType, PhysicEngineIterator};
use crate::renderer_engine::shader::compile_shader_program_from_files;
use crate::renderer_engine::{types::ParticleGPU, utils::texture::load_texture};
use crate::utils::human_bytes::HumanBytes;
use crate::{label_gl_object, pop_debug_group, push_debug_group};

const VERTEX_SHADER_PATH: &str = "assets/shaders/instanced_textured_quad.vert.glsl";
const FRAGMENT_SHADER_PATH: &str = "assets/shaders/instanced_textured_quad.frag.glsl";

pub struct RendererGraphicsInstanced {
    vao: u32,
    vbo_particles: u32,
    vbo_quad: u32,

    mapped_ptr: *mut ParticleGPU,

    shader_program: u32,
    // Shader
    loc_size: i32,
    loc_tex: i32,
    texture_id: u32,
    tex_ratio: f32, // Stocke le ratio de texture pour le reload

    max_particles_on_gpu: usize,

    // Configuration du type de particule
    particle_type: ParticleType,
}

impl RendererGraphicsInstanced {
    pub fn new(
        max_particles_on_gpu: usize,
        particle_type: ParticleType,
        texture_path: &str,
    ) -> Self {
        let shader_program =
            unsafe { compile_shader_program_from_files(VERTEX_SHADER_PATH, FRAGMENT_SHADER_PATH) };

        let loc_size = unsafe { gl::GetUniformLocation(shader_program, cstr!("uSize")) };
        let loc_tex = unsafe { gl::GetUniformLocation(shader_program, cstr!("uTexture")) };

        let (texture_id, tex_width, tex_height) = load_texture(texture_path);
        unsafe {
            gl::UseProgram(shader_program);
            gl::Uniform1f(
                gl::GetUniformLocation(shader_program, cstr!("uTexRatio")),
                tex_width as f32 / tex_height as f32,
            );

            label_gl_object!(gl::PROGRAM, shader_program, "Shader_InstancedQuad");
            label_gl_object!(gl::TEXTURE, texture_id, "Tex_Rocket_Sprite");
        }

        // VAO/VBO setup
        unsafe {
            let (vao, vbo_quad, vbo_particles, mapped_ptr, _buffer_size) =
                RendererGraphicsInstanced::setup_gpu_buffers(max_particles_on_gpu);

            Self {
                vao,
                vbo_particles,
                vbo_quad,
                mapped_ptr,
                shader_program,
                loc_size,
                loc_tex,
                texture_id,
                tex_ratio: tex_width as f32 / tex_height as f32,
                max_particles_on_gpu,
                particle_type,
            }
        }
    }
    /// Recrée les buffers GPU avec une nouvelle taille maximale.
    /// Cette opération libère les anciens buffers et en crée de nouveaux,
    /// puis met à jour les champs internes de la structure.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn recreate_buffers(&mut self, new_max: usize) {
        // 1. Libérer les anciens buffers
        gl::DeleteVertexArrays(1, &self.vao);
        gl::DeleteBuffers(1, &self.vbo_particles);
        gl::DeleteBuffers(1, &self.vbo_quad);

        // 2. Recréer avec la nouvelle taille
        let (vao, vbo_quad, vbo_particles, mapped_ptr, _buffer_size) =
            RendererGraphicsInstanced::setup_gpu_buffers(new_max);

        // 3. Mettre à jour les champs
        self.vao = vao;
        self.vbo_particles = vbo_particles;
        self.vbo_quad = vbo_quad;
        self.mapped_ptr = mapped_ptr;
        self.max_particles_on_gpu = new_max;
    }

    /// Remplit directement le buffer GPU mappé avec les particules du type spécifié.
    ///
    /// Cette fonction :
    /// - itère sur un pipeline paresseux (aucune allocation CPU)
    /// - filtre les particules par type
    /// - écrit séquentiellement dans la mémoire GPU persistently-mapped (optimal)
    /// - flush uniquement la zone écrite
    ///
    /// C'est un pattern AZDO performant : aucune écriture sparse, aucun saut mémoire,
    /// seulement du contigu cpu → gpu.
    /// # Safety
    /// This function is unsafe because it directly manipulates GPU resources.
    /// The caller must ensure that the OpenGL context is valid.
    pub unsafe fn fill_particle_data_direct(&mut self, physic: &dyn PhysicEngineIterator) -> usize {
        let mut count = 0;

        // Slice Rust mutable mappé directement sur la mémoire GPU.
        // Toute écriture dans ce slice écrit physiquement dans la BAR / VRAM.
        let gpu_slice = std::slice::from_raw_parts_mut(self.mapped_ptr, self.max_particles_on_gpu);

        // Utilise for_each_particle_of_type pour filtrer les particules du bon type
        physic.for_each_particle_of_type(self.particle_type, &mut |p| {
            if count < self.max_particles_on_gpu {
                gpu_slice[count] = ParticleGPU {
                    pos_x: p.pos.x,
                    pos_y: p.pos.y,
                    col_r: p.color.x,
                    col_g: p.color.y,
                    col_b: p.color.z,
                    life: p.life,
                    max_life: p.max_life,
                    size: p.size,
                    angle: p.angle,
                    brightness: 0.0, // Bloom disabled for rockets
                };
                count += 1;
            }
        });

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
        &self,
        count: usize,
        window_size: (f32, f32),
    ) {
        // Si aucune particule, on ne fait rien
        if count == 0 {
            return;
        }

        push_debug_group!(30, "Draw Instanced Quads");

        // Active le shader de rendu des particules
        gl::UseProgram(self.shader_program);

        // Envoie les dimensions de la fenêtre au shader (uniforms)
        gl::Uniform2f(self.loc_size, window_size.0, window_size.1);

        // Lie le VAO et VBO correspondant aux particules
        gl::BindVertexArray(self.vao);

        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
        gl::Uniform1i(self.loc_tex, 0);
        //
        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_quad);
        gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, count as i32);

        pop_debug_group!();
    }

    /// Libère les ressources GPU associées à ce RendererGraphics.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn close(&mut self) {
        // Unmap the persistent buffer BEFORE deleting it
        if !self.mapped_ptr.is_null() && self.vbo_particles != 0 {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles);
            gl::UnmapBuffer(gl::ARRAY_BUFFER);
            self.mapped_ptr = std::ptr::null_mut();
        }

        if self.vbo_particles != 0 {
            gl::DeleteBuffers(1, &self.vbo_particles);
            self.vbo_particles = 0;
        }
        if self.vbo_quad != 0 {
            gl::DeleteBuffers(1, &self.vbo_quad);
            self.vbo_quad = 0;
        }
        if self.vao != 0 {
            gl::DeleteVertexArrays(1, &self.vao);
            self.vao = 0;
        }
        if self.texture_id != 0 {
            gl::DeleteTextures(1, &self.texture_id);
            self.texture_id = 0;
        }
        if self.shader_program != 0 {
            gl::DeleteProgram(self.shader_program);
            self.shader_program = 0;
        }
        debug!("Graphic Engine for Instanced Rendering closed and reset.");
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

                // Mettre à jour les uniform locations
                self.loc_size = gl::GetUniformLocation(self.shader_program, cstr!("uSize"));
                self.loc_tex = gl::GetUniformLocation(self.shader_program, cstr!("uTexture"));

                // Remettre à jour le ratio de texture
                gl::UseProgram(self.shader_program);
                gl::Uniform1f(
                    gl::GetUniformLocation(self.shader_program, cstr!("uTexRatio")),
                    self.tex_ratio,
                );

                label_gl_object!(gl::PROGRAM, self.shader_program, "Shader_InstancedQuad");

                info!("✅ Instanced textured quad shaders reloaded successfully");
                Ok(())
            }
            Err(e) => {
                error!(
                    "❌ Failed to reload instanced textured quad shaders:\n{}",
                    e
                );
                Err(e)
            }
        }
    }
    unsafe fn setup_gpu_buffers(
        max_particles_on_gpu: usize,
    ) -> (u32, u32, u32, *mut ParticleGPU, isize) {
        let (mut vao, mut vbo_quad, mut vbo_particles) = (0u32, 0u32, 0u32);

        // === VAO ===
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

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
        ParticleGPU::setup_vertex_attribs_for_instanced_quad();
        // === Nettoyage ===
        gl::BindVertexArray(0);

        label_gl_object!(gl::VERTEX_ARRAY, vao, "VAO_Instanced_Quads");
        label_gl_object!(gl::BUFFER, vbo_quad, "VBO_Static_Quad");
        label_gl_object!(gl::BUFFER, vbo_particles, "VBO_Instanced_Data");

        (vao, vbo_quad, vbo_particles, mapped_ptr, buffer_size)
    }
}
use crate::renderer_engine::particle_renderer::ParticleGraphicsRenderer;

impl ParticleGraphicsRenderer for RendererGraphicsInstanced {
    unsafe fn recreate_buffers(&mut self, new_max: usize) {
        self.recreate_buffers(new_max);
    }

    unsafe fn fill_particle_data_direct(&mut self, physic: &dyn PhysicEngineIterator) -> usize {
        self.fill_particle_data_direct(physic)
    }

    unsafe fn render_particles_with_persistent_buffer(
        &self,
        count: usize,
        window_size: (f32, f32),
    ) {
        self.render_particles_with_persistent_buffer(count, window_size);
    }

    unsafe fn reload_shaders(&mut self) -> Result<(), String> {
        self.reload_shaders()
    }

    unsafe fn close(&mut self) {
        self.close();
    }
}

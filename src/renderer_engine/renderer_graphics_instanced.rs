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
    vaos: [u32; 3],
    vbo_particles: u32,
    vbo_quad: u32,

    mapped_ptr: *mut ParticleGPU,

    shader_program: u32,
    // Shader
    loc_tex: i32,
    texture_id: u32,
    tex_ratio: f32, // Stocke le ratio de texture pour le reload

    max_particles_on_gpu: usize,

    // Configuration du type de particule
    particle_type: ParticleType,

    // Triple buffering
    current_frame: usize,
    fences: [Option<gl::types::GLsync>; 3],
}

impl RendererGraphicsInstanced {
    pub fn new(
        max_particles_on_gpu: usize,
        particle_type: ParticleType,
        texture_path: &str,
    ) -> Self {
        let shader_program =
            unsafe { compile_shader_program_from_files(VERTEX_SHADER_PATH, FRAGMENT_SHADER_PATH) };

        let loc_tex = unsafe { gl::GetUniformLocation(shader_program, cstr!("uTexture")) };

        let (texture_id, tex_width, tex_height) = load_texture(texture_path);
        unsafe {
            gl::UseProgram(shader_program);

            // Lier le block uniform "GlobalData" au binding point 0
            let block_idx = gl::GetUniformBlockIndex(shader_program, cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(shader_program, block_idx, 0);
            }

            gl::Uniform1i(loc_tex, 0);

            label_gl_object!(gl::PROGRAM, shader_program, "Shader_InstancedQuad");
            label_gl_object!(gl::TEXTURE, texture_id, "Tex_Rocket_Sprite");
        }

        // VAO/VBO setup
        unsafe {
            let (vaos, vbo_quad, vbo_particles, mapped_ptr, _buffer_size) =
                RendererGraphicsInstanced::setup_gpu_buffers(max_particles_on_gpu);

            Self {
                vaos,
                vbo_particles,
                vbo_quad,
                mapped_ptr,
                shader_program,
                loc_tex,
                texture_id,
                tex_ratio: tex_width as f32 / tex_height as f32,
                max_particles_on_gpu,
                particle_type,
                current_frame: 0,
                fences: [None, None, None],
            }
        }
    }
    /// Libère le mapping persistant du VBO particules, puis supprime les VAOs et les VBOs
    /// particules + quad.
    ///
    /// Cette fonction centralise la séquence critique :
    ///   1. `UnmapBuffer` sur `vbo_particles` (persistently mapped — obligatoire avant delete)
    ///   2. `DeleteVertexArrays` (3 VAOs triple-buffer)
    ///   3. `DeleteBuffers` pour `vbo_particles` et `vbo_quad`
    ///
    /// Note : `vbo_quad` est un buffer statique non-mappé ; il ne nécessite pas d'`UnmapBuffer`.
    ///
    /// Elle est appelée par [`recreate_buffers`] ET [`close`] pour garantir que les deux
    /// chemins appliquent exactement le même protocole — évitant toute divergence future.
    ///
    /// # Safety
    /// Le contexte OpenGL doit être valide et actif.
    unsafe fn release_particle_buffers(&mut self) {
        // Unmap the persistent buffer BEFORE deleting it (required by OpenGL spec /
        // ARB_buffer_storage): deleting a mapped buffer is undefined behavior.
        // Note: vbo_quad is a static (non-mapped) buffer; no unmap needed for it.
        if !self.mapped_ptr.is_null() && self.vbo_particles != 0 {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles);
            gl::UnmapBuffer(gl::ARRAY_BUFFER);
            self.mapped_ptr = std::ptr::null_mut();
        }
        if self.vaos[0] != 0 {
            gl::DeleteVertexArrays(3, self.vaos.as_ptr());
            self.vaos = [0; 3];
        }
        if self.vbo_particles != 0 {
            gl::DeleteBuffers(1, &self.vbo_particles);
            self.vbo_particles = 0;
        }
        if self.vbo_quad != 0 {
            gl::DeleteBuffers(1, &self.vbo_quad);
            self.vbo_quad = 0;
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
        let (vaos, vbo_quad, vbo_particles, mapped_ptr, _buffer_size) =
            RendererGraphicsInstanced::setup_gpu_buffers(new_max);

        // 4. Mettre à jour les champs
        self.vaos = vaos;
        self.vbo_particles = vbo_particles;
        self.vbo_quad = vbo_quad;
        self.mapped_ptr = mapped_ptr;
        self.max_particles_on_gpu = new_max;
    }

    /// Retourne `true` si le buffer de particules est actuellement mappé en mémoire CPU.
    /// Utilisé principalement par les tests pour vérifier les invariants de lifecycle GPU.
    pub fn is_mapped(&self) -> bool {
        !self.mapped_ptr.is_null()
    }

    /// Retourne la capacité maximale actuelle du buffer GPU (nombre de particules).
    /// Utilisé principalement par les tests pour vérifier la bonne mise à jour après `recreate_buffers`.
    pub fn max_particles(&self) -> usize {
        self.max_particles_on_gpu
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

        // Utilise for_each_particle_of_type pour filtrer les particules du bon type
        physic.for_each_particle_of_type(self.particle_type, &mut |p| {
            if count < self.max_particles_on_gpu {
                // ⏱️ Piste 3 : Fast Cast-Copy (Layout parfait)
                let src_ptr =
                    p as *const crate::physic_engine::particle::Particle as *const ParticleGPU;
                let mut gpu_p = *src_ptr;
                gpu_p.brightness = 0.0; // Bloom disabled for rockets

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
        active_texture: &mut u32,
    ) {
        // Si aucune particule, on ne fait rien
        if count == 0 {
            return;
        }

        push_debug_group!(30, "Draw Instanced Quads");

        // Active le shader de rendu des particules (seulement s'il n'est pas déjà actif)
        if *active_shader != self.shader_program {
            gl::UseProgram(self.shader_program);
            *active_shader = self.shader_program;
        }

        // Lie le VAO correspondant à la frame courante (tous les attributs et offsets y sont pré-configurés !)
        gl::BindVertexArray(self.vaos[self.current_frame]);

        // Active la texture (seulement si elle n'est pas déjà active)
        if *active_texture != self.texture_id {
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
            *active_texture = self.texture_id;
        }

        // Dessiner le quad
        gl::DrawArraysInstanced(gl::TRIANGLE_STRIP, 0, 4, count as i32);

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
                self.loc_tex = gl::GetUniformLocation(self.shader_program, cstr!("uTexture"));

                gl::UseProgram(self.shader_program);
                // Lier le block uniform "GlobalData" au binding point 0
                let block_idx = gl::GetUniformBlockIndex(self.shader_program, cstr!("GlobalData"));
                if block_idx != gl::INVALID_INDEX {
                    gl::UniformBlockBinding(self.shader_program, block_idx, 0);
                }

                gl::Uniform1i(self.loc_tex, 0);

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
    ) -> ([u32; 3], u32, u32, *mut ParticleGPU, isize) {
        let mut vaos = [0u32; 3];
        let (mut vbo_quad, mut vbo_particles) = (0u32, 0u32);

        // === VBOs ===
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

        gl::GenBuffers(1, &mut vbo_particles);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo_particles);

        let buffer_size = (3 * max_particles_on_gpu * std::mem::size_of::<ParticleGPU>()) as isize;
        info!(
            "🎮 Allocating instanced particle buffer: 3x {} particles → {}",
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

        // === VAO Setup for each frame in the triple buffer ===
        gl::GenVertexArrays(3, vaos.as_mut_ptr());
        for (frame, &vao) in vaos.iter().enumerate() {
            gl::BindVertexArray(vao);

            // 1️⃣ QuadVertexAttribPointer unité statique
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad);
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

            // 2️⃣ Particules instanciées (avec offset correspondant à la section triple-buffering)
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_particles);
            let base_offset =
                (frame * max_particles_on_gpu * std::mem::size_of::<ParticleGPU>()) as isize;
            let stride = std::mem::size_of::<ParticleGPU>() as i32;

            // Attrib 1: position (pos_x, pos_y)
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + memoffset::offset_of!(ParticleGPU, pos_x) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribDivisor(1, 1);

            // Attrib 2: color (col_r, col_g, col_b)
            gl::VertexAttribPointer(
                2,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + memoffset::offset_of!(ParticleGPU, col_r) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribDivisor(2, 1);

            // Attrib 3: lifeData
            gl::VertexAttribPointer(
                3,
                4,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + memoffset::offset_of!(ParticleGPU, life) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(3);
            gl::VertexAttribDivisor(3, 1);

            // Attrib 4: brightness
            gl::VertexAttribPointer(
                4,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + memoffset::offset_of!(ParticleGPU, brightness) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(4);
            gl::VertexAttribDivisor(4, 1);
        }

        gl::BindVertexArray(0);

        for (frame, &vao) in vaos.iter().enumerate() {
            label_gl_object!(
                gl::VERTEX_ARRAY,
                vao,
                &format!("VAO_Instanced_Quads_Frame_{}", frame)
            );
        }
        label_gl_object!(gl::BUFFER, vbo_quad, "VBO_Static_Quad");
        label_gl_object!(gl::BUFFER, vbo_particles, "VBO_Instanced_Data");

        (vaos, vbo_quad, vbo_particles, mapped_ptr, buffer_size)
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
        self.texture_id
    }

    fn get_tex_ratio(&self) -> f32 {
        self.tex_ratio
    }

    unsafe fn reload_shaders(&mut self) -> Result<(), String> {
        self.reload_shaders()
    }

    unsafe fn close(&mut self) {
        self.close();
    }
}

impl Drop for RendererGraphicsInstanced {
    fn drop(&mut self) {
        unsafe {
            self.close();
        }
    }
}

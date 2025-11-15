use log::info;

use crate::cstr;
use crate::physic_engine::PhysicEngine;
use crate::renderer_engine::{
    tools::compile_shader_program, types::ParticleGPU, utils::texture::load_texture,
};
use crate::utils::human_bytes::HumanBytes;

pub struct RendererGraphicsInstanced {
    pub vao: u32,
    pub vbo_particles: u32,
    pub vbo_quad: u32,

    pub mapped_ptr: *mut ParticleGPU,

    pub shader_program: u32,
    // Shader
    pub loc_size: i32,
    pub loc_tex: i32,
    pub texture_id: u32,
    pub tex_width: u32,
    pub tex_height: u32,

    pub max_particles_on_gpu: usize,
}

impl RendererGraphicsInstanced {
    pub fn new(max_particles_on_gpu: usize) -> Self {
        let (vertex_src, fragment_src) = RendererGraphicsInstanced::src_shaders_instanced_quads();
        let shader_program = unsafe { compile_shader_program(vertex_src, fragment_src) };

        let loc_size = unsafe { gl::GetUniformLocation(shader_program, cstr!("uSize")) };
        let loc_tex = unsafe { gl::GetUniformLocation(shader_program, cstr!("uTexture")) };
        // let texture_id = load_texture("assets/textures/toppng.com-realistic-smoke-texture-with-soft-particle-edges-png-399x385.png");
        let (texture_id, tex_width, tex_height) =
            load_texture("assets/textures/04ddeae2-7367-45f1-87e0-361d1d242630_scaled.png");
        unsafe {
            gl::UseProgram(shader_program);
            gl::Uniform1f(
                gl::GetUniformLocation(shader_program, cstr!("uTexRatio")),
                tex_width as f32 / tex_height as f32,
            );
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
                tex_width,
                tex_height,
                max_particles_on_gpu,
            }
        }
    }

    fn src_shaders_instanced_quads() -> (&'static str, &'static str) {
        let vertex_src = r#"
        #version 330 core

        // === Quad unité (4 sommets pour TRIANGLE_STRIP)
        layout(location = 0) in vec2 aQuad;

        // === Attributs instanciés (1 par particule)
        layout(location = 1) in vec2 aPos;
        layout(location = 2) in vec3 aColor;
        layout(location = 3) in vec4 aLifeMaxLifeSizeAngle;

        out vec3 vColor;
        out float vAlpha;
        out vec2 vUV;

        uniform vec2 uSize;
        uniform float uTexRatio;

        mat3 build_world_matrix(float size, float angle) {
            // Position du sommet quad dans l'espace clip (avec taille)
            float scale = size * (2.0 + 5.0 * vAlpha);
            
            float sx = scale * uTexRatio;
            float sy = scale * 1.0;            

            mat3 mat_scale = mat3(
                sx, 0.0, 0.0,
                0.0, sy, 0.0,
                0.0, 0.0, 1.0
            );

            float s = sin(angle);
            float c = cos(angle);
            mat3 mat_rotation = mat3(
                c, -s, 0.0,
                s,  c, 0.0,
                0.0, 0.0, 1.0
            );
            
            mat3 mat_translation = mat3(
                1.0, 0.0, 0.0,
                0.0, 1.0, 0.0,
                aPos.x, aPos.y, 1.0
            );

            return mat_translation * mat_rotation * mat_scale;
        }

        void main() {
            float life = aLifeMaxLifeSizeAngle.x;
            float max_life = aLifeMaxLifeSizeAngle.y;
            float size = aLifeMaxLifeSizeAngle.z;
            float angle = aLifeMaxLifeSizeAngle.w;

            // Ratio de vie (comme avant)
            vAlpha = clamp(life / max(max_life, 0.0001), 0.0, 1.0);
            vColor = aColor;

            // On reconstruit les coordonnées UV du quad (-1.0 → -1.0) -> (0.0, 0.0)
            vUV = aQuad * 0.5 + 0.5;            
        
            mat3 mat_model = build_world_matrix(size, angle);
            vec2 world_pos = (mat_model * vec3(aQuad, 1.0)).xy;

            // Clip space
            float x = world_pos.x / uSize.x * 2.0 - 1.0;
            float y = world_pos.y / uSize.y * 2.0 - 1.0;
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

        void main() {
            if (vAlpha <= 0.0) discard;
            FragColor = vec4(vColor, vAlpha) * texture(uTexture, vUV);
        }
        "#;
        (vertex_src, fragment_src)
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

        (vao, vbo_quad, vbo_particles, mapped_ptr, buffer_size)
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
    pub unsafe fn fill_particle_data_direct<P: PhysicEngine>(&mut self, physic: &P) -> usize {
        let mut count = 0;

        // Slice Rust mutable mappé directement sur la mémoire GPU.
        // Toute écriture dans ce slice écrit physiquement dans la BAR / VRAM.
        let gpu_slice = std::slice::from_raw_parts_mut(self.mapped_ptr, self.max_particles_on_gpu);

        // Ici, `iter_active_heads()` fournit un flux paresseux, sans allocation CPU
        // intermédiaire : idéal pour écrire contigu dans le buffer GPU.
        for (i, p) in physic
            .iter_active_heads()
            .take(self.max_particles_on_gpu)
            .enumerate()
        {
            gpu_slice[i] = ParticleGPU {
                pos_x: p.pos.x,
                pos_y: p.pos.y,
                col_r: p.color.x,
                col_g: p.color.y,
                col_b: p.color.z,
                life: p.life,
                max_life: p.max_life,
                size: p.size,
                angle: p.angle,
            };
            count += 1;
        }

        // Flush explicite de la zone écrite.
        // (Si MAP_COHERENT_BIT est utilisé : cette étape peut être omise.)
        // let written_bytes = (count * std::mem::size_of::<ParticleGPU>()) as isize;
        // gl::FlushMappedBufferRange(gl::ARRAY_BUFFER, 0, written_bytes);

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
    }

    /// Libère les ressources GPU associées à ce RendererGraphics.
    ///
    /// # Safety
    /// Cette fonction est unsafe car elle manipule directement des ressources OpenGL.
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn close(&mut self) {
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
        if self.shader_program != 0 {
            gl::DeleteProgram(self.shader_program);
            self.shader_program = 0;
        }
    }
}

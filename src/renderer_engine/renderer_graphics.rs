use log::info;

use crate::physic_engine::PhysicEngine;
use crate::renderer_engine::{tools::compile_shader_program, types::ParticleGPU};
use crate::utils::human_bytes::HumanBytes;

macro_rules! cstr {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const i8
    };
}

pub struct RendererGraphics {
    pub vao: u32,
    pub vbo_particles: u32,

    pub mapped_ptr: *mut ParticleGPU,

    // Shader
    pub shader_program: u32,
    pub loc_size: i32,

    pub max_particles_on_gpu: usize,
}

impl RendererGraphics {
    pub fn new(max_particles_on_gpu: usize) -> Self {
        let (vertex_src, fragment_src) = RendererGraphics::src_shaders_particles();
        let shader_program = unsafe { compile_shader_program(vertex_src, fragment_src) };

        let loc_size = unsafe { gl::GetUniformLocation(shader_program, cstr!("uSize")) };

        // VAO/VBO setup
        unsafe {
            let (vao, vbo_particles, mapped_ptr, _buffer_size) =
                RendererGraphics::setup_gpu_buffers(max_particles_on_gpu);

            Self {
                vao,
                vbo_particles,
                mapped_ptr,
                shader_program,
                loc_size,
                max_particles_on_gpu,
            }
        }
    }

    pub fn src_shaders_particles() -> (&'static str, &'static str) {
        let vertex_src = r#"
        #version 330 core
        layout(location = 0) in vec4 aPos;
        layout(location = 1) in vec3 aColor;
        layout(location = 2) in vec2 aLifeMaxLife;

        out vec3 vertexColor;
        out float alpha;

        uniform vec2 uSize;

        void main() {
            float a = clamp(aLifeMaxLife.x / max(aLifeMaxLife.y, 0.0001), 0.0, 1.0);
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
        ParticleGPU::setup_vertex_attribs();
        // === Nettoyage ===
        gl::BindVertexArray(0);

        (vao, vbo_particles, mapped_ptr, buffer_size)
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

        // 2. Recréer avec la nouvelle taille
        let (vao, vbo_particles, mapped_ptr, _buffer_size) =
            RendererGraphics::setup_gpu_buffers(new_max);

        // 3. Mettre à jour les champs
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
    pub unsafe fn fill_particle_data_direct<P: PhysicEngine>(&mut self, physic: &P) -> usize {
        let mut count = 0;

        // Slice Rust mutable mappé directement sur la mémoire GPU.
        // Toute écriture dans ce slice écrit physiquement dans la BAR / VRAM.
        let gpu_slice = std::slice::from_raw_parts_mut(self.mapped_ptr, self.max_particles_on_gpu);

        // Ici, `iter_active_heads()` fournit un flux paresseux, sans allocation CPU
        // intermédiaire : idéal pour écrire contigu dans le buffer GPU.
        for (i, p) in physic
            .iter_active_particles()
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

        gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles);
        // Dessine les particules sous forme de points
        gl::DrawArrays(gl::POINTS, 0, count as i32);
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

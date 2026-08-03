use log::{debug, info};
use memoffset::offset_of;
use std::mem;

use crate::cstr;
use crate::physic_engine::PhysicEngineIterator;
use crate::renderer_engine::particle_renderer::ParticleGraphicsRenderer;
use crate::renderer_engine::shader::compile_shader_program_from_files;
use crate::renderer_engine::utils::texture::load_texture;
use crate::utils::human_bytes::HumanBytes;
use crate::{label_gl_object, pop_debug_group, push_debug_group};

use crate::renderer_engine::constants;

const VERTEX_SHADER_PATH: &str = constants::SHADER_SMOKE_VERTEX_PATH;
const FRAGMENT_SHADER_PATH: &str = constants::SHADER_SMOKE_FRAGMENT_PATH;

/// GPU instance data structure for instanced smoke rendering.
/// Pass layout: position (vec3), scale (float), alpha (float), rotation (float), intensity (float), color (vec3), normalized_age (float).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SmokeInstanceGPU {
    pub position: [f32; 3],  // location 1: vec3
    pub scale: f32,          // location 2: float
    pub alpha: f32,          // location 3: float
    pub rotation: f32,       // location 4: float
    pub intensity: f32,      // location 5: float
    pub color: [f32; 3],     // location 6: vec3
    pub normalized_age: f32, // location 7: float
}

unsafe impl bytemuck::Pod for SmokeInstanceGPU {}
unsafe impl bytemuck::Zeroable for SmokeInstanceGPU {}

pub struct SmokeRenderer {
    vaos: [u32; 3],
    vbo_particles: u32,
    vbo_quad: u32,

    mapped_ptr: *mut SmokeInstanceGPU,

    shader_program: u32,
    loc_smoke_tex: i32,
    loc_flow_map: i32,
    loc_noise_tex: i32,
    loc_flow_distortion_strength: i32,
    loc_flow_animation_speed: i32,
    loc_erosion_enabled: i32,
    loc_erosion_scale: i32,
    loc_erosion_edge_width: i32,
    loc_erosion_edge_color: i32,
    flow_distortion_strength: f32,
    flow_animation_speed: f32,
    erosion_enabled: bool,
    erosion_scale: f32,
    erosion_edge_width: f32,
    erosion_edge_color: [f32; 3],
    texture_id: u32,
    flow_map_texture_id: u32,
    noise_texture_id: u32,
    tex_ratio: f32,

    max_smoke_particles: usize,

    // Triple buffering
    current_frame: usize,
    fences: [Option<gl::types::GLsync>; 3],
}

impl SmokeRenderer {
    pub fn new(max_smoke_particles: usize, texture_path: &str) -> Self {
        const NOISE_TEXTURE_PATH: &str = constants::TEXTURE_NOISE_PATH;
        const FLOW_MAP_TEXTURE_PATH: &str = constants::TEXTURE_FLOW_MAP_PATH;

        let shader_program =
            unsafe { compile_shader_program_from_files(VERTEX_SHADER_PATH, FRAGMENT_SHADER_PATH) };

        let loc_smoke_tex =
            unsafe { gl::GetUniformLocation(shader_program, cstr!("u_SmokeTexture")) };
        let loc_flow_map = unsafe { gl::GetUniformLocation(shader_program, cstr!("u_FlowMap")) };
        let loc_noise_tex =
            unsafe { gl::GetUniformLocation(shader_program, cstr!("u_NoiseTexture")) };
        let loc_flow_distortion_strength =
            unsafe { gl::GetUniformLocation(shader_program, cstr!("u_FlowDistortionStrength")) };
        let loc_flow_animation_speed =
            unsafe { gl::GetUniformLocation(shader_program, cstr!("u_FlowAnimationSpeed")) };
        let loc_erosion_enabled =
            unsafe { gl::GetUniformLocation(shader_program, cstr!("u_ErosionEnabled")) };
        let loc_erosion_scale =
            unsafe { gl::GetUniformLocation(shader_program, cstr!("u_ErosionScale")) };
        let loc_erosion_edge_width =
            unsafe { gl::GetUniformLocation(shader_program, cstr!("u_ErosionEdgeWidth")) };
        let loc_erosion_edge_color =
            unsafe { gl::GetUniformLocation(shader_program, cstr!("u_ErosionEdgeColor")) };

        let (texture_id, tex_width, tex_height) = load_texture(texture_path);
        let (flow_map_texture_id, _, _) = load_texture(FLOW_MAP_TEXTURE_PATH);
        let (noise_texture_id, _, _) = load_texture(NOISE_TEXTURE_PATH);

        unsafe {
            gl::UseProgram(shader_program);

            let block_idx = gl::GetUniformBlockIndex(shader_program, cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(shader_program, block_idx, 0);
            }

            if loc_smoke_tex != -1 {
                gl::Uniform1i(loc_smoke_tex, 0);
            }
            if loc_flow_map != -1 {
                gl::Uniform1i(loc_flow_map, 1);
            }
            if loc_noise_tex != -1 {
                gl::Uniform1i(loc_noise_tex, 2);
            }

            label_gl_object!(gl::PROGRAM, shader_program, "Shader_SmokeInstanced");
            label_gl_object!(gl::TEXTURE, texture_id, "Tex_Smoke_Sprite");
            label_gl_object!(gl::TEXTURE, flow_map_texture_id, "Tex_Smoke_FlowMap");
            label_gl_object!(gl::TEXTURE, noise_texture_id, "Tex_Noise_Dissolve");
        }

        unsafe {
            let (vaos, vbo_quad, vbo_particles, mapped_ptr, _buffer_size) =
                Self::setup_gpu_buffers(max_smoke_particles);

            Self {
                vaos,
                vbo_particles,
                vbo_quad,
                mapped_ptr,
                shader_program,
                loc_smoke_tex,
                loc_flow_map,
                loc_noise_tex,
                loc_flow_distortion_strength,
                loc_flow_animation_speed,
                loc_erosion_enabled,
                loc_erosion_scale,
                loc_erosion_edge_width,
                loc_erosion_edge_color,
                flow_distortion_strength:
                    crate::physic_engine::constants::DEFAULT_FLOW_DISTORTION_STRENGTH,
                flow_animation_speed: crate::physic_engine::constants::DEFAULT_FLOW_ANIMATION_SPEED,
                erosion_enabled: crate::physic_engine::constants::DEFAULT_SMOKE_EROSION_ENABLED,
                erosion_scale: crate::physic_engine::constants::DEFAULT_SMOKE_EROSION_SCALE,
                erosion_edge_width:
                    crate::physic_engine::constants::DEFAULT_SMOKE_EROSION_EDGE_WIDTH,
                erosion_edge_color:
                    crate::physic_engine::constants::DEFAULT_SMOKE_EROSION_EDGE_COLOR,
                texture_id,
                flow_map_texture_id,
                noise_texture_id,
                tex_ratio: tex_width as f32 / tex_height as f32,
                max_smoke_particles,
                current_frame: 0,
                fences: [None, None, None],
            }
        }
    }

    unsafe fn release_buffers(&mut self) {
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
    ///
    /// # Safety
    /// L'appelant doit s'assurer que le contexte OpenGL est valide et actif.
    pub unsafe fn recreate_buffers(&mut self, new_max: usize) {
        for fence in self.fences.iter_mut() {
            if let Some(sync) = fence.take() {
                gl::DeleteSync(sync);
            }
        }
        self.current_frame = 0;

        self.release_buffers();

        let (vaos, vbo_quad, vbo_particles, mapped_ptr, _buffer_size) =
            Self::setup_gpu_buffers(new_max);

        self.vaos = vaos;
        self.vbo_particles = vbo_particles;
        self.vbo_quad = vbo_quad;
        self.mapped_ptr = mapped_ptr;
        self.max_smoke_particles = new_max;
    }

    /// Remplit directement le buffer GPU persistent avec les données de fumée.
    ///
    /// # Safety
    /// L'appelant doit s'assurer que le contexte OpenGL est valide et que le buffer GPU est mappé.
    pub unsafe fn fill_particle_data_direct(&mut self, physic: &dyn PhysicEngineIterator) -> usize {
        if let Some(sync) = self.fences[self.current_frame] {
            gl::ClientWaitSync(sync, gl::SYNC_FLUSH_COMMANDS_BIT, 10_000_000_000);
            gl::DeleteSync(sync);
            self.fences[self.current_frame] = None;
        }

        let mut count = 0;
        let offset = self.current_frame * self.max_smoke_particles;
        let gpu_slice =
            std::slice::from_raw_parts_mut(self.mapped_ptr.add(offset), self.max_smoke_particles);

        let intensity = physic.get_smoke_intensity();
        let (enabled, scale, edge_w, edge_c) = physic.get_smoke_erosion_params();
        let (flow_strength, flow_speed) = physic.get_smoke_flow_params();
        self.erosion_enabled = enabled;
        self.erosion_scale = scale;
        self.erosion_edge_width = edge_w;
        self.erosion_edge_color = edge_c;
        self.flow_distortion_strength = flow_strength;
        self.flow_animation_speed = flow_speed;

        crate::tracy_zone!("SmokeRenderer::fill_particle_data_direct", 0x888888);

        physic.for_each_smoke_particle(&mut |sp| {
            if count < self.max_smoke_particles {
                gpu_slice[count] = SmokeInstanceGPU {
                    position: [sp.pos.x, sp.pos.y, 0.0],
                    scale: sp.sizing.current_size,
                    alpha: sp.opacity.alpha,
                    rotation: sp.rotation,
                    intensity,
                    color: [sp.color.x, sp.color.y, sp.color.z],
                    normalized_age: sp.lifecycle.progress(),
                };
                count += 1;
            }
        });

        if count > 0 {
            let write_size = (count * mem::size_of::<SmokeInstanceGPU>()) as isize;
            let offset_bytes = (offset * mem::size_of::<SmokeInstanceGPU>()) as isize;
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles);
            gl::FlushMappedBufferRange(gl::ARRAY_BUFFER, offset_bytes, write_size);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        count
    }

    /// Dessine les instances de fumée avec blend alpha et glDepthMask(GL_FALSE).
    ///
    /// # Safety
    /// L'appelant doit s'assurer que le contexte OpenGL est valide et que les ressources GPU sont valides.
    pub unsafe fn render_smoke_with_persistent_buffer(
        &mut self,
        count: usize,
        active_shader: &mut u32,
        active_texture: &mut u32,
    ) {
        if count == 0 {
            return;
        }

        crate::tracy_zone!("SmokeRenderer::render_smoke_instanced", 0x888888);

        push_debug_group!(31, "Draw Instanced Smoke");

        // 1. Enable Alpha Blending for soft particle dissipation
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        // 2. Disable Depth Writing to prevent Z-fighting and quad intersection artifacts
        gl::DepthMask(gl::FALSE);

        // 3. Only write to Color Attachment 0 (Scene), disable writes to Attachment 1 (Bloom)
        gl::ColorMaski(0, gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::ColorMaski(1, gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);

        if *active_shader != self.shader_program {
            gl::UseProgram(self.shader_program);
            *active_shader = self.shader_program;
        }

        gl::BindVertexArray(self.vaos[self.current_frame]);

        // Bind Texture Unit 0: Smoke Texture
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.texture_id);
        *active_texture = self.texture_id;

        // Bind Texture Unit 1: Flow Map Texture
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, self.flow_map_texture_id);

        // Bind Texture Unit 2: Noise Dissolve Texture
        gl::ActiveTexture(gl::TEXTURE2);
        gl::BindTexture(gl::TEXTURE_2D, self.noise_texture_id);

        if self.loc_flow_distortion_strength != -1 {
            gl::Uniform1f(
                self.loc_flow_distortion_strength,
                self.flow_distortion_strength,
            );
        }
        if self.loc_flow_animation_speed != -1 {
            gl::Uniform1f(self.loc_flow_animation_speed, self.flow_animation_speed);
        }

        if self.loc_erosion_enabled != -1 {
            gl::Uniform1i(
                self.loc_erosion_enabled,
                if self.erosion_enabled { 1 } else { 0 },
            );
        }
        if self.loc_erosion_scale != -1 {
            gl::Uniform1f(self.loc_erosion_scale, self.erosion_scale);
        }
        if self.loc_erosion_edge_width != -1 {
            gl::Uniform1f(self.loc_erosion_edge_width, self.erosion_edge_width);
        }
        if self.loc_erosion_edge_color != -1 {
            gl::Uniform3f(
                self.loc_erosion_edge_color,
                self.erosion_edge_color[0],
                self.erosion_edge_color[1],
                self.erosion_edge_color[2],
            );
        }

        gl::DrawArraysInstanced(gl::TRIANGLE_FAN, 0, 10, count as i32);

        // Restore depth write and color mask for attachment 1
        gl::DepthMask(gl::TRUE);
        gl::ColorMaski(1, gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);

        pop_debug_group!();

        if let Some(old_sync) = self.fences[self.current_frame] {
            gl::DeleteSync(old_sync);
        }
        let sync = gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0);
        self.fences[self.current_frame] = Some(sync);

        self.current_frame = (self.current_frame + 1) % 3;
    }

    /// Libère les ressources GPU (VAO, VBO, shaders, textures).
    ///
    /// # Safety
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn close(&mut self) {
        for fence in self.fences.iter_mut() {
            if let Some(sync) = fence.take() {
                gl::DeleteSync(sync);
            }
        }
        self.release_buffers();
        if self.texture_id != 0 {
            gl::DeleteTextures(1, &self.texture_id);
            self.texture_id = 0;
        }
        if self.flow_map_texture_id != 0 {
            gl::DeleteTextures(1, &self.flow_map_texture_id);
            self.flow_map_texture_id = 0;
        }
        if self.noise_texture_id != 0 {
            gl::DeleteTextures(1, &self.noise_texture_id);
            self.noise_texture_id = 0;
        }
        if self.shader_program != 0 {
            gl::DeleteProgram(self.shader_program);
            self.shader_program = 0;
        }
        debug!("SmokeRenderer GPU resources released.");
    }

    /// Recompile les shaders de fumée à chaud.
    ///
    /// # Safety
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn reload_shaders(&mut self) -> Result<(), String> {
        use crate::renderer_engine::shader::try_compile_shader_program_from_files;
        use log::error;

        match try_compile_shader_program_from_files(VERTEX_SHADER_PATH, FRAGMENT_SHADER_PATH) {
            Ok(new_program) => {
                if self.shader_program != 0 {
                    gl::DeleteProgram(self.shader_program);
                }
                self.shader_program = new_program;
                self.loc_smoke_tex =
                    gl::GetUniformLocation(self.shader_program, cstr!("u_SmokeTexture"));
                self.loc_flow_map = gl::GetUniformLocation(self.shader_program, cstr!("u_FlowMap"));
                self.loc_noise_tex =
                    gl::GetUniformLocation(self.shader_program, cstr!("u_NoiseTexture"));
                self.loc_flow_distortion_strength =
                    gl::GetUniformLocation(self.shader_program, cstr!("u_FlowDistortionStrength"));
                self.loc_flow_animation_speed =
                    gl::GetUniformLocation(self.shader_program, cstr!("u_FlowAnimationSpeed"));
                self.loc_erosion_enabled =
                    gl::GetUniformLocation(self.shader_program, cstr!("u_ErosionEnabled"));
                self.loc_erosion_scale =
                    gl::GetUniformLocation(self.shader_program, cstr!("u_ErosionScale"));
                self.loc_erosion_edge_width =
                    gl::GetUniformLocation(self.shader_program, cstr!("u_ErosionEdgeWidth"));
                self.loc_erosion_edge_color =
                    gl::GetUniformLocation(self.shader_program, cstr!("u_ErosionEdgeColor"));

                gl::UseProgram(self.shader_program);
                let block_idx = gl::GetUniformBlockIndex(self.shader_program, cstr!("GlobalData"));
                if block_idx != gl::INVALID_INDEX {
                    gl::UniformBlockBinding(self.shader_program, block_idx, 0);
                }
                if self.loc_smoke_tex != -1 {
                    gl::Uniform1i(self.loc_smoke_tex, 0);
                }
                if self.loc_flow_map != -1 {
                    gl::Uniform1i(self.loc_flow_map, 1);
                }
                if self.loc_noise_tex != -1 {
                    gl::Uniform1i(self.loc_noise_tex, 2);
                }

                label_gl_object!(gl::PROGRAM, self.shader_program, "Shader_SmokeInstanced");
                info!("✅ Smoke instanced shaders reloaded successfully");
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to reload smoke instanced shaders:\n{}", e);
                Err(e)
            }
        }
    }

    unsafe fn setup_gpu_buffers(
        max_smoke_particles: usize,
    ) -> ([u32; 3], u32, u32, *mut SmokeInstanceGPU, isize) {
        let mut vaos = [0u32; 3];
        let (mut vbo_quad, mut vbo_particles) = (0u32, 0u32);

        const OCTAGON_VERTICES: [f32; 20] = [
            0.0, 0.0, // Center vertex for TRIANGLE_FAN
            1.082392, 0.0, 0.765366, 0.765366, 0.0, 1.082392, -0.765366, 0.765366, -1.082392, 0.0,
            -0.765366, -0.765366, 0.0, -1.082392, 0.765366, -0.765366, 1.082392,
            0.0, // Close fan
        ];

        gl::GenBuffers(1, &mut vbo_quad);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (OCTAGON_VERTICES.len() * mem::size_of::<f32>()) as isize,
            OCTAGON_VERTICES.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        gl::GenBuffers(1, &mut vbo_particles);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo_particles);

        let buffer_size = (3 * max_smoke_particles * mem::size_of::<SmokeInstanceGPU>()) as isize;
        info!(
            "💨 Allocating smoke instance buffer: 3x {} particles -> {}",
            max_smoke_particles,
            buffer_size.human_bytes()
        );

        gl::BufferStorage(
            gl::ARRAY_BUFFER,
            buffer_size,
            std::ptr::null(),
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT,
        );

        let mapped_ptr = gl::MapBufferRange(
            gl::ARRAY_BUFFER,
            0,
            buffer_size,
            gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_FLUSH_EXPLICIT_BIT,
        ) as *mut SmokeInstanceGPU;

        gl::GenVertexArrays(3, vaos.as_mut_ptr());
        for (frame, &vao) in vaos.iter().enumerate() {
            gl::BindVertexArray(vao);

            // Attrib 0: Quad vertices (vec2)
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_quad);
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                (2 * mem::size_of::<f32>()) as i32,
                std::ptr::null(),
            );
            gl::VertexAttribDivisor(0, 0);

            // Instanced attributes
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo_particles);
            let base_offset =
                (frame * max_smoke_particles * mem::size_of::<SmokeInstanceGPU>()) as isize;
            let stride = mem::size_of::<SmokeInstanceGPU>() as i32;

            // Attrib 1: position (vec3)
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + offset_of!(SmokeInstanceGPU, position) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribDivisor(1, 1);

            // Attrib 2: scale (float)
            gl::VertexAttribPointer(
                2,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + offset_of!(SmokeInstanceGPU, scale) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribDivisor(2, 1);

            // Attrib 3: alpha (float)
            gl::VertexAttribPointer(
                3,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + offset_of!(SmokeInstanceGPU, alpha) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(3);
            gl::VertexAttribDivisor(3, 1);

            // Attrib 4: rotation (float)
            gl::VertexAttribPointer(
                4,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + offset_of!(SmokeInstanceGPU, rotation) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(4);
            gl::VertexAttribDivisor(4, 1);

            // Attrib 5: intensity (float)
            gl::VertexAttribPointer(
                5,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + offset_of!(SmokeInstanceGPU, intensity) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(5);
            gl::VertexAttribDivisor(5, 1);

            // Attrib 6: color (vec3)
            gl::VertexAttribPointer(
                6,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + offset_of!(SmokeInstanceGPU, color) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(6);
            gl::VertexAttribDivisor(6, 1);

            // Attrib 7: normalized_age (float)
            gl::VertexAttribPointer(
                7,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (base_offset + offset_of!(SmokeInstanceGPU, normalized_age) as isize) as *const _,
            );
            gl::EnableVertexAttribArray(7);
            gl::VertexAttribDivisor(7, 1);
        }

        gl::BindVertexArray(0);

        for (frame, &vao) in vaos.iter().enumerate() {
            label_gl_object!(gl::VERTEX_ARRAY, vao, &format!("VAO_Smoke_Frame_{}", frame));
        }
        label_gl_object!(gl::BUFFER, vbo_quad, "VBO_Smoke_Static_Quad");
        label_gl_object!(gl::BUFFER, vbo_particles, "VBO_Smoke_Instance_Data");

        (vaos, vbo_quad, vbo_particles, mapped_ptr, buffer_size)
    }
}

impl ParticleGraphicsRenderer for SmokeRenderer {
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
        self.render_smoke_with_persistent_buffer(count, active_shader, active_texture);
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

    fn particle_type(&self) -> Option<crate::physic_engine::ParticleType> {
        Some(crate::physic_engine::ParticleType::Smoke)
    }

    fn render_order(&self) -> u32 {
        10
    }

    unsafe fn reload_shaders(&mut self) -> Result<(), String> {
        self.reload_shaders()
    }

    unsafe fn close(&mut self) {
        self.close();
    }
}

impl Drop for SmokeRenderer {
    fn drop(&mut self) {
        unsafe {
            self.close();
        }
    }
}

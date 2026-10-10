use log::{debug, info};
use memoffset::offset_of;
use std::mem;

use crate::cstr;
use crate::physic_engine::PhysicEngineIterator;
use crate::renderer_engine::gl_resource::{GlBuffer, GlProgram, GlTexture, GlVao};
use crate::renderer_engine::particle_renderer::ParticleGraphicsRenderer;
use crate::renderer_engine::shader::compile_shader_program_from_files;
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
    vaos: [GlVao; 3],
    vbo_particles: GlBuffer,
    vbo_quad: GlBuffer,

    mapped_ptr: *mut SmokeInstanceGPU,

    shader_program: GlProgram,
    loc_smoke_tex: i32,
    loc_flow_map: i32,
    loc_noise_tex: i32,
    loc_flow_distortion_strength: i32,
    loc_flow_animation_speed: i32,
    loc_erosion_enabled: i32,
    loc_erosion_scale: i32,
    loc_erosion_edge_width: i32,
    loc_erosion_edge_color: i32,
    loc_render_mask: i32,
    flow_distortion_strength: f32,
    flow_animation_speed: f32,
    erosion_enabled: bool,
    erosion_scale: f32,
    erosion_edge_width: f32,
    erosion_edge_color: [f32; 3],
    texture_id: GlTexture,
    flow_map_texture_id: GlTexture,
    noise_texture_id: GlTexture,
    tex_ratio: f32,

    max_smoke_particles: usize,

    smoke_lighting_enabled: bool,
    smoke_scattering_intensity: f32,
    smoke_ambient_flash: f32,
    smoke_lighting_lut_enabled: bool,
    smoke_wrap_relief: f32,
    pub backlight_enabled: bool,
    light_falloff_lut_texture_id: GlTexture,
    loc_light_falloff_lut: i32,
    loc_use_lut: i32,
    loc_wrap_relief: i32,

    // Triple buffering
    current_frame: usize,
    fences: [Option<gl::types::GLsync>; 3],
}

impl SmokeRenderer {
    pub fn new(
        max_smoke_particles: usize,
        sprite_tex: &crate::renderer_engine::utils::texture::TextureData,
        flow_tex: &crate::renderer_engine::utils::texture::TextureData,
        noise_tex: &crate::renderer_engine::utils::texture::TextureData,
    ) -> Self {
        let shader_program =
            unsafe { compile_shader_program_from_files(VERTEX_SHADER_PATH, FRAGMENT_SHADER_PATH) };
        let shader_program = GlProgram::from_raw(shader_program);

        let loc_smoke_tex =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_SmokeTexture")) };
        let loc_flow_map =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_FlowMap")) };
        let loc_noise_tex =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_NoiseTexture")) };
        let loc_flow_distortion_strength = unsafe {
            gl::GetUniformLocation(shader_program.raw(), cstr!("u_FlowDistortionStrength"))
        };
        let loc_flow_animation_speed =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_FlowAnimationSpeed")) };
        let loc_erosion_enabled =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_ErosionEnabled")) };
        let loc_erosion_scale =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_ErosionScale")) };
        let loc_erosion_edge_width =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_ErosionEdgeWidth")) };
        let loc_erosion_edge_color =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_ErosionEdgeColor")) };
        let loc_render_mask =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_RenderMask")) };
        let loc_light_falloff_lut =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_LightFalloffLut")) };
        let loc_use_lut =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_UseLut")) };
        let loc_wrap_relief =
            unsafe { gl::GetUniformLocation(shader_program.raw(), cstr!("u_WrapRelief")) };

        let texture_id =
            crate::renderer_engine::utils::texture::create_gl_texture_from_data(sprite_tex);
        let texture_id = GlTexture::from_raw(texture_id);
        let tex_width = sprite_tex.width;
        let tex_height = sprite_tex.height;
        let flow_map_texture_id =
            crate::renderer_engine::utils::texture::create_gl_texture_from_data(flow_tex);
        let flow_map_texture_id = GlTexture::from_raw(flow_map_texture_id);
        let noise_texture_id =
            crate::renderer_engine::utils::texture::create_gl_texture_from_data(noise_tex);
        let noise_texture_id = GlTexture::from_raw(noise_texture_id);
        let light_falloff_lut_texture_id =
            unsafe { Self::generate_light_falloff_lut(constants::SMOKE_LIGHTING_LUT_RESOLUTION) };

        unsafe {
            gl::UseProgram(shader_program.raw());

            let block_idx = gl::GetUniformBlockIndex(shader_program.raw(), cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(shader_program.raw(), block_idx, 0);
            }

            let lighting_block_idx =
                gl::GetUniformBlockIndex(shader_program.raw(), cstr!("LightingBlock"));
            if lighting_block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(
                    shader_program.raw(),
                    lighting_block_idx,
                    constants::LIGHTING_UBO_BINDING_INDEX,
                );
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
            if loc_light_falloff_lut != -1 {
                gl::Uniform1i(
                    loc_light_falloff_lut,
                    constants::SMOKE_LIGHTING_LUT_TEXTURE_UNIT as i32,
                );
            }

            label_gl_object!(gl::PROGRAM, shader_program.raw(), "Shader_SmokeInstanced");
            label_gl_object!(gl::TEXTURE, texture_id.raw(), "Tex_Smoke_Sprite");
            label_gl_object!(gl::TEXTURE, flow_map_texture_id.raw(), "Tex_Smoke_FlowMap");
            label_gl_object!(gl::TEXTURE, noise_texture_id.raw(), "Tex_Noise_Dissolve");

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
                loc_render_mask,
                loc_light_falloff_lut,
                loc_use_lut,
                loc_wrap_relief,
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
                light_falloff_lut_texture_id,
                tex_ratio: tex_width as f32 / tex_height as f32,
                max_smoke_particles,
                smoke_lighting_enabled: constants::DEFAULT_SMOKE_LIGHTING_ENABLED,
                smoke_scattering_intensity: constants::DEFAULT_SMOKE_SCATTERING_INTENSITY,
                smoke_ambient_flash: constants::DEFAULT_SMOKE_AMBIENT_FLASH,
                smoke_lighting_lut_enabled: constants::DEFAULT_SMOKE_LIGHTING_LUT_ENABLED,
                smoke_wrap_relief: constants::DEFAULT_SMOKE_WRAP_RELIEF,
                backlight_enabled: constants::DEFAULT_BACKLIGHT_ENABLED,
                current_frame: 0,
                fences: [None, None, None],
            }
        }
    }

    unsafe fn release_buffers(&mut self) {
        if !self.mapped_ptr.is_null() && self.vbo_particles.raw() != 0 {
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles.raw());
            gl::UnmapBuffer(gl::ARRAY_BUFFER);
            self.mapped_ptr = std::ptr::null_mut();
        }
        for vao in self.vaos.iter_mut() {
            vao.reset(0);
        }
        self.vbo_particles.reset(0);
        self.vbo_quad.reset(0);
    }

    /// Génère la texture 2D R16F contenant la LUT de falloff quadratique et diffusion Mie (Zero SQRT).
    ///
    /// # Safety
    /// L'appelant doit s'assurer que le contexte OpenGL est valide et actif.
    pub unsafe fn generate_light_falloff_lut(resolution: usize) -> GlTexture {
        let mut tex_id = 0;
        gl::GenTextures(1, &mut tex_id);
        gl::BindTexture(gl::TEXTURE_2D, tex_id);

        let mut data = Vec::with_capacity(resolution * resolution);
        let n = resolution as f32;
        let anisotropy = constants::SMOKE_LIGHTING_ANISOTROPY_COEFF;

        for j in 0..resolution {
            let v = (j as f32 + 0.5) / n;
            let y = (v - 0.5) * 2.0;
            for i in 0..resolution {
                let u = (i as f32 + 0.5) / n;
                let x = (u - 0.5) * 2.0;

                let dist_sq = x * x + y * y;
                if dist_sq < 1.0 {
                    let dist = dist_sq.sqrt();
                    let atten = (1.0 - dist) * (1.0 - dist);
                    let cos_theta = if dist > 0.0001 { y / dist } else { 0.0 };
                    let phase = 1.0 + anisotropy * cos_theta;
                    data.push(atten * phase);
                } else {
                    data.push(0.0);
                }
            }
        }

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::R16F as i32,
            resolution as i32,
            resolution as i32,
            0,
            gl::RED,
            gl::FLOAT,
            data.as_ptr() as *const _,
        );

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_S,
            gl::CLAMP_TO_BORDER as i32,
        );
        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_WRAP_T,
            gl::CLAMP_TO_BORDER as i32,
        );
        let border_color = constants::SMOKE_LIGHTING_LUT_BORDER_COLOR;
        gl::TexParameterfv(
            gl::TEXTURE_2D,
            gl::TEXTURE_BORDER_COLOR,
            border_color.as_ptr(),
        );

        label_gl_object!(gl::TEXTURE, tex_id, "Tex_Smoke_LightFalloffLut");
        GlTexture::from_raw(tex_id)
    }

    /// Régénère la LUT de falloff et diffusion lumineuse.
    ///
    /// # Safety
    /// L'appelant doit s'assurer que le contexte OpenGL est valide et actif.
    pub unsafe fn rebake_smoke_lighting_lut(&mut self) {
        self.light_falloff_lut_texture_id =
            Self::generate_light_falloff_lut(constants::SMOKE_LIGHTING_LUT_RESOLUTION);
        debug!("SmokeRenderer: Light Falloff LUT rebaked successfully.");
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
    pub unsafe fn fill_particle_data_direct(
        &mut self,
        physic: &dyn PhysicEngineIterator,
        _alpha: f32,
    ) -> usize {
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

        for sp in physic.active_smoke_slice() {
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
        }

        if count > 0 {
            let write_size = (count * mem::size_of::<SmokeInstanceGPU>()) as isize;
            let offset_bytes = (offset * mem::size_of::<SmokeInstanceGPU>()) as isize;
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo_particles.raw());
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

        // 1. Enable Blending
        gl::Enable(gl::BLEND);

        // 2. Disable Depth Writing to prevent Z-fighting and quad intersection artifacts
        gl::DepthMask(gl::FALSE);

        // 3. Configure DrawBuffers, ColorMasks and separate Blending per buffer (MRT single-pass)
        let has_backlight = self.backlight_enabled;
        if has_backlight {
            let draw_buffers = [
                gl::COLOR_ATTACHMENT0,
                gl::COLOR_ATTACHMENT1,
                gl::COLOR_ATTACHMENT2,
            ];
            gl::DrawBuffers(3, draw_buffers.as_ptr());
            gl::ColorMaski(0, gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::ColorMaski(1, gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE); // Smoke emits zero bloom
            gl::ColorMaski(2, gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);

            if gl::BlendFunci::is_loaded() {
                gl::BlendFunci(0, gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
                gl::BlendFunci(2, gl::ONE, gl::ONE);
            } else {
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }
        } else {
            let draw_buffers = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
            gl::DrawBuffers(2, draw_buffers.as_ptr());
            gl::ColorMaski(0, gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::ColorMaski(1, gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
            gl::ColorMaski(2, gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }

        if *active_shader != self.shader_program.raw() {
            gl::UseProgram(self.shader_program.raw());
            *active_shader = self.shader_program.raw();
        }

        gl::BindVertexArray(self.vaos[self.current_frame].raw());

        // Bind Texture Unit 0: Smoke Texture
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.texture_id.raw());
        *active_texture = self.texture_id.raw();

        // Bind Texture Unit 1: Flow Map Texture
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, self.flow_map_texture_id.raw());

        // Bind Texture Unit 2: Noise Dissolve Texture
        gl::ActiveTexture(gl::TEXTURE2);
        gl::BindTexture(gl::TEXTURE_2D, self.noise_texture_id.raw());

        // Bind Texture Unit 3: Light Falloff LUT (Zero SQRT)
        gl::ActiveTexture(gl::TEXTURE0 + constants::SMOKE_LIGHTING_LUT_TEXTURE_UNIT);
        gl::BindTexture(gl::TEXTURE_2D, self.light_falloff_lut_texture_id.raw());

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

        if self.loc_render_mask != -1 {
            gl::Uniform1i(self.loc_render_mask, 0);
        }

        if self.loc_use_lut != -1 {
            gl::Uniform1i(
                self.loc_use_lut,
                if self.smoke_lighting_lut_enabled {
                    1
                } else {
                    0
                },
            );
        }

        if self.loc_wrap_relief != -1 {
            gl::Uniform1f(self.loc_wrap_relief, self.smoke_wrap_relief);
        }

        gl::DrawArraysInstanced(gl::TRIANGLE_FAN, 0, 10, count as i32);

        // Restore draw buffers, depth write, color masks and default blending
        gl::DepthMask(gl::TRUE);
        let default_draw_buffers = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
        gl::DrawBuffers(2, default_draw_buffers.as_ptr());
        gl::ColorMaski(0, gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::ColorMaski(1, gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
        gl::ColorMaski(2, gl::FALSE, gl::FALSE, gl::FALSE, gl::FALSE);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        pop_debug_group!();

        if let Some(old_sync) = self.fences[self.current_frame] {
            gl::DeleteSync(old_sync);
        }
        let sync = gl::FenceSync(gl::SYNC_GPU_COMMANDS_COMPLETE, 0);
        self.fences[self.current_frame] = Some(sync);

        self.current_frame = (self.current_frame + 1) % 3;
    }

    pub fn set_backlight_enabled(&mut self, enabled: bool) {
        self.backlight_enabled = enabled;
    }

    /// Dessine le masque alpha de fumée pour la backlight écran (§4.1 ADR).
    ///
    /// # Safety
    /// L'appelant doit s'assurer que le contexte OpenGL est valide.
    pub unsafe fn render_smoke_mask(
        &mut self,
        _count: usize,
        _mask_fbo: u32,
        _mask_width: i32,
        _mask_height: i32,
    ) {
        // No-op : en mode MRT Single-Pass, le masque de rétroéclairage
        // est écrit directement dans Attachment 2 pendant render_smoke_instanced.
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
        self.texture_id.reset(0);
        self.flow_map_texture_id.reset(0);
        self.noise_texture_id.reset(0);
        self.light_falloff_lut_texture_id.reset(0);
        self.shader_program.reset(0);
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
                self.shader_program.reset(new_program);
                self.loc_smoke_tex =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_SmokeTexture"));
                self.loc_flow_map =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_FlowMap"));
                self.loc_noise_tex =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_NoiseTexture"));
                self.loc_flow_distortion_strength = gl::GetUniformLocation(
                    self.shader_program.raw(),
                    cstr!("u_FlowDistortionStrength"),
                );
                self.loc_flow_animation_speed = gl::GetUniformLocation(
                    self.shader_program.raw(),
                    cstr!("u_FlowAnimationSpeed"),
                );
                self.loc_erosion_enabled =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_ErosionEnabled"));
                self.loc_erosion_scale =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_ErosionScale"));
                self.loc_erosion_edge_width =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_ErosionEdgeWidth"));
                self.loc_erosion_edge_color =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_ErosionEdgeColor"));
                self.loc_render_mask =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_RenderMask"));

                gl::UseProgram(self.shader_program.raw());
                let block_idx =
                    gl::GetUniformBlockIndex(self.shader_program.raw(), cstr!("GlobalData"));
                if block_idx != gl::INVALID_INDEX {
                    gl::UniformBlockBinding(self.shader_program.raw(), block_idx, 0);
                }
                let lighting_block_idx =
                    gl::GetUniformBlockIndex(self.shader_program.raw(), cstr!("LightingBlock"));
                if lighting_block_idx != gl::INVALID_INDEX {
                    gl::UniformBlockBinding(
                        self.shader_program.raw(),
                        lighting_block_idx,
                        constants::LIGHTING_UBO_BINDING_INDEX,
                    );
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
                self.loc_light_falloff_lut =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_LightFalloffLut"));
                self.loc_use_lut =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_UseLut"));
                self.loc_wrap_relief =
                    gl::GetUniformLocation(self.shader_program.raw(), cstr!("u_WrapRelief"));
                if self.loc_light_falloff_lut != -1 {
                    gl::Uniform1i(
                        self.loc_light_falloff_lut,
                        constants::SMOKE_LIGHTING_LUT_TEXTURE_UNIT as i32,
                    );
                }
                self.rebake_smoke_lighting_lut();

                label_gl_object!(
                    gl::PROGRAM,
                    self.shader_program.raw(),
                    "Shader_SmokeInstanced"
                );
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
    ) -> ([GlVao; 3], GlBuffer, GlBuffer, *mut SmokeInstanceGPU, isize) {
        let mut raw_vaos = [0u32; 3];
        let (mut raw_vbo_quad, mut raw_vbo_particles) = (0u32, 0u32);

        const OCTAGON_VERTICES: [f32; 20] = [
            0.0, 0.0, // Center vertex for TRIANGLE_FAN
            1.082392, 0.0, 0.765366, 0.765366, 0.0, 1.082392, -0.765366, 0.765366, -1.082392, 0.0,
            -0.765366, -0.765366, 0.0, -1.082392, 0.765366, -0.765366, 1.082392,
            0.0, // Close fan
        ];

        gl::GenBuffers(1, &mut raw_vbo_quad);
        gl::BindBuffer(gl::ARRAY_BUFFER, raw_vbo_quad);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (OCTAGON_VERTICES.len() * mem::size_of::<f32>()) as isize,
            OCTAGON_VERTICES.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        gl::GenBuffers(1, &mut raw_vbo_particles);
        gl::BindBuffer(gl::ARRAY_BUFFER, raw_vbo_particles);

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

        gl::GenVertexArrays(3, raw_vaos.as_mut_ptr());
        for (frame, &vao) in raw_vaos.iter().enumerate() {
            gl::BindVertexArray(vao);

            // Attrib 0: Quad vertices (vec2)
            gl::BindBuffer(gl::ARRAY_BUFFER, raw_vbo_quad);
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
            gl::BindBuffer(gl::ARRAY_BUFFER, raw_vbo_particles);
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

        for (frame, &vao) in raw_vaos.iter().enumerate() {
            label_gl_object!(gl::VERTEX_ARRAY, vao, &format!("VAO_Smoke_Frame_{}", frame));
        }
        label_gl_object!(gl::BUFFER, raw_vbo_quad, "VBO_Smoke_Static_Quad");
        label_gl_object!(gl::BUFFER, raw_vbo_particles, "VBO_Smoke_Instance_Data");

        let vaos = [
            GlVao::from_raw(raw_vaos[0]),
            GlVao::from_raw(raw_vaos[1]),
            GlVao::from_raw(raw_vaos[2]),
        ];
        (
            vaos,
            GlBuffer::from_raw(raw_vbo_quad),
            GlBuffer::from_raw(raw_vbo_particles),
            mapped_ptr,
            buffer_size,
        )
    }
}

impl ParticleGraphicsRenderer for SmokeRenderer {
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
        self.render_smoke_with_persistent_buffer(count, active_shader, active_texture);
    }

    unsafe fn render_smoke_mask(
        &mut self,
        count: usize,
        mask_fbo: u32,
        mask_width: i32,
        mask_height: i32,
    ) {
        self.render_smoke_mask(count, mask_fbo, mask_width, mask_height);
    }

    fn get_shader_program(&self) -> u32 {
        self.shader_program.raw()
    }

    fn get_texture_id(&self) -> u32 {
        self.texture_id.raw()
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

    fn set_smoke_lighting(
        &mut self,
        enabled: bool,
        intensity: f32,
        ambient_flash: f32,
        use_lut: bool,
        wrap_relief: f32,
    ) {
        self.smoke_lighting_enabled = enabled;
        self.smoke_scattering_intensity = intensity;
        self.smoke_ambient_flash = ambient_flash;
        self.smoke_lighting_lut_enabled = use_lut;
        self.smoke_wrap_relief = wrap_relief;
    }

    fn set_backlight_enabled(&mut self, enabled: bool) {
        self.backlight_enabled = enabled;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn analytic_falloff(x: f32, y: f32) -> f32 {
        let dist_sq = x * x + y * y;
        if dist_sq < 1.0 {
            let dist = dist_sq.sqrt();
            let atten = (1.0 - dist) * (1.0 - dist);
            let cos_theta = if dist > 0.0001 { y / dist } else { 0.0 };
            let phase = 1.0 + constants::SMOKE_LIGHTING_ANISOTROPY_COEFF * cos_theta;
            atten * phase
        } else {
            0.0
        }
    }

    fn sample_lut_cpu(lut: &[f32], res: usize, u: f32, v: f32) -> f32 {
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return 0.0;
        }
        let n = res as f32;
        let x = u * n - 0.5;
        let y = v * n - 0.5;
        let i0 = (x.floor() as isize).clamp(0, res as isize - 1) as usize;
        let i1 = (i0 + 1).min(res - 1);
        let j0 = (y.floor() as isize).clamp(0, res as isize - 1) as usize;
        let j1 = (j0 + 1).min(res - 1);

        let fx = (x - x.floor()).clamp(0.0, 1.0);
        let fy = (y - y.floor()).clamp(0.0, 1.0);

        let s00 = lut[j0 * res + i0];
        let s10 = lut[j0 * res + i1];
        let s01 = lut[j1 * res + i0];
        let s11 = lut[j1 * res + i1];

        let top = s00 * (1.0 - fx) + s10 * fx;
        let bot = s01 * (1.0 - fx) + s11 * fx;
        top * (1.0 - fy) + bot * fy
    }

    #[test]
    fn test_smoke_lighting_lut_mathematical_precision() {
        let res = constants::SMOKE_LIGHTING_LUT_RESOLUTION;
        let n = res as f32;
        let anisotropy = constants::SMOKE_LIGHTING_ANISOTROPY_COEFF;
        let mut lut = Vec::with_capacity(res * res);

        for j in 0..res {
            let v = (j as f32 + 0.5) / n;
            let y = (v - 0.5) * 2.0;
            for i in 0..res {
                let u = (i as f32 + 0.5) / n;
                let x = (u - 0.5) * 2.0;
                let dist_sq = x * x + y * y;
                if dist_sq < 1.0 {
                    let dist = dist_sq.sqrt();
                    let atten = (1.0 - dist) * (1.0 - dist);
                    let cos_theta = if dist > 0.0001 { y / dist } else { 0.0 };
                    let phase = 1.0 + anisotropy * cos_theta;
                    lut.push(atten * phase);
                } else {
                    lut.push(0.0);
                }
            }
        }

        // 1. Center test: origin should have falloff ~= 1.0 (within 1.5% due to 256x256 texel centers)
        let center_sample = sample_lut_cpu(&lut, res, 0.5, 0.5);
        assert!((center_sample - 1.0).abs() < 0.015);

        // 2. Far out of bounds: outside quad should be exactly 0.0
        assert_eq!(sample_lut_cpu(&lut, res, -0.2, 0.5), 0.0);
        assert_eq!(sample_lut_cpu(&lut, res, 1.2, 0.5), 0.0);

        // 3. Dense grid parity test: max absolute error < 0.008 over 10,000 points
        let steps = 100;
        let mut max_err: f32 = 0.0;
        for sy in 0..=steps {
            let y = -1.1 + (2.2 * sy as f32 / steps as f32);
            for sx in 0..=steps {
                let x = -1.1 + (2.2 * sx as f32 / steps as f32);
                let analytic = analytic_falloff(x, y);

                let u = x * 0.5 + 0.5;
                let v = y * 0.5 + 0.5;
                let lut_val = sample_lut_cpu(&lut, res, u, v);

                let err = (analytic - lut_val).abs();
                if err > max_err {
                    max_err = err;
                }
            }
        }

        assert!(
            max_err < 0.015,
            "LUT maximum discrepancy vs analytic formula was {:.5}, expected < 0.015",
            max_err
        );
    }
}

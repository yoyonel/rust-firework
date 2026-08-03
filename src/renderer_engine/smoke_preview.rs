use crate::cstr;
use crate::physic_engine::config::{PhysicConfig, SmokeColorMode};
use crate::physic_engine::smoke_system::SmokeSystem;
use crate::renderer_engine::smoke_renderer::SmokeInstanceGPU;
use crate::renderer_engine::utils::texture::load_texture;
use glam::{Vec2, Vec3};
use memoffset::offset_of;
use std::sync::atomic::{AtomicI32, AtomicU32};

pub static PREVIEW_ZOOM: AtomicU32 = AtomicU32::new(100); // 100 = 1.0x
pub static PREVIEW_PAN_X: AtomicI32 = AtomicI32::new(0); // stored as f32 * 10.0
pub static PREVIEW_PAN_Y: AtomicI32 = AtomicI32::new(0); // stored as f32 * 10.0
pub static PREVIEW_ROT_Z: AtomicI32 = AtomicI32::new(0); // stored as degrees * 10.0

/// Context holding parameters for a preview render pass.
/// Grouped into a struct to avoid Clippy `too_many_arguments` warning.
pub struct PreviewContext<'a> {
    pub config: &'a PhysicConfig,
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
    pub rot_deg: f32,
    pub canvas_aspect: f32,
    pub time: f32,
    pub dt: f32,
    pub rocket_color: [f32; 3],
    pub simulated_speed: f32,
    pub simulated_angle_offset_deg: f32,
}

pub struct SmokePreviewRenderer {
    hdr_fbo: u32,
    hdr_tex: u32,
    fbo: u32,
    color_tex: u32,
    postproc_program: u32,
    loc_postproc_tex: i32,
    ubo_global: u32,
    smoke_program: u32,
    loc_smoke_tex: i32,
    loc_flow_map: i32,
    loc_noise_tex: i32,
    loc_flow_distortion_strength: i32,
    loc_flow_animation_speed: i32,
    loc_erosion_enabled: i32,
    loc_erosion_scale: i32,
    loc_edge_width: i32,
    loc_edge_color: i32,
    smoke_vao: u32,
    _smoke_quad_vbo: u32,
    smoke_inst_vbo: u32,
    pub smoke_tex: u32,
    pub flow_map_tex: u32,
    pub noise_tex: u32,
    rocket_tex: u32,
    quad_program: u32,
    quad_vao: u32,
    _quad_vbo: u32,
    loc_quad_rect: i32,
    loc_quad_size: i32,
    loc_quad_tex: i32,
    loc_quad_rot_z: i32,
    loc_quad_color: i32,

    // Task 4.A: Local isolated SmokeSystem for realistic continuous trail physics
    pub smoke_system: SmokeSystem,
    pub rng: rand::rngs::ThreadRng,
    emit_timer: f32,
}

impl SmokePreviewRenderer {
    pub fn init() -> Self {
        unsafe {
            // 1. Create HDR Scene Framebuffer (linear space) and Final Composite Framebuffer (sRGB / Tone Mapped)
            let mut hdr_fbo = 0;
            gl::GenFramebuffers(1, &mut hdr_fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, hdr_fbo);

            let mut hdr_tex = 0;
            gl::GenTextures(1, &mut hdr_tex);
            gl::BindTexture(gl::TEXTURE_2D, hdr_tex);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA16F as i32,
                480,
                200,
                0,
                gl::RGBA,
                gl::FLOAT,
                std::ptr::null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                hdr_tex,
                0,
            );

            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

            let mut color_tex = 0;
            gl::GenTextures(1, &mut color_tex);
            gl::BindTexture(gl::TEXTURE_2D, color_tex);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA8 as i32,
                480,
                200,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                std::ptr::null(),
            );
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                color_tex,
                0,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            // Post-processing Tone-Mapping + Gamma Correction Shader (loaded from external .glsl files)
            use crate::renderer_engine::constants;
            use crate::renderer_engine::shader::compile_shader_program_from_files;

            let postproc_program = compile_shader_program_from_files(
                constants::SHADER_SMOKE_PREVIEW_POSTPROC_VERTEX_PATH,
                constants::SHADER_SMOKE_PREVIEW_POSTPROC_FRAGMENT_PATH,
            );

            let loc_postproc_tex = gl::GetUniformLocation(postproc_program, cstr!("uTex"));

            // 2. Create GlobalData UBO (binding 0)
            let mut ubo_global = 0;

            gl::GenBuffers(1, &mut ubo_global);
            gl::BindBuffer(gl::UNIFORM_BUFFER, ubo_global);
            gl::BufferData(gl::UNIFORM_BUFFER, 16, std::ptr::null(), gl::DYNAMIC_DRAW);
            gl::BindBufferBase(
                gl::UNIFORM_BUFFER,
                constants::GLOBAL_UBO_BINDING_INDEX,
                ubo_global,
            );

            // 3. Compile Smoke Instanced Shader
            let smoke_program = crate::renderer_engine::shader::compile_shader_program_from_files(
                constants::SHADER_SMOKE_VERTEX_PATH,
                constants::SHADER_SMOKE_FRAGMENT_PATH,
            );

            let block_idx = gl::GetUniformBlockIndex(smoke_program, cstr!("GlobalData"));
            if block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(
                    smoke_program,
                    block_idx,
                    constants::GLOBAL_UBO_BINDING_INDEX,
                );
            }

            let loc_smoke_tex = gl::GetUniformLocation(smoke_program, cstr!("u_SmokeTexture"));
            let loc_flow_map = gl::GetUniformLocation(smoke_program, cstr!("u_FlowMap"));
            let loc_noise_tex = gl::GetUniformLocation(smoke_program, cstr!("u_NoiseTexture"));
            let loc_flow_distortion_strength =
                gl::GetUniformLocation(smoke_program, cstr!("u_FlowDistortionStrength"));
            let loc_flow_animation_speed =
                gl::GetUniformLocation(smoke_program, cstr!("u_FlowAnimationSpeed"));
            let loc_erosion_enabled =
                gl::GetUniformLocation(smoke_program, cstr!("u_ErosionEnabled"));
            let loc_erosion_scale = gl::GetUniformLocation(smoke_program, cstr!("u_ErosionScale"));
            let loc_edge_width = gl::GetUniformLocation(smoke_program, cstr!("u_ErosionEdgeWidth"));
            let loc_edge_color = gl::GetUniformLocation(smoke_program, cstr!("u_ErosionEdgeColor"));

            // 4. VAO/VBO Setup for Smoke Instanced Particles
            let mut smoke_vao = 0;
            let mut smoke_quad_vbo = 0;
            let mut smoke_inst_vbo = 0;

            gl::GenVertexArrays(1, &mut smoke_vao);
            gl::GenBuffers(1, &mut smoke_quad_vbo);
            gl::GenBuffers(1, &mut smoke_inst_vbo);

            gl::BindVertexArray(smoke_vao);

            const OCTAGON_VERTICES: [f32; 20] = [
                0.0, 0.0, // Center vertex for TRIANGLE_FAN
                1.082392, 0.0, 0.765366, 0.765366, 0.0, 1.082392, -0.765366, 0.765366, -1.082392,
                0.0, -0.765366, -0.765366, 0.0, -1.082392, 0.765366, -0.765366, 1.082392, 0.0,
            ];
            gl::BindBuffer(gl::ARRAY_BUFFER, smoke_quad_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (OCTAGON_VERTICES.len() * 4) as isize,
                OCTAGON_VERTICES.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );

            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            gl::EnableVertexAttribArray(0);

            // Instance data buffer (up to 128 instances)
            let max_preview_instances = 128;
            gl::BindBuffer(gl::ARRAY_BUFFER, smoke_inst_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (max_preview_instances * std::mem::size_of::<SmokeInstanceGPU>()) as isize,
                std::ptr::null(),
                gl::DYNAMIC_DRAW,
            );

            let stride = std::mem::size_of::<SmokeInstanceGPU>() as i32;

            // location 1: position (vec3)
            gl::VertexAttribPointer(
                1,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(SmokeInstanceGPU, position) as *const _,
            );
            gl::EnableVertexAttribArray(1);
            gl::VertexAttribDivisor(1, 1);

            // location 2: scale (float)
            gl::VertexAttribPointer(
                2,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(SmokeInstanceGPU, scale) as *const _,
            );
            gl::EnableVertexAttribArray(2);
            gl::VertexAttribDivisor(2, 1);

            // location 3: alpha (float)
            gl::VertexAttribPointer(
                3,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(SmokeInstanceGPU, alpha) as *const _,
            );
            gl::EnableVertexAttribArray(3);
            gl::VertexAttribDivisor(3, 1);

            // location 4: rotation (float)
            gl::VertexAttribPointer(
                4,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(SmokeInstanceGPU, rotation) as *const _,
            );
            gl::EnableVertexAttribArray(4);
            gl::VertexAttribDivisor(4, 1);

            // location 5: intensity (float)
            gl::VertexAttribPointer(
                5,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(SmokeInstanceGPU, intensity) as *const _,
            );
            gl::EnableVertexAttribArray(5);
            gl::VertexAttribDivisor(5, 1);

            // location 6: color (vec3)
            gl::VertexAttribPointer(
                6,
                3,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(SmokeInstanceGPU, color) as *const _,
            );
            gl::EnableVertexAttribArray(6);
            gl::VertexAttribDivisor(6, 1);

            // location 7: normalized_age (float)
            gl::VertexAttribPointer(
                7,
                1,
                gl::FLOAT,
                gl::FALSE,
                stride,
                offset_of!(SmokeInstanceGPU, normalized_age) as *const _,
            );
            gl::EnableVertexAttribArray(7);
            gl::VertexAttribDivisor(7, 1);

            gl::BindVertexArray(0);

            // 5. Load Textures
            let (smoke_tex, _, _) = load_texture(constants::TEXTURE_SMOKE_PARTICLE_PATH);
            let (flow_map_tex, _, _) = load_texture(constants::TEXTURE_FLOW_MAP_PATH);
            let (noise_tex, _, _) = load_texture(constants::TEXTURE_NOISE_PATH);
            let (rocket_tex, _, _) = load_texture(constants::TEXTURE_PRIMARY_PARTICLE_PATH);

            // 6. Quad Shader for Rocket Rendering with Z-Rotation (loaded from external .glsl files)
            let quad_program = compile_shader_program_from_files(
                constants::SHADER_SMOKE_PREVIEW_QUAD_VERTEX_PATH,
                constants::SHADER_SMOKE_PREVIEW_QUAD_FRAGMENT_PATH,
            );

            let loc_quad_rect = gl::GetUniformLocation(quad_program, cstr!("uRect"));
            let loc_quad_size = gl::GetUniformLocation(quad_program, cstr!("uSize"));
            let loc_quad_tex = gl::GetUniformLocation(quad_program, cstr!("uTex"));
            let loc_quad_rot_z = gl::GetUniformLocation(quad_program, cstr!("uRotZ"));
            let loc_quad_color = gl::GetUniformLocation(quad_program, cstr!("uColor"));

            let mut quad_vao = 0;
            let mut quad_vbo = 0;
            gl::GenVertexArrays(1, &mut quad_vao);
            gl::GenBuffers(1, &mut quad_vbo);

            let quad_data: [f32; 16] = [
                -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0,
            ];

            gl::BindVertexArray(quad_vao);
            gl::BindBuffer(gl::ARRAY_BUFFER, quad_vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (quad_data.len() * 4) as isize,
                quad_data.as_ptr() as *const _,
                gl::STATIC_DRAW,
            );
            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, 16, std::ptr::null());
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                1,
                2,
                gl::FLOAT,
                gl::FALSE,
                16,
                (2 * std::mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);
            gl::BindVertexArray(0);

            Self {
                hdr_fbo,
                hdr_tex,
                fbo,
                color_tex,
                postproc_program,
                loc_postproc_tex,
                ubo_global,
                smoke_program,
                loc_smoke_tex,
                loc_flow_map,
                loc_noise_tex,
                loc_flow_distortion_strength,
                loc_flow_animation_speed,
                loc_erosion_enabled,
                loc_erosion_scale,
                loc_edge_width,
                loc_edge_color,
                smoke_vao,
                _smoke_quad_vbo: smoke_quad_vbo,
                smoke_inst_vbo,
                smoke_tex,
                flow_map_tex,
                noise_tex,
                rocket_tex,
                quad_program,
                quad_vao,
                _quad_vbo: quad_vbo,
                loc_quad_rect,
                loc_quad_size,
                loc_quad_tex,
                loc_quad_rot_z,
                loc_quad_color,
                smoke_system: SmokeSystem::new(128),
                rng: rand::rng(),
                emit_timer: 0.0,
            }
        }
    }

    pub fn render(&mut self, ctx: &PreviewContext) -> u32 {
        unsafe {
            let mut prev_fbo = 0;
            gl::GetIntegerv(gl::FRAMEBUFFER_BINDING, &mut prev_fbo);
            let mut prev_viewport = [0; 4];
            gl::GetIntegerv(gl::VIEWPORT, prev_viewport.as_mut_ptr());

            gl::BindFramebuffer(gl::FRAMEBUFFER, self.hdr_fbo);
            gl::Viewport(0, 0, 480, 200);

            // Task 4.B: Clear to pure black (night sky background for visual ISO)
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            // Task 4.B: Exact blend state (premultiplied / standard alpha blend)
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            let sim_h = 200.0 / ctx.zoom;
            let sim_w = sim_h * ctx.canvas_aspect.max(0.1);
            let rot_rad = ctx.rot_deg.to_radians();

            let center_x = sim_w * 0.5 + ctx.pan_x;
            let center_y = sim_h * 0.75 + ctx.pan_y;

            let rocket_w = 22.0;
            let rocket_h = 66.0;

            let nozzle_x = center_x + (rocket_h * 0.5) * rot_rad.sin();
            let nozzle_y = center_y - (rocket_h * 0.5) * rot_rad.cos();

            // Task 4.A: Simulated velocity exhaust vector with configurable speed and angle offset
            let nozzle_pos = Vec2::new(nozzle_x, nozzle_y);
            let total_angle_rad = (ctx.rot_deg + ctx.simulated_angle_offset_deg).to_radians();
            let simulated_rocket_vel =
                Vec2::new(-total_angle_rad.sin(), total_angle_rad.cos()) * ctx.simulated_speed;
            let inherited_col = match ctx.config.smoke_color_mode {
                SmokeColorMode::Custom => Vec3::from_array(ctx.config.smoke_custom_color),
                SmokeColorMode::RocketColor => Vec3::from_array(ctx.rocket_color),
            };

            // Emit particles into isolated local SmokeSystem
            let dt = ctx.dt.clamp(0.001, 0.1);
            self.emit_timer += dt;
            let spawn_rate = ctx.config.smoke_spawn_rate.max(1.0);
            let interval = 1.0 / spawn_rate;
            while self.emit_timer >= interval {
                self.emit_timer -= interval;
                self.smoke_system.emit_preview(
                    nozzle_pos,
                    simulated_rocket_vel,
                    inherited_col,
                    ctx.config,
                    &mut self.rng,
                );
            }

            self.smoke_system.update(dt, ctx.config);

            // 1. RENDER INSTANCED SMOKE PARTICLES (render_order=10 in main scene)
            // Match main scene GL state: disable depth writes for smoke
            gl::DepthMask(gl::FALSE);

            gl::BindBuffer(gl::UNIFORM_BUFFER, self.ubo_global);
            let ubo_data: [f32; 4] = [sim_w, sim_h, 399.0 / 385.0, 1.5];
            gl::BufferSubData(gl::UNIFORM_BUFFER, 0, 16, ubo_data.as_ptr() as *const _);
            gl::BindBufferBase(gl::UNIFORM_BUFFER, 0, self.ubo_global);

            gl::UseProgram(self.smoke_program);

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.smoke_tex);
            if self.loc_smoke_tex != -1 {
                gl::Uniform1i(self.loc_smoke_tex, 0);
            }

            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.flow_map_tex);
            if self.loc_flow_map != -1 {
                gl::Uniform1i(self.loc_flow_map, 1);
            }

            gl::ActiveTexture(gl::TEXTURE2);
            gl::BindTexture(gl::TEXTURE_2D, self.noise_tex);
            if self.loc_noise_tex != -1 {
                gl::Uniform1i(self.loc_noise_tex, 2);
            }

            if self.loc_flow_distortion_strength != -1 {
                gl::Uniform1f(
                    self.loc_flow_distortion_strength,
                    ctx.config.flow_distortion_strength,
                );
            }
            if self.loc_flow_animation_speed != -1 {
                gl::Uniform1f(
                    self.loc_flow_animation_speed,
                    ctx.config.flow_animation_speed,
                );
            }

            if self.loc_erosion_enabled != -1 {
                gl::Uniform1i(
                    self.loc_erosion_enabled,
                    if ctx.config.smoke_erosion_enabled {
                        1
                    } else {
                        0
                    },
                );
            }
            if self.loc_erosion_scale != -1 {
                gl::Uniform1f(self.loc_erosion_scale, ctx.config.smoke_erosion_scale);
            }
            if self.loc_edge_width != -1 {
                gl::Uniform1f(self.loc_edge_width, ctx.config.smoke_erosion_edge_width);
            }
            if self.loc_edge_color != -1 {
                gl::Uniform3f(
                    self.loc_edge_color,
                    ctx.config.smoke_erosion_edge_color[0],
                    ctx.config.smoke_erosion_edge_color[1],
                    ctx.config.smoke_erosion_edge_color[2],
                );
            }

            // Build GPU instances from local isolated SmokeSystem active particles
            let mut instances = Vec::with_capacity(128);
            let preview_intensity = ctx.config.smoke_intensity;

            self.smoke_system.for_each_active(&mut |p| {
                if instances.len() < 128 {
                    instances.push(SmokeInstanceGPU {
                        position: [p.pos.x, p.pos.y, 0.0],
                        scale: p.sizing.current_size,
                        alpha: p.opacity.alpha,
                        rotation: p.rotation,
                        intensity: preview_intensity,
                        color: [p.color.x, p.color.y, p.color.z],
                        normalized_age: p.lifecycle.progress(),
                    });
                }
            });

            if !instances.is_empty() {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.smoke_inst_vbo);
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    (instances.len() * std::mem::size_of::<SmokeInstanceGPU>()) as isize,
                    instances.as_ptr() as *const _,
                );

                gl::BindVertexArray(self.smoke_vao);
                gl::DrawArraysInstanced(gl::TRIANGLE_FAN, 0, 10, instances.len() as i32);
                gl::BindVertexArray(0);
            }

            // Restore depth write (ISO with smoke_renderer.rs)
            gl::DepthMask(gl::TRUE);

            // 2. RENDER ROCKET SPRITE (render_order=20 in main scene — drawn OVER smoke)
            gl::UseProgram(self.quad_program);
            gl::Uniform2f(self.loc_quad_size, sim_w, sim_h);
            gl::Uniform4f(self.loc_quad_rect, center_x, center_y, rocket_w, rocket_h);
            gl::Uniform1f(self.loc_quad_rot_z, rot_rad);
            if self.loc_quad_color != -1 {
                gl::Uniform3f(
                    self.loc_quad_color,
                    ctx.rocket_color[0],
                    ctx.rocket_color[1],
                    ctx.rocket_color[2],
                );
            }
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.rocket_tex);
            gl::Uniform1i(self.loc_quad_tex, 0);

            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            // 3. POST-PROCESS COMPOSITE PASS: Tone-Mapping (KhronosPBR) & Gamma Correction (1.0/2.2)
            // Exactly ISO with main scene bloom composition shader
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::Viewport(0, 0, 480, 200);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::Disable(gl::BLEND);
            gl::UseProgram(self.postproc_program);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.hdr_tex);
            if self.loc_postproc_tex != -1 {
                gl::Uniform1i(self.loc_postproc_tex, 0);
            }

            gl::BindVertexArray(self.quad_vao);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            gl::BindVertexArray(0);

            // Restore previous framebuffer and viewport
            gl::BindFramebuffer(gl::FRAMEBUFFER, prev_fbo as u32);
            gl::Viewport(
                prev_viewport[0],
                prev_viewport[1],
                prev_viewport[2],
                prev_viewport[3],
            );

            self.color_tex
        }
    }
}

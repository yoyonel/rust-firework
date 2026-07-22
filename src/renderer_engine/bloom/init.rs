use super::{BloomPass, BlurMethod};
use crate::label_gl_object;
use crate::renderer_engine::config::{RendererConfig, ToneMappingMode};
use crate::renderer_engine::shader::try_compile_shader_program_from_files;
use log::info;

impl BloomPass {
    pub fn new(width: i32, height: i32) -> Result<Self, String> {
        info!("🌟 Initializing Bloom Pass ({}x{})", width, height);

        unsafe {
            // Create HDR framebuffer
            let mut hdr_fbo = 0;
            gl::GenFramebuffers(1, &mut hdr_fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, hdr_fbo);

            // Create HDR color texture (RGBA16F for high precision)
            let mut hdr_texture = 0;
            gl::GenTextures(1, &mut hdr_texture);
            gl::BindTexture(gl::TEXTURE_2D, hdr_texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA16F as i32,
                width,
                height,
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
                hdr_texture,
                0,
            );

            // Create Brightness/Bloom texture (MRT Attachment 1)
            let mut bright_texture = 0;
            gl::GenTextures(1, &mut bright_texture);
            gl::BindTexture(gl::TEXTURE_2D, bright_texture);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA16F as i32,
                width,
                height,
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
                gl::COLOR_ATTACHMENT1,
                gl::TEXTURE_2D,
                bright_texture,
                0,
            );

            // Configure DrawBuffers for MRT
            let attachments = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
            gl::DrawBuffers(2, attachments.as_ptr());

            // Create depth renderbuffer
            let mut hdr_depth_rbo = 0;
            gl::GenRenderbuffers(1, &mut hdr_depth_rbo);
            gl::BindRenderbuffer(gl::RENDERBUFFER, hdr_depth_rbo);
            gl::RenderbufferStorage(gl::RENDERBUFFER, gl::DEPTH_COMPONENT, width, height);
            gl::FramebufferRenderbuffer(
                gl::FRAMEBUFFER,
                gl::DEPTH_ATTACHMENT,
                gl::RENDERBUFFER,
                hdr_depth_rbo,
            );

            // Check framebuffer completeness
            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                return Err("HDR framebuffer is not complete".to_string());
            }

            // Create ping-pong framebuffers for blur
            // Use downsampling for performance (default: 2x = half resolution)
            let downsample_factor = 2u32; // Default to half-res blur
            let blur_width = width / downsample_factor as i32;
            let blur_height = height / downsample_factor as i32;

            let mut ping_pong_fbo = [0; 2];
            let mut ping_pong_textures = [0; 2];
            gl::GenFramebuffers(2, ping_pong_fbo.as_mut_ptr());
            gl::GenTextures(2, ping_pong_textures.as_mut_ptr());

            for i in 0..2 {
                gl::BindFramebuffer(gl::FRAMEBUFFER, ping_pong_fbo[i]);
                gl::BindTexture(gl::TEXTURE_2D, ping_pong_textures[i]);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA16F as i32,
                    blur_width, // Downsampled resolution
                    blur_height,
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
                    ping_pong_textures[i],
                    0,
                );

                if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                    return Err(format!("Ping-pong framebuffer {} is not complete", i));
                }
            }

            // Unbind framebuffer
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            // Compile shaders
            let blur_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/gaussian_blur.frag.glsl",
            )?;

            let kawase_downsample_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/kawase_downsample.frag.glsl",
            )?;

            let kawase_upsample_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/kawase_upsample.frag.glsl",
            )?;

            let composition_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/bloom_composition.frag.glsl",
            )?;

            // Compile comparison shader (MRT)
            let comparison_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/bloom_composition_compare.frag.glsl",
            )?;

            // Compile passthrough shader (for displaying comparison textures)
            let passthrough_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/passthrough.frag.glsl",
            )?;

            // Create comparison mode resources (FBO + 5 textures)
            let mut comparison_fbo = 0;
            let mut comparison_textures = [0; 5];

            gl::GenFramebuffers(1, &mut comparison_fbo);
            gl::GenTextures(5, comparison_textures.as_mut_ptr());

            gl::BindFramebuffer(gl::FRAMEBUFFER, comparison_fbo);

            for (i, &tex) in comparison_textures.iter().enumerate() {
                gl::BindTexture(gl::TEXTURE_2D, tex);
                gl::TexImage2D(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as i32,
                    width,
                    height,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    std::ptr::null(),
                );
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

                // Attach to FBO
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0 + i as u32,
                    gl::TEXTURE_2D,
                    tex,
                    0,
                );
            }

            // Set draw buffers for MRT
            let draw_buffers = [
                gl::COLOR_ATTACHMENT0,
                gl::COLOR_ATTACHMENT1,
                gl::COLOR_ATTACHMENT2,
                gl::COLOR_ATTACHMENT3,
                gl::COLOR_ATTACHMENT4,
            ];
            gl::DrawBuffers(5, draw_buffers.as_ptr());

            // Check FBO status
            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            if status != gl::FRAMEBUFFER_COMPLETE {
                return Err(format!("Comparison FBO incomplete: {}", status));
            }

            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

            // Cache uniform locations
            let loc_blur_texture = gl::GetUniformLocation(blur_shader, crate::cstr!("uTexture"));
            let loc_blur_direction =
                gl::GetUniformLocation(blur_shader, crate::cstr!("uDirection"));

            let loc_kawase_down_texture =
                gl::GetUniformLocation(kawase_downsample_shader, crate::cstr!("uTexture"));
            let loc_kawase_down_halfpixel =
                gl::GetUniformLocation(kawase_downsample_shader, crate::cstr!("uHalfPixel"));

            let loc_kawase_up_texture =
                gl::GetUniformLocation(kawase_upsample_shader, crate::cstr!("uTexture"));
            let loc_kawase_up_halfpixel =
                gl::GetUniformLocation(kawase_upsample_shader, crate::cstr!("uHalfPixel"));

            let loc_comp_scene =
                gl::GetUniformLocation(composition_shader, crate::cstr!("uSceneTexture"));
            let loc_comp_bloom =
                gl::GetUniformLocation(composition_shader, crate::cstr!("uBloomTexture"));

            // Lier le block uniform "GlobalData" au binding point 0 pour composition_shader
            let comp_block_idx =
                gl::GetUniformBlockIndex(composition_shader, crate::cstr!("GlobalData"));
            if comp_block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(composition_shader, comp_block_idx, 0);
            }

            // Lier le block uniform "GlobalData" au binding point 0 pour comparison_shader
            let compare_block_idx =
                gl::GetUniformBlockIndex(comparison_shader, crate::cstr!("GlobalData"));
            if compare_block_idx != gl::INVALID_INDEX {
                gl::UniformBlockBinding(comparison_shader, compare_block_idx, 0);
            }

            let loc_tone_mapping_mode =
                gl::GetUniformLocation(composition_shader, crate::cstr!("uToneMappingMode"));

            let loc_passthrough_texture =
                gl::GetUniformLocation(passthrough_shader, crate::cstr!("uTexture"));

            // ⚙️ SETUP STATIC UNIFORM TEXTURE MAPPINGS ONCE (AZDO Optimization)
            gl::UseProgram(blur_shader);
            gl::Uniform1i(loc_blur_texture, 0);

            gl::UseProgram(kawase_downsample_shader);
            gl::Uniform1i(loc_kawase_down_texture, 0);

            gl::UseProgram(kawase_upsample_shader);
            gl::Uniform1i(loc_kawase_up_texture, 0);

            gl::UseProgram(composition_shader);
            gl::Uniform1i(loc_comp_scene, 0);
            gl::Uniform1i(loc_comp_bloom, 1);

            gl::UseProgram(comparison_shader);
            gl::Uniform1i(
                gl::GetUniformLocation(comparison_shader, crate::cstr!("uSceneTexture")),
                0,
            );
            gl::Uniform1i(
                gl::GetUniformLocation(comparison_shader, crate::cstr!("uBloomTexture")),
                1,
            );

            gl::UseProgram(passthrough_shader);
            gl::Uniform1i(loc_passthrough_texture, 0);

            // Create dummy VAO for fullscreen quad rendering (Core Profile requirement)
            let mut dummy_vao = 0;
            gl::GenVertexArrays(1, &mut dummy_vao);

            // 🏷️ LABELLISATION DE TOUTES LES RESSOURCES BLOOM
            label_gl_object!(gl::FRAMEBUFFER, hdr_fbo, "FBO_HDR_Main");
            label_gl_object!(gl::TEXTURE, hdr_texture, "Tex_HDR_Scene_Color");
            label_gl_object!(gl::TEXTURE, bright_texture, "Tex_HDR_Brightness_Mask");
            label_gl_object!(gl::RENDERBUFFER, hdr_depth_rbo, "RBO_HDR_Depth");

            label_gl_object!(gl::FRAMEBUFFER, ping_pong_fbo[0], "FBO_Blur_Ping");
            label_gl_object!(gl::FRAMEBUFFER, ping_pong_fbo[1], "FBO_Blur_Pong");
            label_gl_object!(gl::TEXTURE, ping_pong_textures[0], "Tex_Blur_Ping");
            label_gl_object!(gl::TEXTURE, ping_pong_textures[1], "Tex_Blur_Pong");

            label_gl_object!(gl::PROGRAM, blur_shader, "Shader_Bloom_Gaussian");
            label_gl_object!(
                gl::PROGRAM,
                kawase_downsample_shader,
                "Shader_Bloom_Kawase_Down"
            );
            label_gl_object!(
                gl::PROGRAM,
                kawase_upsample_shader,
                "Shader_Bloom_Kawase_Up"
            );
            label_gl_object!(gl::PROGRAM, composition_shader, "Shader_PostFX_ToneMapping");

            // 👉 AJOUTS DES RESSOURCES MANQUANTES :
            label_gl_object!(
                gl::PROGRAM,
                comparison_shader,
                "Shader_Bloom_Composition_Compare"
            );
            label_gl_object!(gl::PROGRAM, passthrough_shader, "Shader_Bloom_Passthrough");
            label_gl_object!(gl::FRAMEBUFFER, comparison_fbo, "FBO_Bloom_Comparison");
            for (i, &tex) in comparison_textures.iter().enumerate() {
                let label = format!("Tex_Bloom_Compare_Mode_{}", i);
                label_gl_object!(gl::TEXTURE, tex, &label);
            }
            label_gl_object!(gl::VERTEX_ARRAY, dummy_vao, "VAO_Fullscreen_Quad_Dummy");

            info!("✅ Bloom Pass initialized successfully (MRT enabled)");

            Ok(Self {
                hdr_fbo,
                hdr_texture,
                bright_texture,
                hdr_depth_rbo,
                ping_pong_fbo,
                ping_pong_textures,
                blur_shader,
                kawase_downsample_shader,
                kawase_upsample_shader,
                composition_shader,
                dummy_vao,
                loc_blur_direction,
                loc_kawase_down_halfpixel,
                loc_kawase_up_halfpixel,
                loc_tone_mapping_mode,
                passthrough_shader,
                intensity: 2.0,
                blur_iterations: 5,
                enabled: true,
                downsample_factor,
                blur_method: BlurMethod::Gaussian, // Default to Gaussian
                tone_mapping_mode: ToneMappingMode::ACES, // Default to ACES
                comparison_mode: false,
                comparison_fbo,
                comparison_textures,
                comparison_shader,
                width,
                height,
                blur_width,
                blur_height,
            })
        }
    }

    pub fn new_dummy() -> Self {
        Self {
            hdr_fbo: 0,
            hdr_texture: 0,
            bright_texture: 0,
            hdr_depth_rbo: 0,
            ping_pong_fbo: [0; 2],
            ping_pong_textures: [0; 2],
            blur_shader: 0,
            kawase_downsample_shader: 0,
            kawase_upsample_shader: 0,
            composition_shader: 0,
            passthrough_shader: 0,
            dummy_vao: 0,
            loc_blur_direction: 0,
            loc_kawase_down_halfpixel: 0,
            loc_kawase_up_halfpixel: 0,
            loc_tone_mapping_mode: 0,
            intensity: 1.0,
            blur_iterations: 1,
            enabled: false,
            downsample_factor: 1,
            blur_method: BlurMethod::Gaussian,
            tone_mapping_mode: ToneMappingMode::ACES,
            comparison_mode: false,
            comparison_fbo: 0,
            comparison_textures: [0; 5],
            comparison_shader: 0,
            width: 800,
            height: 600,
            blur_width: 400,
            blur_height: 300,
        }
    }

    pub(crate) unsafe fn recreate_blur_buffers(&mut self) {
        info!(
            "🔄 Recreating blur buffers with downsample factor {}x",
            self.downsample_factor
        );

        // Delete old ping-pong buffers
        gl::DeleteFramebuffers(2, self.ping_pong_fbo.as_ptr());
        gl::DeleteTextures(2, self.ping_pong_textures.as_ptr());

        // Calculate new blur dimensions
        self.blur_width = self.width / self.downsample_factor as i32;
        self.blur_height = self.height / self.downsample_factor as i32;

        // Create new ping-pong framebuffers
        let mut ping_pong_fbo = [0; 2];
        let mut ping_pong_textures = [0; 2];
        gl::GenFramebuffers(2, ping_pong_fbo.as_mut_ptr());
        gl::GenTextures(2, ping_pong_textures.as_mut_ptr());

        for i in 0..2 {
            gl::BindFramebuffer(gl::FRAMEBUFFER, ping_pong_fbo[i]);
            gl::BindTexture(gl::TEXTURE_2D, ping_pong_textures[i]);
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA16F as i32,
                self.blur_width,
                self.blur_height,
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
                ping_pong_textures[i],
                0,
            );

            if gl::CheckFramebufferStatus(gl::FRAMEBUFFER) != gl::FRAMEBUFFER_COMPLETE {
                log::error!(
                    "Ping-pong framebuffer {} is not complete after recreation",
                    i
                );
            }
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        self.ping_pong_fbo = ping_pong_fbo;
        self.ping_pong_textures = ping_pong_textures;

        info!(
            "✅ Blur buffers recreated at {}x{} ({}x downsample)",
            self.blur_width, self.blur_height, self.downsample_factor
        );
    }

    /// Reloads bloom shaders from disk
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly and manipulates GPU state.
    pub unsafe fn reload_shaders(&mut self) -> Result<(), String> {
        info!("🔄 Reloading bloom shaders...");

        // Try to compile new shaders
        let new_blur = try_compile_shader_program_from_files(
            "assets/shaders/bloom/fullscreen_quad.vert.glsl",
            "assets/shaders/bloom/gaussian_blur.frag.glsl",
        )?;

        let new_composition = try_compile_shader_program_from_files(
            "assets/shaders/bloom/fullscreen_quad.vert.glsl",
            "assets/shaders/bloom/bloom_composition.frag.glsl",
        )?;

        // Delete old shaders
        gl::DeleteProgram(self.blur_shader);
        gl::DeleteProgram(self.composition_shader);

        // Update with new shaders
        self.blur_shader = new_blur;
        self.composition_shader = new_composition;

        // Update uniform locations
        let loc_blur_texture = gl::GetUniformLocation(self.blur_shader, crate::cstr!("uTexture"));
        self.loc_blur_direction =
            gl::GetUniformLocation(self.blur_shader, crate::cstr!("uDirection"));

        let loc_comp_scene =
            gl::GetUniformLocation(self.composition_shader, crate::cstr!("uSceneTexture"));
        let loc_comp_bloom =
            gl::GetUniformLocation(self.composition_shader, crate::cstr!("uBloomTexture"));

        let comp_block_idx =
            gl::GetUniformBlockIndex(self.composition_shader, crate::cstr!("GlobalData"));
        if comp_block_idx != gl::INVALID_INDEX {
            gl::UniformBlockBinding(self.composition_shader, comp_block_idx, 0);
        }

        self.loc_tone_mapping_mode =
            gl::GetUniformLocation(self.composition_shader, crate::cstr!("uToneMappingMode"));

        // Setup reloaded static uniforms
        gl::UseProgram(self.blur_shader);
        gl::Uniform1i(loc_blur_texture, 0);

        gl::UseProgram(self.composition_shader);
        gl::Uniform1i(loc_comp_scene, 0);
        gl::Uniform1i(loc_comp_bloom, 1);

        info!("✅ Bloom shaders reloaded successfully");
        Ok(())
    }

    /// Cleans up OpenGL resources
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly and deletes GPU resources.
    pub unsafe fn close(&mut self) {
        info!("🧹 Cleaning up Bloom Pass");

        if self.hdr_fbo != 0 {
            gl::DeleteFramebuffers(1, &self.hdr_fbo);
            self.hdr_fbo = 0;
        }
        if self.hdr_texture != 0 {
            gl::DeleteTextures(1, &self.hdr_texture);
            self.hdr_texture = 0;
        }
        if self.bright_texture != 0 {
            gl::DeleteTextures(1, &self.bright_texture);
            self.bright_texture = 0;
        }
        if self.hdr_depth_rbo != 0 {
            gl::DeleteRenderbuffers(1, &self.hdr_depth_rbo);
            self.hdr_depth_rbo = 0;
        }
        if self.ping_pong_fbo[0] != 0 {
            gl::DeleteFramebuffers(2, self.ping_pong_fbo.as_ptr());
            self.ping_pong_fbo = [0; 2];
        }
        if self.ping_pong_textures[0] != 0 {
            gl::DeleteTextures(2, self.ping_pong_textures.as_ptr());
            self.ping_pong_textures = [0; 2];
        }
        if self.blur_shader != 0 {
            gl::DeleteProgram(self.blur_shader);
            self.blur_shader = 0;
        }
        if self.composition_shader != 0 {
            gl::DeleteProgram(self.composition_shader);
            self.composition_shader = 0;
        }
        if self.dummy_vao != 0 {
            gl::DeleteVertexArrays(1, &self.dummy_vao);
            self.dummy_vao = 0;
        }
        if self.comparison_fbo != 0 {
            gl::DeleteFramebuffers(1, &self.comparison_fbo);
            self.comparison_fbo = 0;
        }
        if self.comparison_textures[0] != 0 {
            gl::DeleteTextures(5, self.comparison_textures.as_ptr());
            self.comparison_textures = [0; 5];
        }
        if self.comparison_shader != 0 {
            gl::DeleteProgram(self.comparison_shader);
            self.comparison_shader = 0;
        }
        if self.passthrough_shader != 0 {
            gl::DeleteProgram(self.passthrough_shader);
            self.passthrough_shader = 0;
        }
    }

    pub fn sync_with_renderer_config(&mut self, config: &RendererConfig) {
        self.enabled = config.bloom_enabled;
        self.intensity = config.bloom_intensity;
        self.blur_iterations = config.bloom_iterations;
        self.blur_method = match config.bloom_blur_method {
            crate::renderer_engine::config::BlurMethod::Gaussian => {
                crate::renderer_engine::bloom::BlurMethod::Gaussian
            }
            crate::renderer_engine::config::BlurMethod::Kawase => {
                crate::renderer_engine::bloom::BlurMethod::Kawase
            }
        };
        self.tone_mapping_mode = config.tone_mapping_mode;

        // Check for downsample change
        if self.downsample_factor != config.bloom_downsample {
            self.downsample_factor = config.bloom_downsample;
            unsafe {
                self.recreate_blur_buffers();
            }
        }
    }
}

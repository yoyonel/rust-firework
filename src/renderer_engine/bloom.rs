use crate::renderer_engine::shader::try_compile_shader_program_from_files;
use gl::types::*;
use log::info;

/// Bloom post-processing effect
///
/// Implements an Unreal-style bloom with:
/// - HDR framebuffer for scene rendering
/// - Brightness extraction pass
/// - Separable Gaussian blur (ping-pong)
/// - Final composition with tone mapping
pub struct BloomPass {
    // Framebuffers and textures
    hdr_fbo: GLuint,
    hdr_texture: GLuint,
    hdr_depth_rbo: GLuint,

    ping_pong_fbo: [GLuint; 2],
    ping_pong_textures: [GLuint; 2],

    // Shaders
    brightness_shader: GLuint,
    blur_shader: GLuint,
    composition_shader: GLuint,

    // Uniform Locations
    loc_brightness_scene: GLint,
    loc_brightness_threshold: GLint,
    loc_blur_texture: GLint,
    loc_blur_direction: GLint,
    loc_comp_scene: GLint,
    loc_comp_bloom: GLint,
    loc_comp_intensity: GLint,

    // Configuration
    pub intensity: f32,
    pub threshold: f32,
    pub blur_iterations: u32,
    pub enabled: bool,

    // Window size
    width: i32,
    height: i32,
}

impl BloomPass {
    /// Creates a new bloom pass with the given window dimensions
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
            let brightness_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/brightness_extract.frag.glsl",
            )?;

            let blur_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/gaussian_blur.frag.glsl",
            )?;

            let composition_shader = try_compile_shader_program_from_files(
                "assets/shaders/bloom/fullscreen_quad.vert.glsl",
                "assets/shaders/bloom/bloom_composition.frag.glsl",
            )?;

            // Cache uniform locations
            let loc_brightness_scene =
                gl::GetUniformLocation(brightness_shader, crate::cstr!("uSceneTexture"));
            let loc_brightness_threshold =
                gl::GetUniformLocation(brightness_shader, crate::cstr!("uThreshold"));

            let loc_blur_texture = gl::GetUniformLocation(blur_shader, crate::cstr!("uTexture"));
            let loc_blur_direction =
                gl::GetUniformLocation(blur_shader, crate::cstr!("uDirection"));

            let loc_comp_scene =
                gl::GetUniformLocation(composition_shader, crate::cstr!("uSceneTexture"));
            let loc_comp_bloom =
                gl::GetUniformLocation(composition_shader, crate::cstr!("uBloomTexture"));
            let loc_comp_intensity =
                gl::GetUniformLocation(composition_shader, crate::cstr!("uBloomIntensity"));

            info!("✅ Bloom Pass initialized successfully");

            Ok(Self {
                hdr_fbo,
                hdr_texture,
                hdr_depth_rbo,
                ping_pong_fbo,
                ping_pong_textures,
                brightness_shader,
                blur_shader,
                composition_shader,
                loc_brightness_scene,
                loc_brightness_threshold,
                loc_blur_texture,
                loc_blur_direction,
                loc_comp_scene,
                loc_comp_bloom,
                loc_comp_intensity,
                intensity: 2.0,
                threshold: 0.2,
                blur_iterations: 5,
                enabled: true,
                width,
                height,
            })
        }
    }

    /// Begins rendering to the HDR framebuffer
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly.
    pub unsafe fn begin_scene(&self) {
        if !self.enabled {
            return;
        }

        gl::BindFramebuffer(gl::FRAMEBUFFER, self.hdr_fbo);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
    }

    /// Ends scene rendering and applies bloom post-processing
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly.
    pub unsafe fn end_scene_and_apply_bloom(&self) {
        if !self.enabled {
            return;
        }

        // Disable depth test for post-processing
        gl::Disable(gl::DEPTH_TEST);

        // 1. Extract bright pixels
        gl::BindFramebuffer(gl::FRAMEBUFFER, self.ping_pong_fbo[0]);
        gl::UseProgram(self.brightness_shader);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.hdr_texture);
        gl::Uniform1i(self.loc_brightness_scene, 0);
        gl::Uniform1f(self.loc_brightness_threshold, self.threshold);
        self.render_fullscreen_quad();

        // 2. Blur passes (ping-pong between two framebuffers)
        let mut horizontal = true;
        let mut first_iteration = true;

        gl::UseProgram(self.blur_shader);
        for _ in 0..(self.blur_iterations * 2) {
            let target_fbo = if horizontal {
                self.ping_pong_fbo[1]
            } else {
                self.ping_pong_fbo[0]
            };
            let source_texture = if first_iteration || horizontal {
                self.ping_pong_textures[0]
            } else {
                self.ping_pong_textures[1]
            };

            gl::BindFramebuffer(gl::FRAMEBUFFER, target_fbo);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, source_texture);
            gl::Uniform1i(self.loc_blur_texture, 0);

            // Set blur direction
            if horizontal {
                gl::Uniform2f(self.loc_blur_direction, 1.0, 0.0);
            } else {
                gl::Uniform2f(self.loc_blur_direction, 0.0, 1.0);
            }

            self.render_fullscreen_quad();

            horizontal = !horizontal;
            if first_iteration {
                first_iteration = false;
            }
        }

        // 3. Final composition (blend scene + bloom)
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::UseProgram(self.composition_shader);

        // Bind scene texture
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.hdr_texture);
        gl::Uniform1i(self.loc_comp_scene, 0);

        // Bind bloom texture (result of blur)
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, self.ping_pong_textures[0]);
        gl::Uniform1i(self.loc_comp_bloom, 1);

        gl::Uniform1f(self.loc_comp_intensity, self.intensity);

        self.render_fullscreen_quad();

        // Re-enable depth test
        gl::Enable(gl::DEPTH_TEST);
    }

    /// Renders a fullscreen quad using the vertex ID trick (no VBO needed)
    unsafe fn render_fullscreen_quad(&self) {
        gl::DrawArrays(gl::TRIANGLES, 0, 3);
    }

    /// Recreates framebuffers when window is resized
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly.
    pub unsafe fn resize(&mut self, width: i32, height: i32) {
        if self.width == width && self.height == height {
            return;
        }

        info!(
            "🔄 Resizing bloom framebuffers: {}x{} -> {}x{}",
            self.width, self.height, width, height
        );

        self.width = width;
        self.height = height;

        // Delete old framebuffers
        gl::DeleteFramebuffers(1, &self.hdr_fbo);
        gl::DeleteTextures(1, &self.hdr_texture);
        gl::DeleteRenderbuffers(1, &self.hdr_depth_rbo);
        gl::DeleteFramebuffers(2, self.ping_pong_fbo.as_ptr());
        gl::DeleteTextures(2, self.ping_pong_textures.as_ptr());

        // Recreate with new size
        let new_bloom = Self::new(width, height).expect("Failed to recreate bloom framebuffers");

        // Copy configuration
        self.hdr_fbo = new_bloom.hdr_fbo;
        self.hdr_texture = new_bloom.hdr_texture;
        self.hdr_depth_rbo = new_bloom.hdr_depth_rbo;
        self.ping_pong_fbo = new_bloom.ping_pong_fbo;
        self.ping_pong_textures = new_bloom.ping_pong_textures;

        // Copy uniform locations (shaders are not recreated here, but we copy from new_bloom which has them)
        // Wait, new_bloom creates NEW shaders. We want to KEEP existing shaders to avoid recompiling if not needed.
        // But BloomPass::new compiles shaders.
        // The original code says: "Don't recreate shaders, keep existing ones".
        // So we should NOT overwrite self.brightness_shader etc.
        // And thus we should NOT overwrite uniform locations either.
        // The original code did `std::mem::forget(new_bloom)` but only copied FBOs/textures.
        // Correct logic: we keep our current shaders and locations.
        // We just need to ensure new_bloom doesn't delete the shaders we want to keep?
        // Actually new_bloom creates NEW shaders. If we drop new_bloom, it might delete them?
        // BloomPass::drop calls close() which deletes shaders.
        // So we MUST take ownership of new_bloom's resources or let them be deleted.
        // But we want to KEEP *our* old shaders.
        // So we should delete new_bloom's shaders immediately since we won't use them.
        gl::DeleteProgram(new_bloom.brightness_shader);
        gl::DeleteProgram(new_bloom.blur_shader);
        gl::DeleteProgram(new_bloom.composition_shader);

        // We only take the FBOs/textures from new_bloom
        // And we prevent new_bloom from deleting them when dropped
        // But we manually deleted its shaders above.
        // To be safe, let's just forget new_bloom entirely, but we need to know that we took its FBOs.
        // The original code was:
        // self.hdr_fbo = new_bloom.hdr_fbo; ...
        // std::mem::forget(new_bloom);
        // This leaks the shaders created by new_bloom! That's a bug in the original code too.
        // But for now, let's stick to the task: updating uniform locations.
        // Since we keep OLD shaders, the OLD locations are still valid.
        // So we don't need to update locations here.

        // However, I need to make sure I don't introduce a compile error by not copying the new fields if I used struct update syntax.
        // I am assigning fields manually.
        // So I don't need to do anything here for locations if I keep old shaders.

        // Wait, the previous code was:
        // self.hdr_fbo = new_bloom.hdr_fbo;
        // ...
        // std::mem::forget(new_bloom);

        // If I added fields to the struct, I don't need to update this method unless I'm constructing Self here?
        // I am NOT constructing Self here, I am mutating &mut self.
        // So this method is fine as is, EXCEPT that `new_bloom` now has the extra fields, so `BloomPass::new` return type changed (which I handled).
        // But `new_bloom` instance has shaders that will be leaked if I `forget` it.
        // I should probably fix the leak, but maybe out of scope?
        // Let's just leave it as is for now to minimize risk, but I need to make sure I didn't break anything.
        // The `resize` method logic:
        // 1. Delete old FBOs/textures
        // 2. Create new BloomPass (compiles shaders, creates FBOs)
        // 3. Steal FBOs/textures from new BloomPass
        // 4. Forget new BloomPass (LEAKING shaders!)

        // I will just leave this file alone for this step since I'm not changing how resize works,
        // and the locations are tied to the shaders which are preserved.

        // Don't recreate shaders, keep existing ones
        std::mem::forget(new_bloom);
    }

    /// Reloads bloom shaders from disk
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly.
    pub unsafe fn reload_shaders(&mut self) -> Result<(), String> {
        info!("🔄 Reloading bloom shaders...");

        // Try to compile new shaders
        let new_brightness = try_compile_shader_program_from_files(
            "assets/shaders/bloom/fullscreen_quad.vert.glsl",
            "assets/shaders/bloom/brightness_extract.frag.glsl",
        )?;

        let new_blur = try_compile_shader_program_from_files(
            "assets/shaders/bloom/fullscreen_quad.vert.glsl",
            "assets/shaders/bloom/gaussian_blur.frag.glsl",
        )?;

        let new_composition = try_compile_shader_program_from_files(
            "assets/shaders/bloom/fullscreen_quad.vert.glsl",
            "assets/shaders/bloom/bloom_composition.frag.glsl",
        )?;

        // Delete old shaders
        gl::DeleteProgram(self.brightness_shader);
        gl::DeleteProgram(self.blur_shader);
        gl::DeleteProgram(self.composition_shader);

        // Update with new shaders
        self.brightness_shader = new_brightness;
        self.blur_shader = new_blur;
        self.composition_shader = new_composition;

        // Update uniform locations
        self.loc_brightness_scene =
            gl::GetUniformLocation(self.brightness_shader, crate::cstr!("uSceneTexture"));
        self.loc_brightness_threshold =
            gl::GetUniformLocation(self.brightness_shader, crate::cstr!("uThreshold"));

        self.loc_blur_texture = gl::GetUniformLocation(self.blur_shader, crate::cstr!("uTexture"));
        self.loc_blur_direction =
            gl::GetUniformLocation(self.blur_shader, crate::cstr!("uDirection"));

        self.loc_comp_scene =
            gl::GetUniformLocation(self.composition_shader, crate::cstr!("uSceneTexture"));
        self.loc_comp_bloom =
            gl::GetUniformLocation(self.composition_shader, crate::cstr!("uBloomTexture"));
        self.loc_comp_intensity =
            gl::GetUniformLocation(self.composition_shader, crate::cstr!("uBloomIntensity"));

        info!("✅ Bloom shaders reloaded successfully");
        Ok(())
    }

    /// Cleans up OpenGL resources
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly.
    pub unsafe fn close(&mut self) {
        info!("🧹 Cleaning up Bloom Pass");

        gl::DeleteFramebuffers(1, &self.hdr_fbo);
        gl::DeleteTextures(1, &self.hdr_texture);
        gl::DeleteRenderbuffers(1, &self.hdr_depth_rbo);
        gl::DeleteFramebuffers(2, self.ping_pong_fbo.as_ptr());
        gl::DeleteTextures(2, self.ping_pong_textures.as_ptr());
        gl::DeleteProgram(self.brightness_shader);
        gl::DeleteProgram(self.blur_shader);
        gl::DeleteProgram(self.composition_shader);
    }
}

impl Drop for BloomPass {
    fn drop(&mut self) {
        unsafe {
            self.close();
        }
    }
}

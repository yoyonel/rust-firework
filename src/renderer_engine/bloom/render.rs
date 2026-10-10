use super::{BloomPass, BlurMethod, CellRect};
use crate::renderer_engine::gl_resource::GlTexture;
use crate::{pop_debug_group, push_debug_group};
use gl::types::*;
use log::info;

impl BloomPass {
    /// Returns the handle to the HDR scene FBO
    pub fn hdr_fbo(&self) -> GLuint {
        self.hdr_fbo.raw()
    }

    /// Begins rendering to the HDR framebuffer
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly and changes framebuffer bindings.
    pub unsafe fn begin_scene(&self) {
        gl::BindFramebuffer(gl::FRAMEBUFFER, self.hdr_fbo.raw());
        // Clear all 3 MRT attachments (Scene, Brightness, SmokeMask) simultaneously
        let all_attachments = [
            gl::COLOR_ATTACHMENT0,
            gl::COLOR_ATTACHMENT1,
            gl::COLOR_ATTACHMENT2,
        ];
        gl::DrawBuffers(3, all_attachments.as_ptr());
        gl::ClearColor(0.0, 0.0, 0.0, 0.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

        // Reset default active draw buffers to Scene (0) and Brightness (1)
        let default_attachments = [gl::COLOR_ATTACHMENT0, gl::COLOR_ATTACHMENT1];
        gl::DrawBuffers(2, default_attachments.as_ptr());
    }

    /// Ends scene rendering and applies bloom post-processing
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly and binds textures/programs.
    pub unsafe fn end_scene_and_apply_bloom(&self) {
        // Disable depth test and blending for post-processing
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::BLEND);

        // Bind the dummy VAO once for all fullscreen passes (AZDO VAO caching)
        gl::BindVertexArray(self.dummy_vao.raw());

        // 2. Blur passes - method selection
        push_debug_group!(1, "PostFX: Bloom Blur Chain");
        if self.enabled && self.intensity > 0.001 {
            match self.blur_method {
                BlurMethod::Gaussian => self.apply_gaussian_blur(),
                BlurMethod::Kawase => self.apply_kawase_blur(),
            }
        }
        pop_debug_group!();

        // Restore full resolution viewport for composition
        gl::Viewport(0, 0, self.width, self.height);

        // 3. Final composition (blend scene + bloom)
        push_debug_group!(2, "PostFX: ToneMapping & Composition");
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::UseProgram(self.composition_shader.raw());

        // Bind scene texture
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.hdr_texture.raw());

        // Bind bloom texture (result of blur in ping_pong_textures[0])
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, self.ping_pong_textures[0].raw());

        // Bind smoke mask texture (Screen-Space Backlight)
        gl::ActiveTexture(gl::TEXTURE2);
        gl::BindTexture(gl::TEXTURE_2D, self.smoke_mask_texture.raw());

        gl::Uniform1i(self.loc_tone_mapping_mode, self.tone_mapping_mode as i32);
        if self.loc_dither_enabled != -1 {
            gl::Uniform1i(
                self.loc_dither_enabled,
                if self.dither_enabled { 1 } else { 0 },
            );
        }
        if self.loc_dither_strength != -1 {
            gl::Uniform1f(self.loc_dither_strength, self.dither_strength);
        }
        if self.loc_backlight_enabled != -1 {
            gl::Uniform1i(
                self.loc_backlight_enabled,
                if self.backlight_enabled { 1 } else { 0 },
            );
        }
        if self.loc_backlight_strength != -1 {
            gl::Uniform1f(self.loc_backlight_strength, self.backlight_strength);
        }

        self.render_fullscreen_quad();
        pop_debug_group!();

        // Unbind VAO
        gl::BindVertexArray(0);

        // Re-enable depth test and blending
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
    }

    /// Renders all tone mappings to comparison textures and displays them in a 2x3 grid
    ///
    /// # Safety
    /// This function is unsafe because it calls OpenGL functions directly.
    pub unsafe fn render_comparison(&self) {
        if !self.comparison_mode {
            return;
        }

        // Disable depth test and blending for post-processing
        gl::Disable(gl::DEPTH_TEST);
        gl::Disable(gl::BLEND);

        // Bind the dummy VAO once for all fullscreen passes (AZDO VAO caching)
        gl::BindVertexArray(self.dummy_vao.raw());

        push_debug_group!(3, "PostFX: Comparison Mode");
        // Apply blur first (same as normal rendering)
        match self.blur_method {
            BlurMethod::Gaussian => self.apply_gaussian_blur(),
            BlurMethod::Kawase => self.apply_kawase_blur(),
        }

        // Restore full resolution viewport
        gl::Viewport(0, 0, self.width, self.height);

        // Step 1: Render to comparison FBO with MRT to generate all 5 tone mappings
        gl::BindFramebuffer(gl::FRAMEBUFFER, self.comparison_fbo.raw());
        gl::UseProgram(self.comparison_shader.raw());

        // Bind scene texture
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.hdr_texture.raw());

        // Bind bloom texture
        gl::ActiveTexture(gl::TEXTURE1);
        gl::BindTexture(gl::TEXTURE_2D, self.ping_pong_textures[0].raw());

        // Bind smoke mask texture (Screen-Space Backlight)
        gl::ActiveTexture(gl::TEXTURE2);
        gl::BindTexture(gl::TEXTURE_2D, self.smoke_mask_texture.raw());

        if self.loc_comparison_dither_enabled != -1 {
            gl::Uniform1i(
                self.loc_comparison_dither_enabled,
                if self.dither_enabled { 1 } else { 0 },
            );
        }
        if self.loc_comparison_dither_strength != -1 {
            gl::Uniform1f(self.loc_comparison_dither_strength, self.dither_strength);
        }
        if self.loc_comparison_backlight_enabled != -1 {
            gl::Uniform1i(
                self.loc_comparison_backlight_enabled,
                if self.backlight_enabled { 1 } else { 0 },
            );
        }
        if self.loc_comparison_backlight_strength != -1 {
            gl::Uniform1f(
                self.loc_comparison_backlight_strength,
                self.backlight_strength,
            );
        }

        self.render_fullscreen_quad();

        // Step 2: Display the 5 textures in a 2x3 grid on the main framebuffer
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::Clear(gl::COLOR_BUFFER_BIT);

        // Use passthrough shader to display textures as-is (already tone-mapped)
        gl::UseProgram(self.passthrough_shader.raw());

        // Grid layout: 2 columns, 3 rows
        let cols = 2;
        let rows = 3;
        let cell_width = self.width as f32 / cols as f32;
        let cell_height = self.height as f32 / rows as f32;

        for (i, tex) in self.comparison_textures.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;

            // Calculate cell position
            let cell_x = (col as f32 * cell_width) as i32;
            // Flip Y: OpenGL origin is bottom-left, so we need to invert row
            let cell_y = ((rows - 1 - row) as f32 * cell_height) as i32;

            // Calculate viewport with correct aspect ratio (letterbox if needed)
            let source_aspect = self.width as f32 / self.height as f32;
            let cell_aspect = cell_width / cell_height;

            let (vp_w, vp_h, vp_x_offset, vp_y_offset) = if source_aspect > cell_aspect {
                // Source is wider - letterbox top/bottom
                let h = cell_width / source_aspect;
                let y_offset = (cell_height - h) / 2.0;
                (cell_width as i32, h as i32, 0, y_offset as i32)
            } else {
                // Source is taller - letterbox left/right
                let w = cell_height * source_aspect;
                let x_offset = (cell_width - w) / 2.0;
                (w as i32, cell_height as i32, x_offset as i32, 0)
            };

            gl::Viewport(cell_x + vp_x_offset, cell_y + vp_y_offset, vp_w, vp_h);

            // Bind the comparison texture
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex.raw());

            // Render fullscreen quad for this viewport
            self.render_fullscreen_quad();
        }

        // Restore full viewport
        gl::Viewport(0, 0, self.width, self.height);

        pop_debug_group!(); // End Comparison Mode

        // Unbind VAO
        gl::BindVertexArray(0);

        // Re-enable depth test
        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::BLEND);
    }

    pub fn get_comparison_textures(&self) -> &[GlTexture; 5] {
        &self.comparison_textures
    }

    /// Returns grid layout info for displaying labels
    pub fn get_comparison_grid_info(&self) -> (Vec<CellRect>, Vec<&'static str>) {
        let labels = vec![
            "Reinhard",
            "Reinhard Extended",
            "ACES",
            "Uncharted 2",
            "Khronos PBR",
        ];

        let cols = 2;
        let rows = 3;
        let cell_width = self.width as f32 / cols as f32;
        let cell_height = self.height as f32 / rows as f32;

        let mut positions = Vec::new();
        for i in 0..5 {
            let col = i % cols;
            let row = i / cols;
            let x = col as f32 * cell_width;
            let y = row as f32 * cell_height;
            positions.push((x, y, cell_width, cell_height));
        }

        (positions, labels)
    }

    pub(crate) unsafe fn apply_gaussian_blur(&self) {
        gl::Viewport(0, 0, self.blur_width, self.blur_height);

        gl::UseProgram(self.blur_shader.raw());

        // Première passe : bright_texture -> ping_pong[1]
        gl::BindFramebuffer(gl::FRAMEBUFFER, self.ping_pong_fbo[1].raw());
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, self.bright_texture.raw());
        gl::Uniform2f(self.loc_blur_direction, 1.0, 0.0);
        self.render_fullscreen_quad();

        // Boucle ping-pong simplifiée
        for i in 0..(self.blur_iterations * 2 - 1) {
            let horizontal = i % 2 == 0;
            let read_idx = if horizontal { 1 } else { 0 };
            let write_idx = 1 - read_idx;

            gl::BindFramebuffer(gl::FRAMEBUFFER, self.ping_pong_fbo[write_idx].raw());
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.ping_pong_textures[read_idx].raw());
            gl::Uniform2f(
                self.loc_blur_direction,
                if horizontal { 0.0 } else { 1.0 },
                if horizontal { 1.0 } else { 0.0 },
            );
            self.render_fullscreen_quad();
        }
    }

    pub(crate) unsafe fn apply_kawase_blur(&self) {
        gl::Viewport(0, 0, self.blur_width, self.blur_height);

        let half_pixel_x = 0.5 / self.blur_width as f32;
        let half_pixel_y = 0.5 / self.blur_height as f32;

        // Downsample passes (3 iterations: bright -> 0, 0 -> 1, 1 -> 0)
        gl::UseProgram(self.kawase_downsample_shader.raw());

        for i in 0..3 {
            let source_texture = if i == 0 {
                self.bright_texture.raw()
            } else {
                self.ping_pong_textures[(i - 1) % 2].raw()
            };

            let target_fbo = self.ping_pong_fbo[i % 2].raw();

            gl::BindFramebuffer(gl::FRAMEBUFFER, target_fbo);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, source_texture);
            gl::Uniform2f(self.loc_kawase_down_halfpixel, half_pixel_x, half_pixel_y);

            self.render_fullscreen_quad();
        }

        // Upsample passes (4 iterations to land result cleanly in ping_pong_textures[0])
        gl::UseProgram(self.kawase_upsample_shader.raw());

        for i in 0..4 {
            let source_idx = i % 2;
            let target_idx = (i + 1) % 2;

            gl::BindFramebuffer(gl::FRAMEBUFFER, self.ping_pong_fbo[target_idx].raw());
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.ping_pong_textures[source_idx].raw());
            gl::Uniform2f(self.loc_kawase_up_halfpixel, half_pixel_x, half_pixel_y);

            self.render_fullscreen_quad();
        }
    }
    pub(crate) unsafe fn render_fullscreen_quad(&self) {
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

        // Recreate with new size
        let mut new_bloom =
            Self::new(width, height).expect("Failed to recreate bloom framebuffers");

        // Steal newly created FBOs and textures from new_bloom via std::mem::take
        // The old resources in `self` are dropped and freed via RAII upon assignment.
        self.hdr_fbo = std::mem::take(&mut new_bloom.hdr_fbo);
        self.hdr_texture = std::mem::take(&mut new_bloom.hdr_texture);
        self.bright_texture = std::mem::take(&mut new_bloom.bright_texture);
        self.hdr_depth_rbo = std::mem::take(&mut new_bloom.hdr_depth_rbo);
        self.ping_pong_fbo = [
            std::mem::take(&mut new_bloom.ping_pong_fbo[0]),
            std::mem::take(&mut new_bloom.ping_pong_fbo[1]),
        ];
        self.ping_pong_textures = [
            std::mem::take(&mut new_bloom.ping_pong_textures[0]),
            std::mem::take(&mut new_bloom.ping_pong_textures[1]),
        ];
        self.smoke_mask_fbo = std::mem::take(&mut new_bloom.smoke_mask_fbo);
        self.smoke_mask_texture = std::mem::take(&mut new_bloom.smoke_mask_texture);
        self.comparison_fbo = std::mem::take(&mut new_bloom.comparison_fbo);
        self.comparison_textures = [
            std::mem::take(&mut new_bloom.comparison_textures[0]),
            std::mem::take(&mut new_bloom.comparison_textures[1]),
            std::mem::take(&mut new_bloom.comparison_textures[2]),
            std::mem::take(&mut new_bloom.comparison_textures[3]),
            std::mem::take(&mut new_bloom.comparison_textures[4]),
        ];
        self.mask_width = new_bloom.mask_width;
        self.mask_height = new_bloom.mask_height;

        // Update blur dimensions
        self.blur_width = new_bloom.blur_width;
        self.blur_height = new_bloom.blur_height;
        // downsample_factor remains unchanged (user setting)

        // Notice: `new_bloom` drops naturally here at scope exit.
        // Its temporary shaders and dummy_vao are freed by RAII Drop, eliminating the historical shader leak.
    }
}

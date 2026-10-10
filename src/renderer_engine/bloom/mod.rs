use crate::renderer_engine::config::ToneMappingMode;
use crate::renderer_engine::gl_resource::{GlFbo, GlProgram, GlRenderbuffer, GlTexture, GlVao};
use gl::types::*;

/// Blur algorithm selection
pub enum BlurMethod {
    Gaussian, // Separable Gaussian blur (10 passes for 5 iterations)
    Kawase,   // Dual Kawase blur (6 passes: 3 down + 3 up)
}

pub type CellRect = (f32, f32, f32, f32);

/// Bloom post-processing effect
///
/// Implements an Unreal-style bloom with:
/// - HDR framebuffer for scene rendering
/// - Brightness extraction pass
/// - Separable Gaussian blur (ping-pong)
/// - Final composition with tone mapping
pub struct BloomPass {
    // Framebuffers and textures
    hdr_fbo: GlFbo,
    hdr_texture: GlTexture,
    bright_texture: GlTexture, // MRT Attachment 1
    hdr_depth_rbo: GlRenderbuffer,

    ping_pong_fbo: [GlFbo; 2],
    ping_pong_textures: [GlTexture; 2],

    // Shaders
    blur_shader: GlProgram,
    kawase_downsample_shader: GlProgram,
    kawase_upsample_shader: GlProgram,
    composition_shader: GlProgram,
    passthrough_shader: GlProgram, // For displaying comparison textures without processing

    // VAO (required for Core Profile even without VBOs)
    dummy_vao: GlVao,

    // Uniform Locations
    // loc_brightness_* removed (MRT)
    loc_blur_direction: GLint,
    loc_kawase_down_halfpixel: GLint,
    loc_kawase_up_halfpixel: GLint,
    loc_tone_mapping_mode: GLint,
    loc_dither_enabled: GLint,
    loc_dither_strength: GLint,
    loc_backlight_enabled: GLint,
    loc_backlight_strength: GLint,

    // Configuration
    pub intensity: f32,
    pub blur_iterations: u32,
    pub enabled: bool,
    pub downsample_factor: u32, // 1 = full res, 2 = half res, 4 = quarter res
    pub blur_method: BlurMethod,
    pub tone_mapping_mode: ToneMappingMode,
    pub dither_enabled: bool,
    pub dither_strength: f32,
    pub backlight_enabled: bool,
    pub backlight_strength: f32,

    // Comparison mode
    pub comparison_mode: bool,
    comparison_fbo: GlFbo,
    comparison_textures: [GlTexture; 5], // One texture per tone mapping
    comparison_shader: GlProgram,
    loc_comparison_dither_enabled: GLint,
    loc_comparison_dither_strength: GLint,
    loc_comparison_backlight_enabled: GLint,
    loc_comparison_backlight_strength: GLint,

    // Smoke mask framebuffer and texture (Screen-Space Backlight)
    pub smoke_mask_fbo: GlFbo,
    pub smoke_mask_texture: GlTexture,
    pub mask_width: i32,
    pub mask_height: i32,

    // Window size
    width: i32,
    height: i32,
    blur_width: i32, // Actual blur resolution
    blur_height: i32,
}

impl Drop for BloomPass {
    fn drop(&mut self) {
        unsafe {
            self.close();
        }
    }
}

pub mod init;
pub mod render;

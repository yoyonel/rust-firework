use crate::renderer_engine::config::ToneMappingMode;
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
    hdr_fbo: GLuint,
    hdr_texture: GLuint,
    bright_texture: GLuint, // MRT Attachment 1
    hdr_depth_rbo: GLuint,

    ping_pong_fbo: [GLuint; 2],
    ping_pong_textures: [GLuint; 2],

    // Shaders
    blur_shader: GLuint,
    kawase_downsample_shader: GLuint,
    kawase_upsample_shader: GLuint,
    composition_shader: GLuint,
    passthrough_shader: GLuint, // For displaying comparison textures without processing

    // VAO (required for Core Profile even without VBOs)
    dummy_vao: GLuint,

    // Uniform Locations
    // loc_brightness_* removed (MRT)
    loc_blur_direction: GLint,
    loc_kawase_down_halfpixel: GLint,
    loc_kawase_up_halfpixel: GLint,
    loc_tone_mapping_mode: GLint,

    // Configuration
    pub intensity: f32,
    pub blur_iterations: u32,
    pub enabled: bool,
    pub downsample_factor: u32, // 1 = full res, 2 = half res, 4 = quarter res
    pub blur_method: BlurMethod,
    pub tone_mapping_mode: ToneMappingMode,

    // Comparison mode
    pub comparison_mode: bool,
    comparison_fbo: GLuint,
    comparison_textures: [GLuint; 5], // One texture per tone mapping
    comparison_shader: GLuint,

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

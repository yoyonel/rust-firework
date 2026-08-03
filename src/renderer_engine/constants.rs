//! Single Source of Truth (SSOT) constants for the Renderer Engine.
//!
//! Centralizes OpenGL rendering configurations, bloom filter defaults, camera projection bounds,
//! geometry buffers, and audio overlay visualization constants.

use super::config::{BlurMethod, ToneMappingMode};

/// Default bloom effect enable flag.
///
/// - **Unit:** boolean
/// - **Technical meaning:** Enables or disables post-processing HDR bloom pass.
/// - **Bounds:** `true` or `false`.
/// - **System influence:** Toggles bloom framebuffer allocation and shader pass execution.
pub const DEFAULT_BLOOM_ENABLED: bool = true;

/// Default bloom intensity multiplier.
///
/// - **Unit:** dimensionless (intensity factor)
/// - **Technical meaning:** Energy multiplier applied to bright glowing regions during bloom composition.
/// - **Bounds:** `0.0` to `10.0`.
/// - **System influence:** Controls glow brightness and light bleed around intense particles.
pub const DEFAULT_BLOOM_INTENSITY: f32 = 1.5;

/// Default number of blur iterations for bloom downsample/upsample pass.
///
/// - **Unit:** passes (count)
/// - **Technical meaning:** Number of ping-pong blur passes performed on bright texture.
/// - **Bounds:** `1` to `10`.
/// - **System influence:** Higher iteration counts produce wider, smoother bloom at GPU performance cost.
pub const DEFAULT_BLOOM_ITERATIONS: u32 = 3;

/// Default downsample factor for bloom ping-pong framebuffers.
///
/// - **Unit:** dimensionless (downscaling divisor)
/// - **Technical meaning:** Resolution reduction ratio for bloom processing (e.g. 2 = half-resolution).
/// - **Bounds:** `1` to `8`.
/// - **System influence:** Reduces fillrate and memory bandwidth requirements for post-processing.
pub const DEFAULT_BLOOM_DOWNSAMPLE: u32 = 2;

/// Supported downsample divisor options for bloom pipeline GUI selector.
pub const BLOOM_DOWNSAMPLE_OPTIONS: [u32; 3] = [1, 2, 4];

/// Default blur algorithm used in bloom pass.
///
/// - **Unit:** enum (`BlurMethod`)
/// - **Technical meaning:** Selects Gaussian vs Kawase blur kernel for bloom filtering.
/// - **Bounds:** `BlurMethod::Gaussian` or `BlurMethod::Kawase`.
/// - **System influence:** Affects blur quality vs performance ratio.
pub const DEFAULT_BLOOM_BLUR_METHOD: BlurMethod = BlurMethod::Gaussian;

/// Default tone mapping algorithm for HDR to SDR conversion.
///
/// - **Unit:** enum (`ToneMappingMode`)
/// - **Technical meaning:** Curve applied to convert high-dynamic-range color to standard 8-bit display spectrum.
/// - **Bounds:** Any `ToneMappingMode` variant.
/// - **System influence:** Modulates color contrast, highlight compression, and overall visual mood.
pub const DEFAULT_TONE_MAPPING_MODE: ToneMappingMode = ToneMappingMode::KhronosPBR;

/// Default camera vertical field of view (FOV).
///
/// - **Unit:** degrees
/// - **Technical meaning:** Vertical perspective viewing angle for 3D camera projection.
/// - **Bounds:** `10.0` to `170.0` degrees.
/// - **System influence:** Controls perspective magnification and visible scene area.
pub const CAMERA_DEFAULT_FOV_DEGREES: f32 = 45.0;

/// Default camera near clipping plane distance.
///
/// - **Unit:** m (meters)
/// - **Technical meaning:** Minimum distance from camera at which geometry is rendered.
/// - **Bounds:** `0.001` to `10.0` m.
/// - **System influence:** Prevents Z-buffer precision issues near camera lens.
pub const CAMERA_DEFAULT_NEAR_PLANE: f32 = 0.1;

/// Default camera far clipping plane distance.
///
/// - **Unit:** m (meters)
/// - **Technical meaning:** Maximum distance from camera at which geometry is rendered.
/// - **Bounds:** `100.0` to `100000.0` m.
/// - **System influence:** Sets maximum render distance frustum boundary.
pub const CAMERA_DEFAULT_FAR_PLANE: f32 = 1000.0;

/// Default background clear color (RGBA).
///
/// - **Unit:** normalized RGBA (`[0.0 .. 1.0]`)
/// - **Technical meaning:** Clear color used for main OpenGL framebuffer before each frame render.
/// - **Bounds:** Each channel in `0.0` to `1.0`.
/// - **System influence:** Sets background ambient darkness for firework contrast.
pub const DEFAULT_CLEAR_COLOR: [f32; 4] = [0.05, 0.05, 0.05, 1.0];

/// Unit quad vertex coordinates for instanced billboard rendering (-0.5 to 0.5 centered).
///
/// - **Unit:** normalized NDC coordinates
/// - **Technical meaning:** Geometry positions of a centered 2D square quad (2 triangles / 4 vertices).
/// - **Bounds:** `[-0.5, 0.5]`.
/// - **System influence:** Base geometry template for particle billboard rendering.
pub const QUAD_VERTICES: [f32; 8] = [-0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, 0.5];

/// Number of vertices used to generate smooth unit circle outlines.
///
/// - **Unit:** count (vertices)
/// - **Technical meaning:** Tessellation resolution for circle outline rendering.
/// - **Bounds:** `8` to `256`.
/// - **System influence:** Dictates visual smoothness of audio ripple rings and circle indicators.
pub const CIRCLE_OUTLINE_SEGMENTS: usize = 64;

/// Circle radius scaling multiplier to align quad UV space with circle boundary.
///
/// - **Unit:** dimensionless (scale factor)
/// - **Technical meaning:** Radius multiplier matching unit quad half-extent.
/// - **Bounds:** `0.1` to `2.0`.
/// - **System influence:** Ensures circle outlines align precisely with quad texture quad edges.
pub const CIRCLE_RADIUS_MULTIPLIER: f32 = 0.5;

/// Total visual lifetime (TTL) for audio launch ripple effect.
///
/// - **Unit:** s (seconds)
/// - **Technical meaning:** Animation duration of green ripple ring expanding from rocket launch location.
/// - **Bounds:** `0.1` to `5.0` s.
/// - **System influence:** Controls display persistence of launch audio debug indicators.
pub const AUDIO_EVENT_LAUNCH_TTL_SECS: f32 = 0.55;

/// Total visual lifetime (TTL) for audio explosion ripple effect.
///
/// - **Unit:** s (seconds)
/// - **Technical meaning:** Animation duration of orange-red ripple ring expanding from explosion position.
/// - **Bounds:** `0.1` to `5.0` s.
/// - **System influence:** Controls display persistence of explosion audio debug indicators.
pub const AUDIO_EVENT_EXPLOSION_TTL_SECS: f32 = 0.75;

/// Default vertex shader path for particle rendering.
///
/// - **Unit:** file path
/// - **Technical meaning:** Path to GLSL vertex shader for point particle rendering.
/// - **Bounds:** Valid file path string.
/// - **System influence:** Shader source loaded during graphics pipeline initialization.
pub const SHADER_POINT_VERTEX_PATH: &str = "assets/shaders/point_rendering.vert.glsl";

/// Default fragment shader path for particle rendering.
pub const SHADER_POINT_FRAGMENT_PATH: &str = "assets/shaders/point_rendering.frag.glsl";

/// Vertex shader path for audio event rendering.
pub const SHADER_AUDIO_EVENT_VERTEX_PATH: &str = "assets/shaders/audio_event.vert.glsl";

/// Fragment shader path for audio event rendering.
pub const SHADER_AUDIO_EVENT_FRAGMENT_PATH: &str = "assets/shaders/audio_event.frag.glsl";

/// Vertex shader path for circle rendering.
pub const SHADER_CIRCLE_VERTEX_PATH: &str = "assets/shaders/circle.vert.glsl";

/// Fragment shader path for circle rendering.
pub const SHADER_CIRCLE_FRAGMENT_PATH: &str = "assets/shaders/circle.frag.glsl";

/// Vertex shader path for instanced textured quad particle rendering.
pub const SHADER_INSTANCED_QUAD_VERTEX_PATH: &str =
    "assets/shaders/instanced_textured_quad.vert.glsl";

/// Fragment shader path for instanced textured quad particle rendering.
pub const SHADER_INSTANCED_QUAD_FRAGMENT_PATH: &str =
    "assets/shaders/instanced_textured_quad.frag.glsl";

/// Vertex shader path for smoke rendering.
pub const SHADER_SMOKE_VERTEX_PATH: &str = "assets/shaders/smoke_instanced.vert.glsl";

/// Fragment shader path for smoke rendering.
pub const SHADER_SMOKE_FRAGMENT_PATH: &str = "assets/shaders/smoke_instanced.frag.glsl";

/// Vertex shader path for bloom fullscreen quad.
pub const SHADER_BLOOM_FULLSCREEN_QUAD_VERTEX_PATH: &str =
    "assets/shaders/bloom/fullscreen_quad.vert.glsl";

/// Fragment shader path for bloom gaussian blur.
pub const SHADER_BLOOM_GAUSSIAN_BLUR_FRAGMENT_PATH: &str =
    "assets/shaders/bloom/gaussian_blur.frag.glsl";

/// Fragment shader path for bloom kawase downsample.
pub const SHADER_BLOOM_KAWASE_DOWNSAMPLE_FRAGMENT_PATH: &str =
    "assets/shaders/bloom/kawase_downsample.frag.glsl";

/// Fragment shader path for bloom kawase upsample.
pub const SHADER_BLOOM_KAWASE_UPSAMPLE_FRAGMENT_PATH: &str =
    "assets/shaders/bloom/kawase_upsample.frag.glsl";

/// Fragment shader path for bloom composition.
pub const SHADER_BLOOM_COMPOSITION_FRAGMENT_PATH: &str =
    "assets/shaders/bloom/bloom_composition.frag.glsl";

/// Fragment shader path for bloom composition compare.
pub const SHADER_BLOOM_COMPOSITION_COMPARE_FRAGMENT_PATH: &str =
    "assets/shaders/bloom/bloom_composition_compare.frag.glsl";

/// Fragment shader path for bloom passthrough.
pub const SHADER_BLOOM_PASSTHROUGH_FRAGMENT_PATH: &str =
    "assets/shaders/bloom/passthrough.frag.glsl";

/// Vertex shader path for smoke preview post-processing (tone-mapping + gamma).
pub const SHADER_SMOKE_PREVIEW_POSTPROC_VERTEX_PATH: &str =
    "assets/shaders/smoke_preview_postproc.vert.glsl";

/// Fragment shader path for smoke preview post-processing (KhronosPBR + gamma 2.2).
pub const SHADER_SMOKE_PREVIEW_POSTPROC_FRAGMENT_PATH: &str =
    "assets/shaders/smoke_preview_postproc.frag.glsl";

/// Vertex shader path for smoke preview rocket quad (Z-rotation).
pub const SHADER_SMOKE_PREVIEW_QUAD_VERTEX_PATH: &str =
    "assets/shaders/smoke_preview_quad.vert.glsl";

/// Fragment shader path for smoke preview rocket quad (alpha test).
pub const SHADER_SMOKE_PREVIEW_QUAD_FRAGMENT_PATH: &str =
    "assets/shaders/smoke_preview_quad.frag.glsl";

/// Texture path for noise.
pub const TEXTURE_NOISE_PATH: &str = "assets/textures/noise.png";

/// Texture path for flow map.
pub const TEXTURE_FLOW_MAP_PATH: &str = "assets/textures/flowmap.png";

/// Texture path for primary spark particle.
pub const TEXTURE_PRIMARY_PARTICLE_PATH: &str =
    "assets/textures/04ddeae2-7367-45f1-87e0-361d1d242630_scaled.png";

// Renderer GUI Control Bounds
pub const SLIDER_BLOOM_INTENSITY_MIN: f32 = 0.0;
pub const SLIDER_BLOOM_INTENSITY_MAX: f32 = 10.0;

pub const SLIDER_BLOOM_ITERATIONS_MIN: u32 = 1;
pub const SLIDER_BLOOM_ITERATIONS_MAX: u32 = 10;

pub const SLIDER_BLOOM_DOWNSAMPLE_MIN: u32 = 1;
pub const SLIDER_BLOOM_DOWNSAMPLE_MAX: u32 = 8;

/// Texture path for smoke particle.
pub const TEXTURE_SMOKE_PARTICLE_PATH: &str =
    "assets/textures/toppng.com-realistic-smoke-texture-with-soft-particle-edges-png-399x385.png";

/// Global UBO binding index for OpenGL shaders.
pub const GLOBAL_UBO_BINDING_INDEX: u32 = 0;

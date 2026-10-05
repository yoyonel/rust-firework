use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub enum BlurMethod {
    Gaussian = 0,
    Kawase = 1,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub enum ToneMappingMode {
    Reinhard = 0,
    ReinhardExtended = 1,
    ACES = 2,
    Uncharted2 = 3,
    AgX = 4,
    KhronosPBR = 5,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct RendererConfig {
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_iterations: u32,
    pub bloom_downsample: u32,
    pub bloom_blur_method: BlurMethod,
    pub tone_mapping_mode: ToneMappingMode,

    // Graphical Elements Visibility Toggles
    #[serde(default = "default_true")]
    pub render_rockets: bool,
    #[serde(default = "default_true")]
    pub render_smoke: bool,
    #[serde(default = "default_true")]
    pub render_trails: bool,
    #[serde(default = "default_true")]
    pub render_explosions: bool,

    // Master Volumetric Lighting Toggle
    #[serde(default = "default_true")]
    pub volumetric_lighting_enabled: bool,

    // Volumetric Smoke Lighting (Participating Media)
    #[serde(default = "default_true")]
    pub smoke_lighting_enabled: bool,
    #[serde(default = "default_smoke_scattering_intensity")]
    pub smoke_scattering_intensity: f32,
    #[serde(default = "default_smoke_ambient_flash")]
    pub smoke_ambient_flash: f32,

    // Atmospheric Sky Haze (Background Participating Media)
    #[serde(default = "default_true")]
    pub sky_haze_enabled: bool,
    #[serde(default = "default_sky_haze_intensity")]
    pub sky_haze_intensity: f32,
    #[serde(default = "default_sky_haze_ambient_flash")]
    pub sky_haze_ambient_flash: f32,

    // Post-Process Dither Anti-Banding (Interleaved Gradient Noise)
    #[serde(default = "default_dither_enabled")]
    pub dither_enabled: bool,
    #[serde(default = "default_dither_strength")]
    pub dither_strength: f32,

    // Volumetric Lighting Temporal Stabilization (§5 ADR)
    #[serde(default = "default_volumetric_lighting_hysteresis_enabled")]
    pub volumetric_lighting_hysteresis_enabled: bool,
    #[serde(default = "default_volumetric_lighting_fade_in_ms")]
    pub volumetric_lighting_fade_in_ms: f32,
    #[serde(default = "default_volumetric_lighting_radius")]
    pub volumetric_lighting_radius: f32,
    #[serde(default = "default_volumetric_lighting_decay_rate")]
    pub volumetric_lighting_decay_rate: f32,
    #[serde(default = "default_volumetric_lighting_radius_expansion")]
    pub volumetric_lighting_radius_expansion: f32,
    #[serde(default = "default_volumetric_lighting_flash_max_cap")]
    pub volumetric_lighting_flash_max_cap: f32,
    #[serde(default = "default_volumetric_lighting_debug")]
    pub volumetric_lighting_debug: bool,

    // Atmospheric Sky Haze (§4.2 ADR)
    #[serde(default = "default_sky_haze_falloff")]
    pub sky_haze_falloff: f32,

    // Screen-Space Smoke Backlight (§4.1 ADR)
    #[serde(default = "default_backlight_enabled")]
    pub backlight_enabled: bool,
    #[serde(default = "default_backlight_strength")]
    pub backlight_strength: f32,
}

fn default_true() -> bool {
    true
}

fn default_smoke_scattering_intensity() -> f32 {
    constants::DEFAULT_SMOKE_SCATTERING_INTENSITY
}

fn default_smoke_ambient_flash() -> f32 {
    constants::DEFAULT_SMOKE_AMBIENT_FLASH
}

fn default_sky_haze_intensity() -> f32 {
    constants::DEFAULT_SKY_HAZE_INTENSITY
}

fn default_sky_haze_ambient_flash() -> f32 {
    constants::DEFAULT_SKY_HAZE_AMBIENT_FLASH
}

fn default_dither_enabled() -> bool {
    constants::DEFAULT_DITHER_ENABLED
}

fn default_dither_strength() -> f32 {
    constants::DEFAULT_DITHER_STRENGTH
}

fn default_volumetric_lighting_hysteresis_enabled() -> bool {
    constants::DEFAULT_VOLUMETRIC_LIGHTING_HYSTERESIS_ENABLED
}

fn default_volumetric_lighting_fade_in_ms() -> f32 {
    constants::DEFAULT_VOLUMETRIC_LIGHTING_FADE_IN_MS
}

fn default_volumetric_lighting_radius() -> f32 {
    constants::DEFAULT_VOLUMETRIC_LIGHT_RADIUS
}

fn default_volumetric_lighting_decay_rate() -> f32 {
    constants::DEFAULT_VOLUMETRIC_LIGHT_DECAY_RATE
}

fn default_volumetric_lighting_radius_expansion() -> f32 {
    constants::DEFAULT_VOLUMETRIC_LIGHT_RADIUS_EXPANSION
}

fn default_volumetric_lighting_flash_max_cap() -> f32 {
    constants::DEFAULT_VOLUMETRIC_FLASH_MAX_CAP
}

fn default_volumetric_lighting_debug() -> bool {
    constants::DEFAULT_VOLUMETRIC_LIGHTING_DEBUG
}

fn default_sky_haze_falloff() -> f32 {
    constants::DEFAULT_SKY_HAZE_FALLOFF
}

fn default_backlight_enabled() -> bool {
    constants::DEFAULT_BACKLIGHT_ENABLED
}

fn default_backlight_strength() -> f32 {
    constants::DEFAULT_BACKLIGHT_STRENGTH
}

use crate::renderer_engine::constants;

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            bloom_enabled: constants::DEFAULT_BLOOM_ENABLED,
            bloom_intensity: constants::DEFAULT_BLOOM_INTENSITY,
            bloom_iterations: constants::DEFAULT_BLOOM_ITERATIONS,
            bloom_downsample: constants::DEFAULT_BLOOM_DOWNSAMPLE,
            bloom_blur_method: constants::DEFAULT_BLOOM_BLUR_METHOD,
            tone_mapping_mode: constants::DEFAULT_TONE_MAPPING_MODE,
            render_rockets: true,
            render_smoke: true,
            render_trails: true,
            render_explosions: true,
            volumetric_lighting_enabled: constants::DEFAULT_VOLUMETRIC_LIGHTING_ENABLED,
            smoke_lighting_enabled: constants::DEFAULT_SMOKE_LIGHTING_ENABLED,
            smoke_scattering_intensity: constants::DEFAULT_SMOKE_SCATTERING_INTENSITY,
            smoke_ambient_flash: constants::DEFAULT_SMOKE_AMBIENT_FLASH,
            sky_haze_enabled: constants::DEFAULT_SKY_HAZE_ENABLED,
            sky_haze_intensity: constants::DEFAULT_SKY_HAZE_INTENSITY,
            sky_haze_ambient_flash: constants::DEFAULT_SKY_HAZE_AMBIENT_FLASH,
            sky_haze_falloff: constants::DEFAULT_SKY_HAZE_FALLOFF,
            dither_enabled: constants::DEFAULT_DITHER_ENABLED,
            dither_strength: constants::DEFAULT_DITHER_STRENGTH,
            volumetric_lighting_hysteresis_enabled:
                constants::DEFAULT_VOLUMETRIC_LIGHTING_HYSTERESIS_ENABLED,
            volumetric_lighting_fade_in_ms: constants::DEFAULT_VOLUMETRIC_LIGHTING_FADE_IN_MS,
            volumetric_lighting_radius: constants::DEFAULT_VOLUMETRIC_LIGHT_RADIUS,
            volumetric_lighting_decay_rate: constants::DEFAULT_VOLUMETRIC_LIGHT_DECAY_RATE,
            volumetric_lighting_radius_expansion:
                constants::DEFAULT_VOLUMETRIC_LIGHT_RADIUS_EXPANSION,
            volumetric_lighting_flash_max_cap: constants::DEFAULT_VOLUMETRIC_FLASH_MAX_CAP,
            volumetric_lighting_debug: constants::DEFAULT_VOLUMETRIC_LIGHTING_DEBUG,
            backlight_enabled: constants::DEFAULT_BACKLIGHT_ENABLED,
            backlight_strength: constants::DEFAULT_BACKLIGHT_STRENGTH,
        }
    }
}

impl RendererConfig {
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        let text = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_renderer_config_defaults() {
        let config = RendererConfig::default();
        assert!(config.bloom_enabled);
        assert_eq!(config.bloom_intensity, 1.5);
        assert_eq!(config.bloom_iterations, 3);
        assert_eq!(config.bloom_downsample, 2);
        assert_eq!(config.bloom_blur_method, BlurMethod::Gaussian);
        assert_eq!(config.tone_mapping_mode, ToneMappingMode::KhronosPBR);
        assert!(config.render_rockets);
        assert!(config.render_smoke);
        assert!(config.render_trails);
        assert!(config.render_explosions);
        assert!(config.volumetric_lighting_enabled);
        assert!(config.smoke_lighting_enabled);
        assert_eq!(
            config.smoke_scattering_intensity,
            constants::DEFAULT_SMOKE_SCATTERING_INTENSITY
        );
        assert_eq!(
            config.smoke_ambient_flash,
            constants::DEFAULT_SMOKE_AMBIENT_FLASH
        );
        assert!(config.sky_haze_enabled);
        assert_eq!(
            config.sky_haze_intensity,
            constants::DEFAULT_SKY_HAZE_INTENSITY
        );
        assert_eq!(
            config.sky_haze_ambient_flash,
            constants::DEFAULT_SKY_HAZE_AMBIENT_FLASH
        );
        assert_eq!(config.sky_haze_falloff, constants::DEFAULT_SKY_HAZE_FALLOFF);
        assert!(config.dither_enabled);
        assert_eq!(config.dither_strength, constants::DEFAULT_DITHER_STRENGTH);
        assert!(config.volumetric_lighting_hysteresis_enabled);
        assert_eq!(
            config.volumetric_lighting_fade_in_ms,
            constants::DEFAULT_VOLUMETRIC_LIGHTING_FADE_IN_MS
        );
        assert_eq!(
            config.volumetric_lighting_radius,
            constants::DEFAULT_VOLUMETRIC_LIGHT_RADIUS
        );
        assert_eq!(
            config.volumetric_lighting_decay_rate,
            constants::DEFAULT_VOLUMETRIC_LIGHT_DECAY_RATE
        );
        assert_eq!(
            config.volumetric_lighting_radius_expansion,
            constants::DEFAULT_VOLUMETRIC_LIGHT_RADIUS_EXPANSION
        );
        assert_eq!(
            config.volumetric_lighting_flash_max_cap,
            constants::DEFAULT_VOLUMETRIC_FLASH_MAX_CAP
        );
        assert!(config.backlight_enabled);
        assert_eq!(
            config.backlight_strength,
            constants::DEFAULT_BACKLIGHT_STRENGTH
        );
    }

    #[test]
    fn test_renderer_config_file_persistence() -> anyhow::Result<()> {
        let temp_file = NamedTempFile::new()?;
        let file_path = temp_file.path().to_str().unwrap();

        let config = RendererConfig {
            bloom_enabled: false,
            bloom_intensity: 4.2,
            bloom_iterations: 5,
            bloom_downsample: 4,
            bloom_blur_method: BlurMethod::Kawase,
            tone_mapping_mode: ToneMappingMode::ACES,
            render_rockets: false,
            render_smoke: true,
            render_trails: false,
            render_explosions: true,
            volumetric_lighting_enabled: false,
            smoke_lighting_enabled: false,
            smoke_scattering_intensity: 2.5,
            smoke_ambient_flash: 0.8,
            sky_haze_enabled: false,
            sky_haze_intensity: 1.2,
            sky_haze_ambient_flash: 0.5,
            sky_haze_falloff: 5.5,
            dither_enabled: false,
            dither_strength: 0.85,
            volumetric_lighting_hysteresis_enabled: false,
            volumetric_lighting_fade_in_ms: 75.0,
            volumetric_lighting_radius: 220.0,
            volumetric_lighting_decay_rate: 0.91,
            volumetric_lighting_radius_expansion: 1.005,
            volumetric_lighting_flash_max_cap: 0.60,
            volumetric_lighting_debug: true,
            backlight_enabled: false,
            backlight_strength: 2.1,
        };

        config.save_to_file(file_path)?;

        let loaded = RendererConfig::from_file(file_path)?;
        assert_eq!(loaded.bloom_enabled, config.bloom_enabled);
        assert_eq!(loaded.bloom_intensity, config.bloom_intensity);
        assert_eq!(loaded.bloom_iterations, config.bloom_iterations);
        assert_eq!(loaded.bloom_downsample, config.bloom_downsample);
        assert_eq!(loaded.bloom_blur_method, config.bloom_blur_method);
        assert_eq!(loaded.tone_mapping_mode, config.tone_mapping_mode);
        assert!(!loaded.render_rockets);
        assert!(loaded.render_smoke);
        assert!(!loaded.render_trails);
        assert!(loaded.render_explosions);
        assert!(!loaded.volumetric_lighting_enabled);
        assert!(!loaded.smoke_lighting_enabled);
        assert_eq!(
            loaded.smoke_scattering_intensity,
            config.smoke_scattering_intensity
        );
        assert_eq!(loaded.smoke_ambient_flash, config.smoke_ambient_flash);
        assert!(!loaded.sky_haze_enabled);
        assert_eq!(loaded.sky_haze_intensity, config.sky_haze_intensity);
        assert_eq!(loaded.sky_haze_ambient_flash, config.sky_haze_ambient_flash);
        assert_eq!(loaded.sky_haze_falloff, config.sky_haze_falloff);
        assert_eq!(loaded.dither_enabled, config.dither_enabled);
        assert_eq!(loaded.dither_strength, config.dither_strength);
        assert_eq!(
            loaded.volumetric_lighting_hysteresis_enabled,
            config.volumetric_lighting_hysteresis_enabled
        );
        assert_eq!(
            loaded.volumetric_lighting_fade_in_ms,
            config.volumetric_lighting_fade_in_ms
        );
        assert_eq!(
            loaded.volumetric_lighting_radius,
            config.volumetric_lighting_radius
        );
        assert_eq!(
            loaded.volumetric_lighting_decay_rate,
            config.volumetric_lighting_decay_rate
        );
        assert_eq!(
            loaded.volumetric_lighting_radius_expansion,
            config.volumetric_lighting_radius_expansion
        );
        assert_eq!(
            loaded.volumetric_lighting_flash_max_cap,
            config.volumetric_lighting_flash_max_cap
        );
        assert_eq!(
            loaded.volumetric_lighting_debug,
            config.volumetric_lighting_debug
        );
        assert_eq!(loaded.backlight_enabled, config.backlight_enabled);
        assert_eq!(loaded.backlight_strength, config.backlight_strength);
        Ok(())
    }

    #[test]
    fn test_blur_method_and_tonemapping_enums() {
        assert_ne!(BlurMethod::Gaussian, BlurMethod::Kawase);
        assert_ne!(ToneMappingMode::ACES, ToneMappingMode::KhronosPBR);
    }
}

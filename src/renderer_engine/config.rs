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
}

fn default_true() -> bool {
    true
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

        Ok(())
    }

    #[test]
    fn test_blur_method_and_tonemapping_enums() {
        assert_ne!(BlurMethod::Gaussian, BlurMethod::Kawase);
        assert_ne!(ToneMappingMode::ACES, ToneMappingMode::KhronosPBR);
    }
}

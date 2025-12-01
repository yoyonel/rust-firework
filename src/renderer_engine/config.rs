use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
pub struct RendererConfig {
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_iterations: u32,
    pub bloom_downsample: u32,
    pub bloom_blur_method: BlurMethod,
    pub tone_mapping_mode: ToneMappingMode,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            bloom_enabled: true,
            bloom_intensity: 2.0,
            bloom_iterations: 5,
            bloom_downsample: 2,
            bloom_blur_method: BlurMethod::Gaussian,
            tone_mapping_mode: ToneMappingMode::ACES,
        }
    }
}

impl RendererConfig {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

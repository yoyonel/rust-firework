use crate::audio_engine::settings::AudioEngineSettings;
use crate::audio_engine::FireworksAudioConfig;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct AudioConfig {
    pub rocket_path: PathBuf,
    pub explosion_path: PathBuf,

    // Dans TOML, un tuple (f32, f32) est représenté par un tableau [x, y]
    pub listener_pos: [f32; 2],

    pub sample_rate: u32,
    pub block_size: usize,
    pub max_voices: usize,

    // Permet de surcharger optionnellement les paramètres DSP fins dans le TOML
    // S'il est absent du fichier, on utilisera AudioEngineSettings::default()
    #[serde(default)]
    pub settings: Option<AudioEngineSettings>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            rocket_path: PathBuf::from("assets/sounds/rocket.wav"),
            explosion_path: PathBuf::from("assets/sounds/explosion.wav"),
            listener_pos: [0.0, 0.0],
            sample_rate: 48000,
            block_size: 512,
            max_voices: 64,
            settings: None,
        }
    }
}

impl AudioConfig {
    /// Charge la configuration depuis un fichier TOML.
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    /// Convertit cette configuration de fichier vers la structure de build du moteur (`FireworksAudioConfig`).
    /// Permet de plafonner dynamiquement `max_voices` en fonction de la configuration physique.
    pub fn to_engine_config(&self, max_physic_rockets: usize) -> FireworksAudioConfig {
        let settings = self.settings.clone().unwrap_or_default();

        FireworksAudioConfig {
            rocket_path: self.rocket_path.to_string_lossy().into_owned(),
            explosion_path: self.explosion_path.to_string_lossy().into_owned(),
            listener_pos: glam::Vec2::new(self.listener_pos[0], self.listener_pos[1]),
            sample_rate: self.sample_rate,
            block_size: self.block_size,
            // On conserve votre logique de plafonnement dynamique
            max_voices: std::cmp::min(self.max_voices, max_physic_rockets),
            settings,
            doppler_receiver: None, // NOUVEAU : On initialise le champ à None par défaut
        }
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PhysicConfig {
    pub max_rockets: usize,
    pub particles_per_explosion: usize,
    pub particles_per_trail: usize,

    pub rocket_interval_mean: f32,
    pub rocket_interval_variation: f32,
    pub rocket_max_next_interval: f32,

    pub spawn_rocket_margin: f32,
    pub spawn_rocket_vertical_angle: f32,
    pub spawn_rocket_angle_variation: f32,
    pub spawn_rocket_min_speed: f32,
    pub spawn_rocket_max_speed: f32,

    pub explosion_threshold: f32,

    pub gravity: f32,
    pub initial_rocket_speed: f32,
    pub explosion_min_vel: f32,
    pub explosion_max_vel: f32,

    /// Temps d'anticipation pour le lancement de la fusée (ms)
    pub audio_launch_anticipation_ms: f32,

    /// Temps d'anticipation pour l'explosion de la fusée (ms)
    pub audio_explosion_anticipation_ms: f32,
}

use crate::physic_engine::constants;

impl Default for PhysicConfig {
    fn default() -> Self {
        Self {
            max_rockets: constants::DEFAULT_MAX_ROCKETS,
            particles_per_explosion: constants::DEFAULT_PARTICLES_PER_EXPLOSION,
            particles_per_trail: constants::DEFAULT_PARTICLES_PER_TRAIL,
            rocket_interval_mean: constants::DEFAULT_ROCKET_INTERVAL_MEAN,
            rocket_interval_variation: constants::DEFAULT_ROCKET_INTERVAL_VARIATION,
            rocket_max_next_interval: constants::DEFAULT_ROCKET_MAX_NEXT_INTERVAL,
            spawn_rocket_margin: constants::DEFAULT_SPAWN_ROCKET_MARGIN,
            spawn_rocket_vertical_angle: constants::DEFAULT_SPAWN_ROCKET_VERTICAL_ANGLE,
            spawn_rocket_angle_variation: constants::DEFAULT_SPAWN_ROCKET_ANGLE_VARIATION,
            spawn_rocket_min_speed: constants::DEFAULT_SPAWN_ROCKET_MIN_SPEED,
            spawn_rocket_max_speed: constants::DEFAULT_SPAWN_ROCKET_MAX_SPEED,
            explosion_threshold: constants::DEFAULT_EXPLOSION_THRESHOLD_SPEED,
            gravity: constants::DEFAULT_GRAVITY,
            initial_rocket_speed: constants::DEFAULT_INITIAL_ROCKET_SPEED,
            explosion_min_vel: constants::DEFAULT_EXPLOSION_MIN_VELOCITY,
            explosion_max_vel: constants::DEFAULT_EXPLOSION_MAX_VELOCITY,
            audio_launch_anticipation_ms: constants::DEFAULT_AUDIO_LAUNCH_ANTICIPATION_MS,
            audio_explosion_anticipation_ms: constants::DEFAULT_AUDIO_EXPLOSION_ANTICIPATION_MS,
        }
    }
}

impl PhysicConfig {
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

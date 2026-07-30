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

impl Default for PhysicConfig {
    fn default() -> Self {
        Self {
            max_rockets: 1024,
            particles_per_explosion: 256,
            particles_per_trail: 64,
            rocket_interval_mean: 1.0 * 0.025,
            rocket_interval_variation: 0.75 * 0.025,
            rocket_max_next_interval: 0.025,
            spawn_rocket_margin: 50.0,
            spawn_rocket_vertical_angle: std::f32::consts::FRAC_PI_2, // π/2 radians
            spawn_rocket_angle_variation: 0.3, // Amplitude de variation autour de la verticale (±0.3 rad ≈ ±17°)
            spawn_rocket_min_speed: 350.0,
            spawn_rocket_max_speed: 500.0,
            explosion_threshold: 50.0, // en m/s
            gravity: -200.0,
            initial_rocket_speed: 100.0,
            explosion_min_vel: 60.0,
            explosion_max_vel: 200.0,
            audio_launch_anticipation_ms: 25.0,
            audio_explosion_anticipation_ms: 25.0,
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

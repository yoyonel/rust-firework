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

    /// Multiplicateur dynamique de vélocité d'expansion des explosions d'images (1.0 à 10.0)
    #[serde(default = "default_explosion_velocity_boost")]
    pub explosion_velocity_boost: f32,

    /// Temps d'anticipation pour le lancement de la fusée (ms)
    pub audio_launch_anticipation_ms: f32,

    /// Temps d'anticipation pour l'explosion de la fusée (ms)
    pub audio_explosion_anticipation_ms: f32,

    /// Taux d'émission de particules de fumée (particules / seconde / fusée)
    #[serde(default = "default_smoke_spawn_rate")]
    pub smoke_spawn_rate: f32,

    /// Taille initiale des particules de fumée
    #[serde(default = "default_smoke_initial_size")]
    pub smoke_initial_size: f32,

    /// Multiplicateur d'expansion de la fumée au cours de sa vie
    #[serde(default = "default_smoke_growth_rate_multiplier")]
    pub smoke_growth_rate_multiplier: f32,

    /// Durée de dissipation / fondu de la fumée (secondes)
    #[serde(default = "default_smoke_fade_duration")]
    pub smoke_fade_duration: f32,

    /// Capacité maximale de particules de fumée simultanées
    #[serde(default = "default_max_smoke_particles")]
    pub max_smoke_particles: usize,

    /// Intensité / luminosité dynamique du mélange de fumée (0.0 à 2.0)
    #[serde(default = "default_smoke_intensity")]
    pub smoke_intensity: f32,

    /// Mode de couleur de la fumée (RocketColor par défaut, ou Custom)
    #[serde(default)]
    pub smoke_color_mode: SmokeColorMode,

    /// Couleur personnalisée de la fumée lorsque smoke_color_mode est Custom
    #[serde(default = "default_smoke_custom_color")]
    pub smoke_custom_color: [f32; 3],

    /// Intensité de la couleur d'origine/héritée de la fusée à appliquer à la fumée (0.0 à 2.0, default 1.0)
    #[serde(default = "default_smoke_inherited_color_intensity")]
    pub smoke_inherited_color_intensity: f32,

    /// Indique si l'effet d'érosion alpha (dissolution de bruit) est activé
    #[serde(default = "default_smoke_erosion_enabled")]
    pub smoke_erosion_enabled: bool,

    /// Multiplicateur d'agressivité/vitesse d'érosion de la fumée (0.0 à 2.0)
    #[serde(default = "default_smoke_erosion_scale")]
    pub smoke_erosion_scale: f32,

    /// Largeur de la bordure incandescente/bruit pour l'érosion alpha (dissolution)
    #[serde(default = "default_smoke_erosion_edge_width")]
    pub smoke_erosion_edge_width: f32,

    /// Couleur de la bordure incandescente lors de l'érosion de la fumée (RGB)
    #[serde(default = "default_smoke_erosion_edge_color")]
    pub smoke_erosion_edge_color: [f32; 3],

    /// Force/Intensité de la distortion Flow Map UV (0.0 à 1.0)
    #[serde(default = "default_flow_distortion_strength")]
    pub flow_distortion_strength: f32,

    /// Vitesse de l'animation de tourbillonnement Flow Map (0.0 à 5.0)
    #[serde(default = "default_flow_animation_speed")]
    pub flow_animation_speed: f32,
}

pub use crate::domain_contracts::SmokeColorMode;

use crate::physic_engine::constants;

fn default_explosion_velocity_boost() -> f32 {
    constants::DEFAULT_EXPLOSION_VELOCITY_BOOST
}
fn default_smoke_erosion_enabled() -> bool {
    constants::DEFAULT_SMOKE_EROSION_ENABLED
}
fn default_smoke_erosion_scale() -> f32 {
    constants::DEFAULT_SMOKE_EROSION_SCALE
}
fn default_flow_distortion_strength() -> f32 {
    constants::DEFAULT_FLOW_DISTORTION_STRENGTH
}
fn default_flow_animation_speed() -> f32 {
    constants::DEFAULT_FLOW_ANIMATION_SPEED
}
fn default_smoke_spawn_rate() -> f32 {
    constants::DEFAULT_SMOKE_SPAWN_RATE
}
fn default_smoke_initial_size() -> f32 {
    constants::DEFAULT_SMOKE_INITIAL_SIZE
}
fn default_smoke_growth_rate_multiplier() -> f32 {
    constants::DEFAULT_SMOKE_GROWTH_RATE_MULTIPLIER
}
fn default_smoke_fade_duration() -> f32 {
    constants::DEFAULT_SMOKE_FADE_DURATION
}
fn default_max_smoke_particles() -> usize {
    constants::DEFAULT_MAX_SMOKE_PARTICLES
}
fn default_smoke_intensity() -> f32 {
    constants::DEFAULT_SMOKE_INTENSITY
}
fn default_smoke_custom_color() -> [f32; 3] {
    constants::DEFAULT_SMOKE_CUSTOM_COLOR
}
fn default_smoke_inherited_color_intensity() -> f32 {
    constants::DEFAULT_SMOKE_INHERITED_COLOR_INTENSITY
}
fn default_smoke_erosion_edge_width() -> f32 {
    constants::DEFAULT_SMOKE_EROSION_EDGE_WIDTH
}
fn default_smoke_erosion_edge_color() -> [f32; 3] {
    constants::DEFAULT_SMOKE_EROSION_EDGE_COLOR
}

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
            explosion_velocity_boost: constants::DEFAULT_EXPLOSION_VELOCITY_BOOST,
            audio_launch_anticipation_ms: constants::DEFAULT_AUDIO_LAUNCH_ANTICIPATION_MS,
            audio_explosion_anticipation_ms: constants::DEFAULT_AUDIO_EXPLOSION_ANTICIPATION_MS,
            smoke_spawn_rate: constants::DEFAULT_SMOKE_SPAWN_RATE,
            smoke_initial_size: constants::DEFAULT_SMOKE_INITIAL_SIZE,
            smoke_growth_rate_multiplier: constants::DEFAULT_SMOKE_GROWTH_RATE_MULTIPLIER,
            smoke_fade_duration: constants::DEFAULT_SMOKE_FADE_DURATION,
            max_smoke_particles: constants::DEFAULT_MAX_SMOKE_PARTICLES,
            smoke_intensity: constants::DEFAULT_SMOKE_INTENSITY,
            smoke_color_mode: SmokeColorMode::default(),
            smoke_custom_color: constants::DEFAULT_SMOKE_CUSTOM_COLOR,
            smoke_inherited_color_intensity: constants::DEFAULT_SMOKE_INHERITED_COLOR_INTENSITY,
            smoke_erosion_enabled: constants::DEFAULT_SMOKE_EROSION_ENABLED,
            smoke_erosion_scale: constants::DEFAULT_SMOKE_EROSION_SCALE,
            smoke_erosion_edge_width: constants::DEFAULT_SMOKE_EROSION_EDGE_WIDTH,
            smoke_erosion_edge_color: constants::DEFAULT_SMOKE_EROSION_EDGE_COLOR,
            flow_distortion_strength: constants::DEFAULT_FLOW_DISTORTION_STRENGTH,
            flow_animation_speed: constants::DEFAULT_FLOW_ANIMATION_SPEED,
        }
    }
}

impl PhysicConfig {
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

impl From<&PhysicConfig> for crate::domain_contracts::SmokeConfigSnapshot {
    fn from(c: &PhysicConfig) -> Self {
        Self {
            smoke_spawn_rate: c.smoke_spawn_rate,
            smoke_initial_size: c.smoke_initial_size,
            smoke_growth_rate_multiplier: c.smoke_growth_rate_multiplier,
            smoke_fade_duration: c.smoke_fade_duration,
            max_smoke_particles: c.max_smoke_particles,
            smoke_intensity: c.smoke_intensity,
            smoke_color_mode: c.smoke_color_mode,
            smoke_custom_color: c.smoke_custom_color,
            smoke_inherited_color_intensity: c.smoke_inherited_color_intensity,
            smoke_erosion_enabled: c.smoke_erosion_enabled,
            smoke_erosion_scale: c.smoke_erosion_scale,
            smoke_erosion_edge_width: c.smoke_erosion_edge_width,
            smoke_erosion_edge_color: c.smoke_erosion_edge_color,
            flow_distortion_strength: c.flow_distortion_strength,
            flow_animation_speed: c.flow_animation_speed,
        }
    }
}

impl From<&PhysicConfig> for crate::domain_contracts::PhysicConfigSnapshot {
    fn from(c: &PhysicConfig) -> Self {
        Self {
            max_rockets: c.max_rockets,
            particles_per_explosion: c.particles_per_explosion,
            particles_per_trail: c.particles_per_trail,
            rocket_interval_mean: c.rocket_interval_mean,
            rocket_interval_variation: c.rocket_interval_variation,
            rocket_max_next_interval: c.rocket_max_next_interval,
            spawn_rocket_margin: c.spawn_rocket_margin,
            spawn_rocket_vertical_angle: c.spawn_rocket_vertical_angle,
            spawn_rocket_angle_variation: c.spawn_rocket_angle_variation,
            spawn_rocket_min_speed: c.spawn_rocket_min_speed,
            spawn_rocket_max_speed: c.spawn_rocket_max_speed,
            explosion_threshold: c.explosion_threshold,
            gravity: c.gravity,
            initial_rocket_speed: c.initial_rocket_speed,
            explosion_min_vel: c.explosion_min_vel,
            explosion_max_vel: c.explosion_max_vel,
            explosion_velocity_boost: c.explosion_velocity_boost,
            audio_launch_anticipation_ms: c.audio_launch_anticipation_ms,
            audio_explosion_anticipation_ms: c.audio_explosion_anticipation_ms,
            smoke: c.into(),
        }
    }
}

impl From<&crate::domain_contracts::PhysicConfigSnapshot> for PhysicConfig {
    fn from(s: &crate::domain_contracts::PhysicConfigSnapshot) -> Self {
        Self {
            max_rockets: s.max_rockets,
            particles_per_explosion: s.particles_per_explosion,
            particles_per_trail: s.particles_per_trail,
            rocket_interval_mean: s.rocket_interval_mean,
            rocket_interval_variation: s.rocket_interval_variation,
            rocket_max_next_interval: s.rocket_max_next_interval,
            spawn_rocket_margin: s.spawn_rocket_margin,
            spawn_rocket_vertical_angle: s.spawn_rocket_vertical_angle,
            spawn_rocket_angle_variation: s.spawn_rocket_angle_variation,
            spawn_rocket_min_speed: s.spawn_rocket_min_speed,
            spawn_rocket_max_speed: s.spawn_rocket_max_speed,
            explosion_threshold: s.explosion_threshold,
            gravity: s.gravity,
            initial_rocket_speed: s.initial_rocket_speed,
            explosion_min_vel: s.explosion_min_vel,
            explosion_max_vel: s.explosion_max_vel,
            explosion_velocity_boost: s.explosion_velocity_boost,
            audio_launch_anticipation_ms: s.audio_launch_anticipation_ms,
            audio_explosion_anticipation_ms: s.audio_explosion_anticipation_ms,
            smoke_spawn_rate: s.smoke.smoke_spawn_rate,
            smoke_initial_size: s.smoke.smoke_initial_size,
            smoke_growth_rate_multiplier: s.smoke.smoke_growth_rate_multiplier,
            smoke_fade_duration: s.smoke.smoke_fade_duration,
            max_smoke_particles: s.smoke.max_smoke_particles,
            smoke_intensity: s.smoke.smoke_intensity,
            smoke_color_mode: s.smoke.smoke_color_mode,
            smoke_custom_color: s.smoke.smoke_custom_color,
            smoke_inherited_color_intensity: s.smoke.smoke_inherited_color_intensity,
            smoke_erosion_enabled: s.smoke.smoke_erosion_enabled,
            smoke_erosion_scale: s.smoke.smoke_erosion_scale,
            smoke_erosion_edge_width: s.smoke.smoke_erosion_edge_width,
            smoke_erosion_edge_color: s.smoke.smoke_erosion_edge_color,
            flow_distortion_strength: s.smoke.flow_distortion_strength,
            flow_animation_speed: s.smoke.flow_animation_speed,
        }
    }
}

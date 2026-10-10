//! Domain contracts (enums & read-only state reader traits) decoupling UI/UX from core engines.
//!
//! Designed for Data-Oriented, continuous-flow architecture with strict zero-allocation constraints
//! and optimized memory layout for cache-friendly command queue iteration.

use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Identifiant d'un effet DSP audio, encodé comme un bit dans le masque `u32`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioEffect {
    /// Spatialisation binaurale ITD + ILD (retard inter-aural + différence de niveau).
    Binaural = 1 << 0,
    /// Panoramique stéréo standard gauche/droite.
    Panning = 1 << 1,
    /// Atténuation de l'amplitude en fonction de la distance source→auditeur.
    DistanceAtten = 1 << 2,
    /// Filtre passe-bas IIR du 1er ordre, fréquence de coupure dépendante de la distance.
    LowPassFilter = 1 << 3,
    /// Décalage de hauteur (playback_rate) simulant l'effet Doppler pour les sources mobiles.
    Doppler = 1 << 4,
    /// Rampe linéaire de volume en début (fade-in) et fin (fade-out) de chaque son.
    FadeInOut = 1 << 5,
    /// Interpolation linéaire (LERP) des gains gauche/droite entre blocs (anti-zipper).
    GainLerp = 1 << 6,
    /// Normalisation douce et contrôle du gain appliqués en sortie globale (limiteur doux).
    Normalization = 1 << 7,
    /// Bus spatial 2D (Harmoniques Circulaires / Ambisonics 2D W, X, Y pré-accumulés).
    SpatialBus = 1 << 8,
    /// Réverbération spatiale globale sur bus unique (Feedback Delay Network / Schroeder Reverb).
    SpatialReverb = 1 << 9,
    /// Décodeur HRTF binaural sur Bus Spatial 2D (Overlap-Save FFT sur enceintes virtuelles).
    HrtfBus = 1 << 10,
}

impl AudioEffect {
    /// Table de correspondance nom textuel ↔ variant.
    /// Utilisée pour l'autocomplétion de la console et le parsing des commandes.
    pub fn all_names() -> &'static [(&'static str, AudioEffect)] {
        &[
            ("binaural", AudioEffect::Binaural),
            ("panning", AudioEffect::Panning),
            ("distance_atten", AudioEffect::DistanceAtten),
            ("lowpass", AudioEffect::LowPassFilter),
            ("doppler", AudioEffect::Doppler),
            ("fade", AudioEffect::FadeInOut),
            ("gain_lerp", AudioEffect::GainLerp),
            ("normalize", AudioEffect::Normalization),
            ("spatial_bus", AudioEffect::SpatialBus),
            ("spatial_reverb", AudioEffect::SpatialReverb),
            ("hrtf_bus", AudioEffect::HrtfBus),
        ]
    }

    /// Retourne le nom textuel de l'effet (inverse de `FromStr`).
    pub fn name(self) -> &'static str {
        Self::all_names()
            .iter()
            .find(|(_, e)| *e == self)
            .map(|(n, _)| *n)
            .unwrap_or("unknown")
    }
}

/// Implémentation du trait standard pour le parsing depuis une chaîne.
impl std::str::FromStr for AudioEffect {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::all_names()
            .iter()
            .find(|(n, _)| *n == s)
            .map(|(_, e)| *e)
            .ok_or_else(|| format!("Unknown audio effect: '{}'", s))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SmokeColorMode {
    #[default]
    RocketColor,
    Custom,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum BlurMethod {
    Gaussian = 0,
    Kawase = 1,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
pub enum ToneMappingMode {
    Reinhard = 0,
    ReinhardExtended = 1,
    ACES = 2,
    Uncharted2 = 3,
    AgX = 4,
    KhronosPBR = 5,
}

/// Event envoyé par le renderer/physic au moteur audio
#[derive(Clone, Debug)]
pub struct DopplerEvent {
    pub id: u64,
    pub pos: Vec2,
    pub vel: Vec2,
    pub gain: f32,
    pub timestamp: std::time::Instant,
}

impl Default for DopplerEvent {
    fn default() -> Self {
        Self {
            id: 0,
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            gain: 1.0,
            timestamp: std::time::Instant::now(),
        }
    }
}

/// Forme d'explosion basée sur une image noir & blanc.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageShape {
    pub file_stem: String,
    pub sampled_points: Vec<Vec2>,
    pub scale: f32,
    pub flight_time: f32,
}

/// Configuration d'une forme d'explosion
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ExplosionShape {
    #[default]
    Spherical,
    Image(ImageShape),
    MultiImage {
        shapes: Vec<(ImageShape, f32)>,
        total_weight: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioSoundType {
    #[default]
    Rocket,
    Explosion,
}

#[derive(Debug, Clone)]
pub enum AudioDebugEvent {
    Sent {
        request_id: u64,
        sound_type: AudioSoundType,
        entity_id: u64,
        sent_at: std::time::Instant,
    },
    Received {
        request_id: u64,
        received_at: std::time::Instant,
    },
    Started {
        request_id: u64,
        started_at: std::time::Instant,
        voice_index: usize,
    },
    Dropped {
        request_id: u64,
        dropped_at: std::time::Instant,
        reason: &'static str,
    },
    Completed {
        request_id: u64,
        completed_at: std::time::Instant,
    },
    Underrun {
        elapsed_us: u64,
        budget_us: u64,
    },
    BlockProcessed {
        elapsed_us: u64,
        budget_us: u64,
        active_voices: usize,
    },
}

/// Trait commun décrivant le moteur audio temps réel.
pub trait AudioEngine {
    fn play_rocket(&self, pos: Vec2, gain: f32);
    fn play_rocket_with_id(&self, id: u64, pos: Vec2, gain: f32);
    fn play_explosion(&self, pos: Vec2, gain: f32);
    fn play_explosion_with_id(&self, id: u64, pos: Vec2, gain: f32);

    fn play_rocket_scheduled(&self, id: u64, pos: Vec2, gain: f32, _delay_ms: f32) {
        self.play_rocket_with_id(id, pos, gain);
    }

    fn play_explosion_scheduled(&self, id: u64, pos: Vec2, gain: f32, _delay_ms: f32) {
        self.play_explosion_with_id(id, pos, gain);
    }

    fn current_sample_clock(&self) -> u64 {
        0
    }

    fn start_audio_thread(&mut self, export_path: Option<&str>);
    fn stop_audio_thread(&mut self);

    fn set_listener_position(&mut self, pos: Vec2);
    fn get_listener_position(&self) -> Vec2;

    fn mute(&mut self);
    fn unmute(&mut self) -> f32;
    fn is_muted(&self) -> bool {
        false
    }

    fn set_effect_enabled(&self, effect: AudioEffect, enabled: bool);
    fn set_all_effects_enabled(&self, enabled: bool);
    fn get_effect_enabled(&self, effect: AudioEffect) -> bool;
    fn get_effects_status(&self) -> String;

    fn pop_debug_events(&self, _buf: &mut Vec<AudioDebugEvent>) {}

    fn get_max_distance(&self) -> f32 {
        1000.0
    }

    fn set_reverb_wet(&self, _wet: f32) {}
    fn get_reverb_wet(&self) -> f32 {
        0.08
    }

    fn set_master_volume(&self, _volume: f32) {}
    fn get_master_volume(&self) -> f32 {
        0.8
    }

    fn get_saved_master_volume(&self) -> f32 {
        self.get_master_volume()
    }

    fn as_audio_engine(&self) -> &dyn AudioEngine;
}

/// Commands sent from UI to Audio engine without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand {
    SetMasterVolume(f32),
    SetMuted(bool),
    SetSpatialReverb(f32),
    SetHrtfEnabled(bool),
    SetAllEffectsEnabled(bool),
    SetEffectEnabled { effect: AudioEffect, enabled: bool },
    SetListenerPosition(Vec2),
    StartStressTest,
}

/// Commands sent from UI to Physic engine without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicCommand {
    SetGravity(f32),
    SetDrag(f32),
    SetMaxParticles(u32),
    SetExplosionForce(f32),
    ApplyPendingConfig,
    SaveConfig,
    ReloadConfig,
    ResetDefaults,
    ResetCapacityDefaults,
    SetMaxRockets(u32),
    SetParticlesPerExplosion(u32),
    SetParticlesPerTrail(u32),
    SetMaxSmokeParticles(u32),
    ResetSpawnDefaults,
    SetRocketIntervalMean(f32),
    SetRocketIntervalVariation(f32),
    SetRocketMaxNextInterval(f32),
    SetSpawnRocketMargin(f32),
    SetSpawnRocketVerticalAngle(f32),
    SetSpawnRocketAngleVariation(f32),
    SetSpawnRocketMinSpeed(f32),
    SetSpawnRocketMaxSpeed(f32),
    SetInitialRocketSpeed(f32),
    ResetForcesDefaults,
    SetExplosionThreshold(f32),
    SetExplosionMinVel(f32),
    SetExplosionMaxVel(f32),
    SetExplosionVelocityBoost(f32),
    SetExplosionShapeSpherical,
    ResetAllPresetWeights,
    SetPresetWeight { index: u32, weight: f32 },
    SetPresetSingleShape { index: u32 },
    AddPresetShapeWeighted { index: u32, weight: f32 },
    DeleteSingleShape,
    ResetSingleShapeDefaults,
    SetSingleShapeScale(f32),
    SetSingleShapeFlightTime(f32),
    DeleteMultiShapeItem(u32),
    ResetMultiShapeItemDefaults(u32),
    SetMultiShapeItemWeight { index: u32, weight: f32 },
    SetMultiShapeItemScale { index: u32, scale: f32 },
    SetMultiShapeItemFlightTime { index: u32, flight_time: f32 },
}

/// Commands sent from UI to Renderer engine without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererCommand {
    SetBloomIntensity(f32),
    SetExposure(f32),
    SetWireframe(bool),
    SetVsync(bool),
    ReloadShaders,
    SaveConfig,
    ReloadConfig,
    ResetDefaults,
    ResetVisibilityDefaults,
    SetRenderRockets(bool),
    SetRenderSmoke(bool),
    SetRenderTrails(bool),
    SetRenderExplosions(bool),
    ResetTonemapping,
    SetToneMappingMode(ToneMappingMode),
    SetTonemappingComparisonMode(bool),
    ResetBloomDefaults,
    SetBloomEnabled(bool),
    SetBloomIterations(u32),
    SetBloomDownsample(u32),
    SetBloomBlurMethod(BlurMethod),
    SetVolumetricLightingEnabled(bool),
    SetSmokeLightingEnabled(bool),
    SetSmokeLightingLutEnabled(bool),
    RebakeSmokeLightingLut,
    SetSmokeScatteringIntensity(f32),
    SetSmokeAmbientFlash(f32),
    ResetSmokeLightingDefaults,
    SetSkyHazeEnabled(bool),
    SetSkyHazeIntensity(f32),
    SetSkyHazeAmbientFlash(f32),
    ResetSkyHazeDefaults,
    SetDitherEnabled(bool),
    SetDitherStrength(f32),
    ResetDitherDefaults,
    SetVolumetricLightingHysteresisEnabled(bool),
    SetVolumetricLightingFadeInMs(f32),
    SetVolumetricLightingRadius(f32),
    SetVolumetricLightingDecayRate(f32),
    SetVolumetricLightingRadiusExpansion(f32),
    SetVolumetricLightingFlashMaxCap(f32),
    SetVolumetricLightingDebug(bool),
    SetSkyHazeFalloff(f32),
    SetBacklightEnabled(bool),
    SetBacklightStrength(f32),
    ResetBacklightDefaults,
    SetSmokeWrapRelief(f32),
    SetSpectralAfterglowEnabled(bool),
    SetSpectralAfterglowDecay(f32),
    ResetVolumetricRealismDefaults,
}

/// Commands sent from UI to Smoke simulation engine without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmokeCommand {
    SetDensity(f32),
    SetDissipation(f32),
    SetWind([f32; 2]),
    SetErosionEnabled(bool),
    SetErosionScale(f32),
    SetErosionEdgeWidth(f32),
    SetErosionEdgeColor([u8; 3]),
    SetFlowDistortionStrength(f32),
    SetFlowAnimationSpeed(f32),
    SetColorMode(SmokeColorMode),
    SetInheritedColorIntensity(f32),
    SetCustomColor([u8; 3]),
    SetSpawnRate(f32),
    SetInitialSize(f32),
    SetGrowthRateMultiplier(f32),
    SetFadeDuration(f32),
    SetIntensity(f32),
    SetMaxSmokeParticles(u32),
    ResetDefaults,
    ApplyPreset(u8),
}

/// Commands sent from UI for overall session management without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GuiCommand {
    SaveSession,
    ReloadSession,
    SetRocketCursor(bool),
    TogglePause,
    SetPause(bool),
}

/// Unified domain command enum decoupling UI from core engines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineCommand {
    Audio(AudioCommand),
    Physic(PhysicCommand),
    Renderer(RendererCommand),
    Smoke(SmokeCommand),
    Gui(GuiCommand),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmokeConfigSnapshot {
    pub smoke_spawn_rate: f32,
    pub smoke_initial_size: f32,
    pub smoke_growth_rate_multiplier: f32,
    pub smoke_fade_duration: f32,
    pub max_smoke_particles: usize,
    pub smoke_intensity: f32,
    pub smoke_color_mode: SmokeColorMode,
    pub smoke_custom_color: [f32; 3],
    pub smoke_inherited_color_intensity: f32,
    pub smoke_erosion_enabled: bool,
    pub smoke_erosion_scale: f32,
    pub smoke_erosion_edge_width: f32,
    pub smoke_erosion_edge_color: [f32; 3],
    pub flow_distortion_strength: f32,
    pub flow_animation_speed: f32,
}

impl Default for SmokeConfigSnapshot {
    fn default() -> Self {
        Self {
            smoke_spawn_rate: 60.0,
            smoke_initial_size: 4.0,
            smoke_growth_rate_multiplier: 4.0,
            smoke_fade_duration: 3.5,
            max_smoke_particles: 100_000,
            smoke_intensity: 1.0,
            smoke_color_mode: SmokeColorMode::RocketColor,
            smoke_custom_color: [1.0, 1.0, 1.0],
            smoke_inherited_color_intensity: 1.0,
            smoke_erosion_enabled: false,
            smoke_erosion_scale: 1.0,
            smoke_erosion_edge_width: 0.15,
            smoke_erosion_edge_color: [1.0, 0.45, 0.1],
            flow_distortion_strength: 0.0,
            flow_animation_speed: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicConfigSnapshot {
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
    pub explosion_velocity_boost: f32,
    pub audio_launch_anticipation_ms: f32,
    pub audio_explosion_anticipation_ms: f32,
    pub smoke: SmokeConfigSnapshot,
}

impl Default for PhysicConfigSnapshot {
    fn default() -> Self {
        Self {
            max_rockets: 200,
            particles_per_explosion: 800,
            particles_per_trail: 50,
            rocket_interval_mean: 1.5,
            rocket_interval_variation: 0.8,
            rocket_max_next_interval: 3.0,
            spawn_rocket_margin: 50.0,
            spawn_rocket_vertical_angle: 90.0,
            spawn_rocket_angle_variation: 15.0,
            spawn_rocket_min_speed: 600.0,
            spawn_rocket_max_speed: 1000.0,
            explosion_threshold: 50.0,
            gravity: 120.0,
            initial_rocket_speed: 800.0,
            explosion_min_vel: 50.0,
            explosion_max_vel: 400.0,
            explosion_velocity_boost: 1.0,
            audio_launch_anticipation_ms: 0.0,
            audio_explosion_anticipation_ms: 0.0,
            smoke: SmokeConfigSnapshot::default(),
        }
    }
}

impl std::ops::Deref for PhysicConfigSnapshot {
    type Target = SmokeConfigSnapshot;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.smoke
    }
}

impl std::ops::DerefMut for PhysicConfigSnapshot {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.smoke
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RendererConfigSnapshot {
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_iterations: u32,
    pub bloom_downsample: u32,
    pub bloom_blur_method: BlurMethod,
    pub tone_mapping_mode: ToneMappingMode,

    pub render_rockets: bool,
    pub render_smoke: bool,
    pub render_trails: bool,
    pub render_explosions: bool,

    pub volumetric_lighting_enabled: bool,
    pub smoke_lighting_enabled: bool,
    pub smoke_scattering_intensity: f32,
    pub smoke_ambient_flash: f32,
    pub smoke_lighting_lut_enabled: bool,

    pub sky_haze_enabled: bool,
    pub sky_haze_intensity: f32,
    pub sky_haze_ambient_flash: f32,

    pub dither_enabled: bool,
    pub dither_strength: f32,

    pub volumetric_lighting_hysteresis_enabled: bool,
    pub volumetric_lighting_fade_in_ms: f32,
    pub volumetric_lighting_radius: f32,
    pub volumetric_lighting_decay_rate: f32,
    pub volumetric_lighting_radius_expansion: f32,
    pub volumetric_lighting_flash_max_cap: f32,
    pub volumetric_lighting_debug: bool,
    pub sky_haze_falloff: f32,
    pub backlight_enabled: bool,
    pub backlight_strength: f32,
    pub smoke_wrap_relief: f32,
    pub spectral_afterglow_enabled: bool,
    pub spectral_afterglow_decay: f32,
}

impl Default for RendererConfigSnapshot {
    fn default() -> Self {
        Self {
            bloom_enabled: true,
            bloom_intensity: 0.04,
            bloom_iterations: 6,
            bloom_downsample: 2,
            bloom_blur_method: BlurMethod::Kawase,
            tone_mapping_mode: ToneMappingMode::ACES,
            render_rockets: true,
            render_smoke: true,
            render_trails: true,
            render_explosions: true,
            volumetric_lighting_enabled: true,
            smoke_lighting_enabled: true,
            smoke_scattering_intensity: 1.0,
            smoke_ambient_flash: 0.35,
            smoke_lighting_lut_enabled: true,
            sky_haze_enabled: true,
            sky_haze_intensity: 0.45,
            sky_haze_ambient_flash: 0.15,
            sky_haze_falloff: 0.35,
            dither_enabled: true,
            dither_strength: 1.0,
            volumetric_lighting_hysteresis_enabled: true,
            volumetric_lighting_fade_in_ms: 60.0,
            volumetric_lighting_radius: 120.0,
            volumetric_lighting_decay_rate: 6.0,
            volumetric_lighting_radius_expansion: 1.5,
            volumetric_lighting_flash_max_cap: 2.0,
            volumetric_lighting_debug: false,
            backlight_enabled: true,
            backlight_strength: 0.5,
            smoke_wrap_relief: 0.3,
            spectral_afterglow_enabled: true,
            spectral_afterglow_decay: 2.5,
        }
    }
}

/// Read-only interface exposing Audio engine state to UI.
pub trait AudioStateReader {
    fn master_volume(&self) -> f32;
    fn is_muted(&self) -> bool;
    fn spatial_reverb(&self) -> f32;
    fn hrtf_enabled(&self) -> bool;
    fn effect_enabled(&self, effect: AudioEffect) -> bool;
}

impl<T: AudioEngine> AudioStateReader for T {
    #[inline(always)]
    fn master_volume(&self) -> f32 {
        self.get_master_volume()
    }
    #[inline(always)]
    fn is_muted(&self) -> bool {
        self.is_muted()
    }
    #[inline(always)]
    fn spatial_reverb(&self) -> f32 {
        self.get_reverb_wet()
    }
    #[inline(always)]
    fn hrtf_enabled(&self) -> bool {
        self.get_effect_enabled(AudioEffect::HrtfBus)
    }
    #[inline(always)]
    fn effect_enabled(&self, effect: AudioEffect) -> bool {
        self.get_effect_enabled(effect)
    }
}

/// Read-only interface exposing Smoke engine state to UI.
pub trait SmokeStateReader {
    fn density(&self) -> f32;
    fn dissipation(&self) -> f32;
    fn wind(&self) -> [f32; 2];
    fn config(&self) -> PhysicConfigSnapshot;
    fn smoke_config(&self) -> SmokeConfigSnapshot {
        self.config().smoke
    }
}

/// Read-only interface exposing Physic engine state to UI.
pub trait PhysicStateReader: SmokeStateReader {
    fn gravity(&self) -> f32;
    fn drag(&self) -> f32;
    fn max_particles(&self) -> u32;
    fn explosion_force(&self) -> f32;
    fn explosion_shape(&self) -> &ExplosionShape;
}

/// Read-only interface exposing Renderer engine state to UI.
pub trait RendererStateReader {
    fn bloom_intensity(&self) -> f32;
    fn exposure(&self) -> f32;
    fn is_wireframe(&self) -> bool;
    fn vsync_enabled(&self) -> bool;
    fn config(&self) -> RendererConfigSnapshot;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_command_memory_layout() {
        let size = std::mem::size_of::<EngineCommand>();
        assert!(
            size <= 16,
            "EngineCommand footprint ({} bytes) exceeds cache optimization limit of 16 bytes",
            size
        );
    }

    #[test]
    fn test_zero_cost_command_creation() {
        let cmd_audio = EngineCommand::Audio(AudioCommand::SetMasterVolume(0.85));
        let cmd_physic = EngineCommand::Physic(PhysicCommand::SetGravity(9.81));
        let cmd_renderer = EngineCommand::Renderer(RendererCommand::SetBloomIntensity(1.5));
        let cmd_smoke = EngineCommand::Smoke(SmokeCommand::SetWind([1.0, -0.5]));

        assert_eq!(
            cmd_audio,
            EngineCommand::Audio(AudioCommand::SetMasterVolume(0.85))
        );
        assert_eq!(
            cmd_physic,
            EngineCommand::Physic(PhysicCommand::SetGravity(9.81))
        );
        assert_eq!(
            cmd_renderer,
            EngineCommand::Renderer(RendererCommand::SetBloomIntensity(1.5))
        );
        assert_eq!(
            cmd_smoke,
            EngineCommand::Smoke(SmokeCommand::SetWind([1.0, -0.5]))
        );
    }

    struct MockState {
        volume: f32,
        muted: bool,
        reverb: f32,
        hrtf: bool,
        gravity: f32,
        drag: f32,
        particles: u32,
        explosion: f32,
        bloom: f32,
        exposure: f32,
        wireframe: bool,
        vsync: bool,
        smoke_density: f32,
        smoke_dissipation: f32,
        smoke_wind: [f32; 2],
        physic_config: PhysicConfigSnapshot,
        renderer_config: RendererConfigSnapshot,
        explosion_shape: ExplosionShape,
    }

    impl AudioStateReader for MockState {
        #[inline(always)]
        fn master_volume(&self) -> f32 {
            self.volume
        }
        #[inline(always)]
        fn is_muted(&self) -> bool {
            self.muted
        }
        #[inline(always)]
        fn spatial_reverb(&self) -> f32 {
            self.reverb
        }
        #[inline(always)]
        fn hrtf_enabled(&self) -> bool {
            self.hrtf
        }
        #[inline(always)]
        fn effect_enabled(&self, _effect: AudioEffect) -> bool {
            true
        }
    }

    impl SmokeStateReader for MockState {
        #[inline(always)]
        fn density(&self) -> f32 {
            self.smoke_density
        }
        #[inline(always)]
        fn dissipation(&self) -> f32 {
            self.smoke_dissipation
        }
        #[inline(always)]
        fn wind(&self) -> [f32; 2] {
            self.smoke_wind
        }
        #[inline(always)]
        fn config(&self) -> PhysicConfigSnapshot {
            self.physic_config.clone()
        }
    }

    impl PhysicStateReader for MockState {
        #[inline(always)]
        fn gravity(&self) -> f32 {
            self.gravity
        }
        #[inline(always)]
        fn drag(&self) -> f32 {
            self.drag
        }
        #[inline(always)]
        fn max_particles(&self) -> u32 {
            self.particles
        }
        #[inline(always)]
        fn explosion_force(&self) -> f32 {
            self.explosion
        }
        #[inline(always)]
        fn explosion_shape(&self) -> &ExplosionShape {
            &self.explosion_shape
        }
    }

    impl RendererStateReader for MockState {
        #[inline(always)]
        fn bloom_intensity(&self) -> f32 {
            self.bloom
        }
        #[inline(always)]
        fn exposure(&self) -> f32 {
            self.exposure
        }
        #[inline(always)]
        fn is_wireframe(&self) -> bool {
            self.wireframe
        }
        #[inline(always)]
        fn vsync_enabled(&self) -> bool {
            self.vsync
        }
        #[inline(always)]
        fn config(&self) -> RendererConfigSnapshot {
            self.renderer_config.clone()
        }
    }

    #[test]
    fn test_state_readers() {
        let state = MockState {
            volume: 0.75,
            muted: false,
            reverb: 0.3,
            hrtf: true,
            gravity: -9.81,
            drag: 0.01,
            particles: 50000,
            explosion: 100.0,
            bloom: 0.8,
            exposure: 1.0,
            wireframe: false,
            vsync: true,
            smoke_density: 0.5,
            smoke_dissipation: 0.05,
            smoke_wind: [0.2, -0.1],
            physic_config: PhysicConfigSnapshot::default(),
            renderer_config: RendererConfigSnapshot::default(),
            explosion_shape: ExplosionShape::Spherical,
        };

        assert_eq!(state.master_volume(), 0.75);
        assert!(!state.is_muted());
        assert_eq!(state.spatial_reverb(), 0.3);
        assert!(state.hrtf_enabled());

        assert_eq!(state.gravity(), -9.81);
        assert_eq!(state.drag(), 0.01);
        assert_eq!(state.max_particles(), 50000);
        assert_eq!(state.explosion_force(), 100.0);

        assert_eq!(state.bloom_intensity(), 0.8);
        assert_eq!(state.exposure(), 1.0);
        assert!(!state.is_wireframe());
        assert!(state.vsync_enabled());

        assert_eq!(state.density(), 0.5);
        assert_eq!(state.dissipation(), 0.05);
        assert_eq!(state.wind(), [0.2, -0.1]);
    }

    #[test]
    fn test_snapshot_liveness_no_stale_cache() {
        let mut state = MockState {
            volume: 0.5,
            muted: false,
            reverb: 0.1,
            hrtf: false,
            gravity: 10.0,
            drag: 0.0,
            particles: 100,
            explosion: 50.0,
            bloom: 0.2,
            exposure: 1.0,
            wireframe: false,
            vsync: true,
            smoke_density: 0.2,
            smoke_dissipation: 1.0,
            smoke_wind: [0.0, 0.0],
            physic_config: PhysicConfigSnapshot::default(),
            renderer_config: RendererConfigSnapshot::default(),
            explosion_shape: ExplosionShape::Spherical,
        };

        assert_eq!(state.master_volume(), 0.5);
        assert_eq!(SmokeStateReader::config(&state).gravity, 120.0);
        assert_eq!(SmokeStateReader::config(&state).smoke_spawn_rate, 60.0);
        assert_eq!(RendererStateReader::config(&state).bloom_intensity, 0.04);

        // Mutate fields as if EngineCommand was processed
        state.volume = 0.9;
        state.physic_config.gravity = 250.0;
        state.physic_config.smoke.smoke_spawn_rate = 120.0;
        state.renderer_config.bloom_intensity = 0.85;

        // Fresh snapshots must reflect mutations immediately without stale cache
        assert_eq!(state.master_volume(), 0.9);
        let fresh_physic_snap = SmokeStateReader::config(&state);
        assert_eq!(fresh_physic_snap.gravity, 250.0);
        assert_eq!(fresh_physic_snap.smoke_spawn_rate, 120.0);
        let fresh_renderer_snap = RendererStateReader::config(&state);
        assert_eq!(fresh_renderer_snap.bloom_intensity, 0.85);
    }
}

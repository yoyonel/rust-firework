// =========================
// Audio Engine Configuration
// =========================

use derive_builder::Builder;
use serde::Deserialize;

use crate::audio_engine::constants;

/// Parameters controlling spatialization, filtering, and volume.
///
/// All fields are private — configuration is done exclusively via the builder:
///
#[derive(Clone, Builder, Debug, Deserialize)]
#[builder(pattern = "owned", build_fn(error = "anyhow::Error"))]
pub struct AudioEngineSettings {
    /// Human head radius (meters) for ITD calculation
    #[builder(default = "constants::DEFAULT_HEAD_RADIUS")]
    pub head_radius: f32,

    /// Maximum interaural level difference (dB)
    #[builder(default = "constants::DEFAULT_MAX_ILD_DB")]
    pub max_ild_db: f32,

    /// Use full binaural processing or simple stereo panning
    #[builder(default = "true")]
    pub use_binaural: bool,

    /// Maximum distance at which sounds are audible
    #[builder(default = "constants::DEFAULT_MAX_DISTANCE")]
    pub max_distance: f32,

    /// Global gain applied to all output
    #[builder(default = "constants::DEFAULT_GLOBAL_GAIN")]
    pub global_gain: f32,

    /// Fade-in duration (ms)
    #[builder(default = "constants::DEFAULT_FADE_IN_MS")]
    pub fade_in_ms: f32,

    /// Fade-out duration (ms)
    #[builder(default = "constants::DEFAULT_FADE_OUT_MS")]
    pub fade_out_ms: f32,

    /// Minimum frequency for distance-based low-pass filter
    #[builder(default = "constants::DEFAULT_F_MIN_HZ")]
    pub f_min: f32,

    /// Maximum frequency for distance-based low-pass filter
    #[builder(default = "constants::DEFAULT_F_MAX_HZ")]
    pub f_max: f32,

    /// Distance-dependent filter attenuation coefficient
    #[builder(default = "constants::DEFAULT_DISTANCE_ALPHA")]
    pub distance_alpha: f32,
}

impl AudioEngineSettings {
    /// Accessors: read-only public getters
    pub fn head_radius(&self) -> f32 {
        self.head_radius
    }

    pub fn max_ild_db(&self) -> f32 {
        self.max_ild_db
    }

    pub fn use_binaural(&self) -> bool {
        self.use_binaural
    }

    pub fn max_distance(&self) -> f32 {
        self.max_distance
    }

    pub fn global_gain(&self) -> f32 {
        self.global_gain
    }

    pub fn fade_in_ms(&self) -> f32 {
        self.fade_in_ms
    }

    pub fn fade_out_ms(&self) -> f32 {
        self.fade_out_ms
    }

    pub fn f_min(&self) -> f32 {
        self.f_min
    }

    pub fn f_max(&self) -> f32 {
        self.f_max
    }

    pub fn distance_alpha(&self) -> f32 {
        self.distance_alpha
    }
}

/// Keep backward compatibility with `.default()`
impl Default for AudioEngineSettings {
    fn default() -> Self {
        AudioEngineSettingsBuilder::default().build().unwrap()
    }
}

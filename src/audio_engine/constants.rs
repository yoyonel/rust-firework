//! Single Source of Truth (SSOT) constants for the Audio Engine.
//!
//! Provides documented physical constants, hardware defaults, spatialization parameters,
//! and DSP bounds for the audio engine.

/// Default hardware buffer size in audio samples for CPAL stream creation.
///
/// - **Unit:** samples
/// - **Technical meaning:** Number of audio frames in the low-level hardware output buffer.
/// - **Bounds:** `256` to `32768` samples.
/// - **System influence:** Higher values reduce risk of audio underrun/glitches but increase output latency.
pub const HARDWARE_BUFFER_SIZE: u32 = 16384;

/// Bounded crossbeam channel capacity for incoming audio play requests.
///
/// - **Unit:** requests (count)
/// - **Technical meaning:** Maximum number of pending audio play requests queued before dropping.
/// - **Bounds:** `64` to `8192` requests.
/// - **System influence:** Prevents unbound RAM allocation during heavy firework explosions without blocking physics thread.
pub const PLAY_REQUEST_CHANNEL_CAPACITY: usize = 512;

/// Bounded crossbeam channel capacity for audio debug events.
///
/// - **Unit:** events (count)
/// - **Technical meaning:** Buffer size for tracking audio events sent to the GUI / overlay renderer.
/// - **Bounds:** `256` to `16384` events.
/// - **System influence:** Prevents GUI notification backpressure from stalling the DSP processor.
pub const DEBUG_EVENT_CHANNEL_CAPACITY: usize = 2048;

/// Default sample rate for the audio synthesis and playback pipeline.
///
/// - **Unit:** Hz
/// - **Technical meaning:** Standard audio sampling frequency for playback and DSP filters.
/// - **Bounds:** `22050` Hz to `192000` Hz.
/// - **System influence:** Higher rates increase sound fidelity at linear CPU/DSP processing cost.
pub const DEFAULT_SAMPLE_RATE: u32 = 48000;

/// Default block size for DSP buffer processing chunks.
///
/// - **Unit:** samples
/// - **Technical meaning:** Frame length processed in a single lock-free DSP iteration.
/// - **Bounds:** `64` to `4096` samples.
/// - **System influence:** Smaller blocks improve spatial responsiveness, larger blocks improve CPU cache efficiency.
pub const DEFAULT_BLOCK_SIZE: usize = 512;

/// Default maximum number of concurrent audio synthesis voices.
///
/// - **Unit:** voices (count)
/// - **Technical meaning:** Maximum number of active sound instances (rockets/explosions) playing simultaneously.
/// - **Bounds:** `16` to `4096` voices.
/// - **System influence:** Directly scales CPU load in the audio rendering thread and preallocated voice memory.
pub const DEFAULT_MAX_VOICES: usize = 256;

/// Minimum voice floor when dynamically calculating voice pool size.
///
/// - **Unit:** voices (count)
/// - **Technical meaning:** Absolute lower bound for max voice allocation regardless of physics rocket count.
/// - **Bounds:** `16` to `256` voices.
/// - **System influence:** Ensures sufficient headroom for sound overlaps (launches + explosions) under low rocket counts.
pub const MIN_VOICES_FLOOR: usize = 64;

/// Safety multiplier to compute maximum voices from active rocket count.
///
/// - **Unit:** dimensionless (ratio)
/// - **Technical meaning:** Multiplier applied to `max_physic_rockets` when scaling voice capacity.
/// - **Bounds:** `1` to `10`.
/// - **System influence:** Prevents voice exhaustion during rapid multi-part particle explosions.
pub const VOICE_SAFETY_MULTIPLIER: usize = 4;

/// Speed of sound in ambient air.
///
/// - **Unit:** m/s (meters per second)
/// - **Technical meaning:** Acoustic propagation velocity used for binaural ITD calculations.
/// - **Bounds:** `300.0` to `400.0` m/s.
/// - **System influence:** Controls delay time calculated between left and right ear arrival.
pub const SPEED_OF_SOUND_M_S: f32 = 343.0;

/// Reference distance for inverse-distance acoustic attenuation.
///
/// - **Unit:** m (meters)
/// - **Technical meaning:** Distance threshold within which audio sources maintain maximum gain (1.0).
/// - **Bounds:** `1.0` to `200.0` m.
/// - **System influence:** Defines the near-field non-attenuating zone around the listener.
pub const REFERENCE_DISTANCE_METERS: f32 = 50.0;

/// Minimum distance epsilon for spatial calculations to prevent division by zero.
///
/// - **Unit:** m (meters)
/// - **Technical meaning:** Small positive offset applied when normalizing distance vectors.
/// - **Bounds:** `1e-9` to `1e-3` m.
/// - **System influence:** Prevents `NaN` and `Inf` floating point exceptions when sound source overlaps listener.
pub const MIN_DISTANCE_EPSILON: f32 = 1e-6;

/// Maximum interaural time delay (ITD) clamp.
///
/// - **Unit:** s (seconds)
/// - **Technical meaning:** Hard upper limit for acoustic arrival time difference between ears (1.0 ms).
/// - **Bounds:** `0.0005` to `0.002` s.
/// - **System influence:** Prevents extreme phase distortion for sources positioned directly lateral to listener.
pub const MAX_ITD_SECONDS: f32 = 0.001;

/// Elevation attenuation factor for Interaural Level Difference (ILD).
///
/// - **Unit:** dimensionless (scaling factor)
/// - **Technical meaning:** Scaling factor reducing lateral level differences for overhead sound sources.
/// - **Bounds:** `0.0` to `1.0`.
/// - **System influence:** Implements realistic head shadow reduction when rockets explode high above listener.
pub const ILD_ELEVATION_ATTENUATION_FACTOR: f32 = 0.25;

/// Default human head radius for ITD calculation.
///
/// - **Unit:** m (meters)
/// - **Technical meaning:** Approximate physical head radius used in Woodworth's ITD formula.
/// - **Bounds:** `0.05` to `0.15` m.
/// - **System influence:** Modulates interaural time delay magnitude.
pub const DEFAULT_HEAD_RADIUS: f32 = 0.0875;

/// Default maximum interaural level difference (ILD).
///
/// - **Unit:** dB (decibels)
/// - **Technical meaning:** Maximum acoustic shadow attenuation applied to the far ear for lateral sources.
/// - **Bounds:** `0.0` to `30.0` dB.
/// - **System influence:** Controls stereo separation intensity in binaural spatial mode.
pub const DEFAULT_MAX_ILD_DB: f32 = 18.0;

/// Default maximum distance for spatial sound audibility.
///
/// - **Unit:** m (meters)
/// - **Technical meaning:** Cutoff distance beyond which audio events are completely silenced.
/// - **Bounds:** `100.0` to `10000.0` m.
/// - **System influence:** Restricts spatial processing work to audible world bounds.
pub const DEFAULT_MAX_DISTANCE: f32 = 2000.0;

/// Default global master gain.
///
/// - **Unit:** dimensionless (amplitude factor)
/// - **Technical meaning:** Baseline gain multiplier applied to output master mix.
/// - **Bounds:** `0.0` to `2.0`.
/// - **System influence:** Prevents digital clipping when multiple explosion sounds sum together.
pub const DEFAULT_GLOBAL_GAIN: f32 = 0.8;

/// Default sound fade-in duration.
///
/// - **Unit:** ms (milliseconds)
/// - **Technical meaning:** Duration of linear volume ramp-up at sound start.
/// - **Bounds:** `0.0` to `500.0` ms.
/// - **System influence:** Eliminates DC offset clicks when starting audio playback.
pub const DEFAULT_FADE_IN_MS: f32 = 20.0;

/// Default sound fade-out duration.
///
/// - **Unit:** ms (milliseconds)
/// - **Technical meaning:** Duration of linear volume ramp-down at sound termination.
/// - **Bounds:** `0.0` to `1000.0` ms.
/// - **System influence:** Eliminates abrupt clipping clicks when voices are stopped or reused.
pub const DEFAULT_FADE_OUT_MS: f32 = 50.0;

/// Minimum cutoff frequency for distance low-pass filter.
///
/// - **Unit:** Hz
/// - **Technical meaning:** Cutoff frequency for distant sound sources (high-frequency air absorption).
/// - **Bounds:** `20.0` to `5000.0` Hz.
/// - **System influence:** Controls tone muffling for explosions occurring far away.
pub const DEFAULT_F_MIN_HZ: f32 = 1000.0;

/// Maximum cutoff frequency for distance low-pass filter.
///
/// - **Unit:** Hz
/// - **Technical meaning:** Cutoff frequency for near sound sources (unfiltered brilliance).
/// - **Bounds:** `5000.0` to `24000.0` Hz.
/// - **System influence:** Preserves full high-frequency content for nearby rocket launches and explosions.
pub const DEFAULT_F_MAX_HZ: f32 = 15000.0;

/// Distance attenuation coefficient for low-pass filter calculation.
///
/// - **Unit:** 1/m (inverse meters)
/// - **Technical meaning:** Exponential decay rate of high frequencies over distance.
/// - **Bounds:** `0.0001` to `0.05` 1/m.
/// - **System influence:** Modulates realistic atmospheric acoustic damping.
pub const DEFAULT_DISTANCE_ALPHA: f32 = 0.0025;

/// Base sample rate for Schroeder/FDN reverb delay calculations.
///
/// - **Unit:** Hz
/// - **Technical meaning:** Reference sample rate for hardcoded comb and allpass delay loop lengths.
/// - **Bounds:** `44100.0` Hz.
/// - **System influence:** Used to scale delay line buffer sizes dynamically for target sample rate.
pub const REVERB_BASE_SAMPLE_RATE: f32 = 44100.0;

/// Base delay sample lengths for parallel comb filters in spatial reverb.
///
/// - **Unit:** samples (at 44.1 kHz base rate)
/// - **Technical meaning:** Delay line lengths for the 4 parallel Schroeder comb filters.
/// - **Bounds:** `500` to `10000` samples.
/// - **System influence:** Determines modal room resonance density and outdoor echo reflection timing.
pub const REVERB_COMB_DELAYS_BASE_SAMPLES: [usize; 4] = [1553, 2129, 2801, 3547];

/// Stereo uncorrelation sample offset for spatial reverb right channel.
///
/// - **Unit:** samples
/// - **Technical meaning:** Additional delay applied to right channel comb filters to decorrelate stereo field.
/// - **Bounds:** `1` to `500` samples.
/// - **System influence:** Prevents comb-filtering phase cancellation artifacts in mono sum.
pub const REVERB_STEREO_UNCORRELATION_OFFSET_SAMPLES: usize = 47;

/// Base delay sample lengths for series allpass filters in spatial reverb.
///
/// - **Unit:** samples (at 44.1 kHz base rate)
/// - **Technical meaning:** Delay lengths for allpass diffusion filters.
/// - **Bounds:** `50` to `2000` samples.
/// - **System influence:** Increases echo density without adding tonal coloration.
pub const REVERB_ALLPASS_DELAYS_BASE_SAMPLES: [usize; 2] = [641, 317];

/// Default feedback coefficient for spatial reverb comb filters.
///
/// - **Unit:** dimensionless (gain ratio)
/// - **Technical meaning:** Feedback gain in feedback comb filter loops.
/// - **Bounds:** `0.0` to `0.99`.
/// - **System influence:** Dictates reverberation decay time (T60).
pub const REVERB_DEFAULT_FEEDBACK: f32 = 0.68;

/// Default high-frequency damping factor for spatial reverb comb filters.
///
/// - **Unit:** dimensionless (damping ratio)
/// - **Technical meaning:** Low-pass filtering strength inside comb feedback loop simulating open-air absorption.
/// - **Bounds:** `0.0` to `1.0`.
/// - **System influence:** Rapidly dampens high frequencies for natural outdoor echo tail.
pub const REVERB_DEFAULT_DAMPING: f32 = 0.50;

/// Default wet gain for spatial reverb output.
///
/// - **Unit:** dimensionless (gain ratio)
/// - **Technical meaning:** Master wet signal mix level added to dry acoustic bus.
/// - **Bounds:** `0.0` to `1.0`.
/// - **System influence:** Controls perceived spatial warmth and environment size.
pub const REVERB_DEFAULT_WET_GAIN: f32 = 0.08;

/// Minimum FFT chunk clamping bound.
///
/// - **Unit:** samples
/// - **Technical meaning:** Lower bound for FFT chunk processing in HRTF convolver.
/// - **Bounds:** `32` to `256` samples.
/// - **System influence:** Prevents inefficient micro-FFT executions.
pub const FFT_MIN_CHUNK_BOUND: usize = 128;

/// Maximum FFT chunk clamping bound.
///
/// - **Unit:** samples
/// - **Technical meaning:** Upper bound for FFT chunk processing in HRTF convolver.
/// - **Bounds:** `256` to `4096` samples.
/// - **System influence:** Balances FFT latency against algorithmic efficiency.
pub const FFT_MAX_CHUNK_BOUND: usize = 512;

/// Default file path for rocket launch sound asset.
pub const DEFAULT_ROCKET_SOUND_PATH: &str = "assets/sounds/rocket.wav";

/// Default file path for explosion sound asset.
pub const DEFAULT_EXPLOSION_SOUND_PATH: &str = "assets/sounds/explosion.wav";

// Audio GUI Control Bounds & Presets
pub const SLIDER_VOLUME_MIN: f32 = 0.0;
pub const SLIDER_VOLUME_MAX: f32 = 2.0;

pub const SLIDER_REVERB_MIN: f32 = 0.0;
pub const SLIDER_REVERB_MAX: f32 = 1.0;

pub const PRESET_VOL_MUTE: f32 = 0.0;
pub const PRESET_VOL_LOW: f32 = 0.25;
pub const PRESET_VOL_MEDIUM: f32 = 0.50;
pub const PRESET_VOL_DEFAULT: f32 = 0.80;
pub const PRESET_VOL_FULL: f32 = 1.00;
pub const PRESET_VOL_BOOST: f32 = 1.50;

pub const PRESET_REVERB_DRY: f32 = 0.0;
pub const PRESET_REVERB_DEFAULT: f32 = 0.08;
pub const PRESET_REVERB_MEDIUM: f32 = 0.20;
pub const PRESET_REVERB_CATHEDRAL: f32 = 0.50;
pub const PRESET_REVERB_FULL_WET: f32 = 1.00;

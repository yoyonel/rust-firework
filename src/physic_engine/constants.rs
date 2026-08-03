//! Single Source of Truth (SSOT) constants for the Physic Engine.
//!
//! Defines central physical values, arena capacities, rocket kinematics,
//! explosion parameters, and timing anticipations.

/// Default maximum number of concurrent rockets supported in the physics arena.
///
/// - **Unit:** count (rockets)
/// - **Technical meaning:** Preallocated capacity for rocket slots in generational arena.
/// - **Bounds:** `1` to `65536`.
/// - **System influence:** Determines memory footprint for rockets and particle pool arrays.
pub const DEFAULT_MAX_ROCKETS: usize = 1024;

/// Default number of explosion particles spawned per rocket explosion.
///
/// - **Unit:** count (particles)
/// - **Technical meaning:** Particle count allocated per explosion shape event.
/// - **Bounds:** `16` to `4096`.
/// - **System influence:** Impacts GPU instance buffer size and explosion visual density.
pub const DEFAULT_PARTICLES_PER_EXPLOSION: usize = 256;

/// Default number of trail particles spawned per active rocket flight.
///
/// - **Unit:** count (particles)
/// - **Technical meaning:** Particle pool allocation for smoke/flame rocket trails.
/// - **Bounds:** `8` to `512`.
/// - **System influence:** Controls rocket trail visual length and GPU memory overhead.
pub const DEFAULT_PARTICLES_PER_TRAIL: usize = 64;

/// Default gravitational acceleration.
///
/// - **Unit:** m/s² (meters per second squared)
/// - **Technical meaning:** Downward acceleration vector magnitude applied to rockets and particles.
/// - **Bounds:** `-1000.0` to `0.0` m/s².
/// - **System influence:** Dictates parabolic trajectory curves and particle fall rates.
pub const DEFAULT_GRAVITY: f32 = -200.0;

/// Default initial rocket launch speed.
///
/// - **Unit:** m/s (meters per second)
/// - **Technical meaning:** Baseline speed assigned to freshly spawned rockets.
/// - **Bounds:** `10.0` to `1000.0` m/s.
/// - **System influence:** Determines initial upward momentum before motor acceleration/decay.
pub const DEFAULT_INITIAL_ROCKET_SPEED: f32 = 100.0;

/// Default minimum speed range for spawning rockets.
///
/// - **Unit:** m/s (meters per second)
/// - **Technical meaning:** Lower bound of randomized rocket launch speed range.
/// - **Bounds:** `50.0` to `1000.0` m/s.
/// - **System influence:** Affects minimum apex altitude of launched fireworks.
pub const DEFAULT_SPAWN_ROCKET_MIN_SPEED: f32 = 350.0;

/// Default maximum speed range for spawning rockets.
///
/// - **Unit:** m/s (meters per second)
/// - **Technical meaning:** Upper bound of randomized rocket launch speed range.
/// - **Bounds:** `50.0` to `1500.0` m/s.
/// - **System influence:** Affects maximum apex altitude of launched fireworks.
pub const DEFAULT_SPAWN_ROCKET_MAX_SPEED: f32 = 500.0;

/// Default horizontal margin for spawning rockets across window width.
///
/// - **Unit:** m (meters / pixels)
/// - **Technical meaning:** Inset distance from left and right screen boundaries for rocket launch positions.
/// - **Bounds:** `0.0` to `500.0` m.
/// - **System influence:** Keeps rocket launches within visible display boundaries.
pub const DEFAULT_SPAWN_ROCKET_MARGIN: f32 = 50.0;

/// Default vertical trajectory angle for launching rockets (90 degrees / pi/2 radians).
///
/// - **Unit:** radians
/// - **Technical meaning:** Base angle for vertical launch (pointing straight up along Y axis).
/// - **Bounds:** `0.0` to `std::f32::consts::PI` radians.
/// - **System influence:** Controls main launch direction vector.
pub const DEFAULT_SPAWN_ROCKET_VERTICAL_ANGLE: f32 = std::f32::consts::FRAC_PI_2;

/// Default angular variation range around vertical launch angle (~17 degrees).
///
/// - **Unit:** radians
/// - **Technical meaning:** Maximum random angular tilt (+/- angle) applied to rocket launch direction.
/// - **Bounds:** `0.0` to `std::f32::consts::FRAC_PI_4` radians.
/// - **System influence:** Adds realistic spread and variation to firework launch arcs.
pub const DEFAULT_SPAWN_ROCKET_ANGLE_VARIATION: f32 = 0.3;

/// Default mean time interval between automatic rocket spawns.
///
/// - **Unit:** s (seconds)
/// - **Technical meaning:** Average delay before launching the next rocket in automatic mode.
/// - **Bounds:** `0.001` to `10.0` s.
/// - **System influence:** Controls firework show launch cadence.
pub const DEFAULT_ROCKET_INTERVAL_MEAN: f32 = 0.025;

/// Default variation factor for rocket spawn interval timing.
///
/// - **Unit:** s (seconds)
/// - **Technical meaning:** Random jitter range applied to launch interval.
/// - **Bounds:** `0.0` to `5.0` s.
/// - **System influence:** Prevents rigid robotic launch rhythms.
pub const DEFAULT_ROCKET_INTERVAL_VARIATION: f32 = 0.01875;

/// Default maximum allowed next rocket spawn interval clamp.
///
/// - **Unit:** s (seconds)
/// - **Technical meaning:** Maximum interval cap for rocket spawn timer.
/// - **Bounds:** `0.001` to `10.0` s.
/// - **System influence:** Caps maximum gap between automated launches.
pub const DEFAULT_ROCKET_MAX_NEXT_INTERVAL: f32 = 0.025;

/// Default speed threshold for rocket detonation trigger.
///
/// - **Unit:** m/s (meters per second)
/// - **Technical meaning:** Vertical velocity threshold when rocket reaches apex and explodes.
/// - **Bounds:** `0.0` to `200.0` m/s.
/// - **System influence:** Triggers rocket explosion when upward velocity decays below threshold.
pub const DEFAULT_EXPLOSION_THRESHOLD_SPEED: f32 = 50.0;

/// Default minimum velocity magnitude for explosion particles.
///
/// - **Unit:** m/s (meters per second)
/// - **Technical meaning:** Minimum radial burst velocity assigned to explosion particles.
/// - **Bounds:** `0.0` to `500.0` m/s.
/// - **System influence:** Determines inner sphere expansion rate of explosion.
pub const DEFAULT_EXPLOSION_MIN_VELOCITY: f32 = 60.0;

/// Default maximum velocity magnitude for explosion particles.
///
/// - **Unit:** m/s (meters per second)
/// - **Technical meaning:** Maximum radial burst velocity assigned to explosion particles.
/// - **Bounds:** `10.0` to `1000.0` m/s.
/// - **System influence:** Determines outer sphere expansion rate of explosion.
pub const DEFAULT_EXPLOSION_MAX_VELOCITY: f32 = 200.0;

/// Default audio launch anticipation lead time.
///
/// - **Unit:** ms (milliseconds)
/// - **Technical meaning:** Anticipation time window to trigger audio launch event before visual launch.
/// - **Bounds:** `0.0` to `200.0` ms.
/// - **System influence:** Compensates for audio system latency and buffer alignment.
pub const DEFAULT_AUDIO_LAUNCH_ANTICIPATION_MS: f32 = 25.0;

/// Default audio explosion anticipation lead time.
///
/// - **Unit:** ms (milliseconds)
/// - **Technical meaning:** Anticipation time window to trigger audio explosion event before physical detonation.
/// - **Bounds:** `0.0` to `200.0` ms.
/// - **System influence:** Synchronizes sound speed of bang with visual burst expansion.
pub const DEFAULT_AUDIO_EXPLOSION_ANTICIPATION_MS: f32 = 25.0;

/// Black/white pixel threshold intensity for image-based explosion sampling.
///
/// - **Unit:** dimensionless (8-bit pixel value 0..255)
/// - **Technical meaning:** Minimum luma intensity required to sample a pixel for image shape explosions.
/// - **Bounds:** `1` to `254`.
/// - **System influence:** Filters out dark background pixels when converting images to particle positions.
pub const IMAGE_SHAPE_THRESHOLD: u8 = 128;

/// Default smoke spawn rate (particles per second per rocket).
pub const DEFAULT_SMOKE_SPAWN_RATE: f32 = 30.0;

/// Default initial particle scale for smoke sprites.
pub const DEFAULT_SMOKE_INITIAL_SIZE: f32 = 10.0;

/// Default expansion rate multiplier for smoke particles over lifetime.
pub const DEFAULT_SMOKE_GROWTH_RATE_MULTIPLIER: f32 = 1.2;

/// Default fade-out duration (lifetime) of smoke particles in seconds.
pub const DEFAULT_SMOKE_FADE_DURATION: f32 = 0.75;

/// Default maximum number of smoke particles in the physics pool.
pub const DEFAULT_MAX_SMOKE_PARTICLES: usize = 2048;

/// Default smoke intensity / brightness blending factor (0.0 = invisible, 1.0 = normal, 2.0 = boost).
pub const DEFAULT_SMOKE_INTENSITY: f32 = 0.5;

/// Default custom color for smoke particles (RGB).
pub const DEFAULT_SMOKE_CUSTOM_COLOR: [f32; 3] = [0.85, 0.85, 0.85];

/// Default intensity factor for inherited rocket color applied to smoke (0.0 to 2.0, default 1.0).
pub const DEFAULT_SMOKE_INHERITED_COLOR_INTENSITY: f32 = 1.0;

/// Distance offset from rocket center to combustion base exhaust (cyan area above wooden stick).
pub const ROCKET_BASE_EXHAUST_OFFSET: f32 = 6.0;

/// Default toggle flag for noise alpha erosion (dissolve effect) on smoke particles.
pub const DEFAULT_SMOKE_EROSION_ENABLED: bool = true;

/// Default aggressiveness / scale multiplier for smoke noise alpha erosion.
pub const DEFAULT_SMOKE_EROSION_SCALE: f32 = 1.0;

/// Default noise alpha erosion edge width for smoke dissipation dissolve effect.
pub const DEFAULT_SMOKE_EROSION_EDGE_WIDTH: f32 = 0.08;

/// Default glowing burn edge color (RGB) along the alpha erosion dissipation seam.
pub const DEFAULT_SMOKE_EROSION_EDGE_COLOR: [f32; 3] = [1.0, 0.45, 0.15];

/// Default Flow Map UV distortion strength for smoke turbulence.
pub const DEFAULT_FLOW_DISTORTION_STRENGTH: f32 = 0.15;

/// Default Flow Map animation speed multiplier for smoke turbulence.
pub const DEFAULT_FLOW_ANIMATION_SPEED: f32 = 1.0;

/// Default zoom factor for live GPU smoke preview viewport.
pub const DEFAULT_SMOKE_PREVIEW_ZOOM: f32 = 1.0;

/// Default pan X translation (pixels) for live GPU smoke preview viewport.
pub const DEFAULT_SMOKE_PREVIEW_PAN_X: f32 = 0.0;

/// Default pan Y translation (pixels) for live GPU smoke preview viewport.
pub const DEFAULT_SMOKE_PREVIEW_PAN_Y: f32 = 0.0;

/// Default rotation Z angle (degrees) for live GPU smoke preview viewport.
pub const DEFAULT_SMOKE_PREVIEW_ROT_Z: f32 = 0.0;

/// Minimum zoom factor for the live GPU smoke preview viewport.
pub const DEFAULT_SMOKE_PREVIEW_MIN_ZOOM: f32 = 0.4;

/// Maximum zoom factor for the live GPU smoke preview viewport.
pub const DEFAULT_SMOKE_PREVIEW_MAX_ZOOM: f32 = 10.0;

/// Minimum erosion scale slider bound.
pub const SMOKE_EROSION_SCALE_MIN: f32 = 0.0;

/// Maximum erosion scale slider bound.
pub const SMOKE_EROSION_SCALE_MAX: f32 = 2.0;

/// Minimum erosion edge width slider bound.
pub const SMOKE_EROSION_EDGE_WIDTH_MIN: f32 = 0.0;

/// Maximum erosion edge width slider bound.
pub const SMOKE_EROSION_EDGE_WIDTH_MAX: f32 = 0.80;

/// Minimum flow distortion strength slider bound.
pub const FLOW_DISTORTION_STRENGTH_MIN: f32 = 0.0;

/// Maximum flow distortion strength slider bound.
pub const FLOW_DISTORTION_STRENGTH_MAX: f32 = 1.0;

/// Minimum flow animation speed slider bound.
pub const FLOW_ANIMATION_SPEED_MIN: f32 = 0.0;

/// Maximum flow animation speed slider bound.
pub const FLOW_ANIMATION_SPEED_MAX: f32 = 5.0;

/// Minimum smoke inherited rocket color intensity slider bound.
pub const SMOKE_INHERITED_COLOR_INTENSITY_MIN: f32 = 0.0;

/// Maximum smoke inherited rocket color intensity slider bound.
pub const SMOKE_INHERITED_COLOR_INTENSITY_MAX: f32 = 2.0;

/// Minimum preview viewport max zoom slider bound.
pub const SMOKE_PREVIEW_MAX_ZOOM_MIN: f32 = 2.0;

/// Maximum preview viewport max zoom slider bound.
pub const SMOKE_PREVIEW_MAX_ZOOM_MAX: f32 = 20.0;

/// Minimum preview simulated speed slider bound.
pub const SMOKE_PREVIEW_SIMULATED_SPEED_MIN: f32 = 0.0;

/// Maximum preview simulated speed slider bound.
pub const SMOKE_PREVIEW_SIMULATED_SPEED_MAX: f32 = 1000.0;

/// Minimum preview simulated angle offset slider bound.
pub const SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET_MIN: f32 = -180.0;

/// Maximum preview simulated angle offset slider bound.
pub const SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET_MAX: f32 = 180.0;

/// Texture path for smoke puff sprite.
pub const TEXTURE_SMOKE_PUFF_PATH: &str = "assets/textures/smoke_puff.png";

/// Default rocket sprite albedo color for the smoke preview viewport.
pub const DEFAULT_SMOKE_PREVIEW_ROCKET_COLOR: [f32; 3] = [1.0, 1.0, 1.0];

/// Default simulated rocket speed (m/s) for smoke particle ejection in preview.
pub const DEFAULT_SMOKE_PREVIEW_SIMULATED_SPEED: f32 = 400.0;

/// Default simulated rocket trajectory angle offset (degrees) in preview.
pub const DEFAULT_SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET: f32 = 0.0;

/// Velocity inheritance factor for smoke particles in ground frame (-5% of rocket velocity).
pub const SMOKE_EMISSION_VELOCITY_INHERITANCE_FACTOR: f32 = -0.05;

/// Relative exhaust velocity scaling factor for stationary preview viewport (-1.05 * rocket velocity).
pub const SMOKE_PREVIEW_RELATIVE_EXHAUST_SCALE: f32 = -1.05;

/// Minimum position jitter (pixels) for smoke particle emission.
pub const SMOKE_EMISSION_POSITION_OFFSET_MIN: f32 = -3.0;

/// Maximum position jitter (pixels) for smoke particle emission.
pub const SMOKE_EMISSION_POSITION_OFFSET_MAX: f32 = 3.0;

/// Minimum horizontal dispersion velocity for smoke emission.
pub const SMOKE_EMISSION_DISPERSION_X_MIN: f32 = -10.0;

/// Maximum horizontal dispersion velocity for smoke emission.
pub const SMOKE_EMISSION_DISPERSION_X_MAX: f32 = 10.0;

/// Minimum vertical dispersion velocity for smoke emission.
pub const SMOKE_EMISSION_DISPERSION_Y_MIN: f32 = -12.0;

/// Maximum vertical dispersion velocity for smoke emission.
pub const SMOKE_EMISSION_DISPERSION_Y_MAX: f32 = -3.0;

/// Minimum initial opacity (alpha) for volumetric smoke emission.
pub const SMOKE_EMISSION_OPACITY_MIN: f32 = 0.35;

/// Maximum initial opacity (alpha) for volumetric smoke emission.
pub const SMOKE_EMISSION_OPACITY_MAX: f32 = 0.60;

/// Minimum lifetime & size variation multiplier for randomized particle emission.
pub const SMOKE_EMISSION_VARIATION_MIN: f32 = 0.85;

/// Maximum lifetime & size variation multiplier for randomized particle emission.
pub const SMOKE_EMISSION_VARIATION_MAX: f32 = 1.15;

/// Default texture path for Rocket particle type.
pub const TEXTURE_ROCKET_PATH: &str =
    "assets/textures/04ddeae2-7367-45f1-87e0-361d1d242630_scaled.png";

/// Default texture path for Explosion particle type.
pub const TEXTURE_EXPLOSION_CIRCLE_PATH: &str =
    "assets/textures/kenney_particle-pack/PNG (Black background)/circle_05.png";

/// Default texture path for Smoke particle type.
pub const TEXTURE_SMOKE_PATH: &str =
    "assets/textures/toppng.com-realistic-smoke-texture-with-soft-particle-edges-png-399x385.png";

/// Default texture path for Trail particle type.
pub const TEXTURE_TRAIL_TRACE_PATH: &str =
    "assets/textures/kenney_particle-pack/PNG (Black background)/trace_03.png";

/// Texture path for heart explosion shape.
pub const SHAPE_HEART_PATH: &str = "assets/textures/explosion_shapes/heart.png";

/// Texture path for star explosion shape.
pub const SHAPE_STAR_PATH: &str = "assets/textures/explosion_shapes/star.png";

/// Texture path for smiley explosion shape.
pub const SHAPE_SMILEY_PATH: &str = "assets/textures/explosion_shapes/smiley.png";

/// Texture path for note explosion shape.
pub const SHAPE_NOTE_PATH: &str = "assets/textures/explosion_shapes/note.png";

/// Directory containing explosion shape PNG textures.
pub const EXPLOSION_SHAPES_DIR: &str = "assets/textures/explosion_shapes";

/// Formats the texture path for a given explosion shape file stem.
pub fn get_explosion_shape_texture_path(file_stem: &str) -> String {
    format!("{}/{}.png", EXPLOSION_SHAPES_DIR, file_stem)
}

/// Texture path for ring explosion shape.
pub const SHAPE_RING_PATH: &str = "assets/textures/explosion_shapes/ring.png";

// Preset defaults (Scale, Flight Time)
pub const PRESET_HEART_SCALE: f32 = 150.0;
pub const PRESET_HEART_FLIGHT_TIME: f32 = 1.5;

pub const PRESET_STAR_SCALE: f32 = 180.0;
pub const PRESET_STAR_FLIGHT_TIME: f32 = 1.5;

pub const PRESET_SMILEY_SCALE: f32 = 200.0;
pub const PRESET_SMILEY_FLIGHT_TIME: f32 = 2.0;

pub const PRESET_NOTE_SCALE: f32 = 160.0;
pub const PRESET_NOTE_FLIGHT_TIME: f32 = 1.5;

pub const PRESET_RING_SCALE: f32 = 190.0;
pub const PRESET_RING_FLIGHT_TIME: f32 = 1.8;

/// Specification for an explosion shape preset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplosionPresetSpec {
    pub name: &'static str,
    pub stem: &'static str,
    pub path: &'static str,
    pub default_scale: f32,
    pub default_flight_time: f32,
}

impl ExplosionPresetSpec {
    /// Look up a preset specification by its file stem (case-insensitive).
    pub fn find_by_stem(stem: &str) -> Option<&'static ExplosionPresetSpec> {
        let key = stem.trim().to_lowercase();
        PRESET_DEFINITIONS.iter().find(|p| p.stem == key)
    }
}

/// Canonical array of available explosion shape presets.
pub const PRESET_DEFINITIONS: &[ExplosionPresetSpec] = &[
    ExplosionPresetSpec {
        name: "Heart",
        stem: "heart",
        path: SHAPE_HEART_PATH,
        default_scale: PRESET_HEART_SCALE,
        default_flight_time: PRESET_HEART_FLIGHT_TIME,
    },
    ExplosionPresetSpec {
        name: "Star",
        stem: "star",
        path: SHAPE_STAR_PATH,
        default_scale: PRESET_STAR_SCALE,
        default_flight_time: PRESET_STAR_FLIGHT_TIME,
    },
    ExplosionPresetSpec {
        name: "Smiley",
        stem: "smiley",
        path: SHAPE_SMILEY_PATH,
        default_scale: PRESET_SMILEY_SCALE,
        default_flight_time: PRESET_SMILEY_FLIGHT_TIME,
    },
    ExplosionPresetSpec {
        name: "Note",
        stem: "note",
        path: SHAPE_NOTE_PATH,
        default_scale: PRESET_NOTE_SCALE,
        default_flight_time: PRESET_NOTE_FLIGHT_TIME,
    },
    ExplosionPresetSpec {
        name: "Ring",
        stem: "ring",
        path: SHAPE_RING_PATH,
        default_scale: PRESET_RING_SCALE,
        default_flight_time: PRESET_RING_FLIGHT_TIME,
    },
];

// GUI Control Slider Bounds
pub const SLIDER_ROCKETS_MIN: i32 = 1;
pub const SLIDER_ROCKETS_MAX: i32 = 100;

pub const SLIDER_PARTICLES_EXPLOSION_MIN: i32 = 10;
pub const SLIDER_PARTICLES_EXPLOSION_MAX: i32 = 1000;

pub const SLIDER_PARTICLES_TRAIL_MIN: i32 = 0;
pub const SLIDER_PARTICLES_TRAIL_MAX: i32 = 200;

pub const SLIDER_SMOKE_PARTICLES_MIN: i32 = 100;
pub const SLIDER_SMOKE_PARTICLES_MAX: i32 = 16384;

pub const SLIDER_INTERVAL_MEAN_MIN: f32 = 0.05;
pub const SLIDER_INTERVAL_MEAN_MAX: f32 = 5.0;

pub const SLIDER_INTERVAL_VAR_MIN: f32 = 0.0;
pub const SLIDER_INTERVAL_VAR_MAX: f32 = 3.0;

pub const SLIDER_MAX_NEXT_INT_MIN: f32 = 0.1;
pub const SLIDER_MAX_NEXT_INT_MAX: f32 = 10.0;

pub const SLIDER_SPAWN_MARGIN_MIN: f32 = 0.0;
pub const SLIDER_SPAWN_MARGIN_MAX: f32 = 200.0;

pub const SLIDER_ROCKETS_MIN_CLAMP: u32 = 1;
pub const SLIDER_PARTICLES_EXPLOSION_MIN_CLAMP: u32 = 10;

pub const SLIDER_SPAWN_ANGLE_MIN: f32 = 0.0;
pub const SLIDER_ANGLE_VAR_MIN: f32 = 0.0;
pub const SLIDER_ANGLE_VAR_MAX: f32 = 1.57;

pub const SLIDER_SPAWN_SPEED_MIN: f32 = 10.0;
pub const SLIDER_SPAWN_SPEED_MAX: f32 = 2000.0;
pub const SLIDER_INIT_SPEED_MAX: f32 = 1500.0;

pub const SLIDER_GRAVITY_MIN: f32 = -2000.0;
pub const SLIDER_GRAVITY_MAX: f32 = 2000.0;

pub const SLIDER_EXPLOSION_THRESH_MIN: f32 = 0.0;
pub const SLIDER_EXPLOSION_THRESH_MAX: f32 = 500.0;

pub const SLIDER_EXPLOSION_VEL_MIN: f32 = 1.0;
pub const SLIDER_EXPLOSION_VEL_MAX: f32 = 2000.0;

pub const SLIDER_IMAGE_SCALE_MIN: f32 = 20.0;
pub const SLIDER_IMAGE_SCALE_MAX: f32 = 500.0;

pub const SLIDER_FLIGHT_TIME_MIN: f32 = 0.2;
pub const SLIDER_FLIGHT_TIME_MAX: f32 = 5.0;

pub const SLIDER_WEIGHT_MIN: f32 = 0.0;
pub const SLIDER_WEIGHT_MAX: f32 = 10.0;

pub const SLIDER_SMOKE_SPAWN_RATE_MIN: f32 = 0.0;
pub const SLIDER_SMOKE_SPAWN_RATE_MAX: f32 = 120.0;

pub const SLIDER_SMOKE_INIT_SIZE_MIN: f32 = 1.0;
pub const SLIDER_SMOKE_INIT_SIZE_MAX: f32 = 40.0;

pub const SLIDER_SMOKE_GROWTH_MIN: f32 = 0.0;
pub const SLIDER_SMOKE_GROWTH_MAX: f32 = 5.0;

pub const SLIDER_SMOKE_FADE_DUR_MIN: f32 = 0.05;
pub const SLIDER_SMOKE_FADE_DUR_MAX: f32 = 3.0;

pub const SLIDER_SMOKE_INTENSITY_MIN: f32 = 0.0;
pub const SLIDER_SMOKE_INTENSITY_MAX: f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub struct SmokePresetSpec {
    pub name: &'static str,
    pub edge_width: f32,
    pub edge_color: [f32; 3],
    pub custom_color: [f32; 3],
    pub intensity: f32,
}

pub const SMOKE_PRESET_FIRE_EMBER: SmokePresetSpec = SmokePresetSpec {
    name: "Fire & Ember",
    edge_width: 0.12,
    edge_color: [1.0, 0.4, 0.05],
    custom_color: [0.15, 0.15, 0.15],
    intensity: 0.85,
};

pub const SMOKE_PRESET_PLASMA_BLUE: SmokePresetSpec = SmokePresetSpec {
    name: "Plasma Blue",
    edge_width: 0.15,
    edge_color: [0.1, 0.8, 1.0],
    custom_color: [0.8, 0.9, 1.0],
    intensity: 1.0,
};

pub const SMOKE_PRESET_VOLUMETRIC_CLOUD: SmokePresetSpec = SmokePresetSpec {
    name: "Volumetric Cloud",
    edge_width: 0.05,
    edge_color: [0.75, 0.75, 0.75],
    custom_color: [0.85, 0.85, 0.85],
    intensity: 0.5,
};

pub const SMOKE_PRESET_TOXIC_PLASMA: SmokePresetSpec = SmokePresetSpec {
    name: "Toxic Plasma",
    edge_width: 0.18,
    edge_color: [0.2, 1.0, 0.3],
    custom_color: [0.1, 0.25, 0.1],
    intensity: 0.9,
};

pub const SMOKE_PRESET_DEFINITIONS: &[SmokePresetSpec] = &[
    SMOKE_PRESET_FIRE_EMBER,
    SMOKE_PRESET_PLASMA_BLUE,
    SMOKE_PRESET_VOLUMETRIC_CLOUD,
    SMOKE_PRESET_TOXIC_PLASMA,
];

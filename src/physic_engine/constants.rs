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

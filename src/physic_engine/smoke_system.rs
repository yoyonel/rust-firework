use crate::physic_engine::{config::PhysicConfig, particle::Particle, ParticleType};
use glam::{Vec2, Vec3 as Color};
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleLifecycle {
    pub age: f32,
    pub max_life: f32,
}

impl Default for ParticleLifecycle {
    fn default() -> Self {
        Self {
            age: 0.0,
            max_life: 0.75,
        }
    }
}

impl ParticleLifecycle {
    pub fn progress(&self) -> f32 {
        if self.max_life > 0.0 {
            (self.age / self.max_life).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    pub fn is_expired(&self) -> bool {
        self.age >= self.max_life
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleSizing {
    pub initial_size: f32,
    pub current_size: f32,
    pub growth_rate: f32,
}

impl Default for ParticleSizing {
    fn default() -> Self {
        Self {
            initial_size: 0.0,
            current_size: 0.0,
            growth_rate: 1.2,
        }
    }
}

impl ParticleSizing {
    pub fn update(&mut self, progress: f32) {
        self.current_size = self.initial_size * (1.0 + progress * self.growth_rate);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleOpacity {
    pub initial_alpha: f32,
    pub alpha: f32,
}

impl Default for ParticleOpacity {
    fn default() -> Self {
        Self {
            initial_alpha: 0.45,
            alpha: 0.0,
        }
    }
}

impl ParticleOpacity {
    pub fn update(&mut self, progress: f32) {
        self.alpha = (self.initial_alpha * (1.0 - progress)).max(0.0);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SmokeParticle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub color: Color,
    pub rocket_color: Color,
    pub sizing: ParticleSizing,
    pub opacity: ParticleOpacity,
    pub lifecycle: ParticleLifecycle,
    pub rotation: f32,
    pub active: bool,
}

impl Default for SmokeParticle {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            color: Color::splat(0.85),
            rocket_color: Color::splat(0.85),
            sizing: ParticleSizing::default(),
            opacity: ParticleOpacity::default(),
            lifecycle: ParticleLifecycle::default(),
            rotation: 0.0,
            active: false,
        }
    }
}

impl SmokeParticle {
    pub fn to_particle(&self) -> Particle {
        Particle {
            pos: self.pos,
            color: self.color,
            life: (1.0 - self.lifecycle.progress()) * self.lifecycle.max_life,
            max_life: self.lifecycle.max_life,
            size: self.sizing.current_size,
            angle: self.rotation,
            vel: self.vel,
            active: self.active,
            particle_type: ParticleType::Smoke,
        }
    }
}

#[derive(Debug)]
pub struct SmokeSystem {
    pub particles: Vec<SmokeParticle>,
    active_count: usize,
}

impl SmokeSystem {
    pub fn new(capacity: usize) -> Self {
        Self {
            particles: vec![SmokeParticle::default(); capacity],
            active_count: 0,
        }
    }

    pub fn resize(&mut self, capacity: usize) {
        self.particles.resize(capacity, SmokeParticle::default());
        self.active_count = self.active_count.min(capacity);
    }

    /// Shared particle initialization: color, sizing, opacity, rotation, lifecycle.
    /// Called by both `emit` and `emit_preview` after setting position and velocity.
    fn init_particle_common(
        p: &mut SmokeParticle,
        rocket_color: Color,
        config: &PhysicConfig,
        rng: &mut impl Rng,
    ) {
        p.rocket_color = rocket_color;
        p.color = match config.smoke_color_mode {
            crate::physic_engine::config::SmokeColorMode::RocketColor => {
                rocket_color * config.smoke_inherited_color_intensity
            }
            crate::physic_engine::config::SmokeColorMode::Custom => {
                Color::from_array(config.smoke_custom_color)
            }
        };

        let initial_size = config.smoke_initial_size
            * rng.random_range(
                crate::physic_engine::constants::SMOKE_EMISSION_VARIATION_MIN
                    ..=crate::physic_engine::constants::SMOKE_EMISSION_VARIATION_MAX,
            );
        p.sizing = ParticleSizing {
            initial_size,
            current_size: initial_size,
            growth_rate: config.smoke_growth_rate_multiplier,
        };

        let initial_alpha = rng.random_range(
            crate::physic_engine::constants::SMOKE_EMISSION_OPACITY_MIN
                ..=crate::physic_engine::constants::SMOKE_EMISSION_OPACITY_MAX,
        );
        p.opacity = ParticleOpacity {
            initial_alpha,
            alpha: initial_alpha,
        };

        p.rotation = rng.random_range(0.0..std::f32::consts::TAU);
        p.lifecycle = ParticleLifecycle {
            age: 0.0,
            max_life: config.smoke_fade_duration
                * rng.random_range(
                    crate::physic_engine::constants::SMOKE_EMISSION_VARIATION_MIN
                        ..=crate::physic_engine::constants::SMOKE_EMISSION_VARIATION_MAX,
                ),
        };
        p.active = true;
    }

    /// Computes randomized emission offset and tail dispersion vectors.
    fn compute_emission_vectors(rng: &mut impl Rng) -> (Vec2, Vec2) {
        let offset = Vec2::new(
            rng.random_range(
                crate::physic_engine::constants::SMOKE_EMISSION_POSITION_OFFSET_MIN
                    ..=crate::physic_engine::constants::SMOKE_EMISSION_POSITION_OFFSET_MAX,
            ),
            rng.random_range(
                crate::physic_engine::constants::SMOKE_EMISSION_POSITION_OFFSET_MIN
                    ..=crate::physic_engine::constants::SMOKE_EMISSION_POSITION_OFFSET_MAX,
            ),
        );
        let tail_dispersion = Vec2::new(
            rng.random_range(
                crate::physic_engine::constants::SMOKE_EMISSION_DISPERSION_X_MIN
                    ..=crate::physic_engine::constants::SMOKE_EMISSION_DISPERSION_X_MAX,
            ),
            rng.random_range(
                crate::physic_engine::constants::SMOKE_EMISSION_DISPERSION_Y_MIN
                    ..=crate::physic_engine::constants::SMOKE_EMISSION_DISPERSION_Y_MAX,
            ),
        );
        (offset, tail_dispersion)
    }

    /// Spawns soft, volumetric smoke particles at rocket base with dynamic color selection.
    pub fn emit(
        &mut self,
        rocket_pos: Vec2,
        rocket_vel: Vec2,
        rocket_color: Color,
        config: &PhysicConfig,
        rng: &mut impl Rng,
    ) {
        crate::tracy_zone!("SmokeSystem::emit", 0x33AAFF);
        if self.active_count < self.particles.len() {
            let (offset, tail_dispersion) = Self::compute_emission_vectors(rng);
            let p = &mut self.particles[self.active_count];
            p.pos = rocket_pos + offset;
            p.vel = rocket_vel
                * crate::physic_engine::constants::SMOKE_EMISSION_VELOCITY_INHERITANCE_FACTOR
                + tail_dispersion;
            Self::init_particle_common(p, rocket_color, config, rng);
            self.active_count += 1;
        }
    }

    /// Spawns smoke particles for stationary preview viewport where rocket position is fixed.
    /// Particles move backward at relative exhaust speed (-1.05 * simulated_rocket_vel + tail_dispersion)
    /// to accurately match the trail length observed in the main simulation.
    pub fn emit_preview(
        &mut self,
        nozzle_pos: Vec2,
        simulated_rocket_vel: Vec2,
        rocket_color: Color,
        config: &PhysicConfig,
        rng: &mut impl Rng,
    ) {
        crate::tracy_zone!("SmokeSystem::emit_preview", 0x33AAFF);
        if self.active_count < self.particles.len() {
            let (offset, tail_dispersion) = Self::compute_emission_vectors(rng);
            let p = &mut self.particles[self.active_count];
            p.pos = nozzle_pos + offset;
            p.vel = simulated_rocket_vel
                * crate::physic_engine::constants::SMOKE_PREVIEW_RELATIVE_EXHAUST_SCALE
                + tail_dispersion;
            Self::init_particle_common(p, rocket_color, config, rng);
            self.active_count += 1;
        }
    }

    /// Dissipation update: expansion scale growth, dynamic color/size sync, and smooth linear alpha fade-out.
    pub fn update(&mut self, dt: f32, config: &PhysicConfig) {
        crate::tracy_zone!("SmokeSystem::update", 0x33AAFF);
        if self.particles.len() != config.max_smoke_particles {
            self.resize(config.max_smoke_particles);
        }

        let custom_color = Color::from_array(config.smoke_custom_color);
        let active_slice = &mut self.particles[..self.active_count];

        // Phase 1: Vectorized math update
        for p in active_slice.iter_mut() {
            p.sizing.growth_rate = config.smoke_growth_rate_multiplier;
            match config.smoke_color_mode {
                crate::physic_engine::config::SmokeColorMode::Custom => {
                    p.color = custom_color;
                }
                crate::physic_engine::config::SmokeColorMode::RocketColor => {
                    p.color = p.rocket_color * config.smoke_inherited_color_intensity;
                }
            }

            p.lifecycle.age += dt;
            let progress = p.lifecycle.progress();
            p.sizing.update(progress);
            p.opacity.update(progress);

            p.pos += p.vel * dt;
        }

        // Phase 2: Swap-and-Pop for expired particles
        let mut i = 0;
        while i < self.active_count {
            if self.particles[i].lifecycle.is_expired() {
                self.particles[i].active = false;
                self.active_count -= 1;
                let last = self.active_count;
                self.particles.swap(i, last);
            } else {
                i += 1;
            }
        }
    }

    pub fn for_each_active(&self, f: &mut dyn FnMut(&SmokeParticle)) {
        for p in &self.particles[..self.active_count] {
            f(p);
        }
    }

    pub fn clear(&mut self) {
        for p in &mut self.particles[..self.active_count] {
            p.active = false;
        }
        self.active_count = 0;
    }
}

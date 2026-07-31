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
    free_indices: Vec<usize>,
}

impl SmokeSystem {
    pub fn new(capacity: usize) -> Self {
        Self {
            particles: vec![SmokeParticle::default(); capacity],
            free_indices: (0..capacity).rev().collect(),
        }
    }

    pub fn resize(&mut self, capacity: usize) {
        self.particles.resize(capacity, SmokeParticle::default());
        self.free_indices.clear();
        for (i, p) in self.particles.iter().enumerate() {
            if !p.active {
                self.free_indices.push(i);
            }
        }
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
        if let Some(idx) = self.free_indices.pop() {
            let p = &mut self.particles[idx];
            let offset_x = rng.random_range(-3.0..=3.0);
            let offset_y = rng.random_range(-3.0..=3.0);
            let tail_dispersion = Vec2::new(
                rng.random_range(-10.0..=10.0),
                rng.random_range(-12.0..=-3.0),
            );

            p.pos = rocket_pos + Vec2::new(offset_x, offset_y);
            p.vel = rocket_vel * -0.05 + tail_dispersion;
            p.rocket_color = rocket_color;
            p.color = match config.smoke_color_mode {
                crate::physic_engine::config::SmokeColorMode::RocketColor => {
                    rocket_color * config.smoke_inherited_color_intensity
                }
                crate::physic_engine::config::SmokeColorMode::Custom => {
                    Color::from_array(config.smoke_custom_color)
                }
            };

            let initial_size = config.smoke_initial_size * rng.random_range(0.85..=1.15);
            p.sizing = ParticleSizing {
                initial_size,
                current_size: initial_size,
                growth_rate: config.smoke_growth_rate_multiplier,
            };

            // Soft initial opacity (35% to 60%) for visible volumetric smoke
            let initial_alpha = rng.random_range(0.35..=0.60);
            p.opacity = ParticleOpacity {
                initial_alpha,
                alpha: initial_alpha,
            };

            p.rotation = rng.random_range(0.0..std::f32::consts::TAU);
            p.lifecycle = ParticleLifecycle {
                age: 0.0,
                max_life: config.smoke_fade_duration * rng.random_range(0.85..=1.15),
            };
            p.active = true;
        }
    }

    /// Dissipation update: expansion scale growth, dynamic color/size sync, and smooth linear alpha fade-out.
    pub fn update(&mut self, dt: f32, config: &PhysicConfig) {
        crate::tracy_zone!("SmokeSystem::update", 0x33AAFF);
        if self.particles.len() != config.max_smoke_particles {
            self.resize(config.max_smoke_particles);
        }

        let custom_color = Color::from_array(config.smoke_custom_color);

        for (i, p) in self.particles.iter_mut().enumerate() {
            if !p.active {
                continue;
            }

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
            if p.lifecycle.is_expired() {
                p.active = false;
                self.free_indices.push(i);
                continue;
            }

            let progress = p.lifecycle.progress();
            p.sizing.update(progress);
            p.opacity.update(progress);

            p.pos += p.vel * dt;
        }
    }

    pub fn for_each_active(&self, f: &mut dyn FnMut(&SmokeParticle)) {
        for p in self.particles.iter() {
            if p.active {
                f(p);
            }
        }
    }

    pub fn clear(&mut self) {
        self.free_indices.clear();
        for (i, p) in self.particles.iter_mut().enumerate() {
            p.active = false;
            self.free_indices.push(i);
        }
    }
}

#[cfg(debug_assertions)]
use log::debug;
use rand::Rng;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::physic_engine::{
    config::PhysicConfig, particle::Particle, particles_manager::ParticlesPool,
};
use glam::{Vec2, Vec4 as Color};

/// Compteur global pour générer des ID uniques pour les rockets
pub static ROCKET_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub const NB_PARTICLES_PER_EXPLOSION: usize = 256;
pub const NB_PARTICLES_PER_TRAIL: usize = 64;

/// Représentation d’une fusée
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Rocket {
    /// ID unique de la rocket
    pub id: u64,

    /// Position actuelle et précédente
    pub pos: Vec2,
    pub prev_pos: Vec2,

    /// Vitesse et couleur
    pub vel: Vec2,
    pub color: Color,

    /// État de la fusée
    pub exploded: bool,
    pub active: bool,

    /// Indices dans le `ParticlesManager` pour les particules d’explosion
    pub explosion_particle_indices: Option<Range<usize>>,

    /// Indices dans le `ParticlesManager` pour les particules de trail
    pub trail_particle_indices: Option<Range<usize>>,
    pub trail_index: usize,
    pub last_trail_pos: Vec2,
}

impl Default for Rocket {
    fn default() -> Self {
        Self::new(&PhysicConfig::default())
    }
}

impl Rocket {
    /// Crée une nouvelle fusée (non active)
    pub fn new(_config: &PhysicConfig) -> Self {
        Self {
            id: ROCKET_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            pos: Vec2::default(),
            prev_pos: Vec2::default(),
            vel: Vec2::default(),
            color: Color::ONE,
            exploded: false,
            active: false,
            explosion_particle_indices: None,
            trail_particle_indices: None,
            trail_index: 0,
            last_trail_pos: Vec2::default(),
        }
    }

    /// Retourne un itérateur sur toutes les particules actives de la fusée
    pub fn active_particles<'a>(
        &'a self,
        // particles_manager: &'a ParticlesManager,
        particles_pool_for_explosions: &'a ParticlesPool,
        particles_pool_for_trails: &'a ParticlesPool,
    ) -> impl Iterator<Item = &'a Particle> {
        let trails = self
            .trail_particle_indices
            .iter()
            .flat_map(|range| particles_pool_for_trails.get_particles(range))
            .filter(|p| p.active);
        let explosions = self
            .explosion_particle_indices
            .iter()
            .flat_map(|range| particles_pool_for_explosions.get_particles(range))
            .filter(|p| p.active);
        trails.chain(explosions)
    }

    /// Met à jour la fusée (mouvement, trails, explosions)
    pub fn update(
        &mut self,
        rng: &mut impl Rng,
        dt: f32,
        // particles_manager: &mut ParticlesManager,
        particles_pool_for_explosions: &mut ParticlesPool,
        particles_pool_for_trails: &mut ParticlesPool,
    ) {
        if !self.active {
            return;
        }

        const GRAVITY: Vec2 = Vec2::new(0.0, -200.0);

        self.update_movement(dt, GRAVITY);
        self.update_trails(dt, GRAVITY, particles_pool_for_trails);
        self.update_explosions(dt, rng, GRAVITY, particles_pool_for_explosions);
        self.remove_inactive_rockets(particles_pool_for_explosions, particles_pool_for_trails);
    }

    fn remove_inactive_rockets(
        &mut self,
        particles_pool_for_explosions: &ParticlesPool,
        particles_pool_for_trails: &ParticlesPool,
    ) {
        let exploded_done = self
            .explosion_particle_indices
            .as_ref()
            .map(|range| {
                particles_pool_for_explosions
                    .get_particles(range)
                    .iter()
                    .all(|p| !p.active)
            })
            .unwrap_or(true);
        let trail_done = self
            .trail_particle_indices
            .as_ref()
            .map(|range| {
                particles_pool_for_trails
                    .get_particles(range)
                    .iter()
                    .all(|p| !p.active)
            })
            .unwrap_or(true);

        if self.exploded && exploded_done && trail_done {
            #[cfg(debug_assertions)]
            debug!(
                "Rocket {:?} inactive: all particles (explosion + trails) inactive",
                self.id
            );
            self.active = false;
        }
    }

    #[inline(always)]
    fn update_movement(&mut self, dt: f32, gravity: Vec2) {
        self.vel += gravity * dt;
        self.pos += self.vel * dt;
    }

    /// Gère la génération et la mise à jour des particules de trail
    #[inline(always)]
    fn update_trails(&mut self, dt: f32, gravity: Vec2, particles_manager: &mut ParticlesPool) {
        const TRAIL_SPACING: f32 = 2.0;
        let movement = self.pos - self.last_trail_pos;
        let dist = movement.length();

        // Alloue un bloc si nécessaire
        if self.trail_particle_indices.is_none() {
            self.trail_particle_indices = particles_manager.allocate_block();
        }

        if let Some(range) = &self.trail_particle_indices {
            let slice = particles_manager.get_particles_mut(range);

            if !self.exploded {
                let mut remaining_dist = dist;

                while remaining_dist >= TRAIL_SPACING {
                    let t = TRAIL_SPACING / dist;
                    let new_pos = self.last_trail_pos * (1.0 - t) + self.pos * t;
                    let i = self.trail_index % NB_PARTICLES_PER_TRAIL;

                    slice[i] = Particle {
                        pos: new_pos,
                        vel: Vec2::ZERO,
                        color: self.color,
                        life: 0.35,
                        max_life: 0.35,
                        size: 2.0,
                        active: true,
                    };

                    self.trail_index = (self.trail_index + 1) % NB_PARTICLES_PER_TRAIL;
                    self.last_trail_pos = new_pos;
                    remaining_dist -= TRAIL_SPACING;
                }
            }

            // Update trails
            for p in &mut slice[..] {
                if !p.active {
                    continue;
                }
                p.vel.y += gravity.y * dt;
                p.pos.y += p.vel.y * dt;
                p.life -= dt;
                p.active = p.life > 0.0;
            }
        }
    }

    #[inline(always)]
    fn update_explosions(
        &mut self,
        dt: f32,
        rng: &mut impl Rng,
        gravity: Vec2,
        particles_manager: &mut ParticlesPool,
    ) {
        if !self.exploded && self.vel.y <= 0.0 {
            self.trigger_explosion(rng, particles_manager);
        }

        if let Some(range) = &self.explosion_particle_indices {
            let slice = particles_manager.get_particles_mut(range);
            for p in &mut slice[..] {
                if !p.active {
                    continue;
                }
                p.vel.y += gravity.y * dt;
                p.pos += p.vel * dt;
                p.life -= dt;
                p.active = p.life > 0.0;
            }
        }
    }

    #[inline(always)]
    fn trigger_explosion(&mut self, rng: &mut impl Rng, particles_manager: &mut ParticlesPool) {
        self.exploded = true;

        if self.explosion_particle_indices.is_none() {
            self.explosion_particle_indices = particles_manager.allocate_block();
        }

        if let Some(range) = &self.explosion_particle_indices {
            let slice = particles_manager.get_particles_mut(range);
            for p in slice.iter_mut() {
                let angle = rng.random_range(0.0..(2.0 * std::f32::consts::PI));
                let speed = rng.random_range(60.0..200.0);
                let life = rng.random_range(0.75..1.5);

                *p = Particle {
                    pos: self.pos,
                    vel: Vec2::from_angle(angle) * speed,
                    color: self.color,
                    life,
                    max_life: life,
                    size: rng.random_range(3.0..6.0),
                    active: true,
                };
            }
        }
    }

    /// Réinitialise une fusée inactive pour la réutiliser sans réallocation
    pub fn reset(
        &mut self,
        cfg: &PhysicConfig,
        rng: &mut rand::rngs::ThreadRng,
        window_width: f32,
    ) {
        let margin_min_x = cfg.spawn_rocket_margin;
        let margin_max_x = window_width - cfg.spawn_rocket_margin;
        let cx = rng.random_range(margin_min_x..=margin_max_x);

        let angle = rng.random_range(
            (cfg.spawn_rocket_vertical_angle - cfg.spawn_rocket_angle_variation)
                ..=(cfg.spawn_rocket_vertical_angle + cfg.spawn_rocket_angle_variation),
        );

        let speed = rng.random_range(cfg.spawn_rocket_min_speed..=cfg.spawn_rocket_max_speed);
        self.vel = Vec2::from_angle(angle) * speed;
        self.color = Color::new(
            rng.random_range(0.5..=1.0),
            rng.random_range(0.5..=1.0),
            rng.random_range(0.5..=1.0),
            1.0,
        );

        let pos = Vec2::new(cx, 0.0);
        self.pos = pos;
        self.prev_pos = pos;
        self.last_trail_pos = pos;
        self.trail_index = 0;
        self.active = true;
        self.exploded = false;
        self.explosion_particle_indices = None;
        self.trail_particle_indices = None;
    }
}

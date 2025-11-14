#[cfg(debug_assertions)]
use log::debug;
use rand::Rng;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::physic_engine::{
    config::PhysicConfig,
    particle::Particle,
    particles_pools::{ParticlesPool, ParticlesPoolsForRockets},
};
use glam::{Vec2, Vec4 as Color};

/// Compteur global pour générer des ID uniques pour les rockets
pub static ROCKET_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Représentation d’une fusée
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Rocket {
    /// ID unique de la rocket
    pub id: u64,

    /// Position actuelle et précédente
    pub pos: Vec2,

    /// Vitesse et couleur
    pub vel: Vec2,
    pub color: Color,

    /// État de la fusée
    pub exploded: bool,
    pub active: bool,

    /// Indices dans le pool des particules d'explosions
    pub explosion_particle_indices: Option<Range<usize>>,

    /// Indices dans le pool des particules de trails
    pub trail_particle_indices: Option<Range<usize>>,
    pub trail_index: usize,
    pub last_trail_pos: Vec2,

    pub config: PhysicConfig,
}

impl Default for Rocket {
    fn default() -> Self {
        Self::new(&PhysicConfig::default())
    }
}

impl Rocket {
    /// Crée une nouvelle fusée (non active)
    pub fn new(config: &PhysicConfig) -> Self {
        Self {
            id: ROCKET_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            pos: Vec2::default(),
            vel: Vec2::default(),
            color: Color::ONE,
            exploded: false,
            active: false,
            explosion_particle_indices: None,
            trail_particle_indices: None,
            trail_index: 0,
            last_trail_pos: Vec2::default(),
            config: config.clone(),
        }
    }

    /// Retourne un itérateur sur toutes les particules actives de la fusée
    pub fn active_particles<'a>(
        &'a self,
        particles_pools: &'a ParticlesPoolsForRockets,
    ) -> impl Iterator<Item = &'a Particle> {
        let trails = self
            .trail_particle_indices
            .iter()
            .flat_map(|range| {
                particles_pools
                    .particles_pool_for_trails
                    .get_particles(range)
            })
            .filter(|p| p.active);
        let explosions = self
            .explosion_particle_indices
            .iter()
            .flat_map(|range| {
                particles_pools
                    .particles_pool_for_explosions
                    .get_particles(range)
            })
            .filter(|p| p.active);
        trails.chain(explosions)
    }

    /// Retourne un itérateur paresseux sur toutes les particules "actives" (`is_active`)
    /// appartenant à cette fusée.
    ///
    /// Cette fonction est **zéro allocation** :  
    /// - pas de `Vec` temporaire  
    /// - pas de `Box<dyn Iterator>`  
    /// - pas de copie CPU → CPU  
    ///
    /// Le résultat est un pipeline d’itérateurs fusionnés, traité de manière lazy,
    /// extrêmement efficace côté CPU et parfaitement adapté à un transfert contigu
    /// vers un buffer GPU en mode persistent mapping.
    pub fn iter_active_particles<'a>(
        &'a self,
        pools: &'a ParticlesPoolsForRockets,
    ) -> impl Iterator<Item = &'a Particle> + 'a {
        // `trail_particle_indices` contient les indices/ranges des particules de trainée
        // associées à cette fusée.
        let trails = self
            .trail_particle_indices
            .iter()
            // Pour chaque "range", on récupère un itérateur sur les particules
            // correspondantes dans le pool, puis `flat_map` fusionne tout cela
            // en un flux unique.
            .flat_map(move |range| pools.particles_pool_for_trails.get_particles(range))
            // On filtre pour ne garder que les particules actives.
            // Aucun coût mémoire : le filtrage est lazy et ne construit pas de collections.
            .filter(|p| p.active);
        let explosions = self
            .explosion_particle_indices
            .iter()
            // Pour chaque "range", on récupère un itérateur sur les particules
            // correspondantes dans le pool, puis `flat_map` fusionne tout cela
            // en un flux unique.
            .flat_map(move |range| pools.particles_pool_for_explosions.get_particles(range))
            // On filtre pour ne garder que les particules actives.
            // Aucun coût mémoire : le filtrage est lazy et ne construit pas de collections.
            .filter(|p| p.active);
        trails.chain(explosions)
    }

    /// Retourne un itérateur paresseux sur toutes les particules "têtes" (`is_head`)
    /// appartenant à cette fusée.
    ///
    /// Cette fonction est **zéro allocation** :  
    /// - pas de `Vec` temporaire  
    /// - pas de `Box<dyn Iterator>`  
    /// - pas de copie CPU → CPU  
    ///
    /// Le résultat est un pipeline d’itérateurs fusionnés, traité de manière lazy,
    /// extrêmement efficace côté CPU et parfaitement adapté à un transfert contigu
    /// vers un buffer GPU en mode persistent mapping.
    pub fn iter_active_heads<'a>(
        &'a self,
        pools: &'a ParticlesPoolsForRockets,
    ) -> impl Iterator<Item = &'a Particle> + 'a {
        // `trail_particle_indices` contient les indices/ranges des particules de trainée
        // associées à cette fusée.
        self.trail_particle_indices
            .iter()
            // Pour chaque "range", on récupère un itérateur sur les particules
            // correspondantes dans le pool, puis `flat_map` fusionne tout cela
            // en un flux unique.
            .flat_map(move |range| pools.particles_pool_for_trails.get_particles(range))
            // On filtre pour ne garder que les particules actives et marquées `is_head`.
            // Aucun coût mémoire : le filtrage est lazy et ne construit pas de collections.
            .filter(|p| p.active && p.is_head)
    }

    pub fn head_particle<'a>(&'a self, pools: &'a ParticlesPoolsForRockets) -> &'a Particle {
        let range = self
            .trail_particle_indices
            .as_ref()
            .expect("Trail must exist");

        let particles = pools.particles_pool_for_trails.get_particles(range);

        particles
            .first()
            .expect("Trail range should always contain at least one particle")
    }

    /// Met à jour la fusée (mouvement, trails, explosions)
    pub fn update(
        &mut self,
        rng: &mut impl Rng,
        dt: f32,
        particles_pools: &mut ParticlesPoolsForRockets,
    ) {
        if !self.active {
            return;
        }

        const GRAVITY: Vec2 = Vec2::new(0.0, -200.0);

        self.update_movement(dt, GRAVITY);
        self.update_trails(dt, GRAVITY, &mut particles_pools.particles_pool_for_trails);
        self.update_explosions(
            dt,
            rng,
            GRAVITY,
            &mut particles_pools.particles_pool_for_explosions,
        );
        self.remove_inactive_rockets(particles_pools);
    }

    fn remove_inactive_rockets(&mut self, particles_pools: &ParticlesPoolsForRockets) {
        let exploded_done = self
            .explosion_particle_indices
            .as_ref()
            .map(|range| {
                particles_pools
                    .particles_pool_for_explosions
                    .get_particles(range)
                    .iter()
                    .all(|p| !p.active)
            })
            .unwrap_or(true);
        let trail_done = self
            .trail_particle_indices
            .as_ref()
            .map(|range| {
                particles_pools
                    .particles_pool_for_trails
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
    fn update_trails(&mut self, dt: f32, gravity: Vec2, particles_pool: &mut ParticlesPool) {
        const TRAIL_SPACING: f32 = 2.0;
        let nb_particles_per_trail: usize = self.config.particles_per_trail;

        let movement = self.pos - self.last_trail_pos;
        let dist = movement.length();

        // Alloue un bloc si nécessaire
        if self.trail_particle_indices.is_none() {
            self.trail_particle_indices = particles_pool.allocate_block();
        }

        if let Some(range) = &self.trail_particle_indices {
            let slice = particles_pool.get_particles_mut(range);

            if !self.exploded {
                let mut remaining_dist = dist;

                while remaining_dist >= TRAIL_SPACING {
                    let t = TRAIL_SPACING / dist;
                    let new_pos = self.last_trail_pos * (1.0 - t) + self.pos * t;

                    let delta = new_pos - self.last_trail_pos;

                    // Vérifier que delta n'est pas nul
                    let angle = if delta.length_squared() > 0.0 {
                        // La rocket/fusée (son sprite/image) est orientée vers le haut (0.0, +1.0)
                        delta.angle_to(Vec2::new(0.0, 1.0))
                    } else {
                        0.0 // angle par défaut si delta nul
                    };

                    let i = self.trail_index % nb_particles_per_trail;
                    slice[i] = Particle {
                        pos: new_pos,
                        vel: Vec2::ZERO,
                        color: self.color,
                        life: 0.35,
                        max_life: 0.35,
                        size: 2.0,
                        active: true,
                        angle,
                        is_head: true,
                    };

                    self.trail_index = (self.trail_index + 1) % nb_particles_per_trail;
                    self.last_trail_pos = new_pos;
                    remaining_dist -= TRAIL_SPACING;
                }
            }

            // Update trails
            for p in &mut slice[..] {
                if !p.active {
                    continue;
                }
                // FIXME: Trouver un meilleur moyen de déterminer la tête de la traînée
                p.is_head = (p.max_life - p.life) < 0.025;
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
        particles_pool: &mut ParticlesPool,
    ) {
        if !self.exploded && self.vel.y <= 0.0 {
            self.trigger_explosion(rng, particles_pool);
        }

        if let Some(range) = &self.explosion_particle_indices {
            let slice = particles_pool.get_particles_mut(range);
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
    fn trigger_explosion(&mut self, rng: &mut impl Rng, particles_pool: &mut ParticlesPool) {
        self.exploded = true;

        if self.explosion_particle_indices.is_none() {
            self.explosion_particle_indices = particles_pool.allocate_block();
        }

        if let Some(range) = &self.explosion_particle_indices {
            let slice = particles_pool.get_particles_mut(range);
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
                    angle,
                    is_head: false,
                };
            }
        }
    }

    fn random_color(rng: &mut rand::rngs::ThreadRng) -> Color {
        Color::new(
            rng.random_range(0.5..=1.0),
            rng.random_range(0.5..=1.0),
            rng.random_range(0.5..=1.0),
            1.0,
        )
    }

    fn random_vel(cfg: &PhysicConfig, rng: &mut rand::rngs::ThreadRng) -> Vec2 {
        let angle = rng.random_range(
            (cfg.spawn_rocket_vertical_angle - cfg.spawn_rocket_angle_variation)
                ..=(cfg.spawn_rocket_vertical_angle + cfg.spawn_rocket_angle_variation),
        );
        Vec2::from_angle(angle)
            * rng.random_range(cfg.spawn_rocket_min_speed..=cfg.spawn_rocket_max_speed)
    }

    /// Réinitialise une fusée inactive pour la réutiliser sans réallocation
    pub fn reset(
        &mut self,
        cfg: &PhysicConfig,
        rng: &mut rand::rngs::ThreadRng,
        window_width: f32,
    ) {
        let cx = rng.random_range(cfg.spawn_rocket_margin..=window_width - cfg.spawn_rocket_margin);
        let pos = Vec2::new(cx, 0.0);

        // Assignations in-place
        self.pos = pos;
        self.last_trail_pos = pos;
        self.vel = Rocket::random_vel(cfg, rng);
        self.color = Rocket::random_color(rng);
        self.trail_index = 0;
        self.active = true;
        self.exploded = false;
        self.explosion_particle_indices = None;
        self.trail_particle_indices = None;
    }
}

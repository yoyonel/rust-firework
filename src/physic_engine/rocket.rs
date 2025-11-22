#[cfg(debug_assertions)]
use log::debug;
use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::physic_engine::{
    config::PhysicConfig,
    particle::Particle,
    particles_pools::{ParticlesPool, ParticlesPoolsForRockets, PoolKind},
    ParticleType,
};
use glam::{Vec2, Vec4 as Color};

/// Compteur global pour générer des ID uniques pour les rockets
pub static ROCKET_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Représentation d’une fusée
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Rocket {
    rng: SmallRng,

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

    head: Particle,
}

impl Default for Rocket {
    fn default() -> Self {
        let mut thread_rng = rand::rng();
        Self::new(&mut thread_rng)
    }
}

impl Rocket {
    /// Crée une nouvelle fusée (non active)
    pub fn new(global_rng: &mut impl Rng) -> Self {
        let rng = SmallRng::from_rng(global_rng);
        let mut r = Rocket {
            rng,
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
            head: Particle::default(),
        };
        r.update_head_particle();
        r
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
            .flat_map(move |range| pools.access(PoolKind::Trails, range))
            // On filtre pour ne garder que les particules actives.
            // Aucun coût mémoire : le filtrage est lazy et ne construit pas de collections.
            .filter(|p| p.active);
        let explosions = self
            .explosion_particle_indices
            .iter()
            .flat_map(move |range| pools.access(PoolKind::Explosions, range))
            .filter(|p| p.active);
        trails.chain(explosions)
    }

    pub fn head_particle(&self) -> &Particle {
        &self.head
    }

    /// Met à jour la fusée (mouvement, trails, explosions)
    pub fn update(
        &mut self,
        dt: f32,
        particles_pools: &mut ParticlesPoolsForRockets,
        config: &PhysicConfig,
    ) {
        if !self.active {
            return;
        }

        const GRAVITY: Vec2 = Vec2::new(0.0, -200.0);

        self.update_movement(dt, GRAVITY);
        self.update_trails(
            dt,
            GRAVITY,
            &mut particles_pools.particles_pool_for_trails,
            config,
        );
        self.update_explosions(
            dt,
            GRAVITY,
            &mut particles_pools.particles_pool_for_explosions,
        );
        self.remove_inactive_rockets(particles_pools);

        self.update_head_particle();
    }

    fn remove_inactive_rockets(&mut self, particles_pools: &ParticlesPoolsForRockets) {
        let exploded_done = self
            .explosion_particle_indices
            .as_ref()
            .map(|range| {
                particles_pools
                    .access(PoolKind::Explosions, range)
                    .iter()
                    .all(|p| !p.active)
            })
            .unwrap_or(true);
        let trail_done = self
            .trail_particle_indices
            .as_ref()
            .map(|range| {
                particles_pools
                    .access(PoolKind::Trails, range)
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
    fn update_trails(
        &mut self,
        dt: f32,
        gravity: Vec2,
        particles_pool: &mut ParticlesPool,
        config: &PhysicConfig,
    ) {
        // Alloue un bloc si nécessaire
        if self.trail_particle_indices.is_none() {
            self.trail_particle_indices = particles_pool.allocate_block();
        }

        let Some(range) = &self.trail_particle_indices else {
            return;
        };

        let slice = particles_pool.get_particles_mut(range);

        // 1) SPAWN : génération des particules de trail
        if !self.exploded {
            self.spawn_trail_particles(slice, config);
        }

        // 2) UPDATE : intégration physique des particules existantes
        self.integrate_trail_particles(slice, dt, gravity);
    }

    /// Génère les nouvelles particules de trail selon la distance parcourue.
    ///
    /// Cette partie était auparavant intégrée dans `update_trails`.
    /// Elle gère exclusivement :
    ///  - le calcul du nombre de particules à spawn
    ///  - leur position
    ///  - leur orientation
    ///  - l’écriture dans le pool sans toucher à la physique.
    ///
    /// Cette fonction reste **zéro allocation** et n'effectue que l’amorçage
    /// des particules dans la fenêtre du pool.
    #[inline(always)]
    fn spawn_trail_particles(&mut self, slice: &mut [Particle], config: &PhysicConfig) {
        const TRAIL_SPACING: f32 = 2.0;
        let nb_particles_per_trail = config.particles_per_trail;

        let movement = self.pos - self.last_trail_pos;
        let dist = movement.length();

        if dist <= 0.0001 {
            return;
        }

        let inv_dist = 1.0 / dist;
        let t_step = TRAIL_SPACING * inv_dist;
        let count = (dist / TRAIL_SPACING) as u32;

        for _ in 0..count {
            let new_pos = self.last_trail_pos * (1.0 - t_step) + self.pos * t_step;
            let i = self.trail_index % nb_particles_per_trail;

            slice[i] = Particle {
                pos: new_pos,
                vel: Vec2::ZERO,
                color: self.color,
                life: 0.35,
                max_life: 0.35,
                size: 2.0,
                active: true,
                angle: 0.0,
                particle_type: ParticleType::Trail,
            };

            self.trail_index = (self.trail_index + 1) % nb_particles_per_trail;
            self.last_trail_pos = new_pos;
        }
    }

    /// Met à jour les particules de trail existantes.
    ///
    /// Cette fonction est **zéro allocation** et applique uniquement :
    ///  - la gravité
    ///  - l’intégration de position
    ///  - la mise à jour de vie
    ///  - la désactivation automatique
    ///
    /// Aucun spawn, aucune écriture dans les indices de la rocket.
    /// Optimale pour l’inlining.
    #[inline(always)]
    fn integrate_trail_particles(&self, slice: &mut [Particle], dt: f32, gravity: Vec2) {
        // Update trails
        for p in slice {
            if !p.active {
                continue;
            }

            p.vel.y += gravity.y * dt;
            p.pos.y += p.vel.y * dt;
            p.life -= dt;
            p.active = p.life > 0.0;
        }
    }

    #[inline(always)]
    fn update_explosions(&mut self, dt: f32, gravity: Vec2, particles_pool: &mut ParticlesPool) {
        if !self.exploded && self.vel.y <= 0.0 {
            self.trigger_explosion(particles_pool);
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
    fn trigger_explosion(&mut self, particles_pool: &mut ParticlesPool) {
        self.exploded = true;

        if self.explosion_particle_indices.is_none() {
            self.explosion_particle_indices = particles_pool.allocate_block();
        }

        if let Some(range) = &self.explosion_particle_indices {
            let slice = particles_pool.get_particles_mut(range);
            for p in slice.iter_mut() {
                let angle = self.rng.random_range(0.0..(2.0 * std::f32::consts::PI));
                let speed = self.rng.random_range(60.0..200.0);
                let life = self.rng.random_range(0.75..1.5);

                *p = Particle {
                    pos: self.pos,
                    vel: Vec2::from_angle(angle) * speed,
                    color: self.color,
                    life,
                    max_life: life,
                    size: self.rng.random_range(3.0..6.0),
                    active: true,
                    angle,
                    particle_type: ParticleType::Explosion,
                };
            }
        }
    }

    fn random_color(&mut self) -> Color {
        Color::new(
            self.rng.random_range(0.5..=1.0),
            self.rng.random_range(0.5..=1.0),
            self.rng.random_range(0.5..=1.0),
            1.0,
        )
    }

    fn random_vel(&mut self, cfg: &PhysicConfig) -> Vec2 {
        let angle = self.rng.random_range(
            (cfg.spawn_rocket_vertical_angle - cfg.spawn_rocket_angle_variation)
                ..=(cfg.spawn_rocket_vertical_angle + cfg.spawn_rocket_angle_variation),
        );
        Vec2::from_angle(angle)
            * self
                .rng
                .random_range(cfg.spawn_rocket_min_speed..=cfg.spawn_rocket_max_speed)
    }

    /// Réinitialise une fusée inactive pour la réutiliser sans réallocation
    pub fn reset(&mut self, cfg: &PhysicConfig, window_width: f32) {
        let cx = self
            .rng
            .random_range(cfg.spawn_rocket_margin..=window_width - cfg.spawn_rocket_margin);
        let pos = Vec2::new(cx, 0.0);

        // Assignations in-place
        self.pos = pos;
        self.last_trail_pos = pos;
        self.vel = self.random_vel(cfg);
        self.color = self.random_color();
        self.trail_index = 0;
        self.active = true;
        self.exploded = false;
        self.explosion_particle_indices = None;
        self.trail_particle_indices = None;
    }
}

impl Rocket {
    #[inline(always)]
    fn update_head_particle(&mut self) {
        // angle = direction de la fusée
        let angle = if self.vel.length_squared() > 0.0 {
            self.vel.angle_to(Vec2::new(0.0, 1.0))
        } else {
            0.0
        };

        self.head = Particle {
            pos: self.pos,
            vel: self.vel,
            color: self.color,
            life: 1.0,
            max_life: 1.0,
            size: 2.0,
            active: true,
            // FIXME: angle n'est vraiment utilisé que pour les têtes de fusée (pas pour les trails ou explosions)
            angle,
            particle_type: ParticleType::Rocket,
        };
    }
}

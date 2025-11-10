use crate::physic_engine::{config::PhysicConfig, particle::Particle};
use glam::{Vec2, Vec4 as Color};
use log::debug;
use rand::Rng;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

// TODO: revoir la stratégie autour du compteur atomique, si c'est vraiment utile et si c'est au bon endroit !
pub static ROCKET_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub const NB_PARTICLES_PER_EXPLOSION: usize = 256;
pub const NB_PARTICLES_PER_TRAIL: usize = 64;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Rocket {
    // Public
    pub id: u64,
    pub pos: Vec2,      // position actuelle
    pub prev_pos: Vec2, // position précédente

    // TODO
    pub vel: Vec2,
    pub color: Color,
    pub exploded: bool,
    pub active: bool,

    pub explosion_particles: [Particle; NB_PARTICLES_PER_EXPLOSION],
    pub trail_particles: [Particle; NB_PARTICLES_PER_TRAIL],
    pub trail_index: usize,

    pub trail_accum: f32,
    pub last_trail_pos: Vec2,
}

impl Default for Rocket {
    fn default() -> Self {
        Self::new(&PhysicConfig::default())
    }
}

impl Rocket {
    pub fn new(config: &PhysicConfig) -> Self {
        assert!(config.particles_per_explosion <= NB_PARTICLES_PER_EXPLOSION);
        assert!(config.particles_per_trail <= NB_PARTICLES_PER_TRAIL);

        Self {
            id: ROCKET_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            pos: Vec2::default(),
            prev_pos: Vec2::default(),
            vel: Vec2::default(),
            color: Color::ONE,
            exploded: false,
            active: false,
            explosion_particles: [Particle::default(); NB_PARTICLES_PER_EXPLOSION],
            trail_particles: [Particle::default(); NB_PARTICLES_PER_TRAIL],
            trail_index: 0,
            trail_accum: 0.0,
            last_trail_pos: Vec2::default(),
        }
    }

    pub fn active_particles(&self) -> impl Iterator<Item = &Particle> {
        // self.trail_particles et self.explosion_particles sont alloués sur la heap (tailles statiques),
        // donc le iter().filter(...) n'est pas si couteux, mais à vérifier !
        self.trail_particles
            .iter()
            .filter(|p| p.active)
            .chain(self.explosion_particles.iter().filter(|p| p.active))
    }
}

impl Rocket {
    /// Met à jour la fusée (mouvement, trails, explosions)
    pub fn update(&mut self, mut rng: impl Rng, dt: f32) {
        if !self.active {
            return;
        }

        const GRAVITY: Vec2 = Vec2::new(0.0, -200.0);

        // === Movement ===
        self.update_movement(dt, GRAVITY);

        // === Trails ===
        self.update_trails(dt, GRAVITY);

        // === Explosions ===
        self.update_explosions(dt, &mut rng, GRAVITY);

        // === Inactive rocket check ===
        self.remove_inactive_rockets();
    }

    fn remove_inactive_rockets(&mut self) {
        // === Rocket inactive once explosion finished ===
        if self.exploded && self.explosion_particles.iter().all(|p| !p.active) {
            debug!(
                "Rocket {:?} inactive: all explosion particles are inactive",
                self.id
            );
            self.active = false;
        }
    }

    /// Met à jour la position et la vitesse de la fusée
    fn update_movement(&mut self, dt: f32, gravity: Vec2) {
        self.vel += gravity * dt;
        self.pos += self.vel * dt;
    }

    /// Gère la génération et la mise à jour des particules de trail
    fn update_trails(&mut self, dt: f32, gravity: Vec2) {
        // === TRAIL ===
        // La distance entre deux particules consécutives est définie par `trail_spacing`.
        // Si la rocket se déplace rapidement, plusieurs particules peuvent être générées
        // sur un même update pour maintenir un espacement constant.
        //
        // Ce code utilise une interpolation linéaire (lerp) entre la dernière position
        // de trail (`last_trail_pos`) et la position actuelle de la rocket (`pos`) pour
        // calculer les positions exactes des particules.
        const TRAIL_SPACING: f32 = 2.0;
        let movement = self.pos - self.last_trail_pos;
        let dist = movement.length();

        if !self.exploded {
            // Distance restante à couvrir pour générer des particules
            let mut remaining_dist = dist;

            // Tant qu'il reste suffisamment de distance pour une nouvelle particule
            while remaining_dist >= TRAIL_SPACING {
                // Ratio le long du segment [last_trail_pos, pos] pour placer la particule
                let t = TRAIL_SPACING / dist;

                // Interpolation linéaire (lerp) pour trouver la position de la nouvelle particule
                let new_pos = self.last_trail_pos * (1.0 - t) + self.pos * t;

                // Indice circulaire dans le tableau préalloué de particules de trail
                let i = self.trail_index % NB_PARTICLES_PER_TRAIL;

                // Création de la particule de trail
                // TODO: paramétrer aussi ces settings (life, max_life, size)
                self.trail_particles[i] = Particle {
                    pos: new_pos,
                    vel: Vec2::ZERO, // pas de vitesse initiale
                    color: self.color,
                    life: 0.35,
                    max_life: 0.35,
                    size: 2.0,
                    active: true,
                };

                // Incrément de l'indice circulaire
                self.trail_index = (self.trail_index + 1) % NB_PARTICLES_PER_TRAIL;

                // Mise à jour de la dernière position de trail pour la prochaine interpolation
                self.last_trail_pos = new_pos;

                // Consommation de la distance utilisée pour cette particule
                remaining_dist -= TRAIL_SPACING;
            }
        }

        // Update trails
        for p in self.trail_particles.iter_mut().filter(|p| p.active) {
            p.vel.y += gravity.y * dt;
            p.pos.y += p.vel.y * dt;
            p.life -= dt;
            p.active = p.life > 0.0;
        }
    }

    /// Gère la génération et la mise à jour des particules d’explosion
    fn update_explosions(&mut self, dt: f32, rng: &mut impl Rng, gravity: Vec2) {
        if !self.exploded && self.vel.y <= 0.0 {
            self.exploded = true;

            for p in self.explosion_particles.iter_mut() {
                let angle = rng.random_range(0.0..(2.0 * std::f32::consts::PI));
                let speed = rng.random_range(60.0..200.0);
                // Durée de vie de l'explosion
                let life = rng.random_range(0.75..1.5);

                *p = Particle {
                    pos: self.pos,
                    vel: Vec2::from_angle(angle) * speed,
                    color: Color::new(
                        rng.random_range(0.5..1.0),
                        rng.random_range(0.5..1.0),
                        rng.random_range(0.5..1.0),
                        1.0,
                    ),
                    life,
                    max_life: life,
                    size: rng.random_range(3.0..6.0),
                    active: true,
                };
            }
        }

        // Update explosions
        for p in self.explosion_particles.iter_mut().filter(|p| p.active) {
            p.vel.y += gravity.y * dt;
            p.pos += p.vel * dt;
            p.life -= dt;
            p.active = p.life > 0.0;
        }
    }
}

impl Rocket {
    /// Réinitialise une fusée inactive pour la réutiliser sans réallocation.
    ///
    /// # But
    /// Évite d’allouer et de recréer des structures Rocket dans l’arène à chaque lancement.
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

        // On ne recrée pas les buffers → on les vide logiquement
        for p in self.trail_particles.iter_mut() {
            p.active = false;
        }
        for p in self.explosion_particles.iter_mut() {
            p.active = false;
        }
    }
}

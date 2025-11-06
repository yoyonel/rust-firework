use crate::physic_engine::{
    config::PhysicConfig,
    particle::Particle,
    types::{Color, Vec2, NB_PARTICLES_PER_EXPLOSION, NB_PARTICLES_PER_TRAIL},
};
use rand::Rng;
// use std::sync::atomic::AtomicU64;

// static ROCKET_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Rocket {
    // Public
    // pub id: u64,
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
            // id: ROCKET_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            pos: Vec2::default(),
            prev_pos: Vec2::default(),
            vel: Vec2::default(),
            color: Color::WHITE,
            exploded: false,
            active: false,
            explosion_particles: [Particle::default(); NB_PARTICLES_PER_EXPLOSION],
            trail_particles: [Particle::default(); NB_PARTICLES_PER_TRAIL],
            trail_index: 0,
            trail_accum: 0.0,
            last_trail_pos: Vec2::default(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        if !self.active {
            return;
        }

        let gravity = Vec2 {
            x: 0.0,
            y: -200.0,
            ..Default::default()
        };
        let mut rng = rand::rng();

        // Movement
        self.vel.x += gravity.x * dt;
        self.vel.y += gravity.y * dt;
        self.pos.x += self.vel.x * dt;
        self.pos.y += self.vel.y * dt;

        // === TRAIL ===
        // La distance entre deux particules consécutives est définie par `trail_spacing`.
        // Si la rocket se déplace rapidement, plusieurs particules peuvent être générées
        // sur un même update pour maintenir un espacement constant.
        //
        // Ce code utilise une interpolation linéaire (lerp) entre la dernière position
        // de trail (`last_trail_pos`) et la position actuelle de la rocket (`pos`) pour
        // calculer les positions exactes des particules.
        let trail_spacing = 2.0; // distance minimale entre deux particules

        // Calcul du vecteur de déplacement depuis la dernière particule
        let movement = Vec2 {
            x: self.pos.x - self.last_trail_pos.x,
            y: self.pos.y - self.last_trail_pos.y,
            ..Default::default()
        };

        // Calcul de la distance euclidienne parcourue depuis la dernière particule
        let dist = (movement.x * movement.x + movement.y * movement.y).sqrt();

        if !self.exploded {
            // Distance restante à couvrir pour générer des particules
            let mut remaining_dist = dist;

            // Tant qu'il reste suffisamment de distance pour une nouvelle particule
            while remaining_dist >= trail_spacing {
                // Ratio le long du segment [last_trail_pos, pos] pour placer la particule
                let t = trail_spacing / dist;

                // Interpolation linéaire (lerp) pour trouver la position de la nouvelle particule
                let new_pos = Vec2 {
                    x: self.last_trail_pos.x * (1.0 - t) + self.pos.x * t,
                    y: self.last_trail_pos.y * (1.0 - t) + self.pos.y * t,
                    ..Default::default()
                };

                // Indice circulaire dans le tableau préalloué de particules de trail
                let i = self.trail_index % NB_PARTICLES_PER_TRAIL;

                // Création de la particule de trail
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
                remaining_dist -= trail_spacing;
            }
        }

        // === EXPLOSION ===
        if !self.exploded && self.vel.y <= 0.0 {
            self.exploded = true;

            for p in self.explosion_particles.iter_mut() {
                let angle = rng.random_range(0.0..(2.0 * std::f32::consts::PI));
                let speed = rng.random_range(60.0..200.0);
                // Durée de vie de l'explosion
                // let life = rng.random_range(1.5..3.0);
                let life = rng.random_range(0.75..1.5);

                *p = Particle {
                    pos: self.pos,
                    vel: Vec2 {
                        x: angle.cos() * speed,
                        y: angle.sin() * speed,
                        ..Default::default()
                    },
                    color: Color {
                        r: rng.random_range(0.5..1.0),
                        g: rng.random_range(0.5..1.0),
                        b: rng.random_range(0.5..1.0),
                        ..Default::default()
                    },
                    life,
                    max_life: life,
                    size: rng.random_range(3.0..6.0),
                    active: true,
                };
            }
        }

        // === Update trails ===
        for p in self.trail_particles.iter_mut().filter(|p| p.active) {
            p.vel.y += gravity.y * dt;
            p.pos.y += p.vel.y * dt;
            p.life -= dt;
            p.active = p.life > 0.0;
        }

        // === Update explosions ===
        for p in self.explosion_particles.iter_mut().filter(|p| p.active) {
            p.vel.y += gravity.y * dt;
            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;
            p.life -= dt;
            p.active = p.life > 0.0;
        }

        // === Rocket inactive once explosion finished ===
        if self.exploded && self.explosion_particles.iter().all(|p| !p.active) {
            self.active = false;
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

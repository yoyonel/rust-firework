use crate::physic_engine::{config::PhysicConfig, particle::Particle};
use glam::{Vec2, Vec4 as Color};
use log::debug;
use rand::Rng;
use std::sync::atomic::{AtomicU64, Ordering};

// TODO: revoir la stratégie autour du compteur atomique, si c'est vraiment utile et si c'est au bon endroit !
/// Compteur global pour générer des ID uniques pour les rockets
pub static ROCKET_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub const NB_PARTICLES_PER_EXPLOSION: usize = 256;
pub const NB_PARTICLES_PER_TRAIL: usize = 64;

/// Gestion globale des particules d’explosion.
/// Chaque rocket ne possède plus directement ses `explosion_particles`.
/// Elle garde seulement une référence (via indices) dans le manager.
#[derive(Debug, Clone)]
pub struct ParticlesManager {
    /// Tableau global de particules
    particles: Vec<Particle>,

    /// Nombre de particules par explosion
    per_explosion: usize,
}

impl ParticlesManager {
    /// Crée un nouveau manager avec `max_explosions` et `per_explosion` particules par explosion
    pub fn new(max_explosions: usize, per_explosion: usize) -> Self {
        Self {
            particles: vec![Particle::default(); max_explosions * per_explosion],
            per_explosion,
        }
    }

    /// Alloue un bloc de particules pour une explosion et retourne le range associé
    pub fn allocate_explosion(&mut self) -> Option<std::ops::Range<usize>> {
        let total = self.particles.len();
        let per_block = self.per_explosion;

        for start in (0..total).step_by(per_block) {
            let end = start + per_block;
            if self.particles[start..end].iter().all(|p| !p.active) {
                return Some(start..end);
            }
        }

        None
    }

    /// Renvoie un slice immuable pour un bloc donné
    pub fn get_particles(&self, range: &std::ops::Range<usize>) -> &[Particle] {
        &self.particles[range.clone()]
    }

    /// Renvoie un slice mutable pour un bloc donné
    pub fn get_particles_mut(&mut self, range: &std::ops::Range<usize>) -> &mut [Particle] {
        &mut self.particles[range.clone()]
    }
}

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
    pub explosion_particle_indices: Option<std::ops::Range<usize>>,

    /// Particules de trail (toujours gérées localement)
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
            trail_particles: [Particle::default(); NB_PARTICLES_PER_TRAIL],
            trail_index: 0,
            trail_accum: 0.0,
            last_trail_pos: Vec2::default(),
        }
    }

    /// Retourne un itérateur sur toutes les particules actives de la fusée
    pub fn active_particles<'a>(
        &'a self,
        particles_manager: &'a ParticlesManager,
    ) -> impl Iterator<Item = &'a Particle> {
        let trails = self.trail_particles.iter().filter(|p| p.active);
        let explosions = self
            .explosion_particle_indices
            .iter() // itère sur Option<&Range> directement
            .flat_map(|range| particles_manager.get_particles(range)) // idem flatten
            .filter(|p| p.active);
        trails.chain(explosions)
    }

    /// Met à jour la fusée (mouvement, trails, explosions)
    pub fn update(
        &mut self,
        rng: &mut impl Rng,
        dt: f32,
        particles_manager: &mut ParticlesManager,
    ) {
        if !self.active {
            return;
        }

        const GRAVITY: Vec2 = Vec2::new(0.0, -200.0);

        self.update_movement(dt, GRAVITY);
        self.update_trails(dt, GRAVITY);
        self.update_explosions(dt, rng, GRAVITY, particles_manager);
        self.remove_inactive_rockets(particles_manager);
    }

    fn remove_inactive_rockets(&mut self, particles_manager: &ParticlesManager) {
        // La rocket devient inactive une fois toutes les particules d’explosion terminées
        let exploded_done = match self.explosion_particle_indices {
            Some(ref range) => particles_manager
                .get_particles(range)
                .iter()
                .all(|p| !p.active),
            None => true,
        };

        if self.exploded && exploded_done {
            debug!(
                "Rocket {:?} inactive: all explosion particles are inactive",
                self.id
            );
            self.active = false;
        }
    }

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

    fn update_explosions(
        &mut self,
        dt: f32,
        rng: &mut impl Rng,
        gravity: Vec2,
        particles_manager: &mut ParticlesManager,
    ) {
        if !self.exploded && self.vel.y <= 0.0 {
            self.trigger_explosion(rng, particles_manager);
        }

        if let Some(range) = &self.explosion_particle_indices {
            let slice = particles_manager.get_particles_mut(range);
            for p in slice.iter_mut().filter(|p| p.active) {
                p.vel.y += gravity.y * dt;
                p.pos += p.vel * dt;
                p.life -= dt;
                p.active = p.life > 0.0;
            }
        }
    }

    #[inline(always)]
    fn trigger_explosion(&mut self, rng: &mut impl Rng, particles_manager: &mut ParticlesManager) {
        self.exploded = true;

        // Alloue les particules dans le manager si nécessaire
        if self.explosion_particle_indices.is_none() {
            self.explosion_particle_indices = particles_manager.allocate_explosion();
        }

        // Update explosions particles
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

        for p in self.trail_particles.iter_mut() {
            p.active = false;
        }
    }
}

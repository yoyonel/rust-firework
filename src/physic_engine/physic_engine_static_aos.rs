// TODO: s'intéresser au générateur aléatoire rand::Rng,
// en particulier à son partage/stockage entre Rocket et FireworkEngine
use itertools::Itertools;
use log::debug;
use log::info;
use rand::Rng;
use std::cmp::max;

use crate::physic_engine::{
    config::PhysicConfig,
    types::{Color, Particle, Rocket, UpdateResult, Vec2},
    PhysicEngine,
};

#[derive(Debug, Default)]
pub struct PhysicEngineFireworks {
    // Public
    pub rockets: Vec<Rocket>,
    pub triggered_explosions: Vec<Particle>,
    pub active_indices: Vec<usize>,
    pub free_indices: Vec<usize>,

    // Protected
    time_since_last_rocket: f32,
    next_rocket_interval: f32,
    triggered_count: usize,
    window_width: f32,
    rng: rand::rngs::ThreadRng,

    config: PhysicConfig,

    rocket_margin_min_x: f32,
    rocket_margin_max_x: f32,
}

impl PhysicEngineFireworks {
    pub fn new(config: &PhysicConfig, window_width: f32) -> Self {
        let mut e = Self {
            rockets: (0..config.max_rockets)
                .map(|_| Rocket::new(config))
                .collect(),
            triggered_explosions: vec![Particle::default(); config.max_rockets],
            active_indices: Vec::with_capacity(config.max_rockets),
            free_indices: (0..config.max_rockets).rev().collect(),
            window_width,
            ..Default::default()
        };
        e.next_rocket_interval = e.compute_next_interval();
        e.update_spawn_rocket_margin();
        e
    }

    pub fn reload_config(&mut self, new_config: &PhysicConfig) -> bool {
        let old_max_rockets = self.config.max_rockets;
        self.config = new_config.clone();

        let max_rockets_updated = new_config.max_rockets != old_max_rockets;
        if max_rockets_updated {
            info!(
                "Reinitializing physics buffers due to max_rockets change: {} -> {}",
                old_max_rockets, new_config.max_rockets
            );
            self.reinit_buffers();
        }

        // si tu veux, tu peux recalculer d'autres paramètres dépendants
        self.next_rocket_interval = self.compute_next_interval();

        self.update_spawn_rocket_margin();

        max_rockets_updated
    }

    fn update_spawn_rocket_margin(&mut self) {
        /*
        🔹 Explication rapide
            minmax(a, b) renvoie un enum MinMaxResult :
            MinMax(min, max) si a != b
            OneValue(v) si a == b
            NoValues si aucune valeur (rare ici)
            into_option() transforme ça en Some((min, max)) ou None.
            unwrap_or((0.0, 0.0)) fournit un fallback sûr si aucune valeur.
        */
        let margin = self.config.spawn_rocket_margin;
        (self.rocket_margin_min_x, self.rocket_margin_max_x) = [margin, self.window_width - margin]
            .iter() // transforme en slice iterator
            .copied() // optionnel : pour obtenir f32 directement au lieu de &f32
            .minmax() // méthode fournie par Itertools
            .into_option() // Option<(min, max)>
            .unwrap_or((0.0, 0.0));
    }

    fn reinit_buffers(&mut self) {
        self.rockets = (0..self.config.max_rockets)
            .map(|_| Rocket::new(&self.config))
            .collect();
        self.active_indices.clear();
        self.triggered_explosions = vec![Particle::default(); self.config.max_rockets];
    }

    pub fn set_window_width(&mut self, width: f32) {
        self.window_width = width;
        self.update_spawn_rocket_margin();
    }

    fn compute_next_interval(&mut self) -> f32 {
        self.rng
            .random_range(
                (self.config.rocket_interval_mean - self.config.rocket_interval_variation)
                    ..=(self.config.rocket_interval_mean + self.config.rocket_interval_variation),
            )
            .max(self.config.rocket_max_next_interval)
    }


    pub fn spawn_rocket(&mut self) -> Option<&mut Rocket> {
        let i = self.free_indices.pop()?; // récupère un slot libre
        let r = &mut self.rockets[i];

        let cfg = &self.config;
        let angle = self.rng.random_range(
            (cfg.spawn_rocket_vertical_angle - cfg.spawn_rocket_angle_variation)
                ..=(cfg.spawn_rocket_vertical_angle + cfg.spawn_rocket_angle_variation),
        );
        let speed = self
            .rng
            .random_range(cfg.spawn_rocket_min_speed..=cfg.spawn_rocket_max_speed);
        let cx = self
            .rng
            .random_range(self.rocket_margin_min_x..=self.rocket_margin_max_x);

        *r = Rocket {
            pos: Vec2::new(cx, 0.0),
            vel: Vec2::new(angle.cos() * speed, angle.sin() * speed),
            color: Color::new(
                self.rng.random_range(0.5..=1.0),
                self.rng.random_range(0.5..=1.0),
                self.rng.random_range(0.5..=1.0),
            ),
            active: true,
            exploded: false,
            trail_index: 0,
            last_trail_pos: Vec2::new(cx, 0.0),
            ..Default::default()
        };

        self.active_indices.push(i);
        Some(r)
    }

    fn deactivate_rocket(&mut self, index: usize) {
        self.rockets[index].active = false;
        if let Some(pos) = self.active_indices.iter().position(|&i| i == index) {
            self.active_indices.swap_remove(pos); // O(1)
        }
        self.free_indices.push(index);
    }

    pub fn update(&mut self, dt: f32) -> UpdateResult<'_> {
        self.triggered_count = 0;
        let mut new_rocket: Option<Rocket> = None;

        self.time_since_last_rocket += dt;
        if self.time_since_last_rocket >= self.next_rocket_interval {
            if let Some(r) = self.spawn_rocket() {
                debug!("🚀 Rocket spawned at ({}, {})", r.pos.x, r.pos.y);
                new_rocket = Some(*r);
                self.time_since_last_rocket = 0.0;
                self.next_rocket_interval = self.compute_next_interval();
            }
        }

        // === Mise à jour des fusées actives ===
        let mut to_deactivate = Vec::new();

        for &i in &self.active_indices {
            let rocket = &mut self.rockets[i];
            let exploded_before = rocket.exploded;
            rocket.update(dt);

            // Détection de l’explosion (inchangée)
            if !exploded_before && rocket.exploded {
                let e = &mut self.triggered_explosions[self.triggered_count];
                *e = Particle {
                    pos: rocket.pos,
                    vel: Vec2::ZERO,
                    color: rocket.color,
                    life: 0.0,
                    max_life: 0.0,
                    size: 6.0,
                    active: true,
                };
                self.triggered_count += 1;
            }

            // Marque les fusées terminées pour suppression
            if !rocket.active {
                to_deactivate.push(i);
            }
        }

        // === Nettoyage des fusées devenues inactives ===
        for i in to_deactivate {
            self.deactivate_rocket(i);
        }

        UpdateResult {
            new_rocket,
            explosions: &self.triggered_explosions[..self.triggered_count],
        }
    }

    pub fn max_particles(&self) -> usize {
        self.config.max_rockets
            * max(
                self.config.particles_per_explosion,
                self.config.particles_per_trail,
            )
    }
}

impl PhysicEngine for PhysicEngineFireworks {
    /// Itérateur dynamique combinant toutes les particules actives
    /// (traînées + explosions) de toutes les fusées.
    ///
    /// Ici, on crée un `Box` contenant le résultat de :
    ///   `self.rockets.iter().flat_map(|r| r.active_particles())`
    ///
    /// Cela nous évite d’avoir à décrire le type d’itérateur complet,
    /// tout en gardant un code propre et concis.
    fn active_particles<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(
            self.active_indices
                .iter()
                .flat_map(|&i| self.rockets[i].active_particles()),
        )
    }

    fn active_rockets<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Rocket> + 'a> {
        Box::new(self.active_indices.iter().map(|&i| &self.rockets[i]))
    }

    /// Met à jour la largeur de la scène (ex : largeur de la fenêtre d’affichage).
    fn set_window_width(&mut self, width: f32) {
        self.set_window_width(width);
    }

    /// Met à jour la simulation et renvoie les résultats (ex : nouvelles fusées lancées).
    ///
    /// On délègue ici directement à la méthode interne `update` de `PhysicEngineFireworks`.
    fn update(&mut self, dt: f32) -> UpdateResult<'_> {
        self.update(dt)
    }

    fn close(&mut self) {
        // Exemple : réinitialiser les fusées pour "libérer" le moteur
        for r in self.rockets.iter_mut() {
            r.active = false;
            r.exploded = false;
        }
        self.triggered_count = 0;
        debug!("PhysicEngineFireworks closed and reset.");
    }

    fn reload_config(&mut self, config: &PhysicConfig) -> bool {
        self.reload_config(config)
    }
}

#[cfg(any(test, feature = "test_helpers"))]
pub trait PhysicEngineTestHelpers {
    fn force_next_launch(&mut self);
    fn rockets_count(&self) -> usize;
}

#[cfg(any(test, feature = "test_helpers"))]
impl PhysicEngineTestHelpers for PhysicEngineFireworks {
    fn force_next_launch(&mut self) {
        self.time_since_last_rocket = self.next_rocket_interval;
    }

    fn rockets_count(&self) -> usize {
        self.rockets.iter().filter(|r| r.active).count()
    }
}

use generational_arena::{Arena, Index};
use itertools::Itertools;
use log::{debug, info};
use rand::Rng;
use std::cmp::max;
use std::sync::atomic::Ordering;

use crate::physic_engine::{
    config::PhysicConfig,
    particle::Particle,
    rocket::{ParticlesManager, Rocket, ROCKET_ID_COUNTER},
    types::UpdateResult,
    PhysicEngine,
};

#[derive(Debug)]
pub struct PhysicEngineFireworks {
    rockets: Arena<Rocket>,     // Slots pour toutes les fusées
    active_indices: Vec<Index>, // Itération rapide sur les fusées actives
    free_indices: Vec<Index>,   // Slots disponibles à réutiliser
    triggered_explosions: Vec<Particle>,

    time_since_last_rocket: f32,
    next_rocket_interval: f32,
    window_width: f32,
    rng: rand::rngs::ThreadRng,

    config: PhysicConfig,
    rocket_margin_min_x: f32,
    rocket_margin_max_x: f32,

    particles_manager: ParticlesManager,
}

impl PhysicEngineFireworks {
    pub fn new(config: &PhysicConfig, window_width: f32) -> Self {
        let mut rockets = Arena::with_capacity(config.max_rockets);
        let mut free_indices = Vec::with_capacity(config.max_rockets);

        // Pré-remplissage des slots dans l’arena et free_indices
        for _ in 0..config.max_rockets {
            let idx = rockets.insert(Rocket::new(config));
            free_indices.push(idx);
        }

        // reset counter for rocket
        ROCKET_ID_COUNTER.store(0, Ordering::Relaxed);

        // il y a autant d'explositions
        let triggered_explosions = vec![Particle::default(); config.max_rockets];

        let mut engine = Self {
            rockets,
            active_indices: Vec::with_capacity(config.max_rockets),
            free_indices,
            triggered_explosions,
            time_since_last_rocket: 0.0,
            next_rocket_interval: 0.0,
            window_width,
            rng: rand::rng(),
            config: config.clone(),
            rocket_margin_min_x: 0.0,
            rocket_margin_max_x: 0.0,
            particles_manager: ParticlesManager::new(
                config.max_rockets,
                config.particles_per_explosion,
            ),
        };

        engine.next_rocket_interval = engine.compute_next_interval();
        engine.update_spawn_rocket_margin();
        engine
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
            self.triggered_explosions = vec![Particle::default(); new_config.max_rockets];

            // Réinitialisation des slots free_indices et active_indices
            self.active_indices.clear();
            self.free_indices.clear();

            for _ in 0..new_config.max_rockets {
                let idx = self.rockets.insert(Rocket::new(&self.config));
                self.free_indices.push(idx);
            }
        }

        self.next_rocket_interval = self.compute_next_interval();
        self.update_spawn_rocket_margin();
        max_rockets_updated
    }

    fn update_spawn_rocket_margin(&mut self) {
        let margin = self.config.spawn_rocket_margin;
        (self.rocket_margin_min_x, self.rocket_margin_max_x) = [margin, self.window_width - margin]
            .iter() // transforme en slice iterator
            .copied() // optionnel : pour obtenir f32 directement au lieu de &f32
            .minmax() // méthode fournie par Itertools
            .into_option() // Option<(min, max)>
            .unwrap_or((0.0, 0.0));
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
        let idx = self.free_indices.pop()?;
        let cfg = &self.config;

        // --- [CHANGE 3] ---
        if let Some(r) = self.rockets.get_mut(idx) {
            // Réutilisation sans recréer la structure complète
            r.reset(cfg, &mut self.rng, self.window_width);
        }

        self.active_indices.push(idx);
        self.rockets.get_mut(idx)
    }

    fn deactivate_rocket(&mut self, idx: Index) {
        if let Some(r) = self.rockets.get_mut(idx) {
            r.active = false;
        }

        // Retire de active_indices en O(1) grâce à swap_remove
        if let Some(pos) = self.active_indices.iter().position(|&i| i == idx) {
            self.active_indices.swap_remove(pos);
        }

        // Ajoute le slot dans free_indices pour réutilisation
        self.free_indices.push(idx);
    }

    pub fn update(&mut self, dt: f32) -> UpdateResult<'_> {
        let mut triggered_count = 0;
        let mut new_rocket: Option<Rocket> = None;

        self.time_since_last_rocket += dt;
        if self.time_since_last_rocket >= self.next_rocket_interval {
            if let Some(r) = self.spawn_rocket() {
                debug!("🚀 Rocket spawned at ({}, {})", r.pos.x, r.pos.y);
                new_rocket = Some(r.clone());
                self.time_since_last_rocket = 0.0;
                self.next_rocket_interval = self.compute_next_interval();
            }
        }

        let mut to_deactivate = Vec::new();
        // on parcourt la liste des id de rockets actives
        for &idx in &self.active_indices {
            // si la rocket existe
            if let Some(rocket) = self.rockets.get_mut(idx) {
                // on sauvegarde l'état de la rocket avant update
                let exploded_before = rocket.exploded;

                // rocket.update(&mut self.rng, dt);
                rocket.update(&mut self.rng, dt, &mut self.particles_manager);

                // si avant l'update la rocket n'était pas explosée et qu'après elle l'est
                // on incrémente le compteur d'explosion
                triggered_count += (!exploded_before && rocket.exploded) as usize;
                // si la rocket n'est plus active, on place son ix dans la liste des rockets à déactiver.
                // on le fait en déférer car on itère (actuellement) sur la liste (des id) des rockets actives.
                if !rocket.active {
                    to_deactivate.push(idx);
                }
            }
        }
        // on désactive les rockets
        for idx in to_deactivate {
            self.deactivate_rocket(idx);
        }

        UpdateResult {
            new_rocket,
            // on renvoie le slice d'explosions déclenchées
            triggered_explosions: &self.triggered_explosions[..triggered_count],
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

// ==================================
// Trait PhysicEngine
// ==================================
impl PhysicEngine for PhysicEngineFireworks {
    fn active_particles<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(
            self.active_indices
                .iter()
                .filter_map(|&idx| self.rockets.get(idx))
                .flat_map(|r| r.active_particles(&self.particles_manager)),
        )
    }

    fn active_rockets<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Rocket> + 'a> {
        Box::new(
            self.active_indices
                .iter()
                .filter_map(|&idx| self.rockets.get(idx)),
        )
    }

    fn set_window_width(&mut self, width: f32) {
        self.window_width = width;
        self.update_spawn_rocket_margin();
    }

    fn update(&mut self, dt: f32) -> UpdateResult<'_> {
        self.update(dt)
    }

    fn close(&mut self) {
        self.active_indices.clear();
        self.free_indices.clear();
        self.rockets.clear();
        debug!("PhysicEngineFireworks closed and reset.");
    }

    fn reload_config(&mut self, config: &PhysicConfig) -> bool {
        self.reload_config(config)
    }
}

// ==================================
// Helpers pour tests
// ==================================
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
        self.active_indices.len()
    }
}

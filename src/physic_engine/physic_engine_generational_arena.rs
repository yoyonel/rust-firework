use generational_arena::{Arena, Index};
use itertools::Itertools;
use log::{debug, info};
use rand::{Rng, SeedableRng};
use std::sync::atomic::Ordering;

use crate::physic_engine::{
    config::PhysicConfig,
    explosion_shape::ExplosionShape,
    particle::Particle,
    particles_pools::ParticlesPoolsForRockets,
    rocket::{Rocket, ROCKET_ID_COUNTER},
    types::UpdateResult,
    ParticleType, PhysicEngine, PhysicEngineFull, PhysicEngineIterator,
};

use crate::audio_engine::DopplerEvent;
use crossbeam_channel::Sender;
use std::time::Instant;

#[derive(Debug)]
pub struct PhysicEngineFireworks {
    rockets: Arena<Rocket>,     // Slots pour toutes les fusées
    active_indices: Vec<Index>, // Itération rapide sur les fusées actives
    free_indices: Vec<Index>,   // Slots disponibles à réutiliser
    triggered_explosions: Vec<Particle>,
    triggered_explosion_ids: Vec<u64>, // NOUVEAU
    to_deactivate_scratch: Vec<Index>, // Buffer temporaire réutilisable pour éviter les allocations

    audio_launch_triggered: bool,                   // NOUVEAU
    anticipated_launch: Option<(u64, glam::Vec2)>,  // NOUVEAU
    anticipated_explosions: Vec<(u64, glam::Vec2)>, // NOUVEAU

    time_since_last_rocket: f32,
    next_rocket_interval: f32,
    window_width: f32,
    rng: rand::rngs::StdRng,

    config: PhysicConfig,
    pending_config: PhysicConfig,
    rocket_margin_min_x: f32,
    rocket_margin_max_x: f32,

    particles_pools_for_rockets: ParticlesPoolsForRockets,

    /// System handling smoke trail particle lifecycle and rendering buffer
    smoke_system: crate::physic_engine::smoke_system::SmokeSystem,
    smoke_spawn_accumulators: Vec<f32>,

    /// Forme des explosions (sphérique ou basée sur image)
    explosion_shape: ExplosionShape,

    doppler_sender: Option<Sender<DopplerEvent>>,
    last_doppler_time: Instant,
}

impl PhysicEngineFireworks {
    pub fn new(config: &PhysicConfig, window_width: f32, seed: Option<u64>) -> Self {
        let mut rockets = Arena::with_capacity(config.max_rockets);
        let mut free_indices = Vec::with_capacity(config.max_rockets);

        let mut rng = if let Some(s) = seed {
            rand::rngs::StdRng::seed_from_u64(s)
        } else {
            rand::rngs::StdRng::from_os_rng()
        };
        // Pré-remplissage des slots dans l’arena et free_indices
        for _ in 0..config.max_rockets {
            let idx = rockets.insert(Rocket::new(&mut rng));
            free_indices.push(idx);
        }

        // reset counter for rocket
        ROCKET_ID_COUNTER.store(0, Ordering::Relaxed);

        // il y a autant d'explositions
        let triggered_explosions = vec![Particle::default(); config.max_rockets];
        let triggered_explosion_ids = vec![0; config.max_rockets];

        let mut engine = Self {
            rockets,
            active_indices: Vec::with_capacity(config.max_rockets),
            free_indices,
            triggered_explosions,
            triggered_explosion_ids,
            to_deactivate_scratch: Vec::with_capacity(config.max_rockets),
            audio_launch_triggered: false,
            anticipated_launch: None,
            anticipated_explosions: vec![(0, glam::Vec2::ZERO); config.max_rockets],
            time_since_last_rocket: 0.0,
            next_rocket_interval: 0.0,
            window_width,
            rng,
            config: config.clone(),
            pending_config: config.clone(),
            rocket_margin_min_x: 0.0,
            rocket_margin_max_x: 0.0,
            particles_pools_for_rockets: ParticlesPoolsForRockets::new(
                config.max_rockets,
                config.particles_per_explosion,
                config.particles_per_trail,
            ),
            smoke_system: crate::physic_engine::smoke_system::SmokeSystem::new(
                config.max_smoke_particles,
            ),
            smoke_spawn_accumulators: vec![0.0; config.max_rockets],
            explosion_shape: ExplosionShape::default(),
            doppler_sender: None,
            last_doppler_time: Instant::now(),
        };

        engine.next_rocket_interval = engine.compute_next_interval();
        engine.update_spawn_rocket_margin();
        engine
    }

    fn reload_config(&mut self, new_config: &PhysicConfig) -> bool {
        let old_max_rockets = self.config.max_rockets;
        let old_per_explosion = self.config.particles_per_explosion;
        let old_per_trail = self.config.particles_per_trail;

        self.config = new_config.clone();
        self.pending_config = new_config.clone();

        let max_rockets_updated = new_config.max_rockets != old_max_rockets;
        let pool_params_updated = new_config.particles_per_explosion != old_per_explosion
            || new_config.particles_per_trail != old_per_trail;
        let smoke_updated = new_config.max_smoke_particles != self.smoke_system.particles.len();

        if max_rockets_updated || pool_params_updated || smoke_updated {
            info!(
                "Reinitializing physics buffers due to config change: max_rockets ({} -> {}), per_explosion ({} -> {}), per_trail ({} -> {}), max_smoke ({})",
                old_max_rockets, new_config.max_rockets,
                old_per_explosion, new_config.particles_per_explosion,
                old_per_trail, new_config.particles_per_trail,
                new_config.max_smoke_particles
            );
            self.triggered_explosions = vec![Particle::default(); new_config.max_rockets];
            self.triggered_explosion_ids = vec![0; new_config.max_rockets];
            self.anticipated_explosions = vec![(0, glam::Vec2::ZERO); new_config.max_rockets];

            // Réinitialisation des slots free_indices, active_indices et scratch buffer
            self.active_indices.clear();
            self.free_indices.clear();
            self.to_deactivate_scratch.clear();

            self.rockets.clear();
            for _ in 0..new_config.max_rockets {
                let idx = self.rockets.insert(Rocket::new(&mut self.rng));
                self.free_indices.push(idx);
            }

            self.particles_pools_for_rockets = ParticlesPoolsForRockets::new(
                new_config.max_rockets,
                new_config.particles_per_explosion,
                new_config.particles_per_trail,
            );

            self.smoke_system.resize(new_config.max_smoke_particles);
            self.smoke_system.clear();
            self.smoke_spawn_accumulators = vec![0.0; new_config.max_rockets];
        }

        self.next_rocket_interval = self.compute_next_interval();
        self.update_spawn_rocket_margin();
        max_rockets_updated || pool_params_updated
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

    /// Désactive une fusée et libère ses ressources associées (particules, indices, etc.)
    fn deactivate_rocket(&mut self, idx: Index) {
        if let Some(r) = self.rockets.get_mut(idx) {
            r.active = false;
            self.particles_pools_for_rockets.free_blocks(r);
        }

        let slot_idx = idx.into_raw_parts().0;
        if slot_idx < self.smoke_spawn_accumulators.len() {
            self.smoke_spawn_accumulators[slot_idx] = 0.0;
        }

        // Retire de active_indices en O(1) grâce à swap_remove
        if let Some(pos) = self.active_indices.iter().position(|&i| i == idx) {
            self.active_indices.swap_remove(pos);
        }

        // Ajoute le slot dans free_indices pour réutilisation
        self.free_indices.push(idx);
    }

    fn update(&mut self, dt: f32) -> UpdateResult<'_> {
        let mut triggered_count = 0;
        let mut anticipated_count = 0;
        let mut new_rocket: Option<Rocket> = None;

        // Sync dynamic non-structural smoke settings from pending_config (edited by ImGui) to active config
        self.config.smoke_spawn_rate = self.pending_config.smoke_spawn_rate;
        self.config.smoke_initial_size = self.pending_config.smoke_initial_size;
        self.config.smoke_growth_rate_multiplier = self.pending_config.smoke_growth_rate_multiplier;
        self.config.smoke_fade_duration = self.pending_config.smoke_fade_duration;
        self.config.smoke_intensity = self.pending_config.smoke_intensity;
        self.config.smoke_color_mode = self.pending_config.smoke_color_mode;
        self.config.smoke_custom_color = self.pending_config.smoke_custom_color;
        self.config.smoke_inherited_color_intensity =
            self.pending_config.smoke_inherited_color_intensity;
        self.config.max_smoke_particles = self.pending_config.max_smoke_particles;

        self.anticipated_launch = None;
        let gravity = glam::Vec2::new(0.0, self.config.gravity);
        let launch_anticipation_dt = self.config.audio_launch_anticipation_ms / 1000.0;
        let explosion_anticipation_dt = self.config.audio_explosion_anticipation_ms / 1000.0;

        // 1) ANTICIPATION DU DÉPART (LAUNCH)
        self.time_since_last_rocket += dt;
        if self.time_since_last_rocket + launch_anticipation_dt >= self.next_rocket_interval
            && !self.audio_launch_triggered
        {
            if let Some(&idx) = self.free_indices.last() {
                if let Some(r) = self.rockets.get_mut(idx) {
                    // Initialise la fusée à l'avance avec ses caractéristiques aléatoires (position, vitesse, etc.)
                    r.reset(&self.config, self.window_width);
                    r.id = ROCKET_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
                    // Reste non active physiquement et visuellement pour les frames d'anticipation
                    r.active = false;

                    self.anticipated_launch = Some((r.id, r.pos));
                    self.audio_launch_triggered = true;
                }
            }
        }

        // Spawn physique et visuel réel
        if self.time_since_last_rocket >= self.next_rocket_interval {
            if let Some(idx) = self.free_indices.pop() {
                if let Some(r) = self.rockets.get_mut(idx) {
                    r.active = true;
                    new_rocket = Some(r.clone());
                    debug!(
                        "🚀 Rocket spawned at ({}, {}) with ID {}",
                        r.pos.x, r.pos.y, r.id
                    );
                }
                self.active_indices.push(idx);
                self.time_since_last_rocket = 0.0;
                self.next_rocket_interval = self.compute_next_interval();
                self.audio_launch_triggered = false;
            }
        }

        // On extrait temporairement le buffer de travail sans allouer pour contenter le borrow checker.
        let mut to_deactivate = std::mem::take(&mut self.to_deactivate_scratch);
        to_deactivate.clear();

        // Limiteur de fréquence pour les événements Doppler (max 144 Hz)
        let send_doppler =
            self.last_doppler_time.elapsed() >= std::time::Duration::from_secs_f64(1.0 / 144.0);

        // on parcourt la liste des id de rockets actives
        for &idx in &self.active_indices {
            // si la rocket existe
            if let Some(rocket) = self.rockets.get_mut(idx) {
                // on sauvegarde l'état de la rocket avant update
                let exploded_before = rocket.exploded;

                // 2) ANTICIPATION DE L'EXPLOSION
                if !rocket.exploded && !rocket.audio_explosion_triggered {
                    let future_vel_y = rocket.vel.y + gravity.y * explosion_anticipation_dt;
                    if future_vel_y <= self.config.explosion_threshold {
                        // Position future
                        let future_pos = rocket.pos
                            + rocket.vel * explosion_anticipation_dt
                            + 0.5 * gravity * explosion_anticipation_dt * explosion_anticipation_dt;

                        self.anticipated_explosions[anticipated_count] = (rocket.id, future_pos);
                        anticipated_count += 1;
                        rocket.audio_explosion_triggered = true;
                    }
                }

                rocket.update(
                    dt,
                    &mut self.particles_pools_for_rockets,
                    &self.config,
                    &self.explosion_shape,
                );

                // 💨 Emit smoke particles continuously while ascending (stop emission when rocket explodes)
                if rocket.active && !rocket.exploded && self.config.smoke_spawn_rate > 0.0 {
                    let slot_idx = idx.into_raw_parts().0;
                    if slot_idx >= self.smoke_spawn_accumulators.len() {
                        self.smoke_spawn_accumulators.resize(slot_idx + 1, 0.0);
                    }
                    let interval = 1.0 / self.config.smoke_spawn_rate;
                    self.smoke_spawn_accumulators[slot_idx] += dt;
                    while self.smoke_spawn_accumulators[slot_idx] >= interval {
                        self.smoke_spawn_accumulators[slot_idx] -= interval;
                        self.smoke_system.emit(
                            rocket.base_pos(),
                            rocket.vel,
                            rocket.color,
                            &self.config,
                            &mut self.rng,
                        );
                    }
                }

                // On n'envoie le Doppler que si la fusée est active ET n'a pas encore explosé !
                if send_doppler && rocket.active && !rocket.exploded {
                    if let Some(tx) = &self.doppler_sender {
                        let _ = tx.try_send(DopplerEvent {
                            id: rocket.id,
                            pos: rocket.pos,
                            vel: rocket.vel, // Crucial pour le calcul de vitesse radiale !
                            gain: 1.0,
                            timestamp: Instant::now(),
                        });
                    }
                }

                // si avant l'update la rocket n'était pas explosée et qu'après elle l'est
                // on copie la particule de tête et on incrémente le compteur d'explosion
                if !exploded_before && rocket.exploded {
                    self.triggered_explosions[triggered_count] = *rocket.head_particle();
                    self.triggered_explosion_ids[triggered_count] = rocket.id;
                    triggered_count += 1;
                }
                // si la rocket n'est plus active, on place son ix dans la liste des rockets à déactiver.
                // on le fait en déférer car on itère (actuellement) sur la liste (des id) des rockets actives.
                if !rocket.active {
                    to_deactivate.push(idx);
                }
            }
        }

        // update smoke particle simulation (growth, decay, movement)
        self.smoke_system.update(dt, &self.config);

        // on désactive les rockets
        for &idx in &to_deactivate {
            self.deactivate_rocket(idx);
        }

        // On remet le buffer de travail dans la structure pour le réutiliser au prochain tour
        self.to_deactivate_scratch = to_deactivate;

        if send_doppler {
            self.last_doppler_time = Instant::now();
        }

        UpdateResult {
            new_rocket,
            // on renvoie le slice d'explosions déclenchées
            triggered_explosions: &self.triggered_explosions[..triggered_count],
            triggered_explosion_ids: &self.triggered_explosion_ids[..triggered_count],
            anticipated_rocket_launch: self.anticipated_launch,
            anticipated_explosions: &self.anticipated_explosions[..anticipated_count],
        }
    }
}

// ==================================
// Trait PhysicEngine
// ==================================
impl PhysicEngineIterator for PhysicEngineFireworks {
    /// Applique une fonction sur chaque particule active de toutes les fusées actives.
    fn for_each_active_particle(&self, f: &mut dyn FnMut(&Particle)) {
        for &idx in &self.active_indices {
            for p in self.rockets[idx].iter_active_particles(&self.particles_pools_for_rockets) {
                f(p);
            }
        }
    }

    /// Applique une fonction sur chaque tête de fusée active non explosée.
    fn for_each_active_head_not_exploded(&self, f: &mut dyn FnMut(&Particle)) {
        for &idx in &self.active_indices {
            let rocket = &self.rockets[idx];
            if !rocket.exploded {
                f(rocket.head_particle());
            }
        }
    }

    /// Applique une fonction sur chaque particule active d'un type spécifique.
    fn for_each_particle_of_type(&self, particle_type: ParticleType, f: &mut dyn FnMut(&Particle)) {
        if particle_type == ParticleType::Rocket {
            self.for_each_active_head_not_exploded(f);
        } else if particle_type == ParticleType::Smoke {
            self.smoke_system.for_each_active(&mut |sp| {
                f(&sp.to_particle());
            });
        } else {
            self.for_each_active_particle(&mut |p| {
                if p.particle_type == particle_type {
                    f(p);
                }
            });
        }
    }

    fn for_each_smoke_particle(
        &self,
        f: &mut dyn FnMut(&crate::physic_engine::smoke_system::SmokeParticle),
    ) {
        self.smoke_system.for_each_active(f);
    }

    fn get_smoke_intensity(&self) -> f32 {
        self.config.smoke_intensity
    }

    fn get_smoke_erosion_params(&self) -> (bool, f32, f32, [f32; 3]) {
        (
            self.config.smoke_erosion_enabled,
            self.config.smoke_erosion_scale,
            self.config.smoke_erosion_edge_width,
            self.config.smoke_erosion_edge_color,
        )
    }

    fn get_smoke_flow_params(&self) -> (f32, f32) {
        (
            self.config.flow_distortion_strength,
            self.config.flow_animation_speed,
        )
    }
}

impl PhysicEngine for PhysicEngineFireworks {
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

    fn get_config(&self) -> &PhysicConfig {
        &self.config
    }

    fn get_config_mut(&mut self) -> &mut PhysicConfig {
        &mut self.pending_config
    }

    fn get_pending_config(&self) -> &PhysicConfig {
        &self.pending_config
    }

    fn update_anticipation_times(&mut self, launch_ms: f32, explosion_ms: f32) {
        self.config.audio_launch_anticipation_ms = launch_ms;
        self.config.audio_explosion_anticipation_ms = explosion_ms;
        self.pending_config.audio_launch_anticipation_ms = launch_ms;
        self.pending_config.audio_explosion_anticipation_ms = explosion_ms;
    }

    fn set_explosion_shape(&mut self, shape: ExplosionShape) {
        self.explosion_shape = shape;
    }

    fn get_explosion_shape(&self) -> &ExplosionShape {
        &self.explosion_shape
    }

    fn load_explosion_image(
        &mut self,
        path: &str,
        scale: f32,
        flight_time: f32,
    ) -> Result<(), String> {
        let n_samples = self.config.particles_per_explosion;

        match crate::physic_engine::explosion_shape::ImageShape::from_image(
            path,
            n_samples,
            scale,
            flight_time,
        ) {
            Ok(shape) => {
                self.explosion_shape = ExplosionShape::Image(shape);
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    fn load_explosion_image_weighted(
        &mut self,
        path: &str,
        scale: f32,
        flight_time: f32,
        weight: f32,
    ) -> Result<(), String> {
        // Optimization: If weight is <= 0, we treat this as a removal request.
        if weight <= 0.0 {
            let stem = std::path::Path::new(path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            let mut to_spherical = false;
            match &mut self.explosion_shape {
                ExplosionShape::MultiImage {
                    shapes,
                    total_weight,
                } => {
                    if let Some(pos) = shapes.iter().position(|(s, _)| s.file_stem == stem) {
                        let (_, removed_weight) = shapes.remove(pos);
                        *total_weight -= removed_weight;
                        if shapes.is_empty() {
                            to_spherical = true;
                        }
                    }
                }
                ExplosionShape::Image(existing) if existing.file_stem == stem => {
                    to_spherical = true;
                }
                _ => {}
            }
            if to_spherical {
                self.explosion_shape = ExplosionShape::Spherical;
            }
            return Ok(());
        }

        let n_samples = self.config.particles_per_explosion;

        let shape = crate::physic_engine::explosion_shape::ImageShape::from_image(
            path,
            n_samples,
            scale,
            flight_time,
        )
        .map_err(|e| e.to_string())?;

        match &mut self.explosion_shape {
            ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                if let Some((existing_shape, existing_weight)) = shapes
                    .iter_mut()
                    .find(|(s, _)| s.file_stem == shape.file_stem)
                {
                    // Update existing shape parameters and weight
                    *total_weight -= *existing_weight;
                    *existing_shape = shape;
                    *existing_weight = weight;
                    *total_weight += weight;
                } else {
                    // Add new shape
                    shapes.push((shape, weight));
                    *total_weight += weight;
                }
            }
            ExplosionShape::Image(existing_shape) => {
                if existing_shape.file_stem == shape.file_stem {
                    // Just switch mode to MultiImage but it's the same image updated
                    self.explosion_shape = ExplosionShape::MultiImage {
                        shapes: vec![(shape, weight)],
                        total_weight: weight,
                    };
                } else {
                    // Preserve the single image by promoting it to the first element of MultiImage
                    // We assume a default weight of 1.0 for the existing image if none was explicit.
                    let old_shape = existing_shape.clone();
                    self.explosion_shape = ExplosionShape::MultiImage {
                        shapes: vec![(old_shape, 1.0), (shape, weight)],
                        total_weight: 1.0 + weight,
                    };
                }
            }
            _ => {
                // If Spherical (or other), just switch to MultiImage with this single new weighted image
                self.explosion_shape = ExplosionShape::MultiImage {
                    shapes: vec![(shape, weight)],
                    total_weight: weight,
                };
            }
        }
        Ok(())
    }

    fn set_explosion_image_weight(&mut self, name: &str, new_weight: f32) -> Result<(), String> {
        if new_weight <= 0.0 {
            return self.remove_explosion_image(name);
        }
        match &mut self.explosion_shape {
            ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                if let Some((_, weight)) = shapes.iter_mut().find(|(s, _)| s.file_stem == name) {
                    *total_weight -= *weight;
                    *weight = new_weight;
                    *total_weight += *weight;
                    Ok(())
                } else {
                    Err(format!(
                        "Image '{}' not found in current MultiImage set",
                        name
                    ))
                }
            }
            _ => Err("Current explosion shape is not MultiImage".to_string()),
        }
    }

    fn remove_explosion_image(&mut self, name: &str) -> Result<(), String> {
        let mut to_spherical = false;
        match &mut self.explosion_shape {
            ExplosionShape::MultiImage {
                shapes,
                total_weight,
            } => {
                if let Some(pos) = shapes.iter().position(|(s, _)| s.file_stem == name) {
                    let (_, removed_weight) = shapes.remove(pos);
                    *total_weight -= removed_weight;
                    if shapes.is_empty() {
                        to_spherical = true;
                    }
                } else {
                    return Err(format!("Image '{}' not found in MultiImage set", name));
                }
            }
            ExplosionShape::Image(existing) if existing.file_stem == name => {
                to_spherical = true;
            }
            _ => return Err(format!("Image '{}' not active", name)),
        }
        if to_spherical {
            self.explosion_shape = ExplosionShape::Spherical;
        }
        Ok(())
    }

    fn as_physic_engine(&self) -> &dyn PhysicEngine {
        self
    }

    fn set_doppler_sender(
        &mut self,
        sender: crossbeam_channel::Sender<crate::audio_engine::DopplerEvent>,
    ) {
        self.doppler_sender = Some(sender);
    }
}

impl PhysicEngineFull for PhysicEngineFireworks {}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physic_engine::config::PhysicConfig;

    #[test]
    fn test_physic_engine_creation_and_defaults() {
        let config = PhysicConfig::default();
        let engine = PhysicEngineFireworks::new(&config, 800.0, None);

        assert_eq!(engine.rockets_count(), 0);
        assert_eq!(engine.get_config().max_rockets, config.max_rockets);
        assert_eq!(engine.get_explosion_shape(), &ExplosionShape::Spherical);
    }

    #[test]
    fn test_physic_engine_rocket_spawning_and_update() {
        let config = PhysicConfig {
            max_rockets: 5,
            rocket_interval_mean: 0.1,
            ..PhysicConfig::default()
        };
        let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);

        engine.force_next_launch();
        let res = engine.update(0.05);
        let has_new_rocket = res.new_rocket.is_some();
        let _ = res;
        assert!(has_new_rocket || engine.rockets_count() > 0);
    }

    #[test]
    fn test_physic_engine_reload_config() {
        let config = PhysicConfig {
            max_rockets: 10,
            ..PhysicConfig::default()
        };
        let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);

        let mut new_config = config.clone();
        new_config.max_rockets = 20;
        let reinitialized = engine.reload_config(&new_config);

        assert!(reinitialized);
        assert_eq!(engine.get_config().max_rockets, 20);
    }

    #[test]
    fn test_physic_engine_explosion_shape_removal() {
        let config = PhysicConfig::default();
        let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);

        assert!(engine.remove_explosion_image("nonexistent").is_err());

        engine.set_explosion_shape(ExplosionShape::Spherical);
        assert!(engine.remove_explosion_image("heart").is_err());
    }

    #[test]
    fn test_physic_engine_doppler_channel() {
        let config = PhysicConfig::default();
        let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);
        let (tx, rx) = crossbeam_channel::unbounded();

        engine.set_doppler_sender(tx);
        let _ = engine.update(0.1);
        drop(rx);
    }
}

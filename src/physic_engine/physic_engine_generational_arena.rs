use generational_arena::{Arena, Index};
use itertools::Itertools;
use log::{debug, info};
use rand::Rng;
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

    particles_pools_for_rockets: ParticlesPoolsForRockets,

    /// Forme des explosions (sphérique ou basée sur image)
    explosion_shape: ExplosionShape,
}

impl PhysicEngineFireworks {
    pub fn new(config: &PhysicConfig, window_width: f32) -> Self {
        let mut rockets = Arena::with_capacity(config.max_rockets);
        let mut free_indices = Vec::with_capacity(config.max_rockets);

        let mut rng = rand::rng();
        // Pré-remplissage des slots dans l’arena et free_indices
        for _ in 0..config.max_rockets {
            let idx = rockets.insert(Rocket::new(&mut rng));
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
            rng,
            config: config.clone(),
            rocket_margin_min_x: 0.0,
            rocket_margin_max_x: 0.0,
            particles_pools_for_rockets: ParticlesPoolsForRockets::new(
                config.max_rockets,
                config.particles_per_explosion,
                config.particles_per_trail,
            ),
            explosion_shape: ExplosionShape::default(),
        };

        engine.next_rocket_interval = engine.compute_next_interval();
        engine.update_spawn_rocket_margin();
        engine
    }

    fn reload_config(&mut self, new_config: &PhysicConfig) -> bool {
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
                let idx = self.rockets.insert(Rocket::new(&mut self.rng));
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

    fn spawn_rocket(&mut self) -> Option<&mut Rocket> {
        let idx = self.free_indices.pop()?;
        let cfg = &self.config;

        if let Some(r) = self.rockets.get_mut(idx) {
            // Réutilisation sans recréer la structure complète
            r.reset(cfg, self.window_width);
        }

        self.active_indices.push(idx);
        self.rockets.get_mut(idx)
    }

    /// Désactive une fusée et libère ses ressources associées (particules, indices, etc.)
    fn deactivate_rocket(&mut self, idx: Index) {
        if let Some(r) = self.rockets.get_mut(idx) {
            r.active = false;
            self.particles_pools_for_rockets.free_blocks(r);
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

                rocket.update(
                    dt,
                    &mut self.particles_pools_for_rockets,
                    &self.config,
                    &self.explosion_shape,
                );

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
}

// ==================================
// Trait PhysicEngine
// ==================================
impl PhysicEngineIterator for PhysicEngineFireworks {
    /// Itère sur toutes les particules de **toutes** les fusées actives.
    ///
    /// ✔ Aucun `Vec` interne  
    /// ✔ Aucun `Box<dyn Iterator>`  
    /// ✔ Zéro allocation  
    /// ✔ Pipeline d’itérateurs entièrement optimisable par le compilateur  
    ///
    /// Cette approche est idéale pour un rendu GPU basé sur un buffer mappé persistant :
    /// on produit un flux de particules triées, en lecture séquentielle, permettant
    /// une écriture contiguë dans le VBO d'instanciation (meilleur throughput).
    fn iter_active_particles<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(
            self.active_indices
                .iter()
                // Pour chaque rocket active, on concatène son itérateur de heads
                // à l’aide d’un `flat_map`. Le résultat final est un seul pipeline
                // d’itérateurs, entièrement paresseux et zéro-allocation.
                .flat_map(move |&idx| {
                    self.rockets[idx].iter_active_particles(&self.particles_pools_for_rockets)
                }),
        )
    }

    /// Itère sur les particules-têtes (non explosées) référencées statiquement
    /// pour chaque fusée active.
    ///
    /// ✔ Zero allocation
    /// ✔ Transmet une Particle (référencée) par rocket active
    /// ✔ Pas d'accès aux pools
    /// ✔ Parfaitement optimisable
    fn iter_active_heads_not_exploded<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        // TODO: peut être mettre en place une liste d'indices de rockets non-explosées
        Box::new(
            self.active_indices
                .iter()
                // 1) filtrage des fusées non-explosées
                .filter(move |&&idx| !self.rockets[idx].exploded)
                .map(move |&idx| self.rockets[idx].head_particle()),
        )
    }

    /// Retourne un itérateur sur les particules actives d'un type spécifique.
    ///
    /// ✔ Zero allocation
    /// ✔ Filtrage paresseux (lazy)
    /// ✔ Parfaitement optimisable
    ///
    /// Note: Pour les particules de type Rocket, cette méthode combine les particules
    /// de tête (head_particle) avec les particules des pools qui correspondent au type.
    fn iter_particles_by_type<'a>(
        &'a self,
        particle_type: ParticleType,
    ) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        // Pour les particules de type Rocket, on doit inclure les têtes de fusée
        // qui ne sont pas dans les pools mais dans la structure Rocket elle-même
        if particle_type == ParticleType::Rocket {
            Box::new(self.iter_active_heads_not_exploded())
        } else {
            // Pour les autres types, on filtre les particules des pools
            Box::new(
                self.iter_active_particles()
                    .filter(move |p| p.particle_type == particle_type),
            )
        }
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
                ExplosionShape::Image(existing) => {
                    if existing.file_stem == stem {
                        to_spherical = true;
                    }
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

    fn as_physic_engine(&self) -> &dyn PhysicEngine {
        self
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

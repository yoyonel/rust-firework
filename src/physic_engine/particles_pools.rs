#[cfg(debug_assertions)]
use log::debug;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use crate::physic_engine::particle::Particle;
use crate::physic_engine::rocket::Rocket;

#[derive(Debug)]
pub struct ParticlesPoolsForRockets {
    pub particles_pool_for_explosions: ParticlesPool,
    pub particles_pool_for_trails: ParticlesPool,
}

impl ParticlesPoolsForRockets {
    pub fn new(max_rockets: usize, per_explosion: usize, per_trail: usize) -> Self {
        Self {
            particles_pool_for_explosions: ParticlesPool::new(max_rockets, per_explosion),
            particles_pool_for_trails: ParticlesPool::new(max_rockets, per_trail),
        }
    }

    // TODO: Il faut refactorer pour éviter de take (mut) rocket -> 0-copy
    pub fn free_blocks(&mut self, rocket: &mut Rocket) {
        if let Some(range) = rocket.explosion_particle_indices.take() {
            self.particles_pool_for_explosions.free_block(range);
        }
        if let Some(range) = rocket.trail_particle_indices.take() {
            self.particles_pool_for_trails.free_block(range);
        }
    }
}

pub enum PoolKind {
    Trails,
    Explosions,
}

impl ParticlesPoolsForRockets {
    #[inline(always)]
    pub fn access(&self, kind: PoolKind, range: &Range<usize>) -> &[Particle] {
        match kind {
            PoolKind::Trails => self.particles_pool_for_trails.get_particles(range),
            PoolKind::Explosions => self.particles_pool_for_explosions.get_particles(range),
        }
    }
    #[inline(always)]
    pub fn access_mut(&mut self, kind: PoolKind, range: &Range<usize>) -> &mut [Particle] {
        match kind {
            PoolKind::Trails => self.particles_pool_for_trails.get_particles_mut(range),
            PoolKind::Explosions => self.particles_pool_for_explosions.get_particles_mut(range),
        }
    }
}

/// Gère toutes les particules globales du moteur (explosions et trails).
///
/// # Rôle
/// Le `ParticlesPool` maintient un gros vecteur unique de `Particle`,
/// découpé en blocs de taille fixe (une explosion ou un trail = un bloc).
///
/// Chaque fusée (`Rocket`) ne possède plus ses particules,
/// mais détient un simple `Range<usize>` pointant vers une sous-section du tableau.
/// Cette approche évite les copies et réduit la fragmentation mémoire.
#[derive(Debug)]
pub struct ParticlesPool {
    /// Stockage global de toutes les particules
    particles: Vec<Particle>,

    /// Taille d’un bloc (nombre de particules par groupe : explosion ou trail)
    per_block: usize,

    /// Liste des blocs disponibles (pile LIFO)
    free_blocks: Arc<Mutex<VecDeque<usize>>>,
}

impl ParticlesPool {
    /// Crée un nouveau gestionnaire de particules.
    ///
    /// # Arguments
    /// * `max_blocks` – nombre maximum de blocs simultanés (ex: rockets)
    /// * `per_block` – nombre de particules par bloc
    pub fn new(max_blocks: usize, per_block: usize) -> Self {
        let total_particles = max_blocks * per_block;

        // Initialise toutes les particules à leur état par défaut
        let particles = vec![Particle::default(); total_particles];

        // Prépare la pile des blocs libres
        let free_blocks = (0..max_blocks)
            .map(|i| i * per_block)
            .collect::<VecDeque<_>>();

        #[cfg(debug_assertions)]
        debug!(
            "ParticlesPool initialized with {} particles ({} blocks × {} per block)",
            total_particles, max_blocks, per_block
        );

        Self {
            particles,
            per_block,
            free_blocks: Arc::new(Mutex::new(free_blocks)),
        }
    }

    /// Alloue un bloc de particules pour une explosion ou un trail.
    ///
    /// Retourne `Some(range)` si un bloc est disponible, sinon `None`.
    /// Complexité : **O(1)**.
    pub fn allocate_block(&self) -> Option<Range<usize>> {
        let mut free_blocks = self.free_blocks.lock().unwrap();

        if let Some(start) = free_blocks.pop_back() {
            let end = start + self.per_block;
            Some(start..end)
        } else {
            None
        }
    }

    /// Libère un bloc de particules après extinction.
    ///
    /// Le bloc est remis en pile pour réutilisation ultérieure.
    /// Complexité : **O(1)**.
    fn free_block(&self, range: Range<usize>) {
        let mut free_blocks = self.free_blocks.lock().unwrap();
        free_blocks.push_back(range.start);
        #[cfg(debug_assertions)]
        debug!("Freed particle block starting at {}", range.start);
    }

    /// Accès immuable à un bloc de particules.
    #[inline]
    fn get_particles(&self, range: &Range<usize>) -> &[Particle] {
        &self.particles[range.start..range.end]
    }

    /// Accès mutable à un bloc de particules.
    #[inline(always)]
    pub fn get_particles_mut(&mut self, range: &Range<usize>) -> &mut [Particle] {
        &mut self.particles[range.start..range.end]
    }
}

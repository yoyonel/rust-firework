use crate::audio_engine::DopplerEvent;
use crate::physic_engine::config::PhysicConfig;
use crate::physic_engine::explosion_shape::ExplosionShape;
use crate::physic_engine::particle::Particle;
use crate::physic_engine::types::UpdateResult;
use crate::physic_engine::ParticleType;
use crossbeam_channel::Sender;

pub trait PhysicEngineIterator {
    /// Applique une fonction sur chaque particule active.
    fn for_each_active_particle(&self, f: &mut dyn FnMut(&Particle));

    /// Applique une fonction sur chaque tête de fusée active non explosée.
    fn for_each_active_head_not_exploded(&self, f: &mut dyn FnMut(&Particle));

    /// Applique une fonction sur chaque particule active d'un type spécifique.
    fn for_each_particle_of_type(&self, particle_type: ParticleType, f: &mut dyn FnMut(&Particle));
}

/// 🔧 Trait `PhysicEngine`
///
/// Ce trait définit l’interface commune à tous les moteurs physiques.
/// Il permet de manipuler différents moteurs (ex : feux d’artifice, particules, fluides)
/// sans connaître leur implémentation concrète.
///
/// ### Choix de conception : utilisation de `Box<dyn Iterator>`
///
/// - Ici, on retourne un **itérateur dynamiquement dispatché** (`Box<dyn Iterator<...>>`),
///   plutôt qu’un type d’itérateur concret.
/// - Cela permet d’écrire des signatures simples et stables, sans se soucier
///   du type interne très complexe des itérateurs (`Filter`, `Chain`, `FlatMap`, etc.).
///
/// ### ✅ Avantages
/// - ✅ **Lisibilité et simplicité** : pas besoin d’écrire des types d’itérateurs kilométriques.
/// - ✅ **Flexibilité** : n’importe quelle structure peut implémenter ce trait,
///   quelle que soit la complexité de son itérateur interne.
/// - ✅ **Isolation du code** : changer la logique d’itération n’affecte pas la signature du trait.
///
/// ### ⚠️ Inconvénients
/// - ⚠️ **Légère perte de performance** : chaque appel passe par un pointeur de fonction virtuel.
/// - ⚠️ **Petite allocation mémoire** : `Box` alloue sur le tas pour stocker l’itérateur.
///   (Mais ici, c’est négligeable par rapport au coût global d’un moteur de particules.)
///
/// En résumé : cette approche est **le bon compromis** entre performance, clarté et maintenabilité.
pub trait PhysicEngine {
    /// Ajuste la largeur du monde (utile si la fenêtre de rendu change de taille).
    fn set_window_width(&mut self, width: f32);

    /// Met à jour la physique du moteur sur un intervalle de temps `dt`.
    /// Retourne un `UpdateResult` contenant les événements.
    fn update(&mut self, dt: f32) -> UpdateResult<'_>;

    /// Ferme / libère le moteur physique.
    fn close(&mut self) {} // Par défaut, fait rien.

    fn reload_config(&mut self, config: &PhysicConfig) -> bool;

    fn get_config(&self) -> &PhysicConfig;

    fn get_config_mut(&mut self) -> &mut PhysicConfig;

    /// Définit la forme des explosions (sphérique par défaut, ou basée sur image).
    fn set_explosion_shape(&mut self, shape: ExplosionShape);

    /// Retourne la forme d'explosion actuelle.
    fn get_explosion_shape(&self) -> &ExplosionShape;

    /// Charge une image d'explosion avec des paramètres personnalisés.
    ///
    /// # Arguments
    /// * `path` - Chemin vers l'image PNG noir & blanc
    /// * `scale` - Taille de l'image projetée en pixels monde
    /// * `flight_time` - Temps de vol des particules en secondes
    ///
    /// # Returns
    /// `Ok(())` si le chargement réussit, `Err(message)` sinon.
    fn load_explosion_image(
        &mut self,
        path: &str,
        scale: f32,
        flight_time: f32,
    ) -> Result<(), String>;

    /// Charge une image d'explosion et l'ajoute à la liste des formes possibles avec un poids.
    ///
    /// # Arguments
    /// * `path` - Chemin vers l'image PNG noir & blanc
    /// * `scale` - Taille de l'image projetée
    /// * `flight_time` - Temps de vol
    /// * `weight` - Poids relatif (pourcentage de chance d'être choisi)
    fn load_explosion_image_weighted(
        &mut self,
        path: &str,
        scale: f32,
        flight_time: f32,
        weight: f32,
    ) -> Result<(), String>;

    /// Modifie le poids d'une image existante dans la configuration MultiImage.
    ///
    /// # Arguments
    /// * `name` - Nom de l'image (file_stem)
    /// * `weight` - Nouveau poids
    fn set_explosion_image_weight(&mut self, name: &str, weight: f32) -> Result<(), String>;

    /// Helper for upcasting from dyn PhysicEngineFull or other subtraits
    fn as_physic_engine(&self) -> &dyn PhysicEngine;

    // NOUVEAU : Permet de connecter le canal d'émission Doppler.
    // L'implémentation par défaut vide {} évite de casser d'autres moteurs physiques (ex: static_aos).
    fn set_doppler_sender(&mut self, _sender: Sender<DopplerEvent>) {}

    /// Met à jour les temps d'anticipation de l'audio à chaud sans affecter les autres configurations en suspens.
    fn update_anticipation_times(&mut self, _launch_ms: f32, _explosion_ms: f32) {}
}

pub trait PhysicEngineFull: PhysicEngine + PhysicEngineIterator {}

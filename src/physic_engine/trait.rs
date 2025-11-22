use crate::physic_engine::config::PhysicConfig;
use crate::physic_engine::particle::Particle;
use crate::physic_engine::types::UpdateResult;

pub trait PhysicEngineIterator {
    // Les types associés ne sont pas nécessaires ici si 'Particle' est importé.

    /// Retourne un itérateur sur les particules actives.
    fn iter_active_particles<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a>;

    /// Retourne un itérateur sur les têtes de fusées non explosées.
    fn iter_active_heads_not_exploded<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a>;
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
}

pub trait PhysicEngineFull: PhysicEngine + PhysicEngineIterator {}

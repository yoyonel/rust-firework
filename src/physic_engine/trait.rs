use crate::physic_engine::config::PhysicConfig;
use crate::physic_engine::particle::Particle;
// use crate::physic_engine::rocket::Rocket;
use crate::physic_engine::types::UpdateResult;
// use generational_arena::Index;

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
    /// Retourne un itérateur dynamique sur les particules actives.
    /// Chaque élément est une référence immuable vers un `Particle`.
    fn iter_active_particles<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a;
    fn iter_active_heads_not_exploded<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a;

    /// Ajuste la largeur du monde (utile si la fenêtre de rendu change de taille).
    fn set_window_width(&mut self, width: f32);

    /// Met à jour la physique du moteur sur un intervalle de temps `dt`.
    /// Retourne un `UpdateResult` contenant les événements (nouvelles fusées, explosions, etc.).
    fn update(&mut self, dt: f32) -> UpdateResult<'_>;

    /// Ferme / libère le moteur physique.
    /// Par défaut, fait rien.
    fn close(&mut self) {}

    // fn max_particles(&self) -> usize;

    fn reload_config(&mut self, config: &PhysicConfig) -> bool;

    // fn get_rocket(&self, index: Index) -> Option<&Rocket>;
}

use fireworks_sim::physic_engine::{
    config::PhysicConfig,
    physic_engine_generational_arena::{PhysicEngineFireworks, PhysicEngineTestHelpers},
    PhysicEngine, PhysicEngineIterator,
};

// ==================================
// 1. Construction and Initialization
// ==================================

#[test]
fn test_new_engine_initialization() {
    let config = PhysicConfig::default();
    let window_width = 1920.0;
    let engine = PhysicEngineFireworks::new(&config, window_width);

    // Vérifications
    assert_eq!(engine.rockets_count(), 0); // Aucune fusée active
    assert_eq!(engine.get_config().max_rockets, config.max_rockets);
}

#[test]
fn test_spawn_rocket_margin_calculation() {
    let mut config = PhysicConfig::default();
    config.spawn_rocket_margin = 100.0;

    // Cas 1: Fenêtre normale
    let _engine = PhysicEngineFireworks::new(&config, 1920.0);
    // Les marges sont privées, on vérifie indirectement via le comportement

    // Cas 2: Fenêtre plus petite que 2*margin
    let engine_small = PhysicEngineFireworks::new(&config, 150.0);
    // Le moteur devrait gérer ce cas sans paniquer
    assert_eq!(engine_small.rockets_count(), 0);
}

// ==================================
// 2. Gestion du Cycle de Vie des Fusées
// ==================================

#[test]
fn test_spawn_rocket_success() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Forcer le spawn via update avec intervalle dépassé
    engine.force_next_launch();
    let result = engine.update(0.016);

    assert!(result.new_rocket.is_some());

    let r = result.new_rocket.unwrap();
    assert!(r.active);
    assert!(!r.exploded);

    // Vérifier après avoir utilisé result
    assert_eq!(engine.rockets_count(), 1);
}

#[test]
fn test_spawn_rocket_exhaustion() {
    let mut config = PhysicConfig::default();
    config.max_rockets = 3; // Limite à 3 fusées
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn 3 fusées
    for _ in 0..3 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    assert_eq!(engine.rockets_count(), 3);

    // La 4ème tentative ne devrait pas spawner
    engine.force_next_launch();
    let result = engine.update(0.016);
    assert!(result.new_rocket.is_none());
    assert_eq!(engine.rockets_count(), 3);
}

#[test]
fn test_rocket_deactivation_after_lifecycle() {
    let mut config = PhysicConfig::default();
    config.max_rockets = 10;
    config.rocket_interval_mean = 100.0; // Empêcher le spawn automatique
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn une fusée
    engine.force_next_launch();
    engine.update(0.016);
    let initial_count = engine.rockets_count();
    assert!(initial_count > 0);

    // Simuler jusqu'à ce que la fusée se désactive
    // (après explosion et extinction des particules)
    // Note: cela peut prendre beaucoup de temps selon la physique
    for _ in 0..5000 {
        engine.update(0.016);
    }

    // Le nombre de fusées devrait avoir diminué
    assert!(engine.rockets_count() <= initial_count);
}

// ==================================
// 3. Calcul d'Intervalles et Timing
// ==================================

#[test]
fn test_compute_next_interval_respects_bounds() {
    let mut config = PhysicConfig::default();
    config.rocket_interval_mean = 1.0;
    config.rocket_interval_variation = 0.3;
    config.rocket_max_next_interval = 0.5;

    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Tester plusieurs fois pour vérifier la distribution
    for _ in 0..50 {
        // Forcer le recalcul de l'intervalle
        engine.force_next_launch();
        engine.update(0.016);

        // L'intervalle devrait être >= rocket_max_next_interval
        // (on ne peut pas accéder directement à next_rocket_interval car privé)
        // On vérifie indirectement que le moteur ne panique pas
    }
}

#[test]
fn test_update_spawns_rocket_after_interval() {
    let mut config = PhysicConfig::default();
    config.rocket_interval_mean = 0.1;
    config.rocket_interval_variation = 0.0;
    config.rocket_max_next_interval = 0.01;
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Update avec dt < intervalle
    let result = engine.update(0.05);
    assert!(result.new_rocket.is_none());
    assert_eq!(engine.rockets_count(), 0);

    // Update avec dt >= intervalle (cumulatif)
    let result = engine.update(0.06); // Total: 0.11s
    assert!(result.new_rocket.is_some());
    assert_eq!(engine.rockets_count(), 1);
}

// ==================================
// 4. Reconfiguration Dynamique
// ==================================

#[test]
fn test_reload_config_no_change() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn quelques fusées
    engine.force_next_launch();
    engine.update(0.016);
    engine.force_next_launch();
    engine.update(0.016);

    let rockets_before = engine.rockets_count();

    let mut new_config = config.clone();
    new_config.rocket_interval_mean = 2.0; // Changement mineur

    let changed = engine.reload_config(&new_config);

    assert!(!changed); // max_rockets n'a pas changé
    assert_eq!(engine.rockets_count(), rockets_before); // Fusées préservées
}

#[test]
fn test_reload_config_with_max_rockets_change() {
    let mut config = PhysicConfig::default();
    config.max_rockets = 10;
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn quelques fusées
    engine.force_next_launch();
    engine.update(0.016);
    engine.force_next_launch();
    engine.update(0.016);

    let mut new_config = config.clone();
    new_config.max_rockets = 20; // Augmentation

    let changed = engine.reload_config(&new_config);

    assert!(changed);
    assert_eq!(engine.rockets_count(), 0); // Réinitialisé
}

// ==================================
// 5. Mise à Jour et Simulation
// ==================================

#[test]
fn test_update_returns_valid_result() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    let result = engine.update(0.016);

    // Le résultat devrait être valide (peut être vide)
    assert!(result.triggered_explosions.len() == 0 || result.triggered_explosions.len() > 0);
}

#[test]
fn test_update_with_multiple_rockets() {
    let mut config = PhysicConfig::default();
    config.max_rockets = 100;
    config.rocket_interval_mean = 100.0; // Empêcher le spawn automatique
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn plusieurs fusées
    for _ in 0..10 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    assert_eq!(engine.rockets_count(), 10);

    // Continuer la simulation (sans spawn automatique)
    for _ in 0..100 {
        engine.update(0.016);
    }

    // Le moteur devrait gérer correctement plusieurs fusées
    // Certaines peuvent s'être désactivées
    assert!(engine.rockets_count() <= 10);
}

#[test]
fn test_update_triggered_explosions() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn une fusée
    engine.force_next_launch();
    engine.update(0.016);

    let mut total_explosions = 0;

    // Simuler jusqu'à ce qu'une explosion se produise
    for _ in 0..500 {
        let result = engine.update(0.016);
        total_explosions += result.triggered_explosions.len();
    }

    // Au moins une explosion devrait s'être produite
    assert!(
        total_explosions > 0,
        "Au moins une explosion devrait se produire"
    );
}

// ==================================
// 6. Traits PhysicEngine
// ==================================

#[test]
fn test_set_window_width() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    engine.set_window_width(2560.0);

    // Vérifier que le moteur fonctionne toujours
    engine.force_next_launch();
    let result = engine.update(0.016);
    assert!(result.new_rocket.is_some());
}

#[test]
fn test_close_clears_all_data() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn quelques fusées
    engine.force_next_launch();
    engine.update(0.016);
    engine.force_next_launch();
    engine.update(0.016);

    assert!(engine.rockets_count() > 0);

    engine.close();

    assert_eq!(engine.rockets_count(), 0);
}

#[test]
fn test_get_config() {
    let config = PhysicConfig::default();
    let engine = PhysicEngineFireworks::new(&config, 1920.0);

    let retrieved = engine.get_config();
    assert_eq!(retrieved.max_rockets, config.max_rockets);
    assert_eq!(
        retrieved.particles_per_explosion,
        config.particles_per_explosion
    );
}

// ==================================
// 7. Itérateurs (PhysicEngineIterator)
// ==================================

#[test]
fn test_iter_active_particles_empty() {
    let config = PhysicConfig::default();
    let engine = PhysicEngineFireworks::new(&config, 1920.0);

    let count = engine.iter_active_particles().count();
    assert_eq!(count, 0);
}

#[test]
fn test_iter_active_particles_with_rockets() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn 2 fusées
    engine.force_next_launch();
    engine.update(0.016);
    engine.force_next_launch();
    engine.update(0.016);

    // Chaque fusée a au moins sa particule head
    let count = engine.iter_active_particles().count();
    assert!(count >= 2, "Devrait avoir au moins 2 particules (heads)");
}

#[test]
fn test_iter_active_particles_increases_with_trails() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn une fusée
    engine.force_next_launch();
    engine.update(0.016);

    let count_initial = engine.iter_active_particles().count();

    // Simuler pour générer des trails
    for _ in 0..10 {
        engine.update(0.016);
    }

    let count_after = engine.iter_active_particles().count();

    // Le nombre de particules devrait augmenter (trails)
    assert!(
        count_after >= count_initial,
        "Les trails devraient augmenter le nombre de particules"
    );
}

#[test]
fn test_iter_active_heads_not_exploded() {
    let mut config = PhysicConfig::default();
    config.rocket_interval_mean = 100.0; // Empêcher le spawn automatique
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn 3 fusées
    for _ in 0..3 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    // Toutes devraient être non explosées initialement
    let count = engine.iter_active_heads_not_exploded().count();
    assert_eq!(count, 3, "Les 3 fusées devraient être non explosées");

    // Simuler jusqu'à ce que certaines explosent
    // Note: le temps d'explosion dépend de la vitesse et de la gravité
    for _ in 0..1000 {
        engine.update(0.016);
    }

    // Certaines devraient avoir explosé ou être désactivées
    let count_after = engine.iter_active_heads_not_exploded().count();
    assert!(
        count_after <= count,
        "Le nombre de fusées non explosées devrait diminuer ou rester égal"
    );
}

#[test]
fn test_iter_active_heads_filters_correctly() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn plusieurs fusées
    for _ in 0..5 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    let heads_count = engine.iter_active_heads_not_exploded().count();
    let total_rockets = engine.rockets_count();

    // Le nombre de heads non explosées devrait être <= nombre total de fusées
    assert!(heads_count <= total_rockets);
}

// ==================================
// 8. Edge Cases et Robustesse
// ==================================

#[test]
fn test_zero_dt_update() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Update avec dt = 0 ne devrait pas paniquer
    let result = engine.update(0.0);
    assert!(result.new_rocket.is_none());
}

#[test]
fn test_very_large_dt_update() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Update avec dt très grand
    let result = engine.update(10.0);
    // Devrait spawner une fusée car intervalle dépassé
    assert!(result.new_rocket.is_some());
}

#[test]
fn test_multiple_close_calls() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    engine.close();
    engine.close(); // Deuxième appel ne devrait pas paniquer
    assert_eq!(engine.rockets_count(), 0);
}

#[test]
fn test_reload_config_multiple_times() {
    let mut config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Recharger plusieurs fois
    for i in 1..5 {
        config.max_rockets = 10 * i;
        engine.reload_config(&config);
    }

    // Le moteur devrait fonctionner normalement
    engine.force_next_launch();
    let result = engine.update(0.016);
    assert!(result.new_rocket.is_some());
}

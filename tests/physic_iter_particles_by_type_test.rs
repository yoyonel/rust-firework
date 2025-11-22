use fireworks_sim::physic_engine::{
    config::PhysicConfig,
    physic_engine_generational_arena::{PhysicEngineFireworks, PhysicEngineTestHelpers},
    ParticleType, PhysicEngine, PhysicEngineIterator,
};

/// Test que iter_particles_by_type retourne les particules de tête pour ParticleType::Rocket
/// Ce test aurait détecté la régression où les têtes de fusées n'étaient pas visibles
#[test]
fn test_iter_particles_by_type_returns_rocket_heads() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn 3 fusées
    for _ in 0..3 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    // CRITIQUE: iter_particles_by_type(Rocket) doit retourner les têtes de fusées
    let rocket_particles: Vec<_> = engine
        .iter_particles_by_type(ParticleType::Rocket)
        .collect();

    assert_eq!(
        rocket_particles.len(),
        3,
        "iter_particles_by_type(Rocket) devrait retourner 3 particules de tête"
    );

    // Vérifier que toutes les particules retournées sont bien de type Rocket
    for p in &rocket_particles {
        assert_eq!(
            p.particle_type,
            ParticleType::Rocket,
            "Toutes les particules devraient être de type Rocket"
        );
    }
}

/// Test que iter_particles_by_type(Rocket) est équivalent à iter_active_heads_not_exploded
#[test]
fn test_iter_particles_by_type_rocket_equals_heads() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn plusieurs fusées
    for _ in 0..5 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    let rocket_count = engine.iter_particles_by_type(ParticleType::Rocket).count();
    let heads_count = engine.iter_active_heads_not_exploded().count();

    assert_eq!(
        rocket_count, heads_count,
        "iter_particles_by_type(Rocket) devrait retourner le même nombre que iter_active_heads_not_exploded"
    );
}

/// Test que iter_particles_by_type retourne les particules de traînée
#[test]
fn test_iter_particles_by_type_returns_trails() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn une fusée
    engine.force_next_launch();
    engine.update(0.016);

    // Simuler pour générer des trails
    for _ in 0..20 {
        engine.update(0.016);
    }

    let trail_particles: Vec<_> = engine.iter_particles_by_type(ParticleType::Trail).collect();

    // Devrait avoir des particules de traînée
    assert!(
        trail_particles.len() > 0,
        "Devrait avoir des particules de traînée après simulation"
    );

    // Vérifier que toutes sont bien de type Trail
    for p in &trail_particles {
        assert_eq!(
            p.particle_type,
            ParticleType::Trail,
            "Toutes les particules devraient être de type Trail"
        );
    }
}

/// Test que iter_particles_by_type retourne les particules d'explosion
#[test]
fn test_iter_particles_by_type_returns_explosions() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn une fusée
    engine.force_next_launch();
    engine.update(0.016);

    // Simuler jusqu'à l'explosion
    for _ in 0..500 {
        engine.update(0.016);
    }

    let explosion_particles: Vec<_> = engine
        .iter_particles_by_type(ParticleType::Explosion)
        .collect();

    // Devrait avoir des particules d'explosion
    assert!(
        explosion_particles.len() > 0,
        "Devrait avoir des particules d'explosion après simulation"
    );

    // Vérifier que toutes sont bien de type Explosion
    for p in &explosion_particles {
        assert_eq!(
            p.particle_type,
            ParticleType::Explosion,
            "Toutes les particules devraient être de type Explosion"
        );
    }
}

/// Test que iter_particles_by_type ne retourne rien pour un type sans particules
#[test]
fn test_iter_particles_by_type_empty_for_smoke() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn une fusée
    engine.force_next_launch();
    engine.update(0.016);

    // Smoke n'est pas encore implémenté, devrait retourner 0
    let smoke_count = engine.iter_particles_by_type(ParticleType::Smoke).count();

    assert_eq!(
        smoke_count, 0,
        "Aucune particule de fumée ne devrait exister pour l'instant"
    );
}

/// Test que iter_particles_by_type filtre correctement parmi plusieurs types
#[test]
fn test_iter_particles_by_type_filters_correctly() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn plusieurs fusées
    for _ in 0..3 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    // Simuler pour avoir trails et explosions
    for _ in 0..500 {
        engine.update(0.016);
    }

    let rocket_count = engine.iter_particles_by_type(ParticleType::Rocket).count();
    let trail_count = engine.iter_particles_by_type(ParticleType::Trail).count();
    let explosion_count = engine
        .iter_particles_by_type(ParticleType::Explosion)
        .count();
    let total_particles = engine.iter_active_particles().count();

    // La somme des particules par type devrait être <= au total
    // (peut être < car certaines particules peuvent être inactives)
    assert!(
        rocket_count + trail_count + explosion_count <= total_particles + rocket_count,
        "La somme des particules filtrées devrait être cohérente avec le total"
    );
}

/// Test de régression: vérifier que les particules Rocket sont visibles après filtrage
/// Ce test aurait détecté le bug où iter_particles_by_type ne retournait pas les heads
#[test]
fn test_regression_rocket_particles_visible() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    // Spawn 5 fusées non explosées
    for _ in 0..5 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    // CRITIQUE: Ce test aurait échoué avec le bug initial
    let rocket_particles: Vec<_> = engine
        .iter_particles_by_type(ParticleType::Rocket)
        .collect();

    assert!(
        !rocket_particles.is_empty(),
        "RÉGRESSION: Les particules de fusée ne sont pas visibles via iter_particles_by_type!"
    );

    assert_eq!(
        rocket_particles.len(),
        5,
        "Devrait avoir exactement 5 particules de fusée visibles"
    );

    // Vérifier que les particules ont des positions valides (non nulles)
    for (i, p) in rocket_particles.iter().enumerate() {
        assert!(
            p.pos.x != 0.0 || p.pos.y != 0.0,
            "Particule {} devrait avoir une position non nulle",
            i
        );
        assert!(p.active, "Particule {} devrait être active", i);
    }
}

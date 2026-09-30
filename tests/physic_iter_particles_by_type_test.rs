use fireworks_sim::physic_engine::{
    config::PhysicConfig,
    particle::Particle,
    physic_engine_generational_arena::{PhysicEngineFireworks, PhysicEngineTestHelpers},
    ParticleType, PhysicEngine, PhysicEngineIterator,
};

fn count_active_particles(engine: &PhysicEngineFireworks) -> usize {
    let mut count = 0;
    engine.for_each_active_particle(&mut |_| count += 1);
    count
}

fn count_active_heads(engine: &PhysicEngineFireworks) -> usize {
    let mut count = 0;
    engine.for_each_active_head_not_exploded(&mut |_| count += 1);
    count
}

fn count_particles_by_type(engine: &PhysicEngineFireworks, particle_type: ParticleType) -> usize {
    let mut count = 0;
    engine.for_each_particle_of_type(particle_type, &mut |_| count += 1);
    count
}

fn collect_particles_by_type(
    engine: &PhysicEngineFireworks,
    particle_type: ParticleType,
) -> Vec<Particle> {
    let mut v = Vec::new();
    engine.for_each_particle_of_type(particle_type, &mut |p| v.push(*p));
    v
}

/// Test que iter_particles_by_type retourne les particules de tête pour ParticleType::Rocket
/// Ce test aurait détecté la régression où les têtes de fusées n'étaient pas visibles
#[test]
fn test_iter_particles_by_type_returns_rocket_heads() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

    // Spawn 3 fusées
    for _ in 0..3 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    // CRITIQUE: iter_particles_by_type(Rocket) doit retourner les têtes de fusées
    let rocket_particles = collect_particles_by_type(&engine, ParticleType::Rocket);

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
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

    // Spawn plusieurs fusées
    for _ in 0..5 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    let rocket_count = count_particles_by_type(&engine, ParticleType::Rocket);
    let heads_count = count_active_heads(&engine);

    assert_eq!(
        rocket_count, heads_count,
        "iter_particles_by_type(Rocket) devrait retourner le même nombre que iter_active_heads_not_exploded"
    );
}

/// Test que iter_particles_by_type retourne les particules de traînée
#[test]
fn test_iter_particles_by_type_returns_trails() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

    // Spawn une fusée
    engine.force_next_launch();
    engine.update(0.016);

    // Simuler pour générer des trails
    for _ in 0..20 {
        engine.update(0.016);
    }

    let trail_particles = collect_particles_by_type(&engine, ParticleType::Trail);

    // Devrait avoir des particules de traînée
    assert!(
        !trail_particles.is_empty(),
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
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

    // Spawn une fusée
    engine.force_next_launch();
    engine.update(0.016);

    // Simuler jusqu'à l'explosion
    for _ in 0..500 {
        engine.update(0.016);
    }

    let explosion_particles = collect_particles_by_type(&engine, ParticleType::Explosion);

    // Devrait avoir des particules d'explosion
    assert!(
        !explosion_particles.is_empty(),
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
fn test_iter_particles_by_type_emits_smoke_for_active_rocket() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

    // Initial state: no smoke
    assert_eq!(count_particles_by_type(&engine, ParticleType::Smoke), 0);

    // Launch rocket and advance physics step
    engine.force_next_launch();
    for _ in 0..10 {
        engine.update(0.016);
    }

    // Active rocket should emit smoke particles
    let smoke_count = count_particles_by_type(&engine, ParticleType::Smoke);
    assert!(
        smoke_count > 0,
        "Smoke particles should be emitted continuously for active ascending rockets"
    );
}

/// Test que iter_particles_by_type filtre correctement parmi plusieurs types
#[test]
fn test_iter_particles_by_type_filters_correctly() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

    // Spawn plusieurs fusées
    for _ in 0..3 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    // Simuler pour avoir trails et explosions
    for _ in 0..500 {
        engine.update(0.016);
    }

    let rocket_count = count_particles_by_type(&engine, ParticleType::Rocket);
    let trail_count = count_particles_by_type(&engine, ParticleType::Trail);
    let explosion_count = count_particles_by_type(&engine, ParticleType::Explosion);
    let total_particles = count_active_particles(&engine);

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
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

    // Spawn 5 fusées non explosées
    for _ in 0..5 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    // CRITIQUE: Ce test aurait échoué avec le bug initial
    let rocket_particles = collect_particles_by_type(&engine, ParticleType::Rocket);

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

/// Test que les bascules de visibilité des éléments graphiques sont totalement indépendantes
#[test]
fn test_visibility_toggles_independence() {
    use fireworks_sim::renderer_engine::RendererConfig;

    let mut cfg = RendererConfig::default();
    assert!(cfg.render_rockets);
    assert!(cfg.render_smoke);
    assert!(cfg.render_trails);
    assert!(cfg.render_explosions);

    // Désactiver seulement les fusées
    cfg.render_rockets = false;
    cfg.render_trails = true;
    assert!(!cfg.render_rockets);
    assert!(cfg.render_trails);

    // Désactiver seulement les fumées
    cfg.render_smoke = false;
    assert!(!cfg.render_rockets);
    assert!(!cfg.render_smoke);
    assert!(cfg.render_trails);
}

/// Test de non-régression et d'équivalence stricte entre l'itération slice directe et l'itération legacy
#[test]
fn test_slice_iteration_equivalence() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

    for _ in 0..5 {
        engine.force_next_launch();
        engine.update(0.016);
    }

    // 1. Équivalence for_each_active_particle vs for_each_active_particle_slice
    let mut legacy_particles = Vec::new();
    engine.for_each_active_particle(&mut |p| legacy_particles.push(*p));

    let mut slice_particles = Vec::new();
    engine.for_each_active_particle_slice(&mut |slice| {
        slice_particles.extend_from_slice(slice);
    });

    assert_eq!(legacy_particles.len(), slice_particles.len());
    for (i, (p_leg, p_slice)) in legacy_particles.iter().zip(&slice_particles).enumerate() {
        assert_eq!(
            p_leg.pos, p_slice.pos,
            "Position mismatch at particle {}",
            i
        );
        assert_eq!(p_leg.particle_type, p_slice.particle_type);
    }

    // 2. Équivalence for_each_particle_of_type vs for_each_particle_slice_of_type
    for pt in [
        ParticleType::Rocket,
        ParticleType::Trail,
        ParticleType::Explosion,
    ] {
        let mut leg = Vec::new();
        engine.for_each_particle_of_type(pt, &mut |p| leg.push(*p));

        let mut slc = Vec::new();
        engine.for_each_particle_slice_of_type(pt, &mut |slice| slc.extend_from_slice(slice));

        assert_eq!(leg.len(), slc.len(), "Count mismatch for {:?}", pt);
        for (i, (p1, p2)) in leg.iter().zip(&slc).enumerate() {
            assert_eq!(p1.pos, p2.pos, "Pos mismatch for {:?} at index {}", pt, i);
        }
    }

    // 3. Fumée directe
    let smoke_slice = engine.active_smoke_slice();
    let mut smoke_legacy = Vec::new();
    engine.for_each_smoke_particle(&mut |sp| smoke_legacy.push(*sp));
    assert_eq!(smoke_slice.len(), smoke_legacy.len());
}

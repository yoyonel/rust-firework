use fireworks_sim::physic_engine::{
    config::PhysicConfig, particle::Particle,
    physic_engine_generational_arena::PhysicEngineFireworks, PhysicEngine, PhysicEngineIterator,
};

fn run_simulation(seed: u64, frames: u64, dt: f32) -> f32 {
    let mut config = PhysicConfig::default();
    config.max_rockets = 50;
    config.rocket_interval_mean = 0.05;
    config.rocket_interval_variation = 0.0;
    config.particles_per_explosion = 100;
    config.particles_per_trail = 50;

    let mut engine = PhysicEngineFireworks::new(&config, 800.0, Some(seed));

    for _ in 0..frames {
        let _ = engine.update(dt);
    }

    let mut hash_sum = 0.0;
    engine.for_each_active_particle(&mut |p: &Particle| {
        hash_sum += p.pos.x + p.pos.y + p.vel.x + p.vel.y + p.life;
    });

    hash_sum
}

#[test]
fn test_physics_engine_determinism() {
    let frames = 250;
    let dt = 1.0 / 120.0;
    let seed = 42;

    let hash_a = run_simulation(seed, frames, dt);
    let hash_b = run_simulation(seed, frames, dt);

    assert_eq!(
        hash_a, hash_b,
        "Les simulations avec une même seed doivent produire le même état mathématique"
    );

    // Test with different seed
    let hash_c = run_simulation(99, frames, dt);
    assert_ne!(
        hash_a, hash_c,
        "Des seeds différentes doivent produire des résultats différents"
    );
}

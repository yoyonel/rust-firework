use fireworks_sim::physic_engine::particles_pools::{ParticlesPoolsForRockets, PoolKind};
use fireworks_sim::physic_engine::rocket::Rocket;
use rand::SeedableRng;

#[test]
fn test_pool_allocation_and_freeing() {
    // Setup: 2 rockets max, 10 particles per explosion, 5 per trail
    let max_rockets = 2;
    let per_explosion = 10;
    let per_trail = 5;
    let mut pools = ParticlesPoolsForRockets::new(max_rockets, per_explosion, per_trail);

    // 1. Allocate all available blocks for explosions
    let range1 = pools.particles_pool_for_explosions.allocate_block();
    assert!(range1.is_some());
    let range1 = range1.unwrap();
    assert_eq!(range1.len(), per_explosion);

    let range2 = pools.particles_pool_for_explosions.allocate_block();
    assert!(range2.is_some());
    let range2 = range2.unwrap();
    assert_eq!(range2.len(), per_explosion);

    // 2. Verify exhaustion
    let range3 = pools.particles_pool_for_explosions.allocate_block();
    assert!(range3.is_none(), "Pool should be exhausted");

    // 3. Write to allocated memory (sanity check)
    {
        let slice = pools.access_mut(PoolKind::Explosions, &range1);
        assert_eq!(slice.len(), per_explosion);
        slice[0].active = true; // Modify something
    }

    // 4. Free blocks via Rocket
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);

    // Assign the ranges to the rocket so free_blocks can reclaim them
    rocket.explosion_particle_indices = Some(range1);
    // We didn't allocate trails yet, but let's allocate one to test full freeing
    let trail_range = pools.particles_pool_for_trails.allocate_block().unwrap();
    rocket.trail_particle_indices = Some(trail_range);

    pools.free_blocks(&mut rocket);

    // Verify rocket indices are cleared
    assert!(rocket.explosion_particle_indices.is_none());
    assert!(rocket.trail_particle_indices.is_none());

    // 5. Verify we can allocate again
    let range_reclaimed = pools.particles_pool_for_explosions.allocate_block();
    assert!(
        range_reclaimed.is_some(),
        "Should be able to re-allocate freed block"
    );

    // The reclaimed block should be the one we just freed (LIFO usually, but implementation detail)
    // We just care that we got one.
}

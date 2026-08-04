use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fireworks_sim::physic_engine::particles_pools::ParticlesPoolsForRockets;
use fireworks_sim::physic_engine::rocket::Rocket;
use rand::SeedableRng;

fn bench_particles_pool_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("physics/particles_pool");

    group.bench_function("allocate_and_free_1000_blocks", |b| {
        let max_rockets = 1000;
        let per_explosion = 256;
        let per_trail = 64;

        b.iter(|| {
            let mut pools = ParticlesPoolsForRockets::new(max_rockets, per_explosion, per_trail);
            let mut rng = rand::rngs::StdRng::seed_from_u64(42);

            // Allocation intensive de tous les blocs disponibles
            let mut allocated = Vec::with_capacity(max_rockets);
            for _ in 0..max_rockets {
                if let Some(range) = pools.particles_pool_for_explosions.allocate_block() {
                    allocated.push(range);
                }
            }

            // Libération de tous les blocs via des fusées virtuelles
            for range in allocated {
                let mut rocket = Rocket::new(&mut rng);
                rocket.explosion_particle_indices = Some(range);
                pools.free_blocks(&mut rocket);
            }

            black_box(&pools);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_particles_pool_allocation);
criterion_main!(benches);

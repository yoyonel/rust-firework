use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use fireworks_sim::physic_engine::particles_pools::ParticlesPool;

const MAX_BLOCKS: usize = 1_000;
const PER_BLOCK: usize = 256;

/// Benchmark isolé sur `allocate_block` seul.
///
/// Le pool est instancié **avant** la boucle chaude via `iter_batched` avec
/// `BatchSize::SmallInput` (pool clone à chaque batch) pour exclure tout overhead
/// de construction mémoire. Seul le coût de `pop()` sur la pile LIFO est mesuré.
fn bench_allocate_block(c: &mut Criterion) {
    let mut group = c.benchmark_group("physic-pool");

    group.bench_function("allocate-block", |b| {
        b.iter_batched(
            || ParticlesPool::new(MAX_BLOCKS, PER_BLOCK),
            |mut pool| {
                // Mesure : 1 appel allocate_block sur un pool frais
                black_box(pool.allocate_block())
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark isolé sur `free_block` seul.
///
/// Le pool est pré-épuisé avant le batch (tous les blocs alloués) ;
/// seul le coût de `push()` sur la pile LIFO est mesuré.
fn bench_free_block(c: &mut Criterion) {
    let mut group = c.benchmark_group("physic-pool");

    group.bench_function("free-block", |b| {
        b.iter_batched(
            || {
                let mut pool = ParticlesPool::new(MAX_BLOCKS, PER_BLOCK);
                // Épuise tous les blocs, récupère les indices pour la libération
                let allocated: Vec<usize> = (0..MAX_BLOCKS)
                    .filter_map(|_| pool.allocate_block().map(|r| r.start))
                    .collect();
                (pool, allocated)
            },
            |(mut pool, allocated)| {
                // Mesure : libération de tous les blocs alloués
                for start in allocated {
                    pool.free_block_by_start(start);
                }
                black_box(pool)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark sur l'intégration physique (update) des particules dans le pool.
///
/// Mesure le coût du balayage mémoire et de la mise à jour de la physique
/// (position, vitesse, durée de vie, état actif) sur 25 600 particules actives.
fn bench_particle_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("physic-pool");
    let dt = 0.016_f32;
    let gravity = glam::Vec2::new(0.0, -9.81);

    group.bench_function("particle-update-25k", |b| {
        b.iter_batched(
            || {
                let mut pool = ParticlesPool::new(MAX_BLOCKS, PER_BLOCK);
                let mut ranges = Vec::new();
                for _ in 0..MAX_BLOCKS {
                    if let Some(r) = pool.allocate_block() {
                        let slice = pool.get_particles_mut(&r);
                        for p in slice.iter_mut() {
                            p.active = true;
                            p.life = 2.0;
                            p.vel = glam::Vec2::new(1.0, 5.0);
                            p.pos = glam::Vec2::new(10.0, 10.0);
                        }
                        ranges.push(r);
                    }
                }
                (pool, ranges)
            },
            |(mut pool, ranges)| {
                for r in &ranges {
                    let slice = pool.get_particles_mut(r);
                    for p in slice.iter_mut() {
                        if !p.active {
                            continue;
                        }
                        p.vel.y += gravity.y * dt;
                        p.pos += p.vel * dt;
                        p.life -= dt;
                        p.active = p.life > 0.0;
                    }
                }
                black_box(pool)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_allocate_block,
    bench_free_block,
    bench_particle_update
);
criterion_main!(benches);

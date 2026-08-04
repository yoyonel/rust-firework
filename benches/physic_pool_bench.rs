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
                black_box(&pool)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(benches, bench_allocate_block, bench_free_block);
criterion_main!(benches);

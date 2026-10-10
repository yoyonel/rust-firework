use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::{
    PhysicEngineFireworks, PhysicEngineTestHelpers,
};
use fireworks_sim::physic_engine::{PhysicEngine, PhysicEngineIterator};
use fireworks_sim::renderer_engine::constants::RENDER_INTERPOLATION_EPSILON;
use fireworks_sim::renderer_engine::types::ParticleGPU;

fn bench_particle_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("physic/particle_iteration");

    for n_rockets in [50, 200, 1000] {
        let mut config = PhysicConfig::default();
        config.max_rockets = n_rockets * 2;
        let mut engine = PhysicEngineFireworks::new(&config, 1920.0, None);

        // Spawn rockets and advance physics to generate trails and explosions
        for _ in 0..n_rockets {
            engine.force_next_launch();
            engine.update(0.016);
        }

        let max_particles = 100_000;
        let mut gpu_slice = vec![ParticleGPU::default(); max_particles];
        let factor = 0.5f32;

        // 1. Legacy per-particle dynamic callback (dyn FnMut(&Particle))
        group.bench_with_input(
            BenchmarkId::new("legacy_dyn_fnmut", n_rockets),
            &n_rockets,
            |b, _| {
                b.iter(|| {
                    let mut count = 0;
                    engine.for_each_active_particle(&mut |p| {
                        if count < max_particles {
                            let mut core = p.core;
                            if factor > RENDER_INTERPOLATION_EPSILON {
                                core.pos -= p.vel * factor;
                            }
                            let l = core.life / core.max_life.max(0.0001);
                            let l2 = l * l;
                            gpu_slice[count] = ParticleGPU {
                                core,
                                brightness: l2 * l2,
                            };
                            count += 1;
                        }
                    });
                    black_box(count);
                });
            },
        );

        // 2. Optimized slice iteration (for_each_active_particle_slice)
        group.bench_with_input(
            BenchmarkId::new("devirtualized_slice", n_rockets),
            &n_rockets,
            |b, _| {
                b.iter(|| {
                    let mut count = 0;
                    engine.for_each_active_particle_slice(&mut |slice| {
                        for p in slice {
                            if count < max_particles {
                                let mut core = p.core;
                                if factor > RENDER_INTERPOLATION_EPSILON {
                                    core.pos -= p.vel * factor;
                                }
                                let l = core.life / core.max_life.max(0.0001);
                                let l2 = l * l;
                                gpu_slice[count] = ParticleGPU {
                                    core,
                                    brightness: l2 * l2,
                                };
                                count += 1;
                            }
                        }
                    });
                    black_box(count);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_particle_iteration);
criterion_main!(benches);

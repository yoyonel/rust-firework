use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use fireworks_sim::audio_engine::dsp::resample_linear_mono;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn make_sine(n_samples: usize, freq_hz: f32, sample_rate: u32) -> Vec<f32> {
    (0..n_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * freq_hz * t).sin()
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : upsample 44100 → 48000
// ─────────────────────────────────────────────────────────────────────────────

fn bench_resample_upsample(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample/upsample_44100_to_48000");

    for n_samples in [64usize, 128, 512, 1024, 4096, 44100] {
        let input = make_sine(n_samples, 440.0, 44100);

        group.bench_with_input(BenchmarkId::from_parameter(n_samples), &input, |b, inp| {
            b.iter(|| {
                let out = resample_linear_mono(inp, 44100, 48000);
                criterion::black_box(out);
            });
        });
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : downsample 48000 → 44100
// ─────────────────────────────────────────────────────────────────────────────

fn bench_resample_downsample(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample/downsample_48000_to_44100");

    for n_samples in [64usize, 128, 512, 1024, 4096, 48000] {
        let input = make_sine(n_samples, 440.0, 48000);

        group.bench_with_input(BenchmarkId::from_parameter(n_samples), &input, |b, inp| {
            b.iter(|| {
                let out = resample_linear_mono(inp, 48000, 44100);
                criterion::black_box(out);
            });
        });
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : identity (src_rate == dst_rate — chemin rapide)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_resample_identity(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample/identity_48000");

    for n_samples in [512usize, 4096, 48000] {
        let input = make_sine(n_samples, 440.0, 48000);

        group.bench_with_input(BenchmarkId::from_parameter(n_samples), &input, |b, inp| {
            b.iter(|| {
                let out = resample_linear_mono(inp, 48000, 48000);
                criterion::black_box(out);
            });
        });
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : ratio extrême — 8000 → 48000 (× 6)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_resample_extreme_upsample(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample/extreme_upsample_8000_to_48000");

    for n_samples in [512usize, 4096, 8000] {
        let input = make_sine(n_samples, 440.0, 8000);

        group.bench_with_input(BenchmarkId::from_parameter(n_samples), &input, |b, inp| {
            b.iter(|| {
                let out = resample_linear_mono(inp, 8000, 48000);
                criterion::black_box(out);
            });
        });
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : simulation 1 seconde d'audio complet (throughput réel)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_resample_one_second(c: &mut Criterion) {
    let mut group = c.benchmark_group("resample/one_second_throughput");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.sample_size(50);

    // 1 seconde à 44100 Hz → 48000 Hz
    let input_44100 = make_sine(44100, 440.0, 44100);
    group.bench_function("44100_to_48000", |b| {
        b.iter(|| {
            let out = resample_linear_mono(&input_44100, 44100, 48000);
            criterion::black_box(out);
        });
    });

    // 1 seconde à 48000 Hz → 44100 Hz
    let input_48000 = make_sine(48000, 440.0, 48000);
    group.bench_function("48000_to_44100", |b| {
        b.iter(|| {
            let out = resample_linear_mono(&input_48000, 48000, 44100);
            criterion::black_box(out);
        });
    });

    group.finish();
}

// Exemple de structure de bench à ajouter pour évaluer le surcoût du LERP fractionnaire
fn bench_doppler_lerp_resampling(c: &mut Criterion) {
    let mut group = c.benchmark_group("doppler/lerp_resampling");
    let input = make_sine(48000, 440.0, 48000); // 1 seconde de buffer brut

    // Facteurs Doppler typiques :
    // 1.0 = statique (baseline), 1.2 = approche rapide (+20% pitch), 0.8 = éloignement
    for &alpha in &[0.8_f32, 1.0, 1.2, 2.5] {
        group.throughput(Throughput::Elements(1024));
        group.bench_with_input(BenchmarkId::from_parameter(alpha), &alpha, |b, &rate| {
            b.iter(|| {
                // Simulation de la boucle interne de votre Voice (1024 frames)
                let mut pos = 0.0_f64;
                let mut out = [0.0_f32; 1024];
                for sample in out.iter_mut() {
                    let idx = pos as usize;
                    let frac = (pos - idx as f64) as f32;
                    let s0 = input[idx % (input.len() - 1)];
                    let s1 = input[(idx + 1) % input.len()];
                    *sample = s0 + frac * (s1 - s0);
                    pos += rate as f64;
                }
                criterion::black_box(out);
            });
        });
    }
    group.finish();
}

fn bench_doppler_geometry_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("doppler/geometry_block_update");
    let listener_pos = (0.0_f32, 0.0_f32);
    let c_sound = 343.0_f32;

    // Évaluer le coût mathématique pour 1, 16, 64 et 128 sources simultanées
    for n_voices in [1usize, 16, 64, 128] {
        let voices: Vec<((f32, f32), (f32, f32))> = (0..n_voices)
            .map(|i| ((100.0 + i as f32 * 10.0, 50.0), (-50.0, 20.0)))
            .collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(n_voices),
            &voices,
            |b, v_list| {
                b.iter(|| {
                    for &(pos, vel) in v_list {
                        let dx = pos.0 - listener_pos.0;
                        let dy = pos.1 - listener_pos.1;
                        let dist = (dx * dx + dy * dy).sqrt().max(0.001);
                        let dir_x = -dx / dist;
                        let dir_y = -dy / dist;
                        let v_radial = vel.0 * dir_x + vel.1 * dir_y;
                        let alpha = (c_sound / (c_sound - v_radial)).clamp(0.25, 4.0);
                        criterion::black_box(alpha);
                    }
                });
            },
        );
    }
    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Registration
// ─────────────────────────────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_resample_upsample,
    bench_resample_downsample,
    bench_resample_identity,
    bench_resample_extreme_upsample,
    bench_resample_one_second,
    bench_doppler_lerp_resampling,
    bench_doppler_geometry_update,
);
criterion_main!(benches);

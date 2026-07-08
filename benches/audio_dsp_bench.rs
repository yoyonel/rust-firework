use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
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
);
criterion_main!(benches);

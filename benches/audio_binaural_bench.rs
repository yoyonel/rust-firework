use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use fireworks_sim::audio_engine::binaural_processing::binauralize_mono;
use fireworks_sim::AudioEngineSettings;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn make_mono(n_samples: usize) -> Vec<f32> {
    // Signal constant à 1.0 — simple et reproductible
    vec![1.0_f32; n_samples]
}

fn make_sine_mono(n_samples: usize, freq_hz: f32, sample_rate: u32) -> Vec<f32> {
    (0..n_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * freq_hz * t).sin()
        })
        .collect()
}

fn default_settings() -> AudioEngineSettings {
    AudioEngineSettings::default()
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : positions spatiales (bloc fixe de 512 samples)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_binaural_positions(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaural/positions");
    let settings = default_settings();
    let sample_rate = 48_000_u32;
    let mono = make_mono(512);
    let listener = (0.0_f32, 0.0_f32, 0.0_f32);

    // Centre (azimuth = 0 → cas symétrique)
    group.bench_function("center", |b| {
        b.iter(|| {
            let out = binauralize_mono(&mono, (0.0, 0.0, -10.0), listener, sample_rate, &settings);
            criterion::black_box(out);
        });
    });

    // Source à droite (azimuth positif, ITD + ILD droite dominante)
    group.bench_function("right_90deg", |b| {
        b.iter(|| {
            let out = binauralize_mono(&mono, (10.0, 0.0, 0.0), listener, sample_rate, &settings);
            criterion::black_box(out);
        });
    });

    // Source à gauche (azimuth négatif)
    group.bench_function("left_90deg", |b| {
        b.iter(|| {
            let out = binauralize_mono(&mono, (-10.0, 0.0, 0.0), listener, sample_rate, &settings);
            criterion::black_box(out);
        });
    });

    // Source avec élévation (3D complet)
    group.bench_function("above_right_45deg", |b| {
        b.iter(|| {
            let out = binauralize_mono(&mono, (10.0, 10.0, 0.0), listener, sample_rate, &settings);
            criterion::black_box(out);
        });
    });

    // Source derrière (dz positif)
    group.bench_function("behind", |b| {
        b.iter(|| {
            let out = binauralize_mono(&mono, (0.0, 0.0, 10.0), listener, sample_rate, &settings);
            criterion::black_box(out);
        });
    });

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : distance (atténuation)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_binaural_distances(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaural/distances");
    let settings = default_settings();
    let sample_rate = 48_000_u32;
    let mono = make_mono(512);
    let listener = (0.0_f32, 0.0_f32, 0.0_f32);

    for distance in [1.0_f32, 10.0, 50.0, 100.0, 500.0, 999.0] {
        group.bench_with_input(
            BenchmarkId::new("right_at_distance", distance as usize),
            &distance,
            |b, &d| {
                b.iter(|| {
                    let out =
                        binauralize_mono(&mono, (d, 0.0, 0.0), listener, sample_rate, &settings);
                    criterion::black_box(out);
                });
            },
        );
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : taille de bloc (latence audio vs throughput)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_binaural_block_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaural/block_sizes");
    let settings = default_settings();
    let sample_rate = 48_000_u32;
    let listener = (0.0_f32, 0.0_f32, 0.0_f32);
    let src_pos = (10.0_f32, 0.0_f32, 0.0_f32); // droite

    for block_size in [64usize, 128, 256, 512, 1024, 2048, 4096] {
        let mono = make_mono(block_size);
        group.bench_with_input(BenchmarkId::from_parameter(block_size), &mono, |b, inp| {
            b.iter(|| {
                let out = binauralize_mono(inp, src_pos, listener, sample_rate, &settings);
                criterion::black_box(out);
            });
        });
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : signal sinusoïdal vs constant (impact cache/branchement)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_binaural_signal_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaural/signal_types");
    let settings = default_settings();
    let sample_rate = 48_000_u32;
    let listener = (0.0_f32, 0.0_f32, 0.0_f32);
    let src_pos = (10.0_f32, 0.0_f32, 0.0_f32);

    let mono_constant = make_mono(1024);
    group.bench_function("constant_signal_1024", |b| {
        b.iter(|| {
            let out = binauralize_mono(&mono_constant, src_pos, listener, sample_rate, &settings);
            criterion::black_box(out);
        });
    });

    let mono_sine = make_sine_mono(1024, 440.0, sample_rate);
    group.bench_function("sine_440hz_1024", |b| {
        b.iter(|| {
            let out = binauralize_mono(&mono_sine, src_pos, listener, sample_rate, &settings);
            criterion::black_box(out);
        });
    });

    let mono_sine_high = make_sine_mono(1024, 4000.0, sample_rate);
    group.bench_function("sine_4000hz_1024", |b| {
        b.iter(|| {
            let out = binauralize_mono(&mono_sine_high, src_pos, listener, sample_rate, &settings);
            criterion::black_box(out);
        });
    });

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : sample rates variés
// ─────────────────────────────────────────────────────────────────────────────

fn bench_binaural_sample_rates(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaural/sample_rates");
    let settings = default_settings();
    let mono = make_mono(1024);
    let listener = (0.0_f32, 0.0_f32, 0.0_f32);
    let src_pos = (10.0_f32, 0.0_f32, 0.0_f32);

    for sample_rate in [22050_u32, 44100, 48000, 96000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(sample_rate),
            &sample_rate,
            |b, &sr| {
                b.iter(|| {
                    let out = binauralize_mono(&mono, src_pos, listener, sr, &settings);
                    criterion::black_box(out);
                });
            },
        );
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Benchmark : simulation multi-voix (N appels en séquence)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_binaural_multi_voice(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaural/multi_voice");
    group.measurement_time(std::time::Duration::from_secs(10));
    let settings = default_settings();
    let sample_rate = 48_000_u32;
    let mono = make_mono(512); // bloc 512 = ~10.67ms à 48kHz
    let listener = (0.0_f32, 0.0_f32, 0.0_f32);

    // Pool de 128 positions générées de façon déterministe,
    // simulant des feux d'artifice distribués dans un hémisphère supérieur
    // (altitude positive, distances variées 50m–500m).
    //
    // Formule : spirale de Fibonacci projetée sur une sphère tronquée,
    // mise à l'échelle pour correspondre aux distances de scène typiques.
    let positions: Vec<(f32, f32, f32)> = (0..128)
        .map(|i| {
            let golden = std::f32::consts::PI * (3.0 - 5.0_f32.sqrt()); // ~2.399 rad
            let angle = i as f32 * golden;
            // y dans [0.1 .. 1.0] → hémisphère supérieur uniquement
            let y_norm = (i as f32 + 0.5) / 128.0;
            let y_norm = 0.1 + 0.9 * y_norm; // évite l'équateur exact
            let radius = (1.0 - y_norm * y_norm).sqrt();
            // Distance de scène : 50m à 500m selon l'indice
            let scale = 50.0 + (i as f32 / 127.0) * 450.0;
            (
                angle.cos() * radius * scale,
                y_norm * scale * 0.6, // élévation modérée
                angle.sin() * radius * scale,
            )
        })
        .collect();

    // Paliers couvrant : défaut actuel (32), cible court terme (64),
    // et projection future avec Doppler et effets additionnels (128)
    for n_voices in [1usize, 4, 8, 16, 32, 64, 128] {
        let voice_positions: Vec<_> = positions.iter().take(n_voices).cloned().collect();

        group.bench_with_input(
            BenchmarkId::new("voices", n_voices),
            &voice_positions,
            |b, positions| {
                b.iter(|| {
                    for &pos in positions {
                        let out = binauralize_mono(&mono, pos, listener, sample_rate, &settings);
                        criterion::black_box(out);
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
    bench_binaural_positions,
    bench_binaural_distances,
    bench_binaural_block_sizes,
    bench_binaural_signal_types,
    bench_binaural_sample_rates,
    bench_binaural_multi_voice,
);
criterion_main!(benches);

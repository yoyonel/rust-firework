use criterion::{black_box, criterion_group, BenchmarkId, Criterion, Throughput};
use fireworks_sim::audio_engine::dsp_processor::DspProcessor;
use fireworks_sim::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
use fireworks_sim::audio_engine::types::{AudioSoundType, Voice};
use fireworks_sim::profiler::Profiler;
use fireworks_sim::AudioEngineSettings;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn generate_sine_wave(freq: f32, sample_rate: u32, duration_samples: usize) -> Vec<[f32; 2]> {
    let mut data = Vec::with_capacity(duration_samples);
    for i in 0..duration_samples {
        let t = i as f32 / sample_rate as f32;
        let val = (2.0 * std::f32::consts::PI * freq * t).sin();
        data.push([val, val]);
    }
    data
}

fn create_bench_dsp(n_voices: usize, block_size: usize, enable_spatial_bus: bool) -> DspProcessor {
    let sample_rate = 48_000;
    let source_audio = generate_sine_wave(440.0, sample_rate, 48000);
    let source_arc = Arc::new(source_audio);
    let (_play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    let positions: Vec<glam::Vec2> = (0..n_voices)
        .map(|i| {
            let angle = i as f32 * 2.399; // Golden angle
            let scale = 10.0 + (i as f32 / n_voices as f32) * 200.0;
            glam::Vec2::new(angle.cos() * scale, angle.sin() * scale)
        })
        .collect();

    let voices = positions
        .into_iter()
        .enumerate()
        .map(|(i, pos)| Voice {
            id: i as u64 + 1,
            active: true,
            data: Some(source_arc.clone()),
            pos: (i * 37) as f64 % 40000.0,
            playback_rate: 1.0,
            is_dynamic: false,
            world_pos: pos,
            velocity: glam::Vec2::ZERO,
            fade_in_samples: 0,
            fade_out_samples: 0,
            filter_state: [0.0, 0.0],
            filter_a: 0.05,
            user_gain: 0.8,
            current_gains: [0.5, 0.5],
            target_gains: [0.5, 0.5],
            current_itd: [0.0, 0.0],
            target_itd: [0.0, 0.0],
            request_id: i as u64 + 1,
            sound_type: if i % 2 == 0 {
                AudioSoundType::Explosion
            } else {
                AudioSoundType::Rocket
            },
        })
        .collect();

    let effect_flags = AudioEffectFlags::new_all_enabled();
    effect_flags.set(AudioEffect::SpatialBus, enable_spatial_bus);

    DspProcessor {
        voices,
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: AudioEngineSettings::default(),
        listener_pos: Arc::new(fireworks_sim::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; block_size],
        bus_w: vec![0.0; block_size],
        bus_x: vec![0.0; block_size],
        export_buffer: vec![[0.0; 2]; block_size],
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags,
        spatial_reverb: fireworks_sim::audio_engine::SpatialReverb::new(sample_rate),
        hrtf_convolver: fireworks_sim::audio_engine::HrtfConvolver::new_default(
            sample_rate,
            block_size,
        ),
        debug_tx: None,
    }
}

fn bench_legacy_vs_spatial_bus(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_spatial_rendering_comparison");
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(20);

    let block_size = 256; // 5.33 ms audio block budget

    for n_voices in [1, 16, 32, 64, 128, 256, 512] {
        group.throughput(Throughput::Elements(n_voices as u64 * block_size as u64));

        // 1. Legacy (Per-Voice Binaural Direct)
        group.bench_with_input(
            BenchmarkId::new("Legacy_Direct_Binaural", n_voices),
            &n_voices,
            |b, &v_count| {
                let mut dsp = create_bench_dsp(v_count, block_size, false);
                let profiler = Profiler::new(100);
                let mut out_buf = vec![0.0f32; block_size * 2];
                b.iter(|| {
                    dsp.process_block(&mut out_buf, 1.0, &profiler);
                    black_box(&out_buf);
                });
            },
        );

        // 2. Spatial Bus 2D (Ambisonics 2D W, X)
        group.bench_with_input(
            BenchmarkId::new("Spatial_Bus_2D_Ambisonics", n_voices),
            &n_voices,
            |b, &v_count| {
                let mut dsp = create_bench_dsp(v_count, block_size, true);
                let profiler = Profiler::new(100);
                let mut out_buf = vec![0.0f32; block_size * 2];
                b.iter(|| {
                    dsp.process_block(&mut out_buf, 1.0, &profiler);
                    black_box(&out_buf);
                });
            },
        );
    }

    group.finish();
}

fn print_summary_table() {
    let block_size = 256;
    let voice_counts = [1, 16, 32, 64, 128, 256, 512];

    println!("\n");
    println!(
        "┌──────────────────────────────────────────────────────────────────────────────────┐"
    );
    println!(
        "│ 📊 SYNTHÈSE OBJECTIVE (MÉDIANE 5 PASSES) : BUS SPATIAL 2D vs LEGACY              │"
    );
    println!(
        "├──────────────┬──────────────────────┬──────────────────────┬─────────────────────┤"
    );
    println!(
        "│ Voix Actives │ Mode Legacy          │ Bus Spatial 2D       │ Gain / Speedup      │"
    );
    println!(
        "├──────────────┼──────────────────────┼──────────────────────┼─────────────────────┤"
    );

    for &n_voices in &voice_counts {
        let iterations = 2000;
        let passes = 5;

        // Measure Legacy
        let mut legacy_times = Vec::with_capacity(passes);
        for _ in 0..passes {
            let mut dsp_legacy = create_bench_dsp(n_voices, block_size, false);
            let profiler = Profiler::new(10);
            let mut out_buf = vec![0.0f32; block_size * 2];
            for _ in 0..100 {
                dsp_legacy.process_block(&mut out_buf, 1.0, &profiler);
            }
            let start = Instant::now();
            for _ in 0..iterations {
                dsp_legacy.process_block(&mut out_buf, 1.0, &profiler);
                black_box(&out_buf);
            }
            legacy_times.push(start.elapsed() / iterations as u32);
        }
        legacy_times.sort();
        let dur_legacy = legacy_times[passes / 2];

        // Measure Spatial Bus
        let mut bus_times = Vec::with_capacity(passes);
        for _ in 0..passes {
            let mut dsp_bus = create_bench_dsp(n_voices, block_size, true);
            let profiler = Profiler::new(10);
            let mut out_buf = vec![0.0f32; block_size * 2];
            for _ in 0..100 {
                dsp_bus.process_block(&mut out_buf, 1.0, &profiler);
            }
            let start = Instant::now();
            for _ in 0..iterations {
                dsp_bus.process_block(&mut out_buf, 1.0, &profiler);
                black_box(&out_buf);
            }
            bus_times.push(start.elapsed() / iterations as u32);
        }
        bus_times.sort();
        let dur_bus = bus_times[passes / 2];

        let time_legacy_us = dur_legacy.as_nanos() as f64 / 1000.0;
        let time_bus_us = dur_bus.as_nanos() as f64 / 1000.0;
        let speedup = time_legacy_us / time_bus_us.max(0.001);

        println!(
            "│ {:^12} │ {:>17.2} µs │ {:>17.2} µs │   [\x1b[1;32m{:>5.2}x speedup\x1b[0m]   │",
            n_voices, time_legacy_us, time_bus_us, speedup
        );
    }
    println!(
        "└──────────────┴──────────────────────┴──────────────────────┴─────────────────────┘\n"
    );
}

criterion_group!(benches, bench_legacy_vs_spatial_bus);

fn main() {
    benches();
    print_summary_table();
}

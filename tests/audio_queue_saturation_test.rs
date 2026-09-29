use fireworks_sim::audio_engine::constants::PLAY_REQUEST_CHANNEL_CAPACITY;
use fireworks_sim::audio_engine::fireworks_audio::FireworksAudio3D;
use fireworks_sim::audio_engine::types::{AudioDebugEvent, FireworksAudioConfig};
use fireworks_sim::audio_engine::AudioEngine;
use fireworks_sim::AudioEngineSettings;
use glam::Vec2;
use std::time::Instant;

fn build_test_engine() -> FireworksAudio3D {
    FireworksAudio3D::new(FireworksAudioConfig {
        rocket_path: "assets/sounds/rocket.wav".into(),
        explosion_path: "assets/sounds/explosion.wav".into(),
        listener_pos: Vec2::ZERO,
        sample_rate: 48000,
        block_size: 512,
        max_voices: 32,
        settings: AudioEngineSettings::default(),
        doppler_receiver: None,
    })
    .expect("Failed to build test audio engine")
}

#[test]
fn test_audio_queue_saturation_1000_requests_non_blocking() {
    let engine = build_test_engine();

    // 1000 requêtes consécutives envoyées sans consommateur actif
    let count = 1000;
    let start = Instant::now();

    for i in 0..count {
        if i % 2 == 0 {
            engine.play_rocket_scheduled(i as u64, Vec2::new(10.0, 20.0), 0.8, 50.0);
        } else {
            engine.play_explosion_scheduled(i as u64, Vec2::new(-10.0, -20.0), 0.9, 100.0);
        }
    }

    let elapsed = start.elapsed();

    // Doit être ultra-rapide (< 50ms) et strictement non-bloquant
    assert!(
        elapsed.as_millis() < 50,
        "Audio queue send took too long: {:?} (expected < 50ms for 1000 requests)",
        elapsed
    );

    // Vérifier les events de debug : le canal de requête (capacité 512) doit être plein
    // Les requêtes excédentaires ont été rejetées avec un event Dropped
    let mut events = Vec::new();
    engine.pop_debug_events(&mut events);

    let mut sent_count = 0;
    let mut dropped_count = 0;

    for event in events {
        match event {
            AudioDebugEvent::Sent { .. } => sent_count += 1,
            AudioDebugEvent::Dropped { .. } => dropped_count += 1,
            _ => {}
        }
    }

    // Le total d'events enregistrés doit correspondre aux 1000 requêtes (Sent + Dropped)
    // Au moins PLAY_REQUEST_CHANNEL_CAPACITY requêtes ont été bufferisées, le reste droppé
    assert_eq!(
        sent_count, count,
        "All requests should trigger a Sent debug event"
    );
    assert_eq!(
        dropped_count,
        count - PLAY_REQUEST_CHANNEL_CAPACITY,
        "Surplus requests must be dropped gracefully without panic"
    );
}

#[test]
fn test_dsp_drain_saturated_queue_no_leak_and_no_nan() {
    use fireworks_sim::audio_engine::dsp_processor::DspProcessor;
    use fireworks_sim::audio_engine::types::Voice;
    use fireworks_sim::profiler::Profiler;
    use std::sync::atomic::AtomicU64;
    use std::sync::Arc;

    let sample_rate = 48000;
    let block_size = 512;
    let (play_tx, play_rx) = crossbeam_channel::bounded(PLAY_REQUEST_CHANNEL_CAPACITY);
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    let dummy_audio = Arc::new(vec![[0.5, 0.5]; 1024]);
    let sample_clock = Arc::new(AtomicU64::new(0));

    let mut dsp = DspProcessor {
        voices: (0..32).map(|_| Voice::new()).collect(),
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: AudioEngineSettings::default(),
        listener_pos: Arc::new(fireworks_sim::audio_engine::types::AtomicVec2::new(
            Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; block_size],
        bus_w: vec![0.0; block_size],
        bus_x: vec![0.0; block_size],
        export_buffer: vec![[0.0; 2]; block_size],
        last_log: Instant::now(),
        log_interval: std::time::Duration::from_secs(4),
        effect_flags: fireworks_sim::audio_engine::effect_flags::AudioEffectFlags::new_all_enabled(
        ),
        spatial_reverb: fireworks_sim::audio_engine::SpatialReverb::new_with_wet(
            sample_rate,
            Arc::new(std::sync::atomic::AtomicU32::new(0)),
        ),
        hrtf_convolver: fireworks_sim::audio_engine::HrtfConvolver::new_default(
            sample_rate,
            block_size,
        ),
        debug_tx: None,
        current_sample_clock: 0,
        sample_clock: sample_clock.clone(),
        pending_requests: Vec::with_capacity(PLAY_REQUEST_CHANNEL_CAPACITY),
    };

    // Saturer la queue jusqu'à capacité
    for i in 0..PLAY_REQUEST_CHANNEL_CAPACITY {
        let req = fireworks_sim::audio_engine::types::PlayRequest {
            data: dummy_audio.clone(),
            fade_in: 10,
            fade_out: 10,
            gain: 0.5,
            filter_a: 0.1,
            sent_at: Instant::now(),
            target_sample: (i as u64) * 10, // Réparties dans le temps
            request_id: i as u64,
            id: i as u64,
            pos: Vec2::ZERO,
            is_dynamic: false,
            sound_type: fireworks_sim::audio_engine::types::AudioSoundType::Rocket,
        };
        let _ = play_tx.send(req);
    }

    let profiler = Profiler::new(10);
    let mut output_buffer = vec![0.0f32; block_size * 2];

    // Traiter plusieurs blocs pour drainer toutes les requêtes
    for _ in 0..20 {
        dsp.process_block(&mut output_buffer, 1.0, &profiler);

        // Vérifier l'absence totale de NaN ou Inf
        for &sample in &output_buffer {
            assert!(
                sample.is_finite(),
                "Output buffer contains non-finite values (NaN or Inf)"
            );
        }
    }

    // Vérifier l'avancement de l'horloge
    assert_eq!(
        sample_clock.load(std::sync::atomic::Ordering::Relaxed),
        20 * block_size as u64
    );
    assert_eq!(dsp.current_sample_clock, 20 * block_size as u64);
}

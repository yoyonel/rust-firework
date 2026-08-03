use crate::audio_engine::SpatialReverb;
use std::f32::consts::PI;

/// Génère un buffer stéréo contenant une onde sinusoïdale pure de fréquence donnée.
fn generate_sine_wave(freq_hz: f32, sample_rate: u32, duration_samples: usize) -> Vec<[f32; 2]> {
    (0..duration_samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let val = (2.0 * PI * freq_hz * t).sin();
            [val, val]
        })
        .collect()
}

#[test]
fn test_phase_continuity_across_block_boundaries() {
    use crate::audio_engine::types::Voice;
    use crate::profiler::Profiler;
    use crate::AudioEngineSettings;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use crate::audio_engine::effect_flags::AudioEffectFlags;
    let sample_rate = 48_000;
    let block_size = 256;
    let total_blocks = 4;
    let total_samples = block_size * total_blocks;

    // 1. Source : Sinusoïde pure à 440 Hz (La fondamental)
    let source_audio = generate_sine_wave(440.0, sample_rate, total_samples + 100);
    let source_arc = Arc::new(source_audio);

    let (_play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();
    let settings = AudioEngineSettings::default();

    // 2. Traitement Mode A : Un seul bloc continu de 1024 échantillons (La Vérité Terrain)
    let mut dsp_ref = super::DspProcessor {
        voices: vec![Voice {
            id: 1,
            active: true,
            data: Some(source_arc.clone()),
            pos: 0.0,
            playback_rate: 1.0,
            is_dynamic: true,
            world_pos: glam::Vec2::new(0.0, 10.0), // Devant le listener
            velocity: glam::Vec2::ZERO,
            fade_in_samples: 0,
            fade_out_samples: 0,
            filter_state: [0.0, 0.0],
            filter_a: 0.0,
            user_gain: 1.0,
            current_gains: [1.0, 1.0],
            target_gains: [1.0, 1.0],
            current_itd: [0.0, 0.0],
            target_itd: [0.0, 0.0],
            request_id: 1,
            sound_type: crate::audio_engine::types::AudioSoundType::Rocket,
        }],
        play_rx: play_rx.clone(),
        doppler_rx: None,
        garbage_tx: garbage_tx.clone(),
        settings: settings.clone(),
        listener_pos: std::sync::Arc::new(crate::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; total_samples],
        bus_w: vec![0.0; 8192],
        bus_x: vec![0.0; 8192],
        export_buffer: Vec::new(),
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags: AudioEffectFlags::new_all_enabled(),
        spatial_reverb: SpatialReverb::new(sample_rate),
        hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sample_rate, total_samples),
        debug_tx: None,
    };

    let profiler = Profiler::new(1000);
    let fx_mask_ref = dsp_ref.effect_flags.load();
    dsp_ref.process_dsp(total_samples, fx_mask_ref, &profiler);
    let reference_output = dsp_ref.acc.clone();

    // 3. Traitement Mode B : 4 blocs successifs de 256 échantillons
    let mut dsp_chunk = super::DspProcessor {
        voices: vec![Voice {
            id: 1,
            active: true,
            data: Some(source_arc),
            pos: 0.0,
            playback_rate: 1.0,
            is_dynamic: true,
            world_pos: glam::Vec2::new(0.0, 10.0),
            velocity: glam::Vec2::ZERO,
            fade_in_samples: 0,
            fade_out_samples: 0,
            filter_state: [0.0, 0.0],
            filter_a: 0.0,
            user_gain: 1.0,
            current_gains: [1.0, 1.0],
            target_gains: [1.0, 1.0],
            current_itd: [0.0, 0.0],
            target_itd: [0.0, 0.0],
            request_id: 1,
            sound_type: crate::audio_engine::types::AudioSoundType::Rocket,
        }],
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings,
        listener_pos: std::sync::Arc::new(crate::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; block_size],
        bus_w: vec![0.0; 8192],
        bus_x: vec![0.0; 8192],
        export_buffer: Vec::new(),
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags: AudioEffectFlags::new_all_enabled(),
        spatial_reverb: SpatialReverb::new(sample_rate),
        hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sample_rate, block_size),
        debug_tx: None,
    };

    let mut chunked_output = Vec::with_capacity(total_samples);
    for _ in 0..total_blocks {
        dsp_chunk.acc.fill([0.0; 2]);
        let fx_mask_chunk = dsp_chunk.effect_flags.load();
        dsp_chunk.process_dsp(block_size, fx_mask_chunk, &profiler);
        chunked_output.extend_from_slice(&dsp_chunk.acc);
    }

    // 4. Vérification de l'équivalence parfaite et de la continuité de phase aux frontières
    for i in 0..total_samples {
        let ref_l = reference_output[i][0];
        let chunk_l = chunked_output[i][0];
        let diff = (ref_l - chunk_l).abs();

        assert!(
            diff < 1e-5,
            "Discontinuité détectée à l'échantillon {} ! Attendu: {}, Obtenu: {}, Diff: {}",
            i,
            ref_l,
            chunk_l,
            diff
        );

        // Vérification spécifique aux frontières de blocs (ex: 255 -> 256)
        if i > 0 && i % block_size == 0 {
            let slope_before = chunked_output[i][0] - chunked_output[i - 1][0];
            let ref_slope = reference_output[i][0] - reference_output[i - 1][0];

            assert!(
                    (slope_before - ref_slope).abs() < 1e-5,
                    "Rupture de pente (Phase Jump) à la frontière du bloc (Index {}) ! La dérivée n'est pas continue.",
                    i
                );
        }
    }
}

#[test]
fn test_dsp_bypass_distance_attenuation() {
    use crate::audio_engine::binaural_processing::calculate_spatial_params_2d;
    use crate::audio_engine::effect_flags::{AudioEffect, DEFAULT_FLAGS};
    use crate::AudioEngineSettings;
    use glam::Vec2;

    let settings = AudioEngineSettings::default();
    let diff = Vec2::new(0.0, 100.0); // Source straight ahead, far away

    // 1. With distance attenuation active
    let params_active = calculate_spatial_params_2d(diff, &settings, DEFAULT_FLAGS);
    assert!(params_active.gain_left < 1.0);
    assert!(params_active.gain_right < 1.0);

    // 2. Without distance attenuation active
    let mut fx_mask = DEFAULT_FLAGS;
    fx_mask &= !(AudioEffect::DistanceAtten as u32);
    let params_bypass = calculate_spatial_params_2d(diff, &settings, fx_mask);
    assert_eq!(params_bypass.gain_left, 1.0);
    assert_eq!(params_bypass.gain_right, 1.0);
}

#[test]
fn test_dsp_bypass_binaural_and_panning() {
    use crate::audio_engine::binaural_processing::calculate_spatial_params_2d;
    use crate::audio_engine::effect_flags::{AudioEffect, DEFAULT_FLAGS};
    use crate::AudioEngineSettings;
    use glam::Vec2;

    let settings = AudioEngineSettings {
        use_binaural: true,
        ..AudioEngineSettings::default()
    };
    let diff = Vec2::new(100.0, 0.0); // Source fully on the right side

    // 1. With Binaural active: left ear should be quieter than right ear
    let params_binaural = calculate_spatial_params_2d(diff, &settings, DEFAULT_FLAGS);
    assert!(params_binaural.gain_left < params_binaural.gain_right);

    // 2. Disable Binaural but keep Panning: left ear should be quieter (standard panning)
    let mut fx_mask = DEFAULT_FLAGS;
    fx_mask &= !(AudioEffect::Binaural as u32);
    let params_pan = calculate_spatial_params_2d(diff, &settings, fx_mask);
    assert!(params_pan.gain_left < params_pan.gain_right);

    // 3. Disable both Binaural and Panning: gains should be equal (flat center mono)
    fx_mask &= !(AudioEffect::Panning as u32);
    let params_bypass = calculate_spatial_params_2d(diff, &settings, fx_mask);
    assert_eq!(params_bypass.gain_left, params_bypass.gain_right);
}

#[test]
fn test_dsp_bypass_doppler() {
    use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
    use crate::audio_engine::types::Voice;
    use crate::profiler::Profiler;
    use crate::AudioEngineSettings;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let sample_rate = 48_000;
    let block_size = 256;
    let source_audio = generate_sine_wave(440.0, sample_rate, block_size);
    let source_arc = Arc::new(source_audio);
    let (_play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    let mut dsp = super::DspProcessor {
        voices: vec![Voice {
            id: 1,
            active: true,
            data: Some(source_arc),
            pos: 0.0,
            playback_rate: 2.0, // Pre-configured moving doppler rate
            is_dynamic: true,
            world_pos: glam::Vec2::new(0.0, 10.0),
            velocity: glam::Vec2::new(0.0, -100.0), // Moving fast toward listener
            fade_in_samples: 0,
            fade_out_samples: 0,
            filter_state: [0.0, 0.0],
            filter_a: 0.0,
            user_gain: 1.0,
            current_gains: [1.0, 1.0],
            target_gains: [1.0, 1.0],
            current_itd: [0.0, 0.0],
            target_itd: [0.0, 0.0],
            request_id: 1,
            sound_type: crate::audio_engine::types::AudioSoundType::Rocket,
        }],
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: AudioEngineSettings::default(),
        listener_pos: std::sync::Arc::new(crate::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; block_size],
        bus_w: Vec::new(),
        bus_x: Vec::new(),
        export_buffer: Vec::new(),
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags: AudioEffectFlags::new_all_enabled(),
        spatial_reverb: SpatialReverb::new(sample_rate),
        hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sample_rate, block_size),
        debug_tx: None,
    };

    // Disable Doppler
    dsp.effect_flags.set(AudioEffect::Doppler, false);
    let fx_mask = dsp.effect_flags.load();

    let profiler = Profiler::new(1000);
    dsp.process_doppler(fx_mask, &profiler);

    // Doppler processed but disabled -> playback_rate should be reset to 1.0
    assert_eq!(dsp.voices[0].playback_rate, 1.0);
}

#[test]
fn test_dsp_bypass_normalization() {
    use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
    use crate::profiler::Profiler;
    use std::time::{Duration, Instant};

    let sample_rate = 48_000;
    let block_size = 8;
    let (_play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    let mut dsp = super::DspProcessor {
        voices: vec![],
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: crate::AudioEngineSettings::default(),
        listener_pos: std::sync::Arc::new(crate::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[1.5, -2.0]; block_size], // Acc holds values that exceed 1.0
        bus_w: Vec::new(),
        bus_x: Vec::new(),
        export_buffer: Vec::new(),
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags: AudioEffectFlags::new_all_enabled(),
        spatial_reverb: SpatialReverb::new(sample_rate),
        hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sample_rate, block_size),
        debug_tx: None,
    };

    let mut output_data = vec![0.0; block_size * 2];
    let profiler = Profiler::new(1000);

    // 1. With Normalization enabled (GainStage & SoftClip)
    let fx_mask = dsp.effect_flags.load();
    dsp.write_cpal_buffer(&mut output_data, block_size, 0.8, fx_mask, &profiler); // global_gain = 0.8
                                                                                  // Soft clipping via tanh + global gain should smoothly compress
    assert!(output_data[0] < 1.0 && output_data[0] > 0.0);
    assert!(output_data[1] > -1.0 && output_data[1] < 0.0);
    assert!((output_data[0] - (1.5_f32 * 0.8).tanh()).abs() < 1e-3);

    // 2. With Normalization disabled
    dsp.effect_flags.set(AudioEffect::Normalization, false);
    let fx_mask_bypass = dsp.effect_flags.load();
    dsp.write_cpal_buffer(&mut output_data, block_size, 0.8, fx_mask_bypass, &profiler);
    // Normalization stage bypassed -> no global gain scaling, raw clamping to [-1.0, 1.0]
    assert_eq!(output_data[0], 1.0);
    assert_eq!(output_data[1], -1.0);
}

#[test]
fn test_audio_effect_flags_set_all() {
    use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};

    let flags = AudioEffectFlags::new_all_enabled();
    // Initially all are enabled
    assert!(flags.is_enabled(AudioEffect::Binaural));
    assert!(flags.is_enabled(AudioEffect::Panning));

    // Disable all
    flags.set_all(false);
    assert!(!flags.is_enabled(AudioEffect::Binaural));
    assert!(!flags.is_enabled(AudioEffect::Panning));
    assert_eq!(flags.load(), 0);

    // Enable all
    flags.set_all(true);
    assert!(flags.is_enabled(AudioEffect::Binaural));
    assert!(flags.is_enabled(AudioEffect::Panning));
}

#[test]
fn test_strict_event_tracking_and_latency() {
    use crate::audio_engine::effect_flags::AudioEffectFlags;
    use crate::audio_engine::types::{AudioDebugEvent, AudioSoundType, Voice};
    use crate::profiler::Profiler;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let sample_rate = 48_000;
    let block_size = 256;
    let source_audio = generate_sine_wave(440.0, sample_rate, block_size);
    let source_arc = Arc::new(source_audio);

    let (play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();
    let (debug_tx, debug_rx) = crossbeam_channel::unbounded();

    let mut dsp = super::DspProcessor {
        voices: vec![Voice::new()], // Only 1 voice slot!
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: crate::AudioEngineSettings::default(),
        listener_pos: std::sync::Arc::new(crate::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; block_size],
        bus_w: Vec::new(),
        bus_x: Vec::new(),
        export_buffer: Vec::new(),
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags: AudioEffectFlags::new_all_enabled(),
        spatial_reverb: SpatialReverb::new(sample_rate),
        hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sample_rate, block_size),
        debug_tx: Some(debug_tx.clone()),
    };

    // 1. Send first play request
    let req1 = crate::audio_engine::types::PlayRequest {
        data: source_arc.clone(),
        fade_in: 0,
        fade_out: 0,
        gain: 1.0,
        filter_a: 0.05,
        sent_at: Instant::now(),
        request_id: 101,
        id: 10,
        pos: glam::Vec2::ZERO,
        is_dynamic: false,
        sound_type: AudioSoundType::Rocket,
    };
    play_tx.send(req1).unwrap();

    // 2. Consume request
    let profiler = Profiler::new(100);
    dsp.consume_requests(&profiler);

    // Check events popped from debug_rx
    let mut events = Vec::new();
    while let Ok(evt) = debug_rx.try_recv() {
        events.push(evt);
    }

    assert_eq!(events.len(), 2);
    match &events[0] {
        AudioDebugEvent::Received { request_id, .. } => assert_eq!(*request_id, 101),
        _ => panic!("Expected Received event"),
    }
    match &events[1] {
        AudioDebugEvent::Started {
            request_id,
            voice_index,
            ..
        } => {
            assert_eq!(*request_id, 101);
            assert_eq!(*voice_index, 0);
        }
        _ => panic!("Expected Started event"),
    }

    // The voice is now active.
    assert!(dsp.voices[0].active);

    // 3. Send second play request (which should be dropped because voice 0 is active and has higher priority)
    let req2 = crate::audio_engine::types::PlayRequest {
        data: source_arc,
        fade_in: 0,
        fade_out: 0,
        gain: 0.5,
        filter_a: 0.05,
        sent_at: Instant::now(),
        request_id: 102,
        id: 20,
        pos: glam::Vec2::ZERO,
        is_dynamic: false,
        sound_type: AudioSoundType::Rocket,
    };
    play_tx.send(req2).unwrap();

    dsp.consume_requests(&profiler);

    events.clear();
    while let Ok(evt) = debug_rx.try_recv() {
        events.push(evt);
    }

    assert_eq!(events.len(), 2);
    match &events[0] {
        AudioDebugEvent::Received { request_id, .. } => assert_eq!(*request_id, 102),
        _ => panic!("Expected Received event"),
    }
    match &events[1] {
        AudioDebugEvent::Dropped {
            request_id, reason, ..
        } => {
            assert_eq!(*request_id, 102);
            assert_eq!(*reason, "No inactive voice available");
        }
        _ => panic!("Expected Dropped event"),
    }
}

#[test]
fn test_spatial_bus_rendering() {
    use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
    use crate::audio_engine::types::Voice;
    use crate::profiler::Profiler;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let sample_rate = 48_000;
    let block_size = 256;
    let source_audio = generate_sine_wave(440.0, sample_rate, block_size);
    let source_arc = Arc::new(source_audio);
    let (_play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    let mut dsp = super::DspProcessor {
        voices: vec![Voice {
            id: 1,
            active: true,
            data: Some(source_arc),
            pos: 0.0,
            playback_rate: 1.0,
            is_dynamic: false,
            world_pos: glam::Vec2::new(50.0, 0.0), // Sound to the right
            velocity: glam::Vec2::ZERO,
            fade_in_samples: 0,
            fade_out_samples: 0,
            filter_state: [0.0, 0.0],
            filter_a: 0.0,
            user_gain: 1.0,
            current_gains: [1.0, 1.0],
            target_gains: [1.0, 1.0],
            current_itd: [0.0, 0.0],
            target_itd: [0.0, 0.0],
            request_id: 1,
            sound_type: crate::audio_engine::types::AudioSoundType::Rocket,
        }],
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: crate::AudioEngineSettings::default(),
        listener_pos: std::sync::Arc::new(crate::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; block_size],
        bus_w: vec![0.0; 8192],
        bus_x: vec![0.0; 8192],
        export_buffer: Vec::new(),
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags: AudioEffectFlags::new_all_enabled(),
        spatial_reverb: SpatialReverb::new(sample_rate),
        hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sample_rate, block_size),
        debug_tx: None,
    };

    // Enable SpatialBus effect
    dsp.effect_flags.set(AudioEffect::SpatialBus, true);
    let fx_mask = dsp.effect_flags.load();

    let profiler = Profiler::new(100);
    dsp.process_dsp(block_size, fx_mask, &profiler);

    // Right channel should be louder than left channel for a sound at (50, 0)
    let sample_l = dsp.acc[10][0];
    let sample_r = dsp.acc[10][1];
    assert!(
        sample_r.abs() > sample_l.abs(),
        "Right ear should be louder for source on the right in SpatialBus mode"
    );
}

#[test]
fn test_spatial_bus_hrtf_rendering_left_right() {
    use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
    use crate::audio_engine::types::Voice;
    use crate::profiler::Profiler;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let sample_rate = 48_000;
    let block_size = 256;
    let source_audio = generate_sine_wave(440.0, sample_rate, block_size);
    let source_arc = Arc::new(source_audio);
    let (_play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    let mut dsp = super::DspProcessor {
        voices: vec![Voice {
            id: 1,
            active: true,
            data: Some(source_arc),
            pos: 0.0,
            playback_rate: 1.0,
            is_dynamic: false,
            world_pos: glam::Vec2::new(-50.0, 0.0), // Sound to the LEFT
            velocity: glam::Vec2::ZERO,
            fade_in_samples: 0,
            fade_out_samples: 0,
            filter_state: [0.0, 0.0],
            filter_a: 0.0,
            user_gain: 1.0,
            current_gains: [1.0, 1.0],
            target_gains: [1.0, 1.0],
            current_itd: [0.0, 0.0],
            target_itd: [0.0, 0.0],
            request_id: 1,
            sound_type: crate::audio_engine::types::AudioSoundType::Rocket,
        }],
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: crate::AudioEngineSettings::default(),
        listener_pos: std::sync::Arc::new(crate::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; block_size],
        bus_w: vec![0.0; 8192],
        bus_x: vec![0.0; 8192],
        export_buffer: Vec::new(),
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags: AudioEffectFlags::new_all_enabled(),
        spatial_reverb: SpatialReverb::new(sample_rate),
        hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sample_rate, block_size),
        debug_tx: None,
    };

    // Enable SpatialBus AND HrtfBus
    dsp.effect_flags.set(AudioEffect::SpatialBus, true);
    dsp.effect_flags.set(AudioEffect::HrtfBus, true);
    let fx_mask = dsp.effect_flags.load();

    let profiler = Profiler::new(100);
    dsp.process_dsp(block_size, fx_mask, &profiler);

    // Left ear should receive higher amplitude than right ear for sound at (-50, 0)
    let total_energy_l: f32 = dsp.acc.iter().map(|s| s[0].powi(2)).sum();
    let total_energy_r: f32 = dsp.acc.iter().map(|s| s[1].powi(2)).sum();

    assert!(
        total_energy_l > total_energy_r,
        "Left ear energy ({}) should exceed right ear energy ({}) for left source in HRTF bus mode",
        total_energy_l,
        total_energy_r
    );
}

#[test]
fn test_spatial_bus_snr_quality() {
    use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
    use crate::audio_engine::types::Voice;
    use crate::profiler::Profiler;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let sample_rate = 48_000;
    let block_size = 256;
    let n_voices = 16;
    let source_audio = generate_sine_wave(440.0, sample_rate, 48_000);
    let source_arc = Arc::new(source_audio);
    let (_play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    let voices: Vec<Voice> = (0..n_voices)
        .map(|i| {
            let angle = i as f32 * 2.399;
            let scale = 10.0 + (i as f32 / n_voices as f32) * 100.0;
            Voice {
                id: i as u64 + 1,
                active: true,
                data: Some(source_arc.clone()),
                pos: (i * 37) as f64 % 4000.0,
                playback_rate: 1.0,
                is_dynamic: false,
                world_pos: glam::Vec2::new(angle.cos() * scale, angle.sin() * scale),
                velocity: glam::Vec2::ZERO,
                fade_in_samples: 0,
                fade_out_samples: 0,
                filter_state: [0.0, 0.0],
                filter_a: 0.0,
                user_gain: 0.8,
                current_gains: [0.5, 0.5],
                target_gains: [0.5, 0.5],
                current_itd: [0.0, 0.0],
                target_itd: [0.0, 0.0],
                request_id: i as u64 + 1,
                sound_type: crate::audio_engine::types::AudioSoundType::Explosion,
            }
        })
        .collect();

    let effect_flags = AudioEffectFlags::new_all_enabled();
    effect_flags.set(AudioEffect::SpatialBus, true);
    effect_flags.set(AudioEffect::HrtfBus, false); // Stereo decode mode

    let mut dsp = super::DspProcessor {
        voices,
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: crate::AudioEngineSettings::default(),
        listener_pos: Arc::new(crate::audio_engine::types::AtomicVec2::new(
            glam::Vec2::ZERO,
        )),
        sample_rate,
        export_writer: None,
        block_index: 0,
        acc: vec![[0.0; 2]; block_size],
        bus_w: vec![0.0; block_size],
        bus_x: vec![0.0; block_size],
        export_buffer: Vec::new(),
        last_log: Instant::now(),
        log_interval: Duration::from_secs(1),
        effect_flags,
        spatial_reverb: SpatialReverb::new(sample_rate),
        hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sample_rate, block_size),
        debug_tx: None,
    };

    let profiler = Profiler::new(100);
    let fx_mask = dsp.effect_flags.load();
    dsp.process_dsp(block_size, fx_mask, &profiler);

    // Save initial output as reference for SNR comparison
    let ref_out = dsp.acc.clone();

    // Verify SNR against reference render:
    // SNR = 10 * log10( sum(S_ref^2) / sum((S_ref - S_opt)^2) )
    let mut sum_ref_sq = 0.0f64;
    let mut sum_err_sq = 0.0f64;

    for (i, ref_frame) in ref_out.iter().enumerate().take(block_size) {
        for (ch, &s_ref_val) in ref_frame.iter().enumerate() {
            let s_ref = s_ref_val as f64;
            let s_opt = dsp.acc[i][ch] as f64;
            let err = s_ref - s_opt;
            sum_ref_sq += s_ref * s_ref;
            sum_err_sq += err * err;
        }
    }

    let snr_db = if sum_err_sq < 1e-20 {
        200.0 // Infinite SNR -> pass
    } else {
        10.0 * (sum_ref_sq / sum_err_sq).log10()
    };

    assert!(
        snr_db > 100.0,
        "Signal-to-Noise Ratio (SNR) must be > 100 dB, got {:.2} dB",
        snr_db
    );
}

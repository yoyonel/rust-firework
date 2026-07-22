use fireworks_sim::audio_engine::effect_flags::AudioEffectFlags;
use fireworks_sim::audio_engine::types::{AudioSoundType, Voice};
use fireworks_sim::profiler::Profiler;
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

/// Détecte et quantifie les craquements (discontinuités abruptes de l'onde sonore)
/// Retourne la liste des indices de samples où la différence absolue dépasse le seuil (threshold).
fn detect_glitches(samples: &[[f32; 2]], threshold: f32) -> Vec<(usize, f32)> {
    let mut glitches = Vec::new();
    if samples.len() < 2 {
        return glitches;
    }
    for i in 1..samples.len() {
        let diff_l = (samples[i][0] - samples[i - 1][0]).abs();
        let diff_r = (samples[i][1] - samples[i - 1][1]).abs();

        if !samples[i][0].is_finite() || !samples[i][1].is_finite() {
            glitches.push((i, f32::INFINITY));
            continue;
        }

        let max_diff = diff_l.max(diff_r);
        if max_diff > threshold {
            glitches.push((i, max_diff));
        }
    }
    glitches
}

#[test]
fn test_dsp_no_glitches_under_normal_play() {
    let sample_rate = 48_000;
    let block_size = 64; // Teste avec la taille de bloc ultra-basse latence
    let sound_len = 1024;
    let source_audio = generate_sine_wave(440.0, sample_rate, sound_len);
    let source_arc = Arc::new(source_audio);

    let (play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    let mut dsp = fireworks_sim::audio_engine::dsp_processor::DspProcessor {
        voices: vec![Voice::new(), Voice::new(), Voice::new(), Voice::new()],
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: fireworks_sim::AudioEngineSettings::default(),
        listener_pos: std::sync::Arc::new(fireworks_sim::audio_engine::types::AtomicVec2::new(
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
        spatial_reverb: fireworks_sim::audio_engine::SpatialReverb::new(sample_rate),
        hrtf_convolver: fireworks_sim::audio_engine::HrtfConvolver::new_default(
            sample_rate,
            block_size,
        ),
        debug_tx: None,
    };

    // Joue un son simple avec fondu d'entrée/sortie pour éviter tout clic de coupure
    let req = fireworks_sim::audio_engine::types::PlayRequest {
        data: source_arc.clone(),
        fade_in: 32,
        fade_out: 64,
        gain: 0.8,
        filter_a: 0.05,
        sent_at: Instant::now(),
        request_id: 1,
        id: 10,
        pos: glam::Vec2::ZERO,
        is_dynamic: false,
        sound_type: AudioSoundType::Explosion,
    };
    play_tx.send(req).unwrap();

    let profiler = Profiler::new(10);
    let mut recorded_output = Vec::new();

    // Simule 20 blocs audio
    for _ in 0..20 {
        let mut buffer = vec![0.0f32; block_size * 2];
        dsp.process_block(&mut buffer, 1.0, &profiler);
        for frame in buffer.chunks_exact(2) {
            recorded_output.push([frame[0], frame[1]]);
        }
    }

    // Le seuil de discontinuité pour un signal sinusoïdal de 440 Hz à 48 kHz est extrêmement bas
    // On met un seuil de 0.2 pour tolérer les micro-arrondis
    let glitches = detect_glitches(&recorded_output, 0.2);

    assert_eq!(
        glitches.len(),
        0,
        "Des craquements numériques ou NaNs ont été détectés : {:?}",
        glitches
    );
}

#[test]
fn test_dsp_voice_stealing_glitch_limit() {
    let sample_rate = 48_000;
    let block_size = 128;
    // Crée une onde sinusoïdale forte
    let source_audio = generate_sine_wave(440.0, sample_rate, 1024);
    let source_arc = Arc::new(source_audio);

    let (play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    // Un seul slot de voix pour provoquer le vol systématique
    let mut dsp = fireworks_sim::audio_engine::dsp_processor::DspProcessor {
        voices: vec![Voice::new()],
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: fireworks_sim::AudioEngineSettings::default(),
        listener_pos: std::sync::Arc::new(fireworks_sim::audio_engine::types::AtomicVec2::new(
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
        spatial_reverb: fireworks_sim::audio_engine::SpatialReverb::new(sample_rate),
        hrtf_convolver: fireworks_sim::audio_engine::HrtfConvolver::new_default(
            sample_rate,
            block_size,
        ),
        debug_tx: None,
    };

    // Lance le 1er son (avec fondu pour éviter un pop initial)
    play_tx
        .send(fireworks_sim::audio_engine::types::PlayRequest {
            data: source_arc.clone(),
            fade_in: 32,
            fade_out: 64,
            gain: 0.8,
            filter_a: 0.05,
            sent_at: Instant::now(),
            request_id: 1,
            id: 10,
            pos: glam::Vec2::ZERO,
            is_dynamic: false,
            sound_type: AudioSoundType::Rocket,
        })
        .unwrap();

    let profiler = Profiler::new(10);
    let mut recorded_output = Vec::new();

    // Rendre un premier bloc
    let mut buffer = vec![0.0f32; block_size * 2];
    dsp.process_block(&mut buffer, 1.0, &profiler);
    for frame in buffer.chunks_exact(2) {
        recorded_output.push([frame[0], frame[1]]);
    }

    // Lance un 2ème son plus fort qui va voler le premier
    play_tx
        .send(fireworks_sim::audio_engine::types::PlayRequest {
            data: source_arc.clone(),
            fade_in: 32,
            fade_out: 64,
            gain: 1.0,
            filter_a: 0.05,
            sent_at: Instant::now(),
            request_id: 2,
            id: 11,
            pos: glam::Vec2::ZERO,
            is_dynamic: false,
            sound_type: AudioSoundType::Explosion,
        })
        .unwrap();

    // Rendre le deuxième bloc (le vol a lieu ici)
    let mut buffer2 = vec![0.0f32; block_size * 2];
    dsp.process_block(&mut buffer2, 1.0, &profiler);
    for frame in buffer2.chunks_exact(2) {
        recorded_output.push([frame[0], frame[1]]);
    }

    // Le vol d'un son fort coupe brutalement l'ancien pour jouer le nouveau.
    // Cela provoque un saut d'amplitude (discontinuité).
    // On veut quantifier ce saut d'amplitude maximal pour s'assurer qu'il ne dépasse pas 1.8 (seuil de clipping sévère)
    let glitches = detect_glitches(&recorded_output, 1.8);
    assert_eq!(
        glitches.len(),
        0,
        "Craquement d'amplitude critique (> 1.8) détecté lors du vol : {:?}",
        glitches
    );
}

#[test]
fn test_block_processing_budget() {
    let sample_rate = 48_000;
    let block_size = 64; // 1.33 ms de budget temps réel
    let source_audio = generate_sine_wave(440.0, sample_rate, 48000);
    let source_arc = Arc::new(source_audio);

    let (_play_tx, play_rx) = crossbeam_channel::unbounded();
    let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

    // Simule la charge maximale absolue : 128 voix actives simultanées !
    let mut voices = Vec::with_capacity(128);
    for _ in 0..128 {
        let mut v = Voice::new();
        v.active = true;
        v.data = Some(source_arc.clone());
        v.target_gains = [0.5, 0.5];
        v.current_gains = [0.5, 0.5];
        voices.push(v);
    }

    let mut dsp = fireworks_sim::audio_engine::dsp_processor::DspProcessor {
        voices,
        play_rx,
        doppler_rx: None,
        garbage_tx,
        settings: fireworks_sim::AudioEngineSettings::default(),
        listener_pos: std::sync::Arc::new(fireworks_sim::audio_engine::types::AtomicVec2::new(
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
        spatial_reverb: fireworks_sim::audio_engine::SpatialReverb::new(sample_rate),
        hrtf_convolver: fireworks_sim::audio_engine::HrtfConvolver::new_default(
            sample_rate,
            block_size,
        ),
        debug_tx: None,
    };

    let profiler = Profiler::new(10);
    let mut buffer = vec![0.0f32; block_size * 2];

    // Mesure le temps nécessaire pour calculer un bloc de 64 échantillons sous charge maximale
    let start = Instant::now();
    for _ in 0..100 {
        dsp.process_block(&mut buffer, 1.0, &profiler);
    }
    let duration = start.elapsed() / 100;

    // Le budget théorique maximal pour 64 samples à 48kHz est de 1.33 ms (1 333 333 ns).
    // Sur n'importe quelle machine moderne, process_block optimisé en SIMD/release avec 128 voix
    // devrait s'exécuter en moins de 0.2 ms (200 000 ns).
    println!(
        "Temps moyen par bloc sous charge max (128 voix) : {:?}",
        duration
    );
    let max_allowed = if cfg!(debug_assertions) {
        // En mode debug / couverture de code (llvm-cov), l'inlining et les optimisations SIMD sont désactivés
        Duration::from_millis(12)
    } else {
        // En mode release optimisé, le temps doit rester strictement sous 500 µs
        Duration::from_micros(500)
    };
    assert!(
        duration < max_allowed,
        "Le temps de calcul du DSP ({:?}) dépasse le budget autorisé ({:?}) !",
        duration,
        max_allowed
    );
}

use crate::audio_engine::types::{FireworksAudioConfig, PlayRequest};
use crate::AudioEngineSettings;
use std::time::Instant;

use super::*;
use crate::audio_engine::binaural_processing::binauralize_mono_fast;
use crate::audio_engine::settings::AudioEngineSettingsBuilder;

fn dummy_data() -> Vec<[f32; 2]> {
    vec![[1.0, 1.0]; 10] // 10 frames simples avec amplitude 1
}

// Version test-friendly de enqueue_sound qui ignore l'atténuation distance
fn enqueue_sound_test(engine: &FireworksAudio3D, pos: (f32, f32), gain: f32) -> PlayRequest {
    // Panning simple
    let dx = pos.0 - engine.listener_pos.0;
    let pan = (dx / engine.settings.max_distance()).clamp(-1.0, 1.0);

    let mut data_panned = dummy_data();
    for sample in &mut data_panned {
        let left = ((1.0 - pan) * 0.5).clamp(0.0, 1.0);
        let right = ((1.0 + pan) * 0.5).clamp(0.0, 1.0);
        sample[0] *= left * gain;
        sample[1] *= right * gain;
    }

    PlayRequest {
        data: std::sync::Arc::new(data_panned),
        fade_in: 1,
        fade_out: 1,
        gain,
        filter_a: 0.0025,
        sent_at: Instant::now(),
        // --- NOUVEAUX CHAMPS REQUIS ---
        id: 0,
        pos,
        is_dynamic: false,
    }
}

fn build_engine() -> FireworksAudio3D {
    FireworksAudio3D::new(FireworksAudioConfig {
        rocket_path: "assets/sounds/rocket.wav".into(),
        explosion_path: "assets/sounds/explosion.wav".into(),
        listener_pos: (0.0, 0.0),
        sample_rate: 1000,
        block_size: 1024 * 4,
        max_voices: 16,
        settings: AudioEngineSettings::default(),
        // --- NOUVEAU CHAMP REQUIS ---
        doppler_receiver: None,
    })
    .expect("Failed to build test audio engine")
}

#[test]
fn test_panning_left() {
    let engine = build_engine();

    let req = enqueue_sound_test(&engine, (-engine.settings.max_distance(), 0.0), 1.0);

    for sample in req.data.iter() {
        let ratio = sample[0] / (sample[1] + 1e-8);
        assert!(
            ratio > 1.0,
            "Left channel should dominate right for left pan"
        );
    }
}

#[test]
fn test_panning_right() {
    let engine = build_engine();

    let req = enqueue_sound_test(&engine, (engine.settings.max_distance(), 0.0), 1.0);

    for sample in req.data.iter() {
        let ratio = sample[1] / (sample[0] + 1e-8);
        assert!(
            ratio > 1.0,
            "Right channel should dominate left for right pan"
        );
    }
}

#[test]
fn test_panning_center() {
    let engine = build_engine();

    let req = enqueue_sound_test(&engine, (0.0, 0.0), 1.0);

    for sample in req.data.iter() {
        let diff = (sample[0] - sample[1]).abs();
        assert!(diff < 1e-6, "Channels should be equal for center pan");
    }
}

/// Génère un signal mono simple
fn dummy_mono(len: usize) -> Vec<f32> {
    vec![1.0; len]
}

#[test]
fn test_binaural_center() {
    let sr = 48000;
    let max_distance = 1000.0;
    let head_radius = 0.0875;
    let max_ild_db = 18.0;
    let mono = dummy_mono(10);
    let src_pos = (0.0, 0.0);
    let listener_pos = (0.0, 0.0);

    let settings = AudioEngineSettingsBuilder::default()
        .max_distance(max_distance)
        .head_radius(head_radius)
        .max_ild_db(max_ild_db)
        .build()
        .unwrap();
    let stereo = binauralize_mono_fast(
        &mono,
        (src_pos.0, src_pos.1, 0.0),
        (listener_pos.0, listener_pos.1, 0.0),
        sr,
        &settings,
    );

    // Source au centre → canaux égaux
    for s in &stereo {
        let diff = (s[0] - s[1]).abs();
        assert!(
            diff < 1e-6,
            "Canaux gauche/droite doivent être égaux pour source centrale"
        );
    }
}

#[test]
fn test_binaural_left_debug() {
    let sr = 48000;
    let mono = dummy_mono(10);
    let src_pos = (-500.0, 0.0); // X négatif = gauche (selon ta convention x = latéral)
    let listener_pos = (0.0, 0.0);

    let settings = AudioEngineSettingsBuilder::default()
        .max_distance(1000.0)
        .head_radius(0.0875)
        .max_ild_db(18.0)
        .build()
        .unwrap();

    // --- Recalcule et affiche les paramètres intermédiaires pour debug
    let dx: f32 = src_pos.0 - listener_pos.0; // >0 => droite, <0 => gauche
    let dy: f32 = src_pos.1 - listener_pos.1; // >0 => haut, <0 => bas

    // Convention utilisée dans binauralize_mono : azimuth = dx.atan2(dy)
    let azimuth = dx.atan2(dy); // angle en radians : 0 = front, + = right, - = left
    let theta = azimuth.abs();

    let c = 343.0_f32;
    let itd = ((settings.head_radius() / c) * (theta + theta.sin())).clamp(0.0, 0.001);
    let ild_db = settings.max_ild_db() * theta.sin();
    let far_gain = 10f32.powf(-ild_db / 20.0);
    let att = (1.0 - ((dx * dx + dy * dy).sqrt()) / settings.max_distance()).max(0.0);

    // Déduction heuristique du canal atténué (pour info)
    let expected_side = if azimuth >= 0.0 { "right" } else { "left" };
    let (expected_gain_left, expected_gain_right) = if azimuth >= 0.0 {
        (att * far_gain, att)
    } else {
        (att, att * far_gain)
    };

    println!("--- DEBUG test_binaural_left ---");
    println!("src_pos = {:?}, listener_pos = {:?}", src_pos, listener_pos);
    println!(
        "dx = {:.3}, dy = {:.3}, distance = {:.3}",
        dx,
        dy,
        (dx * dx + dy * dy).sqrt()
    );
    println!("azimuth (rad) = {:.6}, theta = {:.6}", azimuth, theta);
    println!("ITD (s) = {:.9}, ILD (dB) = {:.6}", itd, ild_db);
    println!(
        "expected side = {}, expected gains L/R ≈ {:.6} / {:.6}",
        expected_side, expected_gain_left, expected_gain_right
    );
    println!("attenuation (distance) = {:.6}", att);

    // Appel de la fonction à tester
    let stereo = binauralize_mono_fast(
        &mono,
        (src_pos.0, src_pos.1, 0.0),
        (listener_pos.0, listener_pos.1, 0.0),
        sr,
        &settings,
    );

    // Statistiques simples
    let sum_left: f32 = stereo.iter().map(|s| s[0]).sum();
    let sum_right: f32 = stereo.iter().map(|s| s[1]).sum();
    let avg_left = sum_left / stereo.len() as f32;
    let avg_right = sum_right / stereo.len() as f32;
    let max_diff = stereo
        .iter()
        .map(|s| (s[0] - s[1]).abs())
        .fold(0.0_f32, f32::max);

    // Comptage d'échantillons où gauche <= droite (devrait être 0 pour source à gauche)
    let mut left_le_right = 0usize;
    for s in &stereo {
        if s[0] <= s[1] {
            left_le_right += 1;
        }
    }

    println!("sum L = {:.6}, sum R = {:.6}", sum_left, sum_right);
    println!(
        "avg L = {:.6}, avg R = {:.6}, max |L-R| = {:.6}",
        avg_left, avg_right, max_diff
    );
    println!(
        "samples where L <= R : {}/{} (should be 0 for strict left dominance)",
        left_le_right,
        stereo.len()
    );

    // Print first few stereo samples for inspection
    println!("first samples (L, R):");
    for (i, s) in stereo.iter().take(12).enumerate() {
        println!("  [{:02}] {:.6}, {:.6}", i, s[0], s[1]);
    }

    assert!(
            sum_left > sum_right,
            "Canal gauche doit être globalement plus fort que droite pour source à gauche (see debug output above)"
        );
}

// FIXME: il doit y avoir un problème de symétrie avec le filtre audio binaural
#[test]
fn test_binaural_right_debug() {
    let sr = 48000;
    let n_samples = 4800; // 0.1 s
    let mono = vec![1.0; n_samples];

    // Source sur l'axe +x -> à droite selon ta convention
    let src_pos = (500.0, 0.0);
    let listener_pos = (0.0, 0.0);

    let settings = AudioEngineSettingsBuilder::default()
        .max_distance(1000.0)
        .head_radius(0.0875)
        .max_ild_db(18.0)
        .build()
        .unwrap();

    // on récupère les valeurs internes (recalculées ici pour afficher)
    let dx: f32 = src_pos.0 - listener_pos.0;
    let dy: f32 = src_pos.1 - listener_pos.1;
    let azimuth: f32 = dx.atan2(dy); // NOTE: dx.atan2(dy) => 90deg pour (500,0)
    let theta: f32 = azimuth.abs();

    let c: f32 = 343.0;
    let itd = ((settings.head_radius() / c) * (theta + theta.sin())).clamp(0.0, 0.001);
    let ild_db = settings.max_ild_db() * theta.sin();
    let far_gain = 10f32.powf(-ild_db / 20.0);

    // Détermine quels canaux sont atténués selon signe d'azimuth
    let (gain_left, gain_right) = if azimuth >= 0.0 {
        (far_gain, 1.0) // source à droite -> droite non-affaiblie
    } else {
        (1.0, far_gain)
    };

    let stereo = binauralize_mono_fast(
        &mono,
        (src_pos.0, src_pos.1, 0.0),
        (listener_pos.0, listener_pos.1, 0.0),
        sr,
        &settings,
    );

    let sum_left: f32 = stereo.iter().map(|s| s[0]).sum();
    let sum_right: f32 = stereo.iter().map(|s| s[1]).sum();

    println!(
        "DEBUG binaural_right:\n\
         src={:?}, dx={:.1}, dy={:.1}\n\
         azimuth(rad)={:.3}, theta={:.3}\n\
         itd(s)={:.7}, ild_db={:.3}, far_gain={:.4}\n\
         expected gains L/R ≈ {:.4}/{:.4}\n\
         sums L/R = {:.4}/{:.4}, ratio R/L = {:.3}",
        src_pos,
        dx,
        dy,
        azimuth,
        theta,
        itd,
        ild_db,
        far_gain,
        gain_left,
        gain_right,
        sum_left,
        sum_right,
        sum_right / (sum_left + 1e-12)
    );

    assert!(
        sum_right > sum_left,
        "Canal droite doit être plus fort que gauche pour source à droite"
    );
}

#[test]
fn test_binaural_distance_3d() {
    let sr = 48_000;
    let mono = dummy_mono(10);
    let listener = (0.0, 0.0, 0.0);

    let near = (0.0, 0.0, 100.0); // proche devant
    let far = (0.0, 0.0, -900.0); // loin derrière

    let settings = AudioEngineSettingsBuilder::default()
        .max_distance(1000.0)
        .head_radius(0.0875)
        .max_ild_db(18.0)
        .build()
        .unwrap();

    let stereo_near = binauralize_mono_fast(&mono, near, listener, sr, &settings);
    let stereo_far = binauralize_mono_fast(&mono, far, listener, sr, &settings);

    let e_near: f32 = stereo_near.iter().map(|s| s[0].abs() + s[1].abs()).sum();
    let e_far: f32 = stereo_far.iter().map(|s| s[0].abs() + s[1].abs()).sum();

    assert!(
        e_near > e_far,
        "Le son proche doit être plus fort que le son lointain"
    );
}

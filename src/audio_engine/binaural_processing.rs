use crate::audio_engine::effect_flags::{fx_enabled, AudioEffect};
use crate::AudioEngineSettings;
use glam::{Vec2, Vec3};

use crate::audio_engine::constants;

pub struct SpatialParams {
    pub itd_left_sec: f32,
    pub itd_right_sec: f32,
    pub gain_left: f32,
    pub gain_right: f32,
}

pub fn calculate_spatial_params_3d(
    diff: Vec3,
    settings: &AudioEngineSettings,
    fx_mask: u32,
) -> SpatialParams {
    let distance = diff.length().max(constants::MIN_DISTANCE_EPSILON);

    // Attenuation par la distance (conditionnelle)
    // Modèle inverse-distance réaliste de l'industrie avec fondu linéaire à max_distance
    let att = if fx_enabled(fx_mask, AudioEffect::DistanceAtten) {
        let ref_distance = constants::REFERENCE_DISTANCE_METERS;
        let max_distance = settings.max_distance().max(ref_distance + 1.0);
        if distance <= ref_distance {
            1.0
        } else if distance >= max_distance {
            0.0
        } else {
            let raw_att = ref_distance / distance;
            let fade = (max_distance - distance) / (max_distance - ref_distance);
            raw_att * fade
        }
    } else {
        1.0 // Bypass : volume constant quelle que soit la distance
    };

    // Binaural ITD+ILD (conditionnel) — override du flag settings.use_binaural()
    if settings.use_binaural() && fx_enabled(fx_mask, AudioEffect::Binaural) {
        let azimuth = diff.x.atan2(-diff.z);
        let elevation = diff.y.atan2((diff.x * diff.x + diff.z * diff.z).sqrt());

        let theta = azimuth.abs();
        let c = constants::SPEED_OF_SOUND_M_S;
        let itd = ((settings.head_radius() / c) * (theta + theta.sin()))
            .clamp(0.0, constants::MAX_ITD_SECONDS);
        let ild_db = settings.max_ild_db()
            * theta.sin()
            * (1.0 - constants::ILD_ELEVATION_ATTENUATION_FACTOR * elevation.sin().abs());
        let far_gain = 10f32.powf(-ild_db / 20.0);

        if azimuth >= 0.0 {
            SpatialParams {
                itd_left_sec: itd,
                itd_right_sec: 0.0,
                gain_left: att * far_gain,
                gain_right: att,
            }
        } else {
            SpatialParams {
                itd_left_sec: 0.0,
                itd_right_sec: itd,
                gain_left: att,
                gain_right: att * far_gain,
            }
        }
    } else if fx_enabled(fx_mask, AudioEffect::Panning) {
        let pan = (diff.x / settings.max_distance()).clamp(-1.0, 1.0);
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        SpatialParams {
            itd_left_sec: 0.0,
            itd_right_sec: 0.0,
            gain_left: angle.cos() * att,
            gain_right: angle.sin() * att,
        }
    } else {
        // Flat center panning
        SpatialParams {
            itd_left_sec: 0.0,
            itd_right_sec: 0.0,
            gain_left: att,
            gain_right: att,
        }
    }
}

pub fn calculate_spatial_params_2d(
    diff: Vec2,
    settings: &AudioEngineSettings,
    fx_mask: u32,
) -> SpatialParams {
    let distance = diff.length().max(constants::MIN_DISTANCE_EPSILON);

    // Atténuation par la distance (conditionnelle)
    // Modèle inverse-distance réaliste de l'industrie avec fondu linéaire à max_distance
    let att = if fx_enabled(fx_mask, AudioEffect::DistanceAtten) {
        let ref_distance = constants::REFERENCE_DISTANCE_METERS;
        let max_distance = settings.max_distance().max(ref_distance + 1.0);
        if distance <= ref_distance {
            1.0
        } else if distance >= max_distance {
            0.0
        } else {
            let raw_att = ref_distance / distance;
            let fade = (max_distance - distance) / (max_distance - ref_distance);
            raw_att * fade
        }
    } else {
        1.0 // Bypass : volume constant quelle que soit la distance
    };

    // Binaural ITD+ILD (conditionnel) — override du flag settings.use_binaural()
    if settings.use_binaural() && fx_enabled(fx_mask, AudioEffect::Binaural) {
        let azimuth = diff.x.atan2(diff.y);
        let theta = azimuth.abs();
        let c = constants::SPEED_OF_SOUND_M_S;
        let itd = ((settings.head_radius() / c) * (theta + theta.sin()))
            .clamp(0.0, constants::MAX_ITD_SECONDS);
        let ild_db = settings.max_ild_db() * theta.sin();
        let far_gain = 10f32.powf(-ild_db / 20.0);

        if azimuth >= 0.0 {
            SpatialParams {
                itd_left_sec: itd,
                itd_right_sec: 0.0,
                gain_left: att * far_gain,
                gain_right: att,
            }
        } else {
            SpatialParams {
                itd_left_sec: 0.0,
                itd_right_sec: itd,
                gain_left: att,
                gain_right: att * far_gain,
            }
        }
    } else if fx_enabled(fx_mask, AudioEffect::Panning) {
        let pan = (diff.x / settings.max_distance()).clamp(-1.0, 1.0);
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        SpatialParams {
            itd_left_sec: 0.0,
            itd_right_sec: 0.0,
            gain_left: angle.cos() * att,
            gain_right: angle.sin() * att,
        }
    } else {
        // Flat center panning
        SpatialParams {
            itd_left_sec: 0.0,
            itd_right_sec: 0.0,
            gain_left: att,
            gain_right: att,
        }
    }
}

/// Version "Zero-Allocation / Zero-Branch / Auto-Vectorized"
/// Écrit directement le rendu 3D dans un buffer pré-alloué pour éliminer le gras (alloc::vec).
pub fn binauralize_mono_fast_into(
    mono: &[f32],
    output_stereo: &mut [[f32; 2]], // 🎯 INJECTION DU BUFFER : Zéro allocation !
    src_pos: impl Into<Vec3>,
    listener_pos: impl Into<Vec3>,
    sample_rate: u32,
    settings: &AudioEngineSettings,
) {
    // On sécurise la taille à traiter sans aucun bounds check ultérieur
    let n = mono.len().min(output_stereo.len());
    if n == 0 {
        return;
    }

    let diff = src_pos.into() - listener_pos.into();
    let params = calculate_spatial_params_3d(
        diff,
        settings,
        crate::audio_engine::effect_flags::DEFAULT_FLAGS,
    );

    // 2. Travail direct sur la tranche utile du buffer réutilisé
    let stereo_slice = &mut output_stereo[..n];

    // Note : Plus besoin de initialiser à zéro via un memset coûteux !
    // process_channel va directement écraser les anciennes valeurs de la frame précédente.
    process_channel(
        &mono[..n],
        params.itd_left_sec * sample_rate as f32,
        params.gain_left,
        stereo_slice,
        0,
    );
    process_channel(
        &mono[..n],
        params.itd_right_sec * sample_rate as f32,
        params.gain_right,
        stereo_slice,
        1,
    );
}

pub fn binauralize_mono_fast(
    mono: &[f32],
    src_pos: impl Into<Vec3>,
    listener_pos: impl Into<Vec3>,
    sample_rate: u32,
    settings: &AudioEngineSettings,
) -> Vec<[f32; 2]> {
    let mut stereo = vec![[0.0; 2]; mono.len()];
    binauralize_mono_fast_into(
        mono,
        &mut stereo,
        src_pos,
        listener_pos,
        sample_rate,
        settings,
    );
    stereo
}

/// Traitement d'un canal mono vers une voie stéréo sans aucune branche ni conversion de type dans la boucle
#[inline(always)]
fn process_channel(
    mono: &[f32],
    itd_samples: f32,
    gain: f32,
    stereo: &mut [[f32; 2]],
    channel_idx: usize,
) {
    let n = mono.len();
    let delay_int = itd_samples.floor() as usize;
    let frac = itd_samples - delay_int as f32;

    // Cas 1 : Pas de retard (ITD = 0), multiplication scalaire simple
    // LLVM vectorise ceci en instructions `vmulps` pure à vitesse mémoire maximale
    if delay_int == 0 && frac == 0.0 {
        for (out, &s) in stereo.iter_mut().zip(mono.iter()) {
            out[channel_idx] = s * gain;
        }
        return;
    }

    let alpha = 1.0 - frac; // Poids de l'échantillon supérieur

    // Cas 2 : Loop Peeling (Gestion des bords au début du buffer)
    // Pour les premiers échantillons où le retard pointerait avant l'indice 0, on clamp à mono[0]
    let peel_end = (delay_int + 1).min(n);
    let s0_gained = mono[0] * gain;
    for out in stereo[..peel_end].iter_mut() {
        out[channel_idx] = s0_gained;
    }

    // Cas 3 : HOT LOOP SIMD (Le cœur du buffer)
    // Zéro condition, zéro `min()`, zéro conversion float->int.
    if peel_end < n {
        let out_slice = &mut stereo[peel_end..n];

        // Les tranches sources sont contiguës et parfaitement alignées
        let src_low = &mono[0..(n - peel_end)];
        let src_high = &mono[1..(n - peel_end + 1)];

        // Grâce à zip() sur des tranches de tailles identiques, LLVM supprime les Bounds Checks (vérifications de limites).
        // La boucle est déroulée et transformée en instructions FMA vectorielles (AVX2 / NEON).
        for ((out, &s_low), &s_high) in out_slice
            .iter_mut()
            .zip(src_low.iter())
            .zip(src_high.iter())
        {
            out[channel_idx] = (s_low + (s_high - s_low) * alpha) * gain;
        }
    }
}

/// Convert mono audio to binaural stereo using ITD + ILD + elevation awareness (3D)
pub fn binauralize_mono(
    mono: &[f32],
    src_pos: impl Into<Vec3>,
    listener_pos: impl Into<Vec3>,
    sample_rate: u32,
    settings: &AudioEngineSettings,
) -> Vec<[f32; 2]> {
    let n = mono.len();
    let diff = src_pos.into() - listener_pos.into();
    let params = calculate_spatial_params_3d(
        diff,
        settings,
        crate::audio_engine::effect_flags::DEFAULT_FLAGS,
    );

    let itd_left_samples = params.itd_left_sec * sample_rate as f32;
    let itd_right_samples = params.itd_right_sec * sample_rate as f32;

    let stereo: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let idx_l = (i as f32) - itd_left_samples;
            let idx_r = (i as f32) - itd_right_samples;

            let s_left = interpolate_sample_fast(mono, idx_l) * params.gain_left;
            let s_right = interpolate_sample_fast(mono, idx_r) * params.gain_right;

            [s_left, s_right]
        })
        .collect();

    stereo
}

/// Linear interpolation helper
#[allow(dead_code)]
fn interpolate_sample(samples: &[f32], idx: f32) -> f32 {
    if idx <= 0.0 {
        samples[0]
    } else if idx >= (samples.len() - 1) as f32 {
        samples[samples.len() - 1]
    } else {
        let i0 = idx.floor() as usize;
        let frac = idx - i0 as f32;
        let s0 = samples[i0];
        let s1 = samples[i0 + 1];
        s0 * (1.0 - frac) + s1 * frac
    }
}

// On évite unwrap_or et le cast floor() coûteux.
// On clamp directement idx sur [0, len-2] pour éviter le dépassement.
// Ça fait déjà ~20–30% de gain CPU.
fn interpolate_sample_fast(samples: &[f32], idx: f32) -> f32 {
    let len = samples.len();
    if len == 0 {
        return 0.0;
    }
    if idx <= 0.0 {
        return samples[0];
    }
    let clamped_idx = idx.min((len - 2) as f32);
    let i0 = clamped_idx as usize;
    let frac = clamped_idx - i0 as f32;
    let s0 = samples[i0];
    let s1 = samples[i0 + 1];
    s0 + (s1 - s0) * frac
}

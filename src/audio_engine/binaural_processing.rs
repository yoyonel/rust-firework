use crate::AudioEngineSettings;

/// Convert mono audio to binaural stereo using ITD + ILD + elevation awareness (3D)
pub fn binauralize_mono(
    mono: &[f32],
    src_pos: (f32, f32, f32),      // (x, y, z)
    listener_pos: (f32, f32, f32), // (x, y, z)
    sample_rate: u32,
    settings: &AudioEngineSettings,
) -> Vec<[f32; 2]> {
    // ---------------------------------------------------------------
    // 1. Calculs géométriques
    // ---------------------------------------------------------------
    let dx = src_pos.0 - listener_pos.0; // droite-gauche
    let dy = src_pos.1 - listener_pos.1; // haut-bas
    let dz = src_pos.2 - listener_pos.2; // profondeur (z positif = proche)

    let distance = (dx * dx + dy * dy + dz * dz).sqrt().max(1e-6);

    // Azimut : angle horizontal autour de l’axe vertical (Y)
    // 0° = face avant, +X = droite
    let azimuth = dx.atan2(-dz); // inversion du signe z pour avoir +z = vers l’auditeur
    let theta = azimuth.abs();

    // Élévation : angle vertical (0 = plan horizontal)
    let elevation = dy.atan2((dx * dx + dz * dz).sqrt());

    // ---------------------------------------------------------------
    // 2. ITD / ILD
    // ---------------------------------------------------------------
    let c = 343.0_f32; // vitesse du son
    let itd = ((settings.head_radius() / c) * (theta + theta.sin())).clamp(0.0, 0.001);

    // ILD selon azimut, modulé légèrement par l’élévation (haut = moins d’atténuation)
    let ild_db = settings.max_ild_db() * theta.sin() * (1.0 - 0.25 * elevation.sin().abs());
    let far_gain = 10f32.powf(-ild_db / 20.0);

    // Atténuation avec distance (linéaire simple)
    let att = (1.0 - distance / settings.max_distance()).max(0.0);

    // ---------------------------------------------------------------
    // 3. Détermination du côté proche / éloigné
    // ---------------------------------------------------------------
    let (itd_left, itd_right, gain_left, gain_right) = if azimuth >= 0.0 {
        // Source à droite → oreille droite = proche
        (
            itd,            // gauche retardée
            0.0,            // droite sans décalage
            att * far_gain, // gauche atténuée
            att,            // droite pleine intensité
        )
    } else {
        // Source à gauche → oreille gauche = proche
        (
            0.0,            // gauche sans décalage
            itd,            // droite retardée
            att,            // gauche pleine intensité
            att * far_gain, // droite atténuée
        )
    };

    // ---------------------------------------------------------------
    // 4. Application ITD + ILD sur le signal mono
    // ---------------------------------------------------------------
    let n = mono.len();

    let itd_left_samples = itd_left * sample_rate as f32;
    let itd_right_samples = itd_right * sample_rate as f32;

    let stereo: Vec<[f32; 2]> = (0..n)
        .map(|i| {
            let idx_l = (i as f32) - itd_left_samples;
            let idx_r = (i as f32) - itd_right_samples;

            // let s_left = interpolate_sample(mono, idx_l) * gain_left;
            // let s_right = interpolate_sample(mono, idx_r) * gain_right;
            let s_left = interpolate_sample_fast(mono, idx_l) * gain_left;
            let s_right = interpolate_sample_fast(mono, idx_r) * gain_right;

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

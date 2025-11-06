use crate::audio_engine::binaural_processing::binauralize_mono;
use crate::audio_engine::types::Voice;
use crate::AudioEngineSettings;

/// Resample mono audio (linear interpolation).
///
/// - `input` : slice mono (`&[f32]`)
/// - `src_rate` : sample rate du `input` (Hz)
/// - `dst_rate` : sample rate désiré (Hz)
/// - retourne : `Vec<f32>` resamplé à `dst_rate`
///
/// Remarque : cette fonction fait de l'interpolation linéaire simple (rapide,
/// qualité acceptable pour realtime). Pour de la haute qualité, utiliser
/// sinc-windowed resampler (plus coûteux).
pub fn resample_linear_mono(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    // si pas de données, retourner vide
    if input.is_empty() {
        return Vec::new();
    }

    // cas trivial : mêmes sample rates
    if src_rate == dst_rate {
        return input.to_vec();
    }

    let src_len = input.len() as f32;
    let src_rate_f = src_rate as f32;
    let dst_rate_f = dst_rate as f32;

    // longueur de sortie estimée (arrondi vers le haut pour sécurité)
    let out_len = ((src_len / src_rate_f) * dst_rate_f).ceil() as usize;
    let mut out = Vec::with_capacity(out_len);

    // step = how many source-samples advance per output sample
    // step = src_rate / dst_rate  so idx_src = i_out * step
    let step = src_rate_f / dst_rate_f;

    // We iterate output index i_out and compute source index idx = i_out * step
    // Use local variables for speed
    let src = input;
    let src_n_minus1 = src.len() - 1;

    // For each output sample:
    //  idx = i * step
    //  i0 = floor(idx) as usize
    //  frac = idx - i0
    //  s = s0*(1-frac) + s1*frac
    //
    // We use a running `idx` to avoid multiplication each loop.
    let mut idx = 0.0_f32;
    for _ in 0..out_len {
        // clamp idx inside [0, src_len - 1)
        if idx <= 0.0 {
            out.push(src[0]);
        } else if idx >= (src_len - 1.0) {
            out.push(src[src_n_minus1]);
        } else {
            let i0 = idx.floor() as usize;
            let frac = idx - i0 as f32;
            let s0 = src[i0];
            let s1 = src[i0 + 1];
            out.push(s0 + (s1 - s0) * frac);
        }
        idx += step;
    }

    out
}

/// Helper: prepare voice but apply doppler resample first.
/// - raw_stereo: original stereo sample at engine sample_rate
/// - doppler_factor: factor >0 (1.0 = no change)
fn _prepare_voice_with_doppler(
    raw_stereo: &[[f32; 2]],
    pos: (f32, f32),
    listener_pos: (f32, f32),
    _gain: f32,
    doppler_factor: f32,
    sample_rate: u32,
    settings: &AudioEngineSettings,
) -> (Vec<[f32; 2]>, usize, usize, f32) {
    // If factor ≈ 1.0, avoid work
    let stereo: Vec<[f32; 2]> = if (doppler_factor - 1.0).abs() < 1e-3 {
        raw_stereo.to_vec()
    } else {
        // convert stereo -> mono
        let mono: Vec<f32> = raw_stereo.iter().map(|s| (s[0] + s[1]) * 0.5).collect();
        // perform linear resample: src_sr = sample_rate, dst_sr = sample_rate * doppler_factor
        let dst_sr = (sample_rate as f32 * doppler_factor).round() as u32;
        // resample_linear should accept mono input OR stereo input; adapt as needed.
        // Here we assume a resampler that accepts mono Vec<f32> and returns Vec<f32>
        let mono_resampled = resample_linear_mono(&mono, sample_rate, dst_sr);
        // binauralize or panning will expect stereo frames: duplicate mono into stereo for now
        // We'll call binauralize_mono which accepts mono + positions
        if settings.use_binaural() {
            binauralize_mono(
                &mono_resampled,
                (pos.0, pos.1, 0.0),
                (listener_pos.0, listener_pos.1, 0.0),
                sample_rate,
                settings,
            )
        } else {
            // fallback simple pan: create stereo Vec<[f32;2]> from mono_resampled and pan later
            mono_resampled.iter().map(|s| [*s, *s]).collect()
        }
    };

    // After we have stereo samples (already binauralized if chosen),
    // continue the same computations as original prepare_voice to compute fades & filter_a.
    let fade_in_samples = (sample_rate as f32 * (settings.fade_in_ms() / 1000.0)) as usize;
    let fade_out_samples = (sample_rate as f32 * (settings.fade_out_ms() / 1000.0)) as usize;

    // distance-dependent low-pass cutoff
    let dx = pos.0 - listener_pos.0; // adapt to how you store listener pos
    let dy = pos.1 - listener_pos.1;
    let distance = (dx * dx + dy * dy).sqrt();
    let fc = (settings.f_min()
        + (settings.f_max() - settings.f_min()) * (-settings.distance_alpha() * distance).exp())
    .clamp(settings.f_min(), settings.f_max());
    let dt = 1.0 / sample_rate as f32;
    let rc = 1.0 / (2.0 * std::f32::consts::PI * fc);
    let filter_a = dt / (rc + dt);

    (stereo, fade_in_samples, fade_out_samples, filter_a)
}

/// Applique un facteur Doppler sur un bloc d'échantillons d'une voix.
/// `voice` : la voix active contenant les samples originaux.
/// `doppler_factor` : facteur Doppler >0 (1.0 = pas de décalage, >1 = pitch monte, <1 = pitch descend)
/// `chunk` : buffer de sortie (taille <= voix restante)
fn _apply_doppler_to_chunk(voice: &mut Voice, doppler_factor: f32, chunk: &mut [[f32; 2]]) {
    if voice.data.is_none() {
        return;
    }
    let data = voice.data.as_ref().unwrap();

    let n = chunk.len();
    let mut pos = voice.pos as f32; // position flottante pour interpolation

    for sample in chunk.iter_mut().take(n) {
        // indices pour interpolation linéaire
        let idx = pos;
        let idx_floor = idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(data.len() - 1);
        let frac = idx - idx_floor as f32;

        // interpolation linéaire
        let sample_l = data[idx_floor][0] * (1.0 - frac) + data[idx_ceil][0] * frac;
        let sample_r = data[idx_floor][1] * (1.0 - frac) + data[idx_ceil][1] * frac;

        *sample = [sample_l, sample_r];

        // incrémente la position en fonction du facteur Doppler
        pos += doppler_factor;
    }

    // update voice position (float -> usize)
    voice.pos = pos as usize;
}

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

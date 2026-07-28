// =================================================================────────────
// HrtfConvolver — Convoluteur FIR HRTF sur Bus Spatial 2D (Overlap-Save FFT)
// =================================================================────────────
//
// Effectue le décodage binaural HRTF en temps réel sur la sortie du Bus Spatial 2D
// (W, X) via convolution FFT Overlap-Save par sous-blocs optimisés (256-512 samples).
//
// Complexité O(1) par rapport au nombre de fusées. Zéro allocation en runtime.

use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use std::sync::Arc;

/// Convoluteur FFT monopasse pour 1 canal audio et 1 filtre FIR (Overlap-Save).
pub struct SingleChannelConvolver {
    r2c: Arc<dyn RealToComplex<f32>>,
    c2r: Arc<dyn ComplexToReal<f32>>,
    fft_len: usize,
    fir_len: usize,
    input_history: Vec<f32>,
    h_spectrum: Vec<Complex<f32>>,
    fft_in: Vec<f32>,
    spectrum_buf: Vec<Complex<f32>>,
    ifft_out: Vec<f32>,
    r2c_scratch: Vec<Complex<f32>>,
    c2r_scratch: Vec<Complex<f32>>,
}

impl SingleChannelConvolver {
    /// Crée un convoluteur pour une réponse impulsionnelle FIR donnée et un chunk max.
    pub fn new(fir: &[f32], max_chunk_size: usize) -> Self {
        let fir_len = fir.len().max(1);
        let min_fft_len = max_chunk_size + fir_len - 1;
        let fft_len = min_fft_len.next_power_of_two();

        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_len);
        let c2r = planner.plan_fft_inverse(fft_len);

        // Calcul du spectre du filtre FIR zéro-padded
        let mut fir_padded = vec![0.0; fft_len];
        fir_padded[..fir.len()].copy_from_slice(fir);

        let mut h_spectrum = r2c.make_output_vec();
        r2c.process(&mut fir_padded, &mut h_spectrum)
            .expect("FFT calculation for FIR filter failed");

        let input_history = vec![0.0; fir_len - 1];
        let fft_in = r2c.make_input_vec();
        let spectrum_buf = r2c.make_output_vec();
        let ifft_out = c2r.make_output_vec();
        let r2c_scratch = r2c.make_scratch_vec();
        let c2r_scratch = c2r.make_scratch_vec();

        Self {
            r2c,
            c2r,
            fft_len,
            fir_len,
            input_history,
            h_spectrum,
            fft_in,
            spectrum_buf,
            ifft_out,
            r2c_scratch,
            c2r_scratch,
        }
    }

    /// Réinitialise les buffers internes d'historique (silence).
    pub fn reset(&mut self) {
        self.input_history.fill(0.0);
    }

    /// Effectue la convolution FFT Overlap-Save sur le slice d'entrée `input` et écrit dans `output`.
    /// Exécution 100% lock-free et sans aucune allocation dynamique (via scratch buffers pre-alloués).
    pub fn process_block(&mut self, input: &[f32], output: &mut [f32]) {
        let n = input.len();
        debug_assert_eq!(n, output.len());
        if n == 0 {
            return;
        }

        let history_len = self.fir_len - 1;
        debug_assert!(n + history_len <= self.fft_len);

        // 1. Remplissage du buffer d'entrée FFT (Overlap-Save) :
        //    [0 .. history_len] = historique du bloc précédent
        //    [history_len .. history_len + n] = nouveau bloc d'entrée
        //    [history_len + n .. fft_len] = 0.0 (zéro-padding si bloc plus court)
        self.fft_in[..history_len].copy_from_slice(&self.input_history);
        self.fft_in[history_len..history_len + n].copy_from_slice(input);
        if history_len + n < self.fft_len {
            self.fft_in[history_len + n..].fill(0.0);
        }

        // 2. Mise à jour de l'historique d'entrée pour le prochain bloc
        if n >= history_len {
            self.input_history
                .copy_from_slice(&input[n - history_len..]);
        } else {
            // Bloc plus petit que history_len
            self.input_history.copy_within(n.., 0);
            self.input_history[history_len - n..].copy_from_slice(input);
        }

        // 3. Forward FFT sans aucune allocation : X(f) = FFT(x)
        self.r2c
            .process_with_scratch(
                &mut self.fft_in,
                &mut self.spectrum_buf,
                &mut self.r2c_scratch,
            )
            .expect("Forward FFT failed");

        // 4. Multiplication complexe point par point dans le domaine fréquentiel : Y(f) = X(f) * H(f)
        for (y, &h) in self.spectrum_buf.iter_mut().zip(self.h_spectrum.iter()) {
            *y *= h;
        }

        // 5. Inverse FFT sans aucune allocation : y_raw = IFFT(Y(f))
        self.c2r
            .process_with_scratch(
                &mut self.spectrum_buf,
                &mut self.ifft_out,
                &mut self.c2r_scratch,
            )
            .expect("Inverse FFT failed");

        // 6. Normalisation par 1 / fft_len et extraction de la partie valide [history_len .. history_len + n]
        let norm = 1.0 / self.fft_len as f32;
        for (out_sample, &raw_sample) in output
            .iter_mut()
            .zip(&self.ifft_out[history_len..history_len + n])
        {
            *out_sample = raw_sample * norm;
        }
    }
}

/// Convoluteur HRTF Stéréo complet sur Bus Spatial (W, X).
///
/// Décode le bus Ambisonique 2D vers 2 enceintes virtuelles ($\pm 45^\circ$)
/// puis applique la matrice de filtres HRTF (2x2) pour alimenter la sortie stéréo ($L, R$).
pub struct HrtfConvolver {
    chunk_size: usize,
    /// Gauche Virtuelle -> Oreille Gauche (Ipsilatérale)
    conv_vl_l: SingleChannelConvolver,
    /// Gauche Virtuelle -> Oreille Droite (Contralatérale)
    conv_vl_r: SingleChannelConvolver,
    /// Droite Virtuelle -> Oreille Droite (Ipsilatérale)
    conv_vr_r: SingleChannelConvolver,
    /// Droite Virtuelle -> Oreille Gauche (Contralatérale)
    conv_vr_l: SingleChannelConvolver,
    /// Buffers de travail temporaires pré-alloués
    tmp_virtual_l: Vec<f32>,
    tmp_virtual_r: Vec<f32>,
    tmp_out_l1: Vec<f32>,
    tmp_out_l2: Vec<f32>,
    tmp_out_r1: Vec<f32>,
    tmp_out_r2: Vec<f32>,
}

impl HrtfConvolver {
    /// Crée une instance de convoluteur HRTF optimisée par sous-blocs (chunk_size max 512).
    pub fn new_default(sample_rate: u32, target_block_size: usize) -> Self {
        // Borner la taille des sous-blocs FFT entre 128 et 512 samples pour conserver une FFT à 512-1024 points max
        // quelle que soit la taille globale du buffer matériel (ex: 4096 ou 16384).
        let chunk_size = target_block_size.clamp(128, 512);

        let (fir_ipsi, fir_contra) =
            generate_synthetic_hrtf_pair(sample_rate, 45.0_f32.to_radians());

        let conv_vl_l = SingleChannelConvolver::new(&fir_ipsi, chunk_size);
        let conv_vl_r = SingleChannelConvolver::new(&fir_contra, chunk_size);
        let conv_vr_r = SingleChannelConvolver::new(&fir_ipsi, chunk_size);
        let conv_vr_l = SingleChannelConvolver::new(&fir_contra, chunk_size);

        Self {
            chunk_size,
            conv_vl_l,
            conv_vl_r,
            conv_vr_r,
            conv_vr_l,
            tmp_virtual_l: vec![0.0; chunk_size],
            tmp_virtual_r: vec![0.0; chunk_size],
            tmp_out_l1: vec![0.0; chunk_size],
            tmp_out_l2: vec![0.0; chunk_size],
            tmp_out_r1: vec![0.0; chunk_size],
            tmp_out_r2: vec![0.0; chunk_size],
        }
    }

    /// Réinitialise l'état interne de tous les convoluteurs (silence).
    pub fn reset(&mut self) {
        self.conv_vl_l.reset();
        self.conv_vl_r.reset();
        self.conv_vr_r.reset();
        self.conv_vr_l.reset();
    }

    /// Traite un bloc audio d'entrée Bus Spatial (`bus_w`, `bus_x`) et remplace la sortie accumulée `acc_out` (L, R).
    /// Si `frames` est supérieur à `chunk_size`, le traitement est automatiquement découpé en sous-blocs continus.
    pub fn process_bus(
        &mut self,
        bus_w: &[f32],
        bus_x: &[f32],
        acc_out: &mut [[f32; 2]],
        frames: usize,
    ) {
        if frames == 0 {
            return;
        }

        let chunk_size = self.chunk_size;
        let mut offset = 0;

        while offset < frames {
            let n = (frames - offset).min(chunk_size);

            self.process_bus_chunk(
                &bus_w[offset..offset + n],
                &bus_x[offset..offset + n],
                &mut acc_out[offset..offset + n],
                n,
            );

            offset += n;
        }
    }

    fn process_bus_chunk(
        &mut self,
        bus_w: &[f32],
        bus_x: &[f32],
        acc_out: &mut [[f32; 2]],
        n: usize,
    ) {
        // 1. Décodage du Bus Spatial (W, X) vers les 2 enceintes virtuelles (-45° et +45°)
        let frac_1_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
        for i in 0..n {
            let w = bus_w[i];
            let x = bus_x[i];
            // Virtuelle Gauche (-45°)
            self.tmp_virtual_l[i] = w - frac_1_sqrt2 * x;
            // Virtuelle Droite (+45°)
            self.tmp_virtual_r[i] = w + frac_1_sqrt2 * x;
        }

        // 2. Convolution FIR HRTF des enceintes virtuelles
        let vl = &self.tmp_virtual_l[..n];
        let vr = &self.tmp_virtual_r[..n];

        self.conv_vl_l.process_block(vl, &mut self.tmp_out_l1[..n]);
        self.conv_vr_l.process_block(vr, &mut self.tmp_out_l2[..n]);

        self.conv_vl_r.process_block(vl, &mut self.tmp_out_r1[..n]);
        self.conv_vr_r.process_block(vr, &mut self.tmp_out_r2[..n]);

        // 3. Mixage final stéréo : L = VL->L + VR->L, R = VL->R + VR->R
        for (i, acc) in acc_out.iter_mut().enumerate().take(n) {
            acc[0] = self.tmp_out_l1[i] + self.tmp_out_l2[i];
            acc[1] = self.tmp_out_r1[i] + self.tmp_out_r2[i];
        }
    }
}

/// Génère une paire de filtres FIR analytiques HRTF synthétiques (Ipsilatérale & Contralatérale).
///
/// # Arguments
/// * `sample_rate` - Fréquence d'échantillonnage en Hz (ex: 48000).
/// * `azimuth_rad` - Azimut de la source en radians (ex: 45°).
pub fn generate_synthetic_hrtf_pair(sample_rate: u32, azimuth_rad: f32) -> (Vec<f32>, Vec<f32>) {
    let fir_len = 128;
    let head_radius = 0.0875_f32; // ~8.75 cm (rayon moyen de la tête humaine)
    let c = 343.0_f32; // Vitesse du son dans l'air (m/s)

    let theta = azimuth_rad.abs();
    // Formule Woodworth & Schlosser pour le délai ITD
    let itd_sec = (head_radius / c) * (theta + theta.sin());
    let itd_samples = (itd_sec * sample_rate as f32).round() as usize;

    // Filtre Ipsilatéral (Oreille directe) : Impulsion unité centrée à l'index 4
    let direct_idx = 4;
    let mut fir_ipsi = vec![0.0; fir_len];
    if direct_idx < fir_len {
        fir_ipsi[direct_idx] = 1.0;
    }

    // Filtre Contralatéral (Oreille opposée) : Retard ITD + Passe-bas gaussien (atténuation HF par masque de la tête)
    let contra_idx = direct_idx + itd_samples;
    let mut fir_contra = vec![0.0; fir_len];

    // Atténuation d'amplitude globale (ILD)
    let ild_db = 6.0 * theta.sin();
    let contra_gain = 10.0_f32.powf(-ild_db / 20.0);

    // Réponse impulsionnelle passe-bas lissée (gaussienne) autour de contra_idx
    let sigma = 1.8_f32; // Largeur du lissage passe-bas
    for (i, val) in fir_contra.iter_mut().enumerate().take(fir_len) {
        let diff = i as f32 - contra_idx as f32;
        let weight = (-0.5 * (diff / sigma).powi(2)).exp();
        *val = weight;
    }

    // Normalisation de l'énergie du filtre contralatéral
    let sum: f32 = fir_contra.iter().sum();
    if sum > 1e-6 {
        for val in &mut fir_contra {
            *val = (*val / sum) * contra_gain;
        }
    }

    (fir_ipsi, fir_contra)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_channel_dirac_transparency() {
        // Filtre FIR = Dirac [1.0, 0.0, 0.0, ...]
        let fir = vec![1.0, 0.0, 0.0, 0.0];
        let block_size = 256;
        let mut conv = SingleChannelConvolver::new(&fir, block_size);

        let input: Vec<f32> = (0..block_size).map(|i| (i as f32 * 0.1).sin()).collect();
        let mut output = vec![0.0; block_size];

        conv.process_block(&input, &mut output);

        for (i, (&x, &y)) in input.iter().zip(output.iter()).enumerate() {
            assert!(
                (x - y).abs() < 1e-4,
                "Sample mismatch at index {}: expected {}, got {}",
                i,
                x,
                y
            );
        }
    }

    #[test]
    fn test_single_channel_delay_and_attenuation() {
        // Filtre FIR = [0.0, 0.0, 0.5] (Retard de 2 échantillons, gain 0.5)
        let fir = vec![0.0, 0.0, 0.5];
        let block_size = 128;
        let mut conv = SingleChannelConvolver::new(&fir, block_size);

        let mut input = vec![0.0; block_size];
        input[0] = 1.0; // Impulsion à t=0
        let mut output = vec![0.0; block_size];

        conv.process_block(&input, &mut output);

        // t=0 et t=1 doivent être quasi-nuls (< 1e-4)
        assert!(output[0].abs() < 1e-4);
        assert!(output[1].abs() < 1e-4);
        // t=2 doit valoir 0.5
        assert!((output[2] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_single_channel_block_continuity() {
        // Filtre passe-bas / retard arbitraire
        let fir = vec![0.25, 0.5, 0.25];
        let block_size = 64;
        let mut conv = SingleChannelConvolver::new(&fir, block_size);

        // Envoyer 2 blocs consécutifs d'une onde sinusoïdale continue
        let full_input: Vec<f32> = (0..128).map(|i| (i as f32 * 0.05).sin()).collect();

        let mut out_block1 = vec![0.0; 64];
        let mut out_block2 = vec![0.0; 64];

        conv.process_block(&full_input[..64], &mut out_block1);
        conv.process_block(&full_input[64..], &mut out_block2);

        let mut full_output = out_block1;
        full_output.extend(out_block2);

        // Vérifier qu'il n'y a pas de saut brutal entre le sample 63 et 64
        let diff_at_boundary = (full_output[64] - full_output[63]).abs();
        assert!(
            diff_at_boundary < 0.1,
            "Discontinuity detected at block boundary: diff = {}",
            diff_at_boundary
        );
    }

    #[test]
    fn test_hrtf_synthetic_pair_itd() {
        let (ipsi, contra) = generate_synthetic_hrtf_pair(48000, 45.0_f32.to_radians());

        // L'oreille ipsilatérale a son impulsion principale à direct_idx (index 4)
        let max_ipsi_idx = ipsi
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;
        assert_eq!(max_ipsi_idx, 4);

        // L'oreille contralatérale doit avoir son pic retardé de l'ITD
        let max_contra_idx = contra
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;

        assert!(
            max_contra_idx > max_ipsi_idx,
            "Contralateral peak ({}) should be delayed relative to ipsilateral peak ({})",
            max_contra_idx,
            max_ipsi_idx
        );
    }

    #[test]
    fn test_spatial_bus_hrtf_360_rotation_continuity() {
        let sample_rate = 48000;
        let chunk_size = 256;
        let mut hrtf = HrtfConvolver::new_default(sample_rate, chunk_size);

        let num_angles = 360;
        let mut energies = Vec::with_capacity(num_angles);

        let frac_1_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;

        for step in 0..num_angles {
            let angle_rad = (step as f32) * (2.0 * std::f32::consts::PI / num_angles as f32);
            let dir_x = angle_rad.sin(); // -1.0 .. +1.0

            let mut bus_w = vec![0.0f32; chunk_size];
            let mut bus_x = vec![0.0f32; chunk_size];
            let mut acc_out = vec![[0.0f32; 2]; chunk_size];

            for i in 0..chunk_size {
                let s = (i as f32 * 0.1).sin();
                bus_w[i] = s * frac_1_sqrt2;
                bus_x[i] = s * dir_x;
            }

            hrtf.process_bus(&bus_w, &bus_x, &mut acc_out, chunk_size);

            let energy: f32 = acc_out.iter().map(|s| s[0].powi(2) + s[1].powi(2)).sum();
            energies.push(energy);

            assert!(
                energy > 1e-4,
                "Energy dropped to near zero at angle step {} ({:.1} deg): energy = {}",
                step,
                angle_rad.to_degrees(),
                energy
            );
        }

        // Vérifier la continuité angulaire : pas de variation d'énergie extrême entre deux pas angulaires consécutifs
        for i in 0..num_angles {
            let next_i = (i + 1) % num_angles;
            let e1 = energies[i];
            let e2 = energies[next_i];
            let diff_ratio = (e1 - e2).abs() / e1.max(1e-4);

            assert!(
                diff_ratio < 0.35,
                "Energy jump detected between step {} ({:.1} deg) and {} ({:.1} deg): ratio = {:.2}%",
                i,
                (i as f32 * 360.0 / num_angles as f32),
                next_i,
                (next_i as f32 * 360.0 / num_angles as f32),
                diff_ratio * 100.0
            );
        }
    }
}

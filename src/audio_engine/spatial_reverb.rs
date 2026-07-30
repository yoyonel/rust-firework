// =================================================================────────────
// SpatialReverb — Réverbération Spatiale Algorithmique sur Bus Unique (Schroeder/FDN)
// =================================================================────────────
//
// Conçue pour traiter la sortie globale du Bus Spatial 2D à coût constant O(1).
// Elle combine 4 filtres de peigne en parallèle avec amortissement des hautes fréquences
// et 2 filtres tout-passe en série pour générer une densité d'écho naturelle.
//
// Zéro allocation dans la boucle temps réel : les buffers circulaires sont pré-alloués.

struct CombFilter {
    buffer: Vec<f32>,
    pos: usize,
    feedback: f32,
    damp: f32,
    filter_state: f32,
}

impl CombFilter {
    fn new(delay_samples: usize, feedback: f32, damp: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples],
            pos: 0,
            feedback,
            damp,
            filter_state: 0.0,
        }
    }

    #[inline(always)]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.pos];
        // Low-pass filter dans la boucle de feedback (atténuation HF de l'air)
        self.filter_state = output * (1.0 - self.damp) + self.filter_state * self.damp;
        self.buffer[self.pos] = input + self.filter_state * self.feedback;
        self.pos = (self.pos + 1) % self.buffer.len();
        output
    }
}

struct AllPassFilter {
    buffer: Vec<f32>,
    pos: usize,
    feedback: f32,
}

impl AllPassFilter {
    fn new(delay_samples: usize, feedback: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples],
            pos: 0,
            feedback,
        }
    }

    #[inline(always)]
    fn process(&mut self, input: f32) -> f32 {
        let buf_out = self.buffer[self.pos];
        let output = -input + buf_out;
        self.buffer[self.pos] = input + buf_out * self.feedback;
        self.pos = (self.pos + 1) % self.buffer.len();
        output
    }
}

use crate::audio_engine::constants;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct SpatialReverb {
    combs_l: Vec<CombFilter>,
    combs_r: Vec<CombFilter>,
    allpasses_l: Vec<AllPassFilter>,
    allpasses_r: Vec<AllPassFilter>,
    pub wet_gain: Arc<AtomicU32>,
}

impl SpatialReverb {
    /// Crée une nouvelle instance de réverbération spatiale pré-allouée avec un wet_gain partagé.
    pub fn new_with_wet(sample_rate: u32, wet_gain: Arc<AtomicU32>) -> Self {
        let scale = sample_rate as f32 / constants::REVERB_BASE_SAMPLE_RATE;

        let feedback = constants::REVERB_DEFAULT_FEEDBACK;
        let damp = constants::REVERB_DEFAULT_DAMPING;

        let combs_l = constants::REVERB_COMB_DELAYS_BASE_SAMPLES
            .iter()
            .map(|&d| CombFilter::new((d as f32 * scale) as usize, feedback, damp))
            .collect();

        let combs_r = constants::REVERB_COMB_DELAYS_BASE_SAMPLES
            .iter()
            .map(|&d| {
                CombFilter::new(
                    ((d + constants::REVERB_STEREO_UNCORRELATION_OFFSET_SAMPLES) as f32 * scale)
                        as usize,
                    feedback,
                    damp,
                )
            })
            .collect();

        let allpasses_l = constants::REVERB_ALLPASS_DELAYS_BASE_SAMPLES
            .iter()
            .map(|&d| AllPassFilter::new((d as f32 * scale) as usize, 0.35))
            .collect();

        let allpasses_r = constants::REVERB_ALLPASS_DELAYS_BASE_SAMPLES
            .iter()
            .map(|&d| {
                AllPassFilter::new(
                    ((d + constants::REVERB_STEREO_UNCORRELATION_OFFSET_SAMPLES) as f32 * scale)
                        as usize,
                    0.35,
                )
            })
            .collect();

        Self {
            combs_l,
            combs_r,
            allpasses_l,
            allpasses_r,
            wet_gain,
        }
    }

    /// Crée une nouvelle instance avec la valeur par défaut (8% wet).
    pub fn new(sample_rate: u32) -> Self {
        Self::new_with_wet(sample_rate, Arc::new(AtomicU32::new(0.08f32.to_bits())))
    }

    pub fn set_wet_gain(&self, wet: f32) {
        self.wet_gain
            .store(wet.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn get_wet_gain(&self) -> f32 {
        f32::from_bits(self.wet_gain.load(Ordering::Relaxed))
    }

    /// Applique la réverbération spatiale en place sur le buffer d'accumulation stéréo.
    /// Exécutée UNE SEULE FOIS par bloc d'audio (coût O(1)).
    #[inline(always)]
    pub fn process_block(&mut self, acc: &mut [[f32; 2]], frames: usize) {
        let wet = self.get_wet_gain();
        let comb_scale = 0.25; // Normalisation des 4 peignes en parallèle (évite la saturation)

        for frame in acc[..frames].iter_mut() {
            let in_l = frame[0];
            let in_r = frame[1];

            // 1. Filtrage en parallèle par les filtres de peigne avec normalisation
            let mut out_l = 0.0;
            for comb in self.combs_l.iter_mut() {
                out_l += comb.process(in_l) * comb_scale;
            }

            let mut out_r = 0.0;
            for comb in self.combs_r.iter_mut() {
                out_r += comb.process(in_r) * comb_scale;
            }

            // 2. Diffusion à travers les filtres tout-passe en série
            for ap in self.allpasses_l.iter_mut() {
                out_l = ap.process(out_l);
            }
            for ap in self.allpasses_r.iter_mut() {
                out_r = ap.process(out_r);
            }

            // 3. Mixage du signal réverbéré (Wet) avec le signal sec (Dry)
            frame[0] = in_l + out_l * wet;
            frame[1] = in_r + out_r * wet;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_reverb_decay() {
        let mut reverb = SpatialReverb::new(48000);
        let mut buffer = vec![[0.0; 2]; 3000];
        // Incurser une impulsion d'entrée
        buffer[0] = [1.0, 1.0];

        reverb.process_block(&mut buffer, 3000);

        // Vérifier qu'un écho/réverbération s'est propagé après le délai initial (~1200 échantillons)
        let has_tail = buffer[1300..2500]
            .iter()
            .any(|&[l, r]| l.abs() > 0.001 || r.abs() > 0.001);
        assert!(has_tail, "La réverbération doit générer une traîne d'écho");
    }
}

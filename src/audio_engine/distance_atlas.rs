// =================================================================────────────
// DistanceAudioAtlas — Stratification & Échantillons Pré-Spatialisés en Distance
// =================================================================────────────
//
// Pré-calcule et stocke 3 variantes filtrées de chaque échantillon sonore au chargement :
//  - `near` : Plein spectre d'origine (0m à 200m)
//  - `mid`  : Filtrage passe-bas pré-calculé à fc = 4000 Hz (200m à 800m)
//  - `far`  : Filtrage passe-bas pré-calculé à fc = 1200 Hz (> 800m)
//
// L'utilisation du SoundAtlas élimine totalement le calcul du filtre IIR passe-bas
// dans la boucle temps réel hot-path lors du rendu par voix.

use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct SoundAtlas {
    pub near: Arc<Vec<[f32; 2]>>,
    pub mid: Arc<Vec<[f32; 2]>>,
    pub far: Arc<Vec<[f32; 2]>>,
}

impl SoundAtlas {
    /// Génère les 3 variantes pré-filtrées d'un signal audio brut.
    pub fn from_raw_data(data: Arc<Vec<[f32; 2]>>, sample_rate: u32) -> Self {
        let mid = prefilter_lowpass(&data, sample_rate, 4000.0);
        let far = prefilter_lowpass(&data, sample_rate, 1200.0);

        Self {
            near: data,
            mid: Arc::new(mid),
            far: Arc::new(far),
        }
    }

    /// Sélectionne la variante audio adaptée à la distance temps réel de la source (coût O(1)).
    #[inline(always)]
    pub fn select(&self, distance: f32) -> &Arc<Vec<[f32; 2]>> {
        if distance <= 200.0 {
            &self.near
        } else if distance <= 800.0 {
            &self.mid
        } else {
            &self.far
        }
    }
}

/// Applique un filtre passe-bas du 1er ordre offline sur l'échantillon brut.
fn prefilter_lowpass(input: &[[f32; 2]], sample_rate: u32, fc: f32) -> Vec<[f32; 2]> {
    if input.is_empty() {
        return Vec::new();
    }
    let dt = 1.0 / sample_rate as f32;
    let rc = 1.0 / (2.0 * std::f32::consts::PI * fc);
    let a = dt / (rc + dt);

    let mut output = Vec::with_capacity(input.len());
    let mut state = input[0]; // Initialisation propre sans saut de tension (pas de clic)

    for &[l, r] in input {
        state[0] += a * (l - state[0]);
        state[1] += a * (r - state[1]);
        output.push(state);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_atlas_selection() {
        let raw = Arc::new(vec![[1.0, 1.0]; 100]);
        let atlas = SoundAtlas::from_raw_data(raw.clone(), 48000);

        assert!(Arc::ptr_eq(atlas.select(50.0), &atlas.near));
        assert!(Arc::ptr_eq(atlas.select(400.0), &atlas.mid));
        assert!(Arc::ptr_eq(atlas.select(1000.0), &atlas.far));
    }
}

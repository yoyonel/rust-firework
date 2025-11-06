// =========================
// Audio File Loading
// =========================
use hound::WavReader; // WAV file loader

/// Charge un fichier WAV et le convertit en tampon stéréo `[f32; 2]`
///
/// - Gère les fichiers mono et stéréo (duplique le canal gauche si mono)
/// - Convertit les échantillons 16-bit PCM (`i16`) en `f32` normalisés entre [-1.0, 1.0]
/// - Ignore silencieusement les erreurs de lecture individuelles grâce à `.flatten()`
///
/// # Arguments
/// * `path` — chemin du fichier WAV à charger
///
/// # Retour
/// * `Vec<[f32; 2]>` — échantillons stéréo prêtes à être joués ou traités
pub fn load_audio(path: &str) -> Vec<[f32; 2]> {
    // Ouvre le fichier WAV
    let mut reader = WavReader::open(path).unwrap();

    // Récupère la description du flux audio (nombre de canaux, format, etc.)
    let spec = reader.spec();

    // Vecteur final contenant les échantillons stéréo [gauche, droite]
    let mut data = Vec::new();

    // Tampon temporaire pour regrouper les échantillons d’un même frame (ex: L/R)
    let mut temp = Vec::with_capacity(spec.channels as usize);

    // Parcourt tous les échantillons 16-bit, ignore les erreurs (`flatten`)
    for s in reader.samples::<i16>().flatten() {
        // Convertit en flottant entre -1.0 et 1.0
        temp.push(s as f32 / 32768.0);

        // Quand on a accumulé assez d’échantillons pour une frame complète (1 ou 2 canaux)
        if temp.len() == spec.channels as usize {
            // Canal gauche
            let l = temp[0];

            // Canal droit : identique si mono
            let r = if spec.channels > 1 {
                temp[1]
            } else {
                temp[0] // duplique le canal gauche
            };

            // Ajoute la frame stéréo au tampon final
            data.push([l, r]);

            // Réinitialise le tampon temporaire pour la prochaine frame
            temp.clear();
        }
    }

    // Retourne le buffer stéréo complet
    data
}

/// Resample audio linearly from src_sr → dst_sr
pub fn resample_linear(data: &[[f32; 2]], src_sr: u32, dst_sr: u32) -> Vec<[f32; 2]> {
    if src_sr == dst_sr {
        return data.to_owned();
    }

    let n_samples = data.len();
    let new_len = ((n_samples as f64) * (dst_sr as f64 / src_sr as f64)).ceil() as usize;
    let mut out = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let pos = (i as f64) * (n_samples as f64 - 1.0) / (new_len as f64 - 1.0);
        let idx = pos.floor() as usize;
        let frac = pos - idx as f64;

        let s0 = data[idx];
        let s1 = if idx + 1 < n_samples {
            data[idx + 1]
        } else {
            s0
        };
        out.push([
            s0[0] + (s1[0] - s0[0]) * frac as f32,
            s0[1] + (s1[1] - s0[1]) * frac as f32,
        ]);
    }
    out
}

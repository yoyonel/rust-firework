// =================================================================────────────
// Exemple / Démo Interactive : Rotation Audio 3D 360° avec Décodeur HRTF
// =================================================================────────────
//
// Génère un fichier audio stéréo binaural (`hrtf_360_rotation_demo.wav`) de 8 secondes
// où une source sonore effectue deux rotations complètes à 360° autour de l'auditeur.
//
// Usage :
//   cargo run --example hrtf_360_demo
//   aplay hrtf_360_rotation_demo.wav   (ou écouter avec n'importe quel lecteur/casque)

use fireworks_sim::audio_engine::hrtf_convolver::HrtfConvolver;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::f32::consts::PI;

fn main() {
    let sample_rate = 48000;
    let duration_secs = 8.0_f32;
    let total_samples = (sample_rate as f32 * duration_secs) as usize;
    let block_size = 256;

    let output_path = "hrtf_360_rotation_demo.wav";
    println!("🎧 Génération du sample binaural 360° HRTF...");
    println!("   Fichier de sortie : {}", output_path);
    println!(
        "   Durée : {:.1}s | Sample rate : {} Hz | Block size : {}",
        duration_secs, sample_rate, block_size
    );

    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer =
        WavWriter::create(output_path, spec).expect("Impossible de créer le fichier WAV de démo");

    let mut hrtf_convolver = HrtfConvolver::new_default(sample_rate, block_size);

    let mut bus_w = vec![0.0f32; block_size];
    let mut bus_x = vec![0.0f32; block_size];
    let mut acc_out = vec![[0.0f32; 2]; block_size];

    let frac_1_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;

    let total_blocks = total_samples / block_size;

    for block_idx in 0..total_blocks {
        let block_start_sample = block_idx * block_size;
        let t_start = block_start_sample as f32 / sample_rate as f32;

        // Angle de rotation 360° :
        // 0s à 4s : 1ère rotation trigonométrique (sens anti-horaire : face -> gauche -> arrière -> droite -> face)
        // 4s à 8s : 2ème rotation horaire (face -> droite -> arrière -> gauche -> face)
        let angle_rad = if t_start < 4.0 {
            (t_start / 4.0) * 2.0 * PI
        } else {
            2.0 * PI - ((t_start - 4.0) / 4.0) * 2.0 * PI
        };

        // composante X (droite = +1, gauche = -1)
        let dir_x = angle_rad.sin();

        // Synthèse du son source (combinaison harmonique + crépitement d'étincelles)
        for i in 0..block_size {
            let sample_idx = block_start_sample + i;
            let t = sample_idx as f32 / sample_rate as f32;

            // Fréquence fondamentale 320 Hz + harmonique 640 Hz
            let tone = 0.4 * (2.0 * PI * 320.0 * t).sin() + 0.2 * (2.0 * PI * 640.0 * t).sin();
            // Crépitement / Modulation d'amplitude à 8 Hz
            let pulse = 0.5 + 0.5 * (2.0 * PI * 8.0 * t).sin();
            // Bruit léger d'étincelle
            let noise = ((sample_idx * 1103515245 + 12345) % 32768) as f32 / 32768.0 - 0.5;

            let source_sample = (tone * pulse + noise * 0.15) * 0.6;

            // Encodage Bus Spatial 2D (W, X)
            bus_w[i] = source_sample * frac_1_sqrt2;
            bus_x[i] = source_sample * dir_x;
        }

        // Décodage HRTF Binaural par sous-blocs Overlap-Save FFT
        hrtf_convolver.process_bus(&bus_w, &bus_x, &mut acc_out, block_size);

        // Écriture dans le fichier WAV 16-bit
        for sample in &acc_out {
            let l_int = (sample[0].clamp(-1.0, 1.0) * 32767.0) as i16;
            let r_int = (sample[1].clamp(-1.0, 1.0) * 32767.0) as i16;
            writer.write_sample(l_int).unwrap();
            writer.write_sample(r_int).unwrap();
        }
    }

    writer
        .finalize()
        .expect("Erreur lors de la finalisation du fichier WAV");

    println!("\n✅ Démo générée avec succès dans : {}", output_path);
    println!("🎧 Mettez votre casque audio et écoutez avec :");
    println!("   aplay {}", output_path);
    println!("   ou vlc {}", output_path);
}

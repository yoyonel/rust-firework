use fireworks_sim::audio_engine::safewavwriter::{AudioBlock, SafeWavWriter};
use std::time::Duration;

// ==================================
// 1. Structure AudioBlock
// ==================================

#[test]
fn test_audio_block_creation() {
    let frames = vec![[0.5, -0.5], [0.3, 0.7]];
    let block = AudioBlock {
        index: 42,
        frames: frames.clone(),
    };

    assert_eq!(block.index, 42);
    assert_eq!(block.frames.len(), 2);
    assert_eq!(block.frames[0], [0.5, -0.5]);
}

// ==================================
// 2. SafeWavWriter - Cycle de Vie
// ==================================

#[test]
fn test_safewavwriter_create_and_stop() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("test_output.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);

    // Arrêt immédiat
    writer.stop();

    // Vérifier que le fichier existe
    assert!(path.exists());
}

#[test]
fn test_safewavwriter_write_single_block() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("single_block.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);

    // Créer un bloc avec 100 frames
    let frames = vec![[0.5, -0.5]; 100];
    let block = AudioBlock { index: 0, frames };

    writer.push_block(block);

    // Attendre un peu pour que le thread traite
    std::thread::sleep(Duration::from_millis(100));

    writer.stop();

    // Vérifier que le fichier existe et n'est pas vide
    assert!(path.exists());
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 44); // Header WAV = 44 bytes
}

#[test]
fn test_safewavwriter_write_multiple_blocks() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("multiple_blocks.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);

    // Écrire 10 blocs
    for i in 0..10 {
        let frames = vec![[0.1 * i as f32, -0.1 * i as f32]; 50];
        let block = AudioBlock { index: i, frames };
        writer.push_block(block);
    }

    // Attendre le traitement
    std::thread::sleep(Duration::from_millis(200));

    writer.stop();

    // Vérifier la taille du fichier
    let metadata = std::fs::metadata(&path).unwrap();
    // 10 blocs * 50 frames * 2 channels * 2 bytes (i16) + 44 bytes header
    let expected_size = 10 * 50 * 2 * 2 + 44;
    assert_eq!(metadata.len(), expected_size as u64);
}

#[test]
fn test_safewavwriter_clamping() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("clamping.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);

    // Valeurs hors limites [-1.0, 1.0]
    let frames = vec![
        [2.0, -2.0],   // Devrait être clampé à [1.0, -1.0]
        [0.5, 0.5],    // Normal
        [10.0, -10.0], // Devrait être clampé
    ];

    let block = AudioBlock { index: 0, frames };

    writer.push_block(block);
    std::thread::sleep(Duration::from_millis(100));
    writer.stop();

    // Le fichier devrait être créé sans erreur
    assert!(path.exists());
}

#[test]
fn test_safewavwriter_empty_blocks() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("empty.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);

    // Bloc vide
    let block = AudioBlock {
        index: 0,
        frames: vec![],
    };

    writer.push_block(block);
    std::thread::sleep(Duration::from_millis(100));
    writer.stop();

    // Fichier devrait exister avec juste le header
    assert!(path.exists());
    let metadata = std::fs::metadata(&path).unwrap();
    assert_eq!(metadata.len(), 44); // Juste le header WAV
}

#[test]
fn test_safewavwriter_long_write() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("long_write.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);

    // Écrire pendant plus de 2 secondes pour déclencher le flush périodique
    for i in 0..50 {
        let frames = vec![[0.1, -0.1]; 1000];
        let block = AudioBlock { index: i, frames };
        writer.push_block(block);
        std::thread::sleep(Duration::from_millis(50));
    }

    writer.stop();

    // Vérifier que le fichier est bien écrit
    assert!(path.exists());
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(metadata.len() > 44);
}

#[test]
fn test_safewavwriter_different_sample_rates() {
    for sample_rate in [22050, 44100, 48000, 96000] {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join(format!("sr_{}.wav", sample_rate));
        let path_str = path.to_str().unwrap();

        let mut writer = SafeWavWriter::new(path_str, sample_rate);

        let frames = vec![[0.5, -0.5]; 100];
        let block = AudioBlock { index: 0, frames };

        writer.push_block(block);
        std::thread::sleep(Duration::from_millis(100));
        writer.stop();

        assert!(path.exists());
    }
}

#[test]
fn test_safewavwriter_verify_wav_format() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("verify_format.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);

    let frames = vec![[0.5, -0.5]; 100];
    let block = AudioBlock { index: 0, frames };

    writer.push_block(block);
    std::thread::sleep(Duration::from_millis(100));
    writer.stop();

    // Relire le fichier avec hound pour vérifier le format
    let reader = hound::WavReader::open(&path).unwrap();
    let spec = reader.spec();

    assert_eq!(spec.channels, 2);
    assert_eq!(spec.sample_rate, 44100);
    assert_eq!(spec.bits_per_sample, 16);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);
}

#[test]
fn test_safewavwriter_verify_audio_content() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("verify_content.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);

    // Écrire un signal connu
    let frames = vec![
        [1.0, -1.0],
        [0.5, 0.5],
        [0.0, 0.0],
        [-0.5, -0.5],
        [-1.0, 1.0],
    ];

    let block = AudioBlock {
        index: 0,
        frames: frames.clone(),
    };

    writer.push_block(block);
    std::thread::sleep(Duration::from_millis(100));
    writer.stop();

    // Relire et vérifier
    let mut reader = hound::WavReader::open(&path).unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

    // Vérifier le nombre de samples (5 frames * 2 channels = 10 samples)
    assert_eq!(samples.len(), 10);

    // Vérifier les valeurs approximatives (conversion f32 -> i16)
    // Note: -1.0 * i16::MAX as f32 = -32767, pas -32768 (i16::MIN)
    assert_eq!(samples[0], i16::MAX); // 1.0 -> 32767
    assert!(samples[1] == i16::MIN || samples[1] == -32767); // -1.0 -> -32768 ou -32767
}

// ==================================
// 3. Edge Cases et Robustesse
// ==================================

#[test]
fn test_safewavwriter_stop_without_write() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("no_write.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);
    writer.stop();

    // Fichier devrait exister avec juste le header
    assert!(path.exists());
}

#[test]
fn test_safewavwriter_double_stop() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("double_stop.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);
    writer.stop();
    writer.stop(); // Ne devrait pas paniquer

    assert!(path.exists());
}

#[test]
fn test_safewavwriter_push_after_stop() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("push_after_stop.wav");
    let path_str = path.to_str().unwrap();

    let mut writer = SafeWavWriter::new(path_str, 44100);
    writer.stop();

    // Essayer de pousser après stop
    let block = AudioBlock {
        index: 0,
        frames: vec![[0.5, -0.5]; 10],
    };
    writer.push_block(block); // Ne devrait pas paniquer

    assert!(path.exists());
}

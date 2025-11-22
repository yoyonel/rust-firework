use fireworks_sim::audio_engine::binaural_processing::binauralize_mono;
use fireworks_sim::audio_engine::dsp::resample_linear_mono;
use fireworks_sim::AudioEngineSettings;

#[test]
fn test_resample_linear_mono_empty() {
    let input: &[f32] = &[];
    let output = resample_linear_mono(input, 44100, 48000);
    assert!(output.is_empty());
}

#[test]
fn test_resample_linear_mono_identity() {
    let input = vec![0.1, 0.2, 0.3, 0.4];
    let output = resample_linear_mono(&input, 44100, 44100);
    assert_eq!(output, input);
}

#[test]
fn test_resample_linear_mono_upsample() {
    // Doubling sample rate: 2 samples -> 4 samples
    // Linear interpolation: 0.0 -> 1.0
    // Input: [0.0, 1.0]
    // Output should be approx: [0.0, 0.5, 1.0, 1.0] (last one clamped)
    // Let's trace the logic:
    // src_len=2, src_rate=100, dst_rate=200. out_len=4. step=0.5.
    // i=0, idx=0.0 -> src[0] = 0.0
    // i=1, idx=0.5 -> src[0] + 0.5*(src[1]-src[0]) = 0.5
    // i=2, idx=1.0 -> src[1] = 1.0
    // i=3, idx=1.5 -> src[1] = 1.0 (clamped)

    let input = vec![0.0, 1.0];
    let output = resample_linear_mono(&input, 100, 200);

    assert_eq!(output.len(), 4);
    assert!((output[0] - 0.0).abs() < 1e-5);
    assert!((output[1] - 0.5).abs() < 1e-5);
    assert!((output[2] - 1.0).abs() < 1e-5);
    assert!((output[3] - 1.0).abs() < 1e-5);
}

#[test]
fn test_resample_linear_mono_downsample() {
    // Halving sample rate: 4 samples -> 2 samples
    // Input: [0.0, 0.5, 1.0, 1.5]
    // src_rate=200, dst_rate=100. step=2.0.
    // i=0, idx=0.0 -> src[0] = 0.0
    // i=1, idx=2.0 -> src[2] = 1.0

    let input = vec![0.0, 0.5, 1.0, 1.5];
    let output = resample_linear_mono(&input, 200, 100);

    assert_eq!(output.len(), 2);
    assert!((output[0] - 0.0).abs() < 1e-5);
    assert!((output[1] - 1.0).abs() < 1e-5);
}

#[test]
fn test_binauralize_mono_basic() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];
    let sample_rate = 44100;

    // Source at (10, 0, 0) -> Right side
    let src_pos = (10.0, 0.0, 0.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    assert_eq!(stereo.len(), mono.len());

    // Check that we have valid float values
    for frame in &stereo {
        assert!(frame[0].is_finite());
        assert!(frame[1].is_finite());
    }

    // Since source is on the right, right channel should be louder/earlier (ignoring ITD complexity for amplitude)
    // Actually, ILD makes the far side (left) quieter.
    // Let's check average amplitude.
    let avg_l: f32 = stereo.iter().map(|s| s[0].abs()).sum::<f32>() / stereo.len() as f32;
    let avg_r: f32 = stereo.iter().map(|s| s[1].abs()).sum::<f32>() / stereo.len() as f32;

    // Right channel should be significantly louder than Left channel
    assert!(
        avg_r > avg_l,
        "Right channel should be louder for source on right"
    );
}

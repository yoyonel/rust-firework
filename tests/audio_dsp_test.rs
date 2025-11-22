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

// ==================================
// Additional Binaural Tests
// ==================================

#[test]
fn test_binauralize_mono_left_side() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];
    let sample_rate = 44100;

    // Source at (-10, 0, 0) -> Left side
    let src_pos = (-10.0, 0.0, 0.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    let avg_l: f32 = stereo.iter().map(|s| s[0].abs()).sum::<f32>() / stereo.len() as f32;
    let avg_r: f32 = stereo.iter().map(|s| s[1].abs()).sum::<f32>() / stereo.len() as f32;

    // Left channel should be louder for source on left
    assert!(
        avg_l > avg_r,
        "Left channel should be louder for source on left"
    );
}

#[test]
fn test_binauralize_mono_center() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];
    let sample_rate = 44100;

    // Source at (0, 0, -10) -> Center (in front)
    let src_pos = (0.0, 0.0, -10.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    let avg_l: f32 = stereo.iter().map(|s| s[0].abs()).sum::<f32>() / stereo.len() as f32;
    let avg_r: f32 = stereo.iter().map(|s| s[1].abs()).sum::<f32>() / stereo.len() as f32;

    // Channels should be approximately equal for centered source
    let ratio = avg_l / avg_r.max(1e-6);
    assert!(
        ratio > 0.9 && ratio < 1.1,
        "Channels should be balanced for centered source, ratio: {}",
        ratio
    );
}

#[test]
fn test_binauralize_mono_with_elevation() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];
    let sample_rate = 44100;

    // Source above and to the right (10, 5, 0)
    let src_pos = (10.0, 5.0, 0.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    assert_eq!(stereo.len(), mono.len());

    // Right channel should still be louder (source on right)
    let avg_l: f32 = stereo.iter().map(|s| s[0].abs()).sum::<f32>() / stereo.len() as f32;
    let avg_r: f32 = stereo.iter().map(|s| s[1].abs()).sum::<f32>() / stereo.len() as f32;

    assert!(avg_r > avg_l, "Right channel should be louder");
}

#[test]
fn test_binauralize_mono_very_close() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];
    let sample_rate = 44100;

    // Source very close (1, 0, 0)
    let src_pos = (1.0, 0.0, 0.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    // Should have valid output
    assert_eq!(stereo.len(), mono.len());
    for frame in &stereo {
        assert!(frame[0].is_finite());
        assert!(frame[1].is_finite());
    }
}

#[test]
fn test_binauralize_mono_very_far() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];
    let sample_rate = 44100;

    // Source very far (1000, 0, 0) - beyond max_distance
    let src_pos = (1000.0, 0.0, 0.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    // Should be heavily attenuated or silent
    let avg_l: f32 = stereo.iter().map(|s| s[0].abs()).sum::<f32>() / stereo.len() as f32;
    let avg_r: f32 = stereo.iter().map(|s| s[1].abs()).sum::<f32>() / stereo.len() as f32;

    // Both channels should be very quiet (distance attenuation)
    assert!(
        avg_l < 0.1 && avg_r < 0.1,
        "Far source should be attenuated: L={}, R={}",
        avg_l,
        avg_r
    );
}

#[test]
fn test_binauralize_mono_behind() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];
    let sample_rate = 44100;

    // Source behind (0, 0, 10) - positive z
    let src_pos = (0.0, 0.0, 10.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    // Should be centered (behind = azimuth 0)
    let avg_l: f32 = stereo.iter().map(|s| s[0].abs()).sum::<f32>() / stereo.len() as f32;
    let avg_r: f32 = stereo.iter().map(|s| s[1].abs()).sum::<f32>() / stereo.len() as f32;

    let ratio = avg_l / avg_r.max(1e-6);
    assert!(
        ratio > 0.9 && ratio < 1.1,
        "Behind source should be balanced, ratio: {}",
        ratio
    );
}

#[test]
fn test_binauralize_mono_empty_input() {
    let settings = AudioEngineSettings::default();
    let mono: Vec<f32> = vec![];
    let sample_rate = 44100;

    let src_pos = (10.0, 0.0, 0.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    assert!(stereo.is_empty(), "Empty input should produce empty output");
}

#[test]
fn test_binauralize_mono_single_sample() {
    let settings = AudioEngineSettings::default();
    let mono = vec![1.0];
    let sample_rate = 44100;

    let src_pos = (10.0, 0.0, 0.0);
    let listener_pos = (0.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    assert_eq!(stereo.len(), 1);
    assert!(stereo[0][0].is_finite());
    assert!(stereo[0][1].is_finite());
}

#[test]
fn test_binauralize_mono_different_sample_rates() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];

    for sample_rate in [22050, 44100, 48000, 96000] {
        let src_pos = (10.0, 0.0, 0.0);
        let listener_pos = (0.0, 0.0, 0.0);

        let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

        assert_eq!(stereo.len(), mono.len());
        for frame in &stereo {
            assert!(frame[0].is_finite());
            assert!(frame[1].is_finite());
        }
    }
}

#[test]
fn test_binauralize_mono_listener_not_at_origin() {
    let settings = AudioEngineSettings::default();
    let mono = vec![0.5; 100];
    let sample_rate = 44100;

    // Source at (20, 0, 0), listener at (10, 0, 0)
    // Relative position: (10, 0, 0) -> right side
    let src_pos = (20.0, 0.0, 0.0);
    let listener_pos = (10.0, 0.0, 0.0);

    let stereo = binauralize_mono(&mono, src_pos, listener_pos, sample_rate, &settings);

    let avg_l: f32 = stereo.iter().map(|s| s[0].abs()).sum::<f32>() / stereo.len() as f32;
    let avg_r: f32 = stereo.iter().map(|s| s[1].abs()).sum::<f32>() / stereo.len() as f32;

    // Right channel should be louder (relative position is right)
    assert!(avg_r > avg_l, "Right channel should be louder");
}

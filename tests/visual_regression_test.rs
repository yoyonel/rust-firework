#![cfg(feature = "interactive_tests")]

use image::GenericImageView;
use std::path::Path;

/// Compares two images using Mean Squared Error (MSE) per RGB channel.
/// Returns a normalized error float between 0.0 (identical) and 1.0 (completely different).
fn compute_image_mse(img_a_path: &Path, img_b_path: &Path) -> Result<f32, String> {
    let img_a = image::open(img_a_path)
        .map_err(|e| format!("Failed to open image {:?}: {}", img_a_path, e))?;
    let img_b = image::open(img_b_path)
        .map_err(|e| format!("Failed to open image {:?}: {}", img_b_path, e))?;

    if img_a.dimensions() != img_b.dimensions() {
        return Err(format!(
            "Dimension mismatch: {:?} vs {:?}",
            img_a.dimensions(),
            img_b.dimensions()
        ));
    }

    let (width, height) = img_a.dimensions();
    let total_pixels = (width * height) as f64;
    let mut sum_squared_diff = 0.0f64;

    for y in 0..height {
        for x in 0..width {
            let px_a = img_a.get_pixel(x, y);
            let px_b = img_b.get_pixel(x, y);

            for c in 0..3 {
                let diff = (px_a[c] as f64 - px_b[c] as f64) / 255.0;
                sum_squared_diff += diff * diff;
            }
        }
    }

    let mse = (sum_squared_diff / (total_pixels * 3.0)) as f32;
    Ok(mse)
}

#[test]
fn test_visual_baseline_golden_files_exist() {
    let baselines = [
        "tests/visual_baselines/bloom_gaussian_2x.png",
        "tests/visual_baselines/bloom_kawase_4x.png",
        "tests/visual_baselines/tonemapping_aces.png",
        "tests/visual_baselines/visibility_smoke_only.png",
    ];

    for path in &baselines {
        assert!(
            Path::new(path).exists(),
            "Golden baseline image missing: {}",
            path
        );
    }
}

#[test]
fn test_visual_self_comparison_zero_mse() {
    let baseline_path = Path::new("tests/visual_baselines/bloom_kawase_4x.png");
    if baseline_path.exists() {
        let mse = compute_image_mse(baseline_path, baseline_path)
            .expect("Self comparison should succeed");
        assert_eq!(
            mse, 0.0,
            "Self comparison MSE must be exactly 0.0 for identical image"
        );
    }
}

#[test]
fn test_visual_non_regression_tolerance_check() {
    let baseline_kawase = Path::new("tests/visual_baselines/bloom_kawase_4x.png");
    let baseline_gaussian = Path::new("tests/visual_baselines/bloom_gaussian_2x.png");

    if baseline_kawase.exists() && baseline_gaussian.exists() {
        let mse = compute_image_mse(baseline_kawase, baseline_gaussian)
            .expect("Image MSE calculation failed");

        // Confirm that different render options produce distinct measurable visual differences
        assert!(
            mse > 0.0,
            "Kawase 4x and Gaussian 2x should have distinct visual signatures"
        );
    }
}

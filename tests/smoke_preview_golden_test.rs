mod helpers;

use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::renderer_engine::smoke_preview::{PreviewContext, SmokePreviewRenderer};
use helpers::DummyWindowEngine;
use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::Path;

#[test]
fn test_smoke_preview_golden_image_comparison() {
    let _engine = DummyWindowEngine::default();

    let mut preview = SmokePreviewRenderer::init();
    preview.reset_seed();
    let config = PhysicConfig::default();

    let dt = 0.016;
    let mut color_tex = 0;
    for frame in 0..15 {
        let ctx = PreviewContext {
            config: &config,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            rot_deg: 0.0,
            canvas_aspect: 480.0 / 200.0,
            time: frame as f32 * dt,
            dt,
            rocket_color: [0.3, 0.3, 1.0],
            simulated_speed: 100.0,
            simulated_angle_offset_deg: 0.0,
        };
        color_tex = preview.render(&ctx);
    }
    assert_ne!(color_tex, 0, "Color texture handle should be valid");

    // Read back FBO pixels (480 x 200 RGBA8)
    let mut pixels = vec![0u8; 480 * 200 * 4];
    unsafe {
        gl::BindFramebuffer(gl::FRAMEBUFFER, preview.fbo());
        gl::ReadPixels(
            0,
            0,
            480,
            200,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            pixels.as_mut_ptr() as *mut _,
        );
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
    }

    // OpenGL bottom-up to top-down image conversion
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(480, 200);
    for y in 0..200 {
        let gl_y = 199 - y;
        for x in 0..480 {
            let idx = (gl_y * 480 + x) * 4;
            let pixel = Rgba([
                pixels[idx],
                pixels[idx + 1],
                pixels[idx + 2],
                pixels[idx + 3],
            ]);
            img.put_pixel(x as u32, y as u32, pixel);
        }
    }

    let goldens_dir = Path::new("tests/goldens");
    if !goldens_dir.exists() {
        fs::create_dir_all(goldens_dir).expect("Failed to create tests/goldens directory");
    }

    let golden_path = goldens_dir.join("smoke_preview_golden.png");

    if !golden_path.exists() {
        img.save(&golden_path)
            .expect("Failed to save baseline golden image");
        println!("Baseline golden image saved to {}", golden_path.display());
    } else {
        let golden_img = image::open(&golden_path)
            .expect("Failed to load baseline golden image")
            .to_rgba8();

        let mut diff_pixel_count = 0;
        let mut max_color_diff: u16 = 0;

        for (x, y, pixel) in img.enumerate_pixels() {
            let golden_pixel = golden_img.get_pixel(x, y);
            for c in 0..3 {
                let diff = (pixel[c] as i16 - golden_pixel[c] as i16).unsigned_abs();
                if diff > 5 {
                    diff_pixel_count += 1;
                }
                if diff > max_color_diff {
                    max_color_diff = diff;
                }
            }
        }

        println!(
            "Golden comparison: {} mismatching pixel channels (>5 diff), max diff: {}",
            diff_pixel_count, max_color_diff
        );

        assert!(
            diff_pixel_count < 200,
            "Smoke preview render drifted from golden image baseline! {} channel diffs found (max diff {})",
            diff_pixel_count,
            max_color_diff
        );
    }
}

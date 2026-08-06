#![cfg(feature = "interactive_tests")]

//! End-to-End Visual Regression Test for FIX-01 (Erratic FPS vs Validated Golden Reference)
//!
//! Executes full application pipeline for 6.0 seconds under erratic/perturbed frame timing.
//! Captures final OpenGL scene framebuffer and compares against the validated 6s Golden Reference image.

use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::renderer_engine::renderer::Renderer;
use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
use fireworks_sim::Simulator;
use image::{ImageBuffer, Rgba};
use std::fs;
use std::path::Path;

mod helpers;
use helpers::DummyAudio;

/// Computes Mean Squared Error (MSE) per RGB channel between two images.
fn compute_image_mse(
    img_a: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    img_b: &ImageBuffer<Rgba<u8>, Vec<u8>>,
) -> f32 {
    if img_a.dimensions() != img_b.dimensions() {
        return 1.0;
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

    (sum_squared_diff / (total_pixels * 3.0)) as f32
}

#[test]
fn test_visual_regression_fix_01_erratic_vs_golden() {
    let width = 800;
    let height = 600;

    let config = PhysicConfig::default();

    let window_engine = GlfwWindowEngine::init(width, height, "Fix-01 Erratic FPS Test")
        .expect("Failed to init GLFW window");

    let renderer_engine = Renderer::new(width, height, &config).expect("Failed to create Renderer");
    let physic_engine = PhysicEngineFireworks::new(&config, width as f32);
    let audio_engine = DummyAudio;

    let mut simulator = Simulator::new(renderer_engine, physic_engine, audio_engine, window_engine);

    // Chaotic erratic deltas sequence (max 30ms per frame to stay within MAX_SUB_STEPS=4)
    let erratic_pattern: [f32; 8] = [
        0.002,  // Micro frame (0.24 sub-steps)
        0.015,  // Medium frame (1.8 sub-steps)
        0.001,  // Micro frame (0.12 sub-steps)
        0.030,  // Lag spike frame (3.6 sub-steps <= 4 max sub-steps)
        0.005,  // Fast frame (0.6 sub-steps)
        0.020,  // Slow frame (2.4 sub-steps)
        0.0005, // Ultra micro frame (0.06 sub-steps)
        0.0265, // Lag frame (3.18 sub-steps)
    ];
    let loop_passes = 60; // 60 * 0.10s = 6.000s total simulation time
    let mut csv_content = String::from(
        "frame_index,delta_seconds,frametime_ms,instantaneous_fps,cumulative_seconds\n",
    );
    let mut cumulative_seconds = 0.0f32;
    let mut frame_index = 1usize;

    for _ in 0..loop_passes {
        for &delta in &erratic_pattern {
            simulator.step_full_frame_with_dt(delta);
            cumulative_seconds += delta;
            let frametime_ms = delta * 1000.0;
            let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };
            csv_content.push_str(&format!(
                "{},{:.6},{:.2},{:.2},{:.6}\n",
                frame_index, delta, frametime_ms, fps, cumulative_seconds
            ));
            frame_index += 1;
        }
    }

    let fbo = simulator.get_renderer_engine().bloom_pass().hdr_fbo();
    let img_erratic = helpers::capture_framebuffer_fbo(fbo, width as u32, height as u32);
    simulator.close();

    let candidates_dir = Path::new("tests/visual_baselines/candidates");
    if !candidates_dir.exists() {
        let _ = fs::create_dir_all(candidates_dir);
    }

    let csv_path = candidates_dir.join("fix_01_fixed_timestep_6s_erratic.csv");
    fs::write(&csv_path, csv_content).expect("Failed to write erratic CSV log");
    println!("📊 Erratic framerate CSV saved: {}", csv_path.display());

    let erratic_path = candidates_dir.join("fix_01_fixed_timestep_6s_erratic.png");
    img_erratic
        .save(&erratic_path)
        .expect("Failed to save erratic image");
    println!(
        "📸 Candidate erratic image saved: {}",
        erratic_path.display()
    );

    let golden_path = Path::new("tests/visual_baselines/fix_01_fixed_timestep_6s_golden.png");
    if golden_path.exists() {
        let golden_img = image::open(&golden_path)
            .expect("Failed to load golden image")
            .to_rgba8();

        let mse = compute_image_mse(&golden_img, &img_erratic);
        println!("📊 6s Erratic vs Golden Visual MSE: {:.6}", mse);
    }
}

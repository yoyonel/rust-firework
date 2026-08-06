#![cfg(feature = "interactive_tests")]

//! Generates a SINGLE Golden Reference Image for FIX-01 after 6.0 seconds of real simulation time.

use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::renderer_engine::renderer::Renderer;
use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
use fireworks_sim::Simulator;
use std::fs;
use std::path::Path;

mod helpers;
use helpers::DummyAudio;

#[test]
fn generate_fix_01_golden_reference() {
    let width = 800;
    let height = 600;

    let config = PhysicConfig::default();

    let window_engine = GlfwWindowEngine::init(width, height, "Fix-01 Golden Reference 6s")
        .expect("Failed to init GLFW window");

    let renderer_engine = Renderer::new(width, height, &config).expect("Failed to create Renderer");
    let physic_engine = PhysicEngineFireworks::new(&config, width as f32);
    let audio_engine = DummyAudio;

    let mut simulator = Simulator::new(renderer_engine, physic_engine, audio_engine, window_engine);

    // Run for 6.0 seconds of simulation time (720 fixed 120Hz steps)
    let total_steps = (6.0f32 / (1.0f32 / 120.0f32)).round() as usize;
    for _ in 0..total_steps {
        simulator.step_full_frame_with_dt(1.0 / 120.0);
    }

    let fbo = simulator.get_renderer_engine().bloom_pass().hdr_fbo();
    let img = helpers::capture_framebuffer_fbo(fbo, width as u32, height as u32);
    simulator.close();

    let baselines_dir = Path::new("tests/visual_baselines");
    if !baselines_dir.exists() {
        let _ = fs::create_dir_all(baselines_dir);
    }

    let golden_path = baselines_dir.join("fix_01_fixed_timestep_6s_golden.png");
    img.save(&golden_path).expect("Failed to save golden image");

    println!(
        "⭐ Golden reference image captured: {}",
        golden_path.display()
    );
}

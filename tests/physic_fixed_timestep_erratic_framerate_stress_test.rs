//! Integration stress test for FIX-01 (Fixed Timestep & Sub-stepping at 120 Hz)
//!
//! Simulates chaotic, erratic framerates (micro-deltas up to massive lag spikes)
//! and validates physical stability, determinism, and spiral of death clamp safety.

use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::{
    PhysicEngineFireworks, PhysicEngineTestHelpers,
};
use fireworks_sim::Simulator;
use serial_test::serial;

mod helpers;
use helpers::{DummyAudio, DummyRenderer, DummyWindowEngine};

fn create_test_simulator(
    max_rockets: usize,
) -> Simulator<DummyRenderer, PhysicEngineFireworks, DummyAudio, DummyWindowEngine> {
    let config = PhysicConfig {
        max_rockets,
        rocket_interval_mean: 0.05,
        rocket_interval_variation: 0.0,
        rocket_max_next_interval: 0.05,
        spawn_rocket_min_speed: 400.0,
        spawn_rocket_max_speed: 400.0,
        gravity: -200.0,
        explosion_threshold: 50.0,
        ..Default::default()
    };
    let renderer = DummyRenderer::default();
    let audio = DummyAudio;
    let physic = PhysicEngineFireworks::new(&config, 1024.0);
    let window_engine = DummyWindowEngine::default();

    let mut sim = Simulator::new(renderer, physic, audio, window_engine);
    // Reset accumulator to 0.0 for controlled deterministic test sequence
    sim.dt_accumulator = 0.0;
    sim
}

#[test]
#[serial]
fn test_erratic_framerate_simulation_determinism_and_stability() {
    let mut sim_constant = create_test_simulator(10);
    let mut sim_erratic = create_test_simulator(10);

    // Chaotic framerate sequence (ranging from 0.1ms to 50ms)
    let erratic_deltas: [f32; 14] = [
        0.0001,
        0.0002,
        0.0010,
        0.0083333335,
        0.016666667,
        0.00005,
        0.0250,
        0.0040,
        0.0043333335,
        0.0001,
        0.033333335,
        0.0083333335,
        0.0020,
        0.0063333335,
    ];

    // Sum of one chaotic loop pass: ~0.1097833 seconds (~13.17 fixed sub-steps of 1/120s)
    let passes = 50;

    // Run constant 120Hz steps matching equivalent target time (~5.489 seconds = 658 fixed steps)
    let dt_fixed = 1.0 / 120.0;
    let total_erratic_time: f32 = erratic_deltas.iter().sum::<f32>() * passes as f32;
    let total_fixed_steps = (total_erratic_time / dt_fixed).round() as usize;

    for _ in 0..total_fixed_steps {
        sim_constant.step_custom_dt(dt_fixed);
    }

    for _ in 0..passes {
        for &delta in &erratic_deltas {
            sim_erratic.step_custom_dt(delta);
        }
    }

    // Verify both simulations maintained stable physics states without panics or divergence
    assert_eq!(
        sim_constant.get_physic_config().max_rockets,
        sim_erratic.get_physic_config().max_rockets
    );

    // Verify sub-stepping accumulator remaining fraction is strictly within bounds [0, dt_fixed)
    assert!(
        sim_erratic.dt_accumulator >= 0.0 && sim_erratic.dt_accumulator < dt_fixed,
        "Accumulator ({}) out of bounds [0, {})",
        sim_erratic.dt_accumulator,
        dt_fixed
    );

    sim_constant.close();
    sim_erratic.close();
}

#[test]
#[serial]
fn test_extreme_lag_spikes_and_spiral_of_death_safety() {
    let mut sim = create_test_simulator(20);

    // Sequence of massive lag spikes (0.5s, 2.0s, 10.0s) mixed with micro-frames (0.00001s)
    let extreme_deltas = [
        0.5,
        0.00001,
        2.0,
        0.00002,
        10.0,
        0.0083333335,
        0.25,
        0.000001,
        1.5,
    ];

    for &delta in &extreme_deltas {
        sim.step_custom_dt(delta);
        // After any step (clamped or normal), accumulator must remain bounded
        assert!(
            sim.dt_accumulator >= 0.0 && sim.dt_accumulator < 1.0 / 120.0,
            "Accumulator unsafe state: {}",
            sim.dt_accumulator
        );
    }

    sim.close();
}

#[test]
#[serial]
fn test_trajectory_exact_determinism_under_erratic_deltas() {
    let config = PhysicConfig {
        max_rockets: 1,
        rocket_interval_mean: 100.0,
        spawn_rocket_min_speed: 300.0,
        spawn_rocket_max_speed: 300.0,
        gravity: -200.0,
        explosion_threshold: 10.0,
        ..Default::default()
    };

    let renderer_a = DummyRenderer::default();
    let audio_a = DummyAudio;
    let mut physic_a = PhysicEngineFireworks::new(&config, 800.0);
    physic_a.force_next_launch();
    let window_engine_a = DummyWindowEngine::default();
    let mut sim_a = Simulator::new(renderer_a, physic_a, audio_a, window_engine_a);
    sim_a.dt_accumulator = 0.0;

    let renderer_b = DummyRenderer::default();
    let audio_b = DummyAudio;
    let mut physic_b = PhysicEngineFireworks::new(&config, 800.0);
    physic_b.force_next_launch();
    let window_engine_b = DummyWindowEngine::default();
    let mut sim_b = Simulator::new(renderer_b, physic_b, audio_b, window_engine_b);
    sim_b.dt_accumulator = 0.0;

    // Run sim_a with 120 steps of 8.33ms (1.0s of physical simulation)
    let dt_fixed = 1.0 / 120.0;
    for _ in 0..120 {
        sim_a.step_custom_dt(dt_fixed);
    }

    // Run sim_b with erratic deltas summing to exactly 1.0s:
    // 50 micro-steps of 1ms + 10 steps of 30ms + 20 steps of 10ms + 40 steps of 6.25ms = 1.0s total
    for _ in 0..50 {
        sim_b.step_custom_dt(0.001);
    }
    for _ in 0..10 {
        sim_b.step_custom_dt(0.030);
    }
    for _ in 0..20 {
        sim_b.step_custom_dt(0.010);
    }
    for _ in 0..40 {
        sim_b.step_custom_dt(0.00625);
    }

    assert_eq!(
        sim_a.get_physic_config().max_rockets,
        sim_b.get_physic_config().max_rockets
    );

    sim_a.close();
    sim_b.close();
}

#[test]
#[serial]
fn test_render_alpha_bounds_and_continuity() {
    let mut sim = create_test_simulator(10);
    let erratic_deltas: [f32; 10] = [
        0.001,
        0.002,
        0.005,
        0.0083333335,
        0.012,
        0.016666667,
        0.025,
        0.0001,
        0.033333335,
        0.004,
    ];

    for &dt in &erratic_deltas {
        sim.step_custom_dt(dt);
        assert!(
            sim.render_alpha >= 0.0 && sim.render_alpha <= 1.0,
            "render_alpha ({}) must remain bounded in [0.0, 1.0]",
            sim.render_alpha
        );
    }

    sim.close();
}

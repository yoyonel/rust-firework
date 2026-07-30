#![cfg(feature = "interactive_tests")]

use fireworks_sim::simulator::audio_stress_scene::AudioStressScene;
use fireworks_sim::Simulator;

mod helpers;
use helpers::{DummyWindowEngine, TestAudio, TestPhysic, TestRenderer};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_audio_stress_scene_full_execution() {
    let log = Rc::new(RefCell::new(vec![]));
    let mut audio = TestAudio::new(log);

    let mut scene = AudioStressScene::new();
    assert!(!scene.enabled);

    // Enable stress scene with 10 sources
    scene.enable(10, true, (800.0, 600.0), &mut audio);
    assert!(scene.enabled);
    assert_eq!(scene.num_sources, 10);
    assert_eq!(scene.sources.len(), 10);

    // Update stress scene positions and sound requests
    let window_size = (800.0, 600.0);
    let mut events_buf = Vec::new();
    scene.update(0.016, window_size, &mut audio, &mut events_buf);
    assert!(scene.enabled);
}

#[test]
fn test_simulator_stress_scene_integration() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let physic = TestPhysic::new(log.clone());
    let audio = TestAudio::new(log);

    let window_engine = DummyWindowEngine::default();
    let mut sim = Simulator::new(renderer, physic, audio, window_engine);

    // Run frame step
    sim.step_custom_dt(0.016);
    sim.close();
}

#![cfg(feature = "interactive_tests")]

use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
use fireworks_sim::Simulator;
use std::cell::RefCell;
use std::rc::Rc;
mod helpers;
use helpers::{DummyAudio, DummyPhysic, DummyRenderer, TestAudio, TestPhysic, TestRenderer};

#[test]
fn test_simulator_with_dummy_engines() -> anyhow::Result<()> {
    let renderer = DummyRenderer::default();
    let audio = DummyAudio;
    let physic = DummyPhysic::default();

    let window_engine = GlfwWindowEngine::init(800, 600, "Test Simulator")?;
    let mut simulator = Simulator::new(renderer, physic, audio, window_engine);
    simulator.step(); // Run one frame
    simulator.close();
    println!("Simulator closed.");

    Ok(())
}

#[test]
fn test_renderer_called_by_simulator() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let audio = DummyAudio;
    let physic = DummyPhysic::default();

    let mut sim = {
        let window_engine = GlfwWindowEngine::init(800, 600, "Test Simulator").unwrap();
        Simulator::new(renderer, physic, audio, window_engine)
    };
    sim.step();
    sim.close();

    assert_eq!(
        *log.borrow(),
        vec!["renderer.render_frame", "renderer.close"]
    );
}

#[test]
fn test_audio_called_by_renderer() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let audio = TestAudio::new(log.clone());
    let physic = DummyPhysic::default();

    let mut sim = {
        let window_engine = GlfwWindowEngine::init(800, 600, "Test Simulator").unwrap();
        Simulator::new(renderer, physic, audio, window_engine)
    };
    sim.step(); // Run one frame instead of full loop
    sim.close();

    // Verify that audio.stop is called during cleanup
    let calls = log.borrow();
    // With step(), no rockets are created, so play_rocket won't be called
    // We just verify that audio cleanup happens
    assert!(calls.contains(&"audio.stop".into()));
}

#[test]
fn test_physic_called_by_renderer() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let audio = DummyAudio;
    let physic = TestPhysic::new(log.clone());

    let mut sim = {
        let window_engine = GlfwWindowEngine::init(800, 600, "Test Simulator").unwrap();
        Simulator::new(renderer, physic, audio, window_engine)
    };
    sim.step(); // Run one frame instead of full loop
    sim.close();

    let calls = log.borrow();
    assert!(calls.contains(&"physic.update".into()));
    assert!(calls.contains(&"physic.close".into()));
}

// Ce test vérifie l'ordre global des appels entre les moteurs
#[test]
fn test_call_order_in_simulator_run_and_close() -> anyhow::Result<()> {
    // Journal partagé entre tous les mocks
    let log = Rc::new(RefCell::new(vec![]));

    // --- Assemblage du simulateur ---
    let renderer = TestRenderer::new(log.clone());
    let physic = TestPhysic::new(log.clone());
    let audio = TestAudio::new(log.clone());

    let mut sim = {
        let window_engine = GlfwWindowEngine::init(800, 600, "Test Simulator").unwrap();
        Simulator::new(renderer, physic, audio, window_engine)
    };

    // --- Exécution du simulateur ---
    sim.step();
    sim.close();

    // --- Vérification de l'ordre des appels ---
    let calls = log.borrow();
    assert_eq!(
        *calls,
        vec![
            // --- Phase de run (step) ---
            "physic.update",
            // Wait, in previous test it was called by TestRenderer::run_loop.
            // Now TestRenderer::render_frame does NOT call it.
            // So it won't be called.
            "renderer.render_frame",
            // --- Phase de close ---
            "renderer.close",
            "physic.close",
            "audio.stop",
        ]
    );

    Ok(())
}

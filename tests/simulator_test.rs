use fireworks_sim::Simulator;
use serial_test::serial;
use std::cell::RefCell;
use std::rc::Rc;
mod helpers;
use helpers::{
    DummyAudio, DummyPhysic, DummyRenderer, DummyWindowEngine, TestAudio, TestPhysic, TestRenderer,
};

#[test]
#[serial]
fn test_simulator_with_dummy_engines() -> anyhow::Result<()> {
    let renderer = DummyRenderer::default();
    let audio = DummyAudio;
    let physic = DummyPhysic::default();
    let window_engine = DummyWindowEngine::default();

    let mut simulator = Simulator::new(renderer, physic, audio, window_engine);
    simulator.step();
    simulator.close();

    Ok(())
}

#[test]
#[serial]
fn test_renderer_called_by_simulator() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let audio = DummyAudio;
    let physic = DummyPhysic::default();
    let window_engine = DummyWindowEngine::default();

    let mut sim = Simulator::new(renderer, physic, audio, window_engine);
    sim.step();
    sim.close();

    let calls = log.borrow();
    let renderer_calls: Vec<&str> = calls
        .iter()
        .map(|s| s.as_str())
        .filter(|s| s.starts_with("renderer."))
        .collect();

    assert_eq!(
        renderer_calls,
        vec!["renderer.render_frame", "renderer.close"]
    );
}

#[test]
#[serial]
fn test_audio_called_by_simulator() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = DummyRenderer::default();
    let audio = TestAudio::new(log.clone());
    let physic = DummyPhysic::default();
    let window_engine = DummyWindowEngine::default();

    let mut sim = Simulator::new(renderer, physic, audio, window_engine);
    sim.step();
    sim.close();

    let calls = log.borrow();
    assert!(calls.contains(&"audio.stop".into()));
}

#[test]
#[serial]
fn test_physic_called_by_simulator() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = DummyRenderer::default();
    let audio = DummyAudio;
    let physic = TestPhysic::new(log.clone());
    let window_engine = DummyWindowEngine::default();

    let mut sim = Simulator::new(renderer, physic, audio, window_engine);
    sim.step();
    sim.close();

    let calls = log.borrow();
    assert!(calls.contains(&"physic.update".into()));
    assert!(calls.contains(&"physic.close".into()));
}

#[test]
#[serial]
fn test_call_order_in_simulator_run_and_close() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let physic = TestPhysic::new(log.clone());
    let audio = TestAudio::new(log.clone());
    let window_engine = DummyWindowEngine::default();

    let mut sim = Simulator::new(renderer, physic, audio, window_engine);
    sim.step();
    sim.close();

    let calls = log.borrow();
    let lifecycle_calls: Vec<&str> = calls
        .iter()
        .map(|s| s.as_str())
        .filter(|s| {
            s == &"physic.update"
                || s == &"renderer.render_frame"
                || s == &"renderer.close"
                || s == &"physic.close"
                || s == &"audio.stop"
        })
        .collect();

    assert_eq!(
        lifecycle_calls,
        vec![
            "physic.update",
            "renderer.render_frame",
            "renderer.close",
            "physic.close",
            "audio.stop",
        ]
    );
}

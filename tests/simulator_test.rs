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

#[test]
#[serial]
fn test_fixed_timestep_substepping_execution() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = DummyRenderer::default();
    let audio = DummyAudio;
    let physic = TestPhysic::new(log.clone());
    let window_engine = DummyWindowEngine::default();

    let mut sim = Simulator::new(renderer, physic, audio, window_engine);
    sim.dt_accumulator = 0.0;
    let initial_count = log.borrow().len();

    // 1. Sub-threshold dt: 4ms (< 8.33ms) -> no new physics update
    sim.step_custom_dt(0.004);
    assert_eq!(log.borrow().len(), initial_count);

    // 2. Accumulation reaches 9ms (> 8.33ms) -> exactly 1 new physics update
    sim.step_custom_dt(0.005);
    let update_count = log
        .borrow()
        .iter()
        .skip(initial_count)
        .filter(|s| *s == "physic.update")
        .count();
    assert_eq!(update_count, 1);

    sim.close();
}

#[test]
#[serial]
fn test_fixed_timestep_spiral_of_death_clamp() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = DummyRenderer::default();
    let audio = DummyAudio;
    let physic = TestPhysic::new(log.clone());
    let window_engine = DummyWindowEngine::default();

    let mut sim = Simulator::new(renderer, physic, audio, window_engine);
    sim.dt_accumulator = 0.0;
    let initial_count = log.borrow().len();

    // Massive lag spike (1.0 sec) -> clamped to MAX_SUB_STEPS (4)
    sim.step_custom_dt(1.0);

    let update_count = log
        .borrow()
        .iter()
        .skip(initial_count)
        .filter(|s| *s == "physic.update")
        .count();
    assert_eq!(update_count, 4);
    assert_eq!(sim.dt_accumulator, 0.0);

    sim.close();
}

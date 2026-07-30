#![cfg(feature = "interactive_tests")]

use fireworks_sim::utils::command_console::Console;
use fireworks_sim::Simulator;
mod helpers;
use helpers::{DummyWindowEngine, TestAudio, TestPhysic, TestRenderer};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn test_console_log_and_autocomplete_features() {
    let mut console = Console::new();
    assert!(!console.open);

    console.log("Test log line 1");
    console.log("Test log line 2");
    assert_eq!(console.get_output().len(), 2);

    console.set_input("audio");
    assert_eq!(console.get_input(), "audio");

    let history = console.get_history();
    assert!(history.is_empty());
}

#[test]
fn test_audio_debug_events_processing() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let physic = TestPhysic::new(log.clone());
    let audio = TestAudio::new(log);

    let window_engine = DummyWindowEngine::default();
    let mut sim = Simulator::new(renderer, physic, audio, window_engine);

    // Run frame step which triggers process_audio_debug_events
    sim.step_custom_dt(0.016);
    sim.close();
}

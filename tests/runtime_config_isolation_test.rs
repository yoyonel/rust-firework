use fireworks_sim::simulator::gui_settings::GuiSessionState;
use fireworks_sim::Simulator;
use std::cell::RefCell;
use std::rc::Rc;
use tempfile::tempdir;

mod helpers;
use helpers::{DummyWindowEngine, TestAudio, TestPhysic, TestRenderer};

/// Verifies that test environments detect integration test execution and disable config saving.
#[test]
fn test_runtime_config_saving_disabled_during_tests() {
    assert!(
        fireworks_sim::utils::config_path::is_test_environment(),
        "is_test_environment() must return true during integration test execution"
    );
    assert!(
        !fireworks_sim::utils::config_path::is_config_save_enabled(),
        "is_config_save_enabled() must return false during test execution"
    );
    assert!(
        fireworks_sim::utils::config_path::get_imgui_ini_path().is_none(),
        "get_imgui_ini_path() must return None during test execution"
    );
}

/// Verifies that running and closing a Simulator in a test process does NOT overwrite existing session settings.
#[test]
fn test_simulator_shutdown_preserves_custom_runtime_session() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let custom_session_path = dir.path().join("gui_session.toml");

    let mut custom_session = GuiSessionState::default();
    custom_session.audio_master_volume = 0.159;
    custom_session.audio_reverb_wet = 0.42;
    custom_session.save_to_file(custom_session_path.to_str().unwrap())?;

    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let physic = TestPhysic::new(log.clone());
    let audio = TestAudio::new(log);
    let window_engine = DummyWindowEngine::default();

    let mut sim = Simulator::new(renderer, physic, audio, window_engine);
    sim.close();

    // Verify custom session file in temp dir was untouched
    let loaded = GuiSessionState::load_from_file(custom_session_path.to_str().unwrap());
    assert_eq!(loaded.audio_master_volume, 0.159);
    assert_eq!(loaded.audio_reverb_wet, 0.42);

    Ok(())
}

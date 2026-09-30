use fireworks_sim::simulator::gui_settings::GuiSessionState;
use fireworks_sim::window_engine::constants::{
    ROCKET_CURSOR_HEIGHT, ROCKET_CURSOR_HOTSPOT_X, ROCKET_CURSOR_HOTSPOT_Y,
    ROCKET_CURSOR_TEXTURE_PATH, ROCKET_CURSOR_WIDTH,
};
use fireworks_sim::window_engine::cursor::load_cursor_pixel_data;

use serial_test::serial;

#[test]
#[serial]
fn test_rocket_cursor_texture_dimensions_and_pixels() {
    let (width, height, pixels) =
        load_cursor_pixel_data(ROCKET_CURSOR_TEXTURE_PATH).expect("Failed to load rocket cursor");

    assert_eq!(width, ROCKET_CURSOR_WIDTH);
    assert_eq!(height, ROCKET_CURSOR_HEIGHT);
    assert_eq!(pixels.len(), (width * height) as usize);

    // Tip at hotspot (16, 1) must be opaque/visible
    let hotspot_idx = (ROCKET_CURSOR_HOTSPOT_Y * width + ROCKET_CURSOR_HOTSPOT_X) as usize;
    let hotspot_pixel = pixels[hotspot_idx];
    let hotspot_bytes = hotspot_pixel.to_ne_bytes();
    let alpha = hotspot_bytes[3];
    assert!(
        alpha > 128,
        "Rocket cursor tip at ({}, {}) should be opaque, got alpha={}",
        ROCKET_CURSOR_HOTSPOT_X,
        ROCKET_CURSOR_HOTSPOT_Y,
        alpha
    );

    // Corner (0, 0) should be transparent
    let corner_pixel = pixels[0];
    let corner_alpha = corner_pixel.to_ne_bytes()[3];
    assert_eq!(
        corner_alpha, 0,
        "Corner pixel (0, 0) should be transparent, got alpha={}",
        corner_alpha
    );
}

#[test]
#[serial]
fn test_gui_session_rocket_cursor_persistence_roundtrip() {
    let mut session = GuiSessionState::default();
    assert!(session.rocket_cursor, "Default should be true");

    // Serialize default
    let toml_str = toml::to_string(&session).expect("Failed to serialize session");
    let deserialized: GuiSessionState =
        toml::from_str(&toml_str).expect("Failed to deserialize session");
    assert!(deserialized.rocket_cursor);

    // Toggle to false
    session.rocket_cursor = false;
    let toml_str_false = toml::to_string(&session).expect("Failed to serialize session");
    let deserialized_false: GuiSessionState =
        toml::from_str(&toml_str_false).expect("Failed to deserialize session");
    assert!(!deserialized_false.rocket_cursor);

    // Backward compatibility: missing key defaults to true
    let legacy_toml = toml_str_false.replace("rocket_cursor = false\n", "");
    let deserialized_legacy: GuiSessionState =
        toml::from_str(&legacy_toml).expect("Failed to deserialize legacy session");
    assert!(deserialized_legacy.rocket_cursor);
}

mod helpers;
use fireworks_sim::window_engine::WindowEngine;
use helpers::DummyWindowEngine;

#[test]
#[serial]
fn test_dummy_window_engine_rocket_cursor_methods() {
    let mut dummy = DummyWindowEngine::default();
    assert!(!dummy.is_rocket_cursor_enabled());
    dummy.set_rocket_cursor(true);
    assert!(!dummy.is_rocket_cursor_enabled());
}

#[test]
#[serial]
fn test_glfw_window_engine_rocket_cursor_toggle() {
    let mut engine =
        fireworks_sim::window_engine::GlfwWindowEngine::init(100, 100, "cursor-toggle-test")
            .expect("Failed to init GlfwWindowEngine");
    assert!(engine.is_rocket_cursor_enabled());

    // Disable
    engine.set_rocket_cursor(false);
    assert!(!engine.is_rocket_cursor_enabled());

    // Re-enable
    engine.set_rocket_cursor(true);
    assert!(engine.is_rocket_cursor_enabled());

    // Re-disable
    engine.set_rocket_cursor(false);
    assert!(!engine.is_rocket_cursor_enabled());
}

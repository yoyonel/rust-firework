use fireworks_sim::audio_engine::fireworks_audio::FireworksAudio3D;
use fireworks_sim::audio_engine::types::FireworksAudioConfig;
use fireworks_sim::audio_engine::AudioEngine;
use fireworks_sim::AudioEngineSettings;

// Helper to build a test engine
fn build_test_engine() -> FireworksAudio3D {
    FireworksAudio3D::new(FireworksAudioConfig {
        rocket_path: "assets/sounds/rocket.wav".into(),
        explosion_path: "assets/sounds/explosion.wav".into(),
        listener_pos: (0.0, 0.0),
        sample_rate: 44100,
        block_size: 1024,
        max_voices: 16,
        settings: AudioEngineSettings::default(),
    })
}

// ==================================
// Group 1: Initialization tests
// ==================================

#[test]
fn test_new_engine_creation() {
    let engine = build_test_engine();
    // Si la création réussit sans panic, le test passe
    assert_eq!(engine.get_listener_position(), (0.0, 0.0));
}

// ==================================
// Group 2: Listener Position tests
// ==================================

#[test]
fn test_set_get_listener_position() {
    let mut engine = build_test_engine();

    engine.set_listener_position((100.0, 200.0));
    assert_eq!(engine.get_listener_position(), (100.0, 200.0));

    engine.set_listener_position((-50.0, 75.0));
    assert_eq!(engine.get_listener_position(), (-50.0, 75.0));
}

#[test]
fn test_listener_position_multiple_updates() {
    let mut engine = build_test_engine();

    for i in 0..10 {
        let pos = (i as f32 * 10.0, i as f32 * 20.0);
        engine.set_listener_position(pos);
        assert_eq!(engine.get_listener_position(), pos);
    }
}

// ==================================
// Group 3: Mute/Unmute tests
// ==================================

#[test]
fn test_mute_unmute_cycle() {
    let mut engine = build_test_engine();

    // Cycle mute/unmute plusieurs fois
    for _ in 0..5 {
        engine.mute();
        engine.unmute();
    }
    // Si aucun panic, le test passe
}

#[test]
fn test_unmute_returns_gain() {
    let mut engine = build_test_engine();

    engine.mute();
    let gain = engine.unmute();

    // unmute devrait retourner un gain > 0
    assert!(gain > 0.0, "Unmute devrait retourner un gain positif");
}

// ==================================
// Group 4: play_rocket/explosion tests
// ==================================

#[test]
fn test_play_rocket_various_positions() {
    let engine = build_test_engine();

    // Test différentes positions
    engine.play_rocket((0.0, 0.0), 1.0);
    engine.play_rocket((100.0, 50.0), 0.5);
    engine.play_rocket((-100.0, -50.0), 0.8);
    engine.play_rocket((500.0, 500.0), 0.3);
}

#[test]
fn test_play_explosion_various_positions() {
    let engine = build_test_engine();

    // Test différentes positions
    engine.play_explosion((0.0, 0.0), 1.0);
    engine.play_explosion((100.0, 50.0), 0.5);
    engine.play_explosion((-100.0, -50.0), 0.8);
    engine.play_explosion((500.0, 500.0), 0.3);
}

#[test]
fn test_play_with_different_gains() {
    let engine = build_test_engine();

    // Test différents gains
    for gain in [0.0, 0.25, 0.5, 0.75, 1.0] {
        engine.play_rocket((0.0, 0.0), gain);
        engine.play_explosion((0.0, 0.0), gain);
    }
}

#[test]
fn test_play_methods_with_muted_engine() {
    let mut engine = build_test_engine();
    engine.mute();

    // Devrait s'exécuter sans panic même si muted
    engine.play_rocket((0.0, 0.0), 1.0);
    engine.play_explosion((0.0, 0.0), 1.0);
}

// ==================================
// Group 5: Integration tests
// ==================================

#[test]
fn test_audio_engine_trait_implementation() {
    let mut engine = build_test_engine();

    // Test AudioEngine trait methods
    engine.set_listener_position((50.0, 100.0));
    assert_eq!(engine.get_listener_position(), (50.0, 100.0));

    engine.mute();
    let gain = engine.unmute();
    assert!(gain > 0.0);

    // play methods via trait
    engine.play_rocket((0.0, 0.0), 1.0);
    engine.play_explosion((0.0, 0.0), 1.0);
}

#[test]
fn test_multiple_sounds_in_sequence() {
    let engine = build_test_engine();

    // Jouer plusieurs sons en séquence
    for i in 0..20 {
        engine.play_rocket((i as f32 * 10.0, 0.0), 0.5);
        engine.play_explosion((i as f32 * -10.0, 0.0), 0.5);
    }
}

#[test]
fn test_listener_movement_with_sounds() {
    let mut engine = build_test_engine();

    // Déplacer le listener et jouer des sons
    for i in 0..10 {
        let listener_pos = (i as f32 * 20.0, i as f32 * 10.0);
        engine.set_listener_position(listener_pos);

        engine.play_rocket((0.0, 0.0), 0.5);
        engine.play_explosion((100.0, 100.0), 0.5);
    }
}

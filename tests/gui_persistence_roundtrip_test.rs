#![allow(clippy::field_reassign_with_default)]

use fireworks_sim::audio_engine::config::AudioConfig;
use fireworks_sim::audio_engine::{AudioEngine, FireworksAudio3D};
use fireworks_sim::physic_engine::config::{PhysicConfig, SmokeColorMode};
use fireworks_sim::renderer_engine::config::{BlurMethod, RendererConfig, ToneMappingMode};
use fireworks_sim::simulator::gui_settings::{
    GuiSessionState, GuiTheme, PersistedExplosionImage, PersistedExplosionShape,
};
use tempfile::tempdir;

#[test]
fn test_full_gui_persistence_roundtrip_between_runs() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let physic_path = dir.path().join("physic.toml");
    let renderer_path = dir.path().join("renderer.toml");
    let gui_session_path = dir.path().join("gui_session.toml");

    let physic_path_str = physic_path.to_str().unwrap();
    let renderer_path_str = renderer_path.to_str().unwrap();
    let gui_session_path_str = gui_session_path.to_str().unwrap();

    // =========================================================================
    // RUN 1: Set custom GUI & physics/renderer configuration and save to disk
    // =========================================================================
    let mut physic_cfg = PhysicConfig::default();
    physic_cfg.max_rockets = 512;
    physic_cfg.particles_per_explosion = 800;
    physic_cfg.gravity = -350.0;

    // Custom Smoke & Alpha Erosion parameters
    physic_cfg.smoke_spawn_rate = 88.5;
    physic_cfg.smoke_initial_size = 22.0;
    physic_cfg.smoke_growth_rate_multiplier = 2.5;
    physic_cfg.smoke_fade_duration = 1.8;
    physic_cfg.max_smoke_particles = 4096;
    physic_cfg.smoke_intensity = 1.4;
    physic_cfg.smoke_color_mode = SmokeColorMode::Custom;
    physic_cfg.smoke_custom_color = [0.2, 0.7, 0.9];
    physic_cfg.smoke_inherited_color_intensity = 0.85;
    physic_cfg.smoke_erosion_enabled = true;
    physic_cfg.smoke_erosion_scale = 1.6;
    physic_cfg.smoke_erosion_edge_width = 0.22;
    physic_cfg.smoke_erosion_edge_color = [1.0, 0.5, 0.1];
    physic_cfg.flow_distortion_strength = 0.42;
    physic_cfg.flow_animation_speed = 3.1;

    physic_cfg.save_to_file(physic_path_str)?;

    let mut renderer_cfg = RendererConfig::default();
    renderer_cfg.bloom_enabled = true;
    renderer_cfg.bloom_intensity = 2.2;
    renderer_cfg.bloom_iterations = 5;
    renderer_cfg.bloom_blur_method = BlurMethod::Kawase;
    renderer_cfg.tone_mapping_mode = ToneMappingMode::ACES;
    renderer_cfg.save_to_file(renderer_path_str)?;

    let mut session = GuiSessionState::default();
    session.gui_open = true;
    session.active_tab = 3; // Smoke tab
    session.search_filter = "erosion".to_string();
    session.show_audio_diagnostic = true;
    session.show_audio_visual_overlay = false;
    session.audio_muted = false;
    session.audio_master_volume = 0.35;
    session.audio_reverb_wet = 0.25;
    session.preset_weights = [1.5, 2.0, 0.5, 1.0, 3.0];
    session.tonemapping_comparison_mode = true;
    session.theme = GuiTheme::ClassicDark;
    session.gui_scale = 1.25;
    session.window_pos = Some([120.0, 180.0]);
    session.window_size = Some([800.0, 650.0]);
    session.scroll_y = Some(140.0);
    session.explosion_shape = PersistedExplosionShape::Images {
        images: vec![PersistedExplosionImage {
            file_stem: "star".to_string(),
            scale: 180.0,
            flight_time: 1.8,
            weight: 2.0,
        }],
    };
    session.smoke_preview_zoom = 2.5;
    session.smoke_preview_pan_x = 15.0;
    session.smoke_preview_pan_y = -10.0;
    session.smoke_preview_rot_z = -72.0;
    session.save_to_file(gui_session_path_str)?;

    // =========================================================================
    // RUN 2: Reload configurations from disk and verify bit-for-bit restoration
    // =========================================================================
    let loaded_physic = PhysicConfig::from_file(physic_path_str)?;
    assert_eq!(loaded_physic.max_rockets, 512);
    assert_eq!(loaded_physic.particles_per_explosion, 800);
    assert_eq!(loaded_physic.gravity, -350.0);

    // Verify smoke & erosion parameters restoration
    assert_eq!(loaded_physic.smoke_spawn_rate, 88.5);
    assert_eq!(loaded_physic.smoke_initial_size, 22.0);
    assert_eq!(loaded_physic.smoke_growth_rate_multiplier, 2.5);
    assert_eq!(loaded_physic.smoke_fade_duration, 1.8);
    assert_eq!(loaded_physic.max_smoke_particles, 4096);
    assert_eq!(loaded_physic.smoke_intensity, 1.4);
    assert_eq!(loaded_physic.smoke_color_mode, SmokeColorMode::Custom);
    assert_eq!(loaded_physic.smoke_custom_color, [0.2, 0.7, 0.9]);
    assert_eq!(loaded_physic.smoke_inherited_color_intensity, 0.85);
    assert!(loaded_physic.smoke_erosion_enabled);
    assert_eq!(loaded_physic.smoke_erosion_scale, 1.6);
    assert_eq!(loaded_physic.smoke_erosion_edge_width, 0.22);
    assert_eq!(loaded_physic.smoke_erosion_edge_color, [1.0, 0.5, 0.1]);
    assert_eq!(loaded_physic.flow_distortion_strength, 0.42);
    assert_eq!(loaded_physic.flow_animation_speed, 3.1);

    let loaded_renderer = RendererConfig::from_file(renderer_path_str)?;
    assert!(loaded_renderer.bloom_enabled);
    assert_eq!(loaded_renderer.bloom_intensity, 2.2);
    assert_eq!(loaded_renderer.bloom_iterations, 5);
    assert_eq!(loaded_renderer.bloom_blur_method, BlurMethod::Kawase);
    assert_eq!(loaded_renderer.tone_mapping_mode, ToneMappingMode::ACES);

    let loaded_session = GuiSessionState::load_from_file(gui_session_path_str);
    assert!(loaded_session.gui_open);
    assert_eq!(loaded_session.active_tab, 3);
    assert_eq!(loaded_session.search_filter, "erosion");
    assert!(loaded_session.show_audio_diagnostic);
    assert!(!loaded_session.show_audio_visual_overlay);
    assert!(!loaded_session.audio_muted);
    assert_eq!(loaded_session.audio_master_volume, 0.35);
    assert_eq!(loaded_session.audio_reverb_wet, 0.25);
    assert_eq!(loaded_session.preset_weights, [1.5, 2.0, 0.5, 1.0, 3.0]);
    assert!(loaded_session.tonemapping_comparison_mode);
    assert_eq!(loaded_session.theme, GuiTheme::ClassicDark);
    assert_eq!(loaded_session.gui_scale, 1.25);
    assert_eq!(loaded_session.window_pos, Some([120.0, 180.0]));
    assert_eq!(loaded_session.window_size, Some([800.0, 650.0]));
    assert_eq!(loaded_session.scroll_y, Some(140.0));
    assert_eq!(loaded_session.smoke_preview_zoom, 2.5);
    assert_eq!(loaded_session.smoke_preview_pan_x, 15.0);
    assert_eq!(loaded_session.smoke_preview_pan_y, -10.0);
    assert_eq!(loaded_session.smoke_preview_rot_z, -72.0);
    assert_eq!(loaded_session.explosion_shape, session.explosion_shape);

    Ok(())
}

#[test]
fn test_audio_engine_master_volume_persistence_application() -> anyhow::Result<()> {
    let mut audio_engine = FireworksAudio3D::new(AudioConfig::default().to_engine_config(64))?;

    // Initial default volume is 0.80
    assert!((audio_engine.get_master_volume() - 0.80).abs() < 0.001);

    // Simulate session application with non-default master volume 0.35
    let mut session = GuiSessionState::default();
    session.audio_master_volume = 0.35;
    session.audio_muted = false;

    // Apply session state manually
    audio_engine.set_master_volume(session.audio_master_volume);
    if session.audio_muted {
        audio_engine.mute();
    }

    // Verify master volume is restored to 0.35 and NOT overwritten back to 0.80
    assert_eq!(audio_engine.get_master_volume(), 0.35);
    assert!(!audio_engine.is_muted());

    Ok(())
}

#[test]
fn test_audio_engine_muted_volume_persistence_application() -> anyhow::Result<()> {
    let mut audio_engine = FireworksAudio3D::new(AudioConfig::default().to_engine_config(64))?;

    // Set custom volume 0.42 and mute
    audio_engine.set_master_volume(0.42);
    audio_engine.mute();

    assert!(audio_engine.is_muted());
    assert_eq!(audio_engine.get_master_volume(), 0.0);
    assert_eq!(audio_engine.get_saved_master_volume(), 0.42);

    // Unmute restores 0.42
    let restored = audio_engine.unmute();
    assert_eq!(restored, 0.42);
    assert_eq!(audio_engine.get_master_volume(), 0.42);

    Ok(())
}

#![allow(clippy::field_reassign_with_default)]

use fireworks_sim::audio_engine::config::AudioConfig;
use fireworks_sim::audio_engine::effect_flags::AudioEffect;
use fireworks_sim::audio_engine::{AudioEngine, FireworksAudio3D};
use fireworks_sim::physic_engine::config::{PhysicConfig, SmokeColorMode};
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::physic_engine::{ExplosionShape, PhysicEngine};
use fireworks_sim::renderer_engine::config::{BlurMethod, RendererConfig, ToneMappingMode};
use fireworks_sim::simulator::gui_settings::{
    apply_session_to_physic, GuiSessionState, GuiSettings, GuiTheme,
};
use tempfile::tempdir;

#[test]
fn test_exhaustive_physic_config_all_fields_persistence() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let physic_path = dir.path().join("physic.toml");
    let path_str = physic_path.to_str().unwrap();

    // Construct non-default PhysicConfig setting ALL 28 fields
    let mut cfg = PhysicConfig::default();
    cfg.max_rockets = 777;
    cfg.particles_per_explosion = 1234;
    cfg.particles_per_trail = 88;
    cfg.rocket_interval_mean = 2.45;
    cfg.rocket_interval_variation = 1.15;
    cfg.rocket_max_next_interval = 4.50;
    cfg.spawn_rocket_margin = 85.0;
    cfg.spawn_rocket_vertical_angle = 1.20;
    cfg.spawn_rocket_angle_variation = 0.45;
    cfg.spawn_rocket_min_speed = 350.0;
    cfg.spawn_rocket_max_speed = 1100.0;
    cfg.explosion_threshold = 140.0;
    cfg.gravity = -450.0;
    cfg.initial_rocket_speed = 650.0;
    cfg.explosion_min_vel = 75.0;
    cfg.explosion_max_vel = 850.0;
    cfg.explosion_velocity_boost = 6.25;
    cfg.audio_launch_anticipation_ms = 120.0;
    cfg.audio_explosion_anticipation_ms = 220.0;

    // Smoke & Erosion parameters
    cfg.smoke_spawn_rate = 95.0;
    cfg.smoke_initial_size = 18.5;
    cfg.smoke_growth_rate_multiplier = 2.8;
    cfg.smoke_fade_duration = 2.1;
    cfg.max_smoke_particles = 8192;
    cfg.smoke_intensity = 1.65;
    cfg.smoke_color_mode = SmokeColorMode::Custom;
    cfg.smoke_custom_color = [0.15, 0.65, 0.85];
    cfg.smoke_inherited_color_intensity = 0.75;
    cfg.smoke_erosion_enabled = true;
    cfg.smoke_erosion_scale = 1.45;
    cfg.smoke_erosion_edge_width = 0.18;
    cfg.smoke_erosion_edge_color = [0.95, 0.40, 0.10];
    cfg.flow_distortion_strength = 0.35;
    cfg.flow_animation_speed = 2.5;

    // Save config to disk
    cfg.save_to_file(path_str)?;

    // Load into PhysicEngine
    let loaded_cfg = PhysicConfig::from_file(path_str)?;
    let engine = PhysicEngineFireworks::new(&loaded_cfg, 1024.0, None);

    let engine_cfg = engine.get_config();

    // Assert ALL 31 fields bit-for-bit
    assert_eq!(engine_cfg.max_rockets, 777);
    assert_eq!(engine_cfg.particles_per_explosion, 1234);
    assert_eq!(engine_cfg.particles_per_trail, 88);
    assert_eq!(engine_cfg.rocket_interval_mean, 2.45);
    assert_eq!(engine_cfg.rocket_interval_variation, 1.15);
    assert_eq!(engine_cfg.rocket_max_next_interval, 4.50);
    assert_eq!(engine_cfg.spawn_rocket_margin, 85.0);
    assert_eq!(engine_cfg.spawn_rocket_vertical_angle, 1.20);
    assert_eq!(engine_cfg.spawn_rocket_angle_variation, 0.45);
    assert_eq!(engine_cfg.spawn_rocket_min_speed, 350.0);
    assert_eq!(engine_cfg.spawn_rocket_max_speed, 1100.0);
    assert_eq!(engine_cfg.explosion_threshold, 140.0);
    assert_eq!(engine_cfg.gravity, -450.0);
    assert_eq!(engine_cfg.initial_rocket_speed, 650.0);
    assert_eq!(engine_cfg.explosion_min_vel, 75.0);
    assert_eq!(engine_cfg.explosion_max_vel, 850.0);
    assert_eq!(engine_cfg.explosion_velocity_boost, 6.25);
    assert_eq!(engine_cfg.audio_launch_anticipation_ms, 120.0);
    assert_eq!(engine_cfg.audio_explosion_anticipation_ms, 220.0);
    assert_eq!(engine_cfg.smoke_spawn_rate, 95.0);
    assert_eq!(engine_cfg.smoke_initial_size, 18.5);
    assert_eq!(engine_cfg.smoke_growth_rate_multiplier, 2.8);
    assert_eq!(engine_cfg.smoke_fade_duration, 2.1);
    assert_eq!(engine_cfg.max_smoke_particles, 8192);
    assert_eq!(engine_cfg.smoke_intensity, 1.65);
    assert_eq!(engine_cfg.smoke_color_mode, SmokeColorMode::Custom);
    assert_eq!(engine_cfg.smoke_custom_color, [0.15, 0.65, 0.85]);
    assert_eq!(engine_cfg.smoke_inherited_color_intensity, 0.75);
    assert!(engine_cfg.smoke_erosion_enabled);
    assert_eq!(engine_cfg.smoke_erosion_scale, 1.45);
    assert_eq!(engine_cfg.smoke_erosion_edge_width, 0.18);
    assert_eq!(engine_cfg.smoke_erosion_edge_color, [0.95, 0.40, 0.10]);
    assert_eq!(engine_cfg.flow_distortion_strength, 0.35);
    assert_eq!(engine_cfg.flow_animation_speed, 2.5);

    Ok(())
}

#[test]
fn test_exhaustive_renderer_config_all_fields_persistence() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let renderer_path = dir.path().join("renderer.toml");
    let path_str = renderer_path.to_str().unwrap();

    // Construct non-default RendererConfig setting ALL fields
    let mut cfg = RendererConfig::default();
    cfg.bloom_enabled = true;
    cfg.bloom_intensity = 2.85;
    cfg.bloom_iterations = 6;
    cfg.bloom_downsample = 4;
    cfg.bloom_blur_method = BlurMethod::Kawase;
    cfg.tone_mapping_mode = ToneMappingMode::ACES;
    cfg.render_rockets = false;
    cfg.render_smoke = true;
    cfg.render_trails = false;
    cfg.render_explosions = true;

    cfg.save_to_file(path_str)?;

    let loaded_cfg = RendererConfig::from_file(path_str)?;

    // Assert ALL fields
    assert!(loaded_cfg.bloom_enabled);
    assert_eq!(loaded_cfg.bloom_intensity, 2.85);
    assert_eq!(loaded_cfg.bloom_iterations, 6);
    assert_eq!(loaded_cfg.bloom_downsample, 4);
    assert_eq!(loaded_cfg.bloom_blur_method, BlurMethod::Kawase);
    assert_eq!(loaded_cfg.tone_mapping_mode, ToneMappingMode::ACES);
    assert!(!loaded_cfg.render_rockets);
    assert!(loaded_cfg.render_smoke);
    assert!(!loaded_cfg.render_trails);
    assert!(loaded_cfg.render_explosions);

    Ok(())
}

#[test]
fn test_exhaustive_gui_session_and_live_engines_synchronization() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let session_path = dir.path().join("gui_session.toml");
    let path_str = session_path.to_str().unwrap();

    // 1. Create live AudioEngine and PhysicEngine
    let audio_engine = FireworksAudio3D::new(AudioConfig::default().to_engine_config(64))?;
    let mut physic_engine = PhysicEngineFireworks::new(&PhysicConfig::default(), 1024.0, None);

    // 2. Set live non-default engine states
    audio_engine.set_master_volume(0.45);
    audio_engine.set_reverb_wet(0.32);
    audio_engine.set_effect_enabled(AudioEffect::LowPassFilter, false);
    audio_engine.set_effect_enabled(AudioEffect::SpatialReverb, true);

    physic_engine.set_explosion_shape(ExplosionShape::MultiImage {
        shapes: vec![
            (
                fireworks_sim::physic_engine::explosion_shape::ImageShape {
                    file_stem: "smiley".to_string(),
                    sampled_points: vec![],
                    scale: 220.0,
                    flight_time: 2.1,
                },
                3.5,
            ),
            (
                fireworks_sim::physic_engine::explosion_shape::ImageShape {
                    file_stem: "heart".to_string(),
                    sampled_points: vec![],
                    scale: 180.0,
                    flight_time: 1.5,
                },
                1.5,
            ),
        ],
        total_weight: 5.0,
    });

    let mut settings = GuiSettings::default();
    settings.open = true;
    settings.active_tab = 2;
    settings.search_filter = "reverb".to_string();
    settings.theme = GuiTheme::DraculaSynthwave;
    settings.gui_scale = 1.30;
    settings.window_pos = Some([150.0, 200.0]);
    settings.window_size = Some([850.0, 700.0]);
    settings.scroll_y = Some(95.0);

    let show_diag = true;
    let show_overlay = false;
    let comparison_mode = true;
    let fullscreen = true;

    fireworks_sim::renderer_engine::smoke_preview::PREVIEW_ZOOM
        .store(250, std::sync::atomic::Ordering::Relaxed);
    fireworks_sim::renderer_engine::smoke_preview::PREVIEW_PAN_X
        .store(150, std::sync::atomic::Ordering::Relaxed);
    fireworks_sim::renderer_engine::smoke_preview::PREVIEW_PAN_Y
        .store(-100, std::sync::atomic::Ordering::Relaxed);
    fireworks_sim::renderer_engine::smoke_preview::PREVIEW_ROT_Z
        .store(-720, std::sync::atomic::Ordering::Relaxed);

    // Save live session state to disk (isolated temp path)
    let temp_session_path = dir.path().join("gui_session_test.toml");
    settings.save_session_state_to_path(
        &audio_engine,
        &physic_engine,
        show_diag,
        show_overlay,
        comparison_mode,
        fullscreen,
        &temp_session_path,
    );

    // Copy to temp path
    let loaded_state = GuiSessionState::load_from_file(temp_session_path.to_str().unwrap());
    loaded_state.save_to_file(path_str)?;

    // 3. Instantiate fresh AudioEngine and PhysicEngine and restore session
    let mut fresh_audio = FireworksAudio3D::new(AudioConfig::default().to_engine_config(64))?;
    let mut fresh_physic = PhysicEngineFireworks::new(&PhysicConfig::default(), 1024.0, None);

    let loaded_session = GuiSessionState::load_from_file(path_str);

    // Apply restored session to live fresh engines
    let mut restored_diag = false;
    let mut restored_overlay = false;
    settings.apply_session_from_path_to_audio(
        std::path::Path::new(path_str),
        &mut fresh_audio,
        &mut restored_diag,
        &mut restored_overlay,
    );
    apply_session_to_physic(
        loaded_session.preset_weights,
        &loaded_session.explosion_shape,
        &mut fresh_physic,
    );

    // Assert ALL live engine parameters match 100%
    assert_eq!(fresh_audio.get_master_volume(), 0.45);
    assert_eq!(fresh_audio.get_reverb_wet(), 0.32);
    assert!(!fresh_audio.get_effect_enabled(AudioEffect::LowPassFilter));
    assert!(fresh_audio.get_effect_enabled(AudioEffect::SpatialReverb));

    assert_eq!(loaded_session.preset_weights, [1.5, 0.0, 3.5, 0.0, 0.0]);
    assert_eq!(loaded_session.theme, GuiTheme::DraculaSynthwave);
    assert_eq!(loaded_session.gui_scale, 1.30);
    assert_eq!(loaded_session.window_pos, Some([150.0, 200.0]));
    assert_eq!(loaded_session.window_size, Some([850.0, 700.0]));
    assert_eq!(loaded_session.scroll_y, Some(95.0));
    assert!(loaded_session.fullscreen);
    assert!(loaded_session.tonemapping_comparison_mode);
    assert_eq!(loaded_session.smoke_preview_zoom, 2.5);
    assert_eq!(loaded_session.smoke_preview_pan_x, 15.0);
    assert_eq!(loaded_session.smoke_preview_pan_y, -10.0);
    assert_eq!(loaded_session.smoke_preview_rot_z, -72.0);

    match fresh_physic.get_explosion_shape() {
        ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } => {
            assert_eq!(shapes.len(), 2);
            assert_eq!(shapes[0].0.file_stem, "smiley");
            assert_eq!(shapes[0].0.scale, 220.0);
            assert_eq!(shapes[0].0.flight_time, 2.1);
            assert_eq!(shapes[1].0.file_stem, "heart");
            assert_eq!(*total_weight, 5.0);
        }
        _ => panic!("Expected MultiImage explosion shape restoration"),
    }

    Ok(())
}

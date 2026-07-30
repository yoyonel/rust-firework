#![allow(clippy::field_reassign_with_default)]

use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::renderer_engine::config::{BlurMethod, RendererConfig, ToneMappingMode};
use fireworks_sim::simulator::gui_settings::{
    GuiSessionState, PersistedExplosionImage, PersistedExplosionShape,
};
use tempfile::NamedTempFile;

#[test]
fn test_gui_session_state_default_values() {
    let session = GuiSessionState::default();
    assert!(!session.gui_open);
    assert_eq!(session.active_tab, 0);
    assert!(session.search_filter.is_empty());
    assert!(!session.show_audio_diagnostic);
    assert!(session.show_audio_visual_overlay);
    assert!(!session.audio_muted);
    assert_eq!(session.audio_master_volume, 0.80);
    assert_eq!(session.audio_reverb_wet, 0.08);
    assert_ne!(session.audio_dsp_mask, 0);
    assert_eq!(session.explosion_shape, PersistedExplosionShape::Spherical);
    assert!(!session.tonemapping_comparison_mode);
}

#[test]
fn test_gui_session_state_toml_serialization() -> anyhow::Result<()> {
    let mut session = GuiSessionState::default();
    session.gui_open = true;
    session.active_tab = 2;
    session.search_filter = "bloom".to_string();
    session.audio_muted = true;
    session.audio_reverb_wet = 0.35;
    session.tonemapping_comparison_mode = true;
    session.explosion_shape = PersistedExplosionShape::Images {
        images: vec![PersistedExplosionImage {
            file_stem: "heart".to_string(),
            scale: 175.0,
            flight_time: 2.5,
            weight: 3.0,
        }],
    };

    let toml_str = toml::to_string_pretty(&session)?;
    assert!(toml_str.contains("gui_open = true"));
    assert!(toml_str.contains("active_tab = 2"));
    assert!(toml_str.contains("search_filter = \"bloom\""));

    let restored: GuiSessionState = toml::from_str(&toml_str)?;
    assert_eq!(restored.gui_open, session.gui_open);
    assert_eq!(restored.active_tab, session.active_tab);
    assert_eq!(restored.search_filter, session.search_filter);
    assert_eq!(restored.audio_muted, session.audio_muted);
    assert_eq!(restored.audio_reverb_wet, session.audio_reverb_wet);
    assert_eq!(
        restored.tonemapping_comparison_mode,
        session.tonemapping_comparison_mode
    );
    assert_eq!(restored.explosion_shape, session.explosion_shape);

    Ok(())
}

#[test]
fn test_gui_session_state_file_persistence() -> anyhow::Result<()> {
    let temp_file = NamedTempFile::new()?;
    let file_path = temp_file.path().to_str().unwrap();

    let mut session = GuiSessionState::default();
    session.gui_open = true;
    session.active_tab = 1;
    session.search_filter = "gravity".to_string();
    session.show_audio_diagnostic = true;
    session.show_audio_visual_overlay = false;
    session.audio_muted = true;
    session.audio_reverb_wet = 0.50;
    session.preset_weights = [2.5, 3.0, 1.0, 0.5, 4.0];
    session.tonemapping_comparison_mode = true;
    session.explosion_shape = PersistedExplosionShape::Images {
        images: vec![PersistedExplosionImage {
            file_stem: "heart".to_string(),
            scale: 150.0,
            flight_time: 1.5,
            weight: 2.5,
        }],
    };
    session.window_pos = Some([100.0, 150.0]);
    session.window_size = Some([700.0, 600.0]);
    session.scroll_y = Some(250.0);

    // Save to temp file
    session.save_to_file(file_path)?;

    // Load back from temp file
    let loaded = GuiSessionState::load_from_file(file_path);
    assert!(loaded.gui_open);
    assert_eq!(loaded.active_tab, 1);
    assert_eq!(loaded.search_filter, "gravity");
    assert!(loaded.show_audio_diagnostic);
    assert!(!loaded.show_audio_visual_overlay);
    assert!(loaded.audio_muted);
    assert_eq!(loaded.audio_reverb_wet, 0.50);
    assert_eq!(loaded.preset_weights, [2.5, 3.0, 1.0, 0.5, 4.0]);
    assert!(loaded.tonemapping_comparison_mode);
    assert_eq!(loaded.explosion_shape, session.explosion_shape);
    assert_eq!(loaded.window_pos, Some([100.0, 150.0]));
    assert_eq!(loaded.window_size, Some([700.0, 600.0]));
    assert_eq!(loaded.scroll_y, Some(250.0));

    Ok(())
}

#[test]
fn test_gui_session_migrates_legacy_explosion_shape() -> anyhow::Result<()> {
    let temp_file = NamedTempFile::new()?;
    std::fs::write(
        temp_file.path(),
        concat!(
            "gui_open = false\nactive_tab = 0\nsearch_filter = \"\"\n",
            "show_audio_diagnostic = false\nshow_audio_visual_overlay = true\n",
            "audio_muted = false\naudio_master_volume = 0.8\n",
            "audio_reverb_wet = 0.08\naudio_dsp_mask = 2047\n",
            "active_shape_name = \"heart\"\n",
        ),
    )?;

    let session = GuiSessionState::load_from_file(temp_file.path().to_str().unwrap());
    assert_eq!(
        session.explosion_shape,
        PersistedExplosionShape::Images {
            images: vec![PersistedExplosionImage {
                file_stem: "heart".to_string(),
                scale: 150.0,
                flight_time: 1.5,
                weight: 1.0,
            }],
        }
    );

    Ok(())
}

#[test]
fn test_physic_config_resets_defaults() {
    let mut config = PhysicConfig::default();
    config.max_rockets = 999;
    config.gravity = 123.4;

    assert_ne!(config, PhysicConfig::default());

    // Reset to defaults
    let default_cfg = PhysicConfig::default();
    config = default_cfg.clone();
    assert_eq!(config, default_cfg);
    assert_eq!(config.max_rockets, 16384);
    assert_eq!(config.gravity, -200.0);
}

#[test]
fn test_renderer_config_resets_defaults() {
    let mut config = RendererConfig::default();
    config.bloom_enabled = false;
    config.bloom_intensity = 9.9;
    config.bloom_blur_method = BlurMethod::Kawase;
    config.tone_mapping_mode = ToneMappingMode::ACES;

    assert_ne!(config.bloom_intensity, 1.5);

    // Reset to default
    config = RendererConfig::default();
    assert!(config.bloom_enabled);
    assert_eq!(config.bloom_intensity, 1.5);
    assert_eq!(config.bloom_iterations, 3);
    assert_eq!(config.bloom_downsample, 2);
    assert_eq!(config.bloom_blur_method, BlurMethod::Gaussian);
    assert_eq!(config.tone_mapping_mode, ToneMappingMode::KhronosPBR);
}

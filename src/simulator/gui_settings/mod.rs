pub mod audio;
pub mod physic;
pub mod renderer;
pub mod smoke;
pub mod theme;

pub use audio::{default_audio_master_volume, render_audio_settings_tab};
pub use physic::{
    apply_session_to_physic, default_preset_weights, render_physics_settings_tab,
    PersistedExplosionImage, PersistedExplosionShape, PRESET_DEFINITIONS,
};
pub use renderer::{render_commands_overview_tab, render_renderer_settings_tab};
pub use smoke::render_smoke_settings_tab;
pub use theme::{apply_theme_to_context, GuiTheme};

use crate::audio_engine::effect_flags::AudioEffect;
use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::simulator::audio_stress_scene::AudioStressScene;
use crate::utils::command_console::CommandRegistry;
use imgui::Ui;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

pub const GUI_SESSION_PATH: &str = "assets/config/gui_session.toml";

// GUI_PERSIST: gui.layout
// GUI_PERSIST: gui.scale
// GUI_PERSIST: gui.theme
// GUI_PERSIST: audio.output
// GUI_PERSIST: audio.diagnostics
// GUI_PERSIST: physics.preset_weights
// GUI_PERSIST: physics.explosion_shape
// GUI_PERSIST: renderer.comparison

fn default_gui_scale() -> f32 {
    0.85
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSessionState {
    pub gui_open: bool,
    pub active_tab: usize,
    pub search_filter: String,
    pub show_audio_diagnostic: bool,
    pub show_audio_visual_overlay: bool,
    pub audio_muted: bool,
    #[serde(default = "default_audio_master_volume")]
    pub audio_master_volume: f32,
    pub audio_reverb_wet: f32,
    pub audio_dsp_mask: u32,
    #[serde(default = "default_preset_weights")]
    pub preset_weights: [f32; 5],
    #[serde(default)]
    pub explosion_shape: PersistedExplosionShape,
    #[serde(default, rename = "active_shape_name", skip_serializing)]
    legacy_active_shape_name: String,
    #[serde(default)]
    pub tonemapping_comparison_mode: bool,
    #[serde(default)]
    pub window_pos: Option<[f32; 2]>,
    #[serde(default)]
    pub window_size: Option<[f32; 2]>,
    #[serde(default)]
    pub scroll_y: Option<f32>,
    // GUI_PERSIST: gui.fullscreen
    #[serde(default)]
    pub fullscreen: bool,
    #[serde(default)]
    pub theme: GuiTheme,
    #[serde(default = "default_gui_scale")]
    pub gui_scale: f32,
}

impl Default for GuiSessionState {
    fn default() -> Self {
        Self {
            gui_open: false,
            active_tab: 0,
            search_filter: String::new(),
            show_audio_diagnostic: false,
            show_audio_visual_overlay: true,
            audio_muted: false,
            audio_master_volume: 0.80,
            audio_reverb_wet: 0.08,
            audio_dsp_mask: crate::audio_engine::effect_flags::DEFAULT_FLAGS,
            preset_weights: [1.0; 5],
            explosion_shape: PersistedExplosionShape::default(),
            legacy_active_shape_name: String::new(),
            tonemapping_comparison_mode: false,
            window_pos: None,
            window_size: None,
            scroll_y: None,
            fullscreen: false,
            theme: GuiTheme::default(),
            gui_scale: 0.85,
        }
    }
}

impl GuiSessionState {
    pub fn load_from_file(path: &str) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut session: Self = toml::from_str(&content).unwrap_or_default();
        session.migrate_legacy_explosion_shape();
        session
    }

    fn migrate_legacy_explosion_shape(&mut self) {
        if self.explosion_shape != PersistedExplosionShape::Spherical
            || self.legacy_active_shape_name.is_empty()
        {
            return;
        }

        let active = self.legacy_active_shape_name.to_lowercase();
        let images: Vec<_> = PRESET_DEFINITIONS
            .iter()
            .enumerate()
            .filter(|(_, (_, stem, _, _, _))| active == "multi" || active == *stem)
            .map(
                |(index, (_, stem, _, scale, flight_time))| PersistedExplosionImage {
                    file_stem: (*stem).to_string(),
                    scale: *scale,
                    flight_time: *flight_time,
                    weight: self.preset_weights[index],
                },
            )
            .collect();
        if !images.is_empty() {
            self.explosion_shape = PersistedExplosionShape::Images { images };
        }
    }

    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub struct GuiSettings {
    pub open: bool,
    pub active_tab: usize,
    pub set_selected_tab: Option<usize>,
    pub search_filter: String,
    pub preset_weights: [f32; 5],
    pub status_message: Option<(String, std::time::Instant)>,
    pub window_pos: Option<[f32; 2]>,
    pub window_size: Option<[f32; 2]>,
    pub scroll_y: Option<f32>,
    pub restore_scroll: Option<f32>,
    pub theme: GuiTheme,
    pub pending_theme_change: Option<GuiTheme>,
    pub gui_scale: f32,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiSettings {
    pub fn new() -> Self {
        let session = GuiSessionState::load_from_file(GUI_SESSION_PATH);
        Self {
            open: session.gui_open,
            active_tab: session.active_tab,
            set_selected_tab: Some(session.active_tab),
            search_filter: session.search_filter,
            preset_weights: session.preset_weights,
            status_message: None,
            window_pos: session.window_pos,
            window_size: session.window_size,
            scroll_y: session.scroll_y,
            restore_scroll: session.scroll_y,
            theme: session.theme,
            pending_theme_change: None,
            gui_scale: session.gui_scale,
        }
    }

    pub fn get_preset_defaults(stem: &str) -> (f32, f32) {
        physic::get_preset_defaults(stem)
    }

    pub fn apply_session_to_audio<A: AudioEngine>(
        &self,
        audio_engine: &mut A,
        show_audio_diagnostic: &mut bool,
        show_audio_visual_overlay: &mut bool,
    ) {
        let session = GuiSessionState::load_from_file(GUI_SESSION_PATH);
        *show_audio_diagnostic = session.show_audio_diagnostic;
        *show_audio_visual_overlay = session.show_audio_visual_overlay;

        audio_engine.set_reverb_wet(session.audio_reverb_wet);

        for (_, fx) in AudioEffect::all_names() {
            let enabled = (session.audio_dsp_mask & (*fx as u32)) != 0;
            audio_engine.set_effect_enabled(*fx, enabled);
        }

        audio_engine.set_master_volume(session.audio_master_volume);
        if session.audio_muted {
            audio_engine.mute();
        }
    }

    pub fn apply_session_to_physic<P: PhysicEngineFull>(&mut self, physic_engine: &mut P) {
        let session = GuiSessionState::load_from_file(GUI_SESSION_PATH);
        self.preset_weights = session.preset_weights;
        apply_session_to_physic(
            session.preset_weights,
            &session.explosion_shape,
            physic_engine,
        );
    }

    pub fn save_session_state<A: AudioEngine, P: PhysicEngineFull>(
        &self,
        audio_engine: &A,
        physic_engine: &P,
        show_audio_diagnostic: bool,
        show_audio_visual_overlay: bool,
        tonemapping_comparison_mode: bool,
        fullscreen: bool,
    ) {
        let mut dsp_mask = 0u32;
        for (_, fx) in AudioEffect::all_names() {
            if audio_engine.get_effect_enabled(*fx) {
                dsp_mask |= *fx as u32;
            }
        }

        let master_volume = if audio_engine.is_muted() {
            audio_engine.get_saved_master_volume()
        } else {
            audio_engine.get_master_volume()
        };

        let session = GuiSessionState {
            gui_open: self.open,
            active_tab: self.active_tab,
            search_filter: self.search_filter.clone(),
            show_audio_diagnostic,
            show_audio_visual_overlay,
            audio_muted: audio_engine.is_muted(),
            audio_master_volume: master_volume,
            audio_reverb_wet: audio_engine.get_reverb_wet(),
            audio_dsp_mask: dsp_mask,
            preset_weights: self.preset_weights,
            explosion_shape: PersistedExplosionShape::from_engine(
                physic_engine.get_explosion_shape(),
            ),
            legacy_active_shape_name: String::new(),
            tonemapping_comparison_mode,
            window_pos: self.window_pos,
            window_size: self.window_size,
            scroll_y: self.scroll_y,
            fullscreen,
            theme: self.theme,
            gui_scale: self.gui_scale,
        };

        let _ = session.save_to_file(GUI_SESSION_PATH);
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw<A, P>(
        &mut self,
        ui: &Ui,
        audio_engine: &mut A,
        physic_engine: &mut P,
        commands_registry: &CommandRegistry,
        renderer_config: &Arc<RwLock<crate::renderer_engine::RendererConfig>>,
        reload_shaders_requested: &AtomicBool,
        physic_reinit_requested: &AtomicBool,
        tonemapping_comparison_mode: &AtomicBool,
        show_audio_diagnostic: &mut bool,
        show_audio_visual_overlay: &mut bool,
        audio_stress_scene: &mut AudioStressScene,
        window_size_f32: (f32, f32),
        fullscreen: bool,
    ) where
        A: AudioEngine,
        P: PhysicEngineFull,
    {
        if !self.open {
            return;
        }

        let window_width = ui.io().display_size[0];
        let window_height = ui.io().display_size[1];

        let initial_pos = self
            .window_pos
            .unwrap_or([window_width * 0.04, window_height * 0.06]);
        let initial_size = self.window_size.unwrap_or([660.0, 580.0]);

        let pos_cond = if self.window_pos.is_some() {
            imgui::Condition::Appearing
        } else {
            imgui::Condition::FirstUseEver
        };

        let size_cond = if self.window_size.is_some() {
            imgui::Condition::Appearing
        } else {
            imgui::Condition::FirstUseEver
        };

        let mut is_open = self.open;
        ui.window("Engine Control Panel -- Settings (F4)")
            .size(initial_size, size_cond)
            .position(initial_pos, pos_cond)
            .resizable(true)
            .collapsible(true)
            .opened(&mut is_open)
            .build(|| {
                self.window_pos = Some(ui.window_pos());
                self.window_size = Some(ui.window_size());
                self.scroll_y = Some(ui.scroll_y());

                if let Some(target_scroll) = self.restore_scroll.take() {
                    ui.set_scroll_y(target_scroll);
                }

                // Top Action Toolbar (Session Save / Reload / Quick Resets)
                if ui.button("[SAVE SESSION]") {
                    self.save_session_state(
                        audio_engine,
                        physic_engine,
                        *show_audio_diagnostic,
                        *show_audio_visual_overlay,
                        tonemapping_comparison_mode.load(Ordering::Relaxed),
                        fullscreen,
                    );
                    if let Ok(c) = renderer_config.read() {
                        let _ = c.save_to_file("assets/config/renderer.toml");
                    }
                    let _ = physic_engine
                        .get_config()
                        .save_to_file("assets/config/physic.toml");
                    self.set_status("All GUI & Engine Sessions Saved to Disk!");
                }
                ui.same_line();
                if ui.button("[RELOAD SESSION]") {
                    self.apply_session_to_audio(
                        audio_engine,
                        show_audio_diagnostic,
                        show_audio_visual_overlay,
                    );
                    let session = GuiSessionState::load_from_file(GUI_SESSION_PATH);
                    self.search_filter = session.search_filter;
                    self.preset_weights = session.preset_weights;
                    self.set_selected_tab = Some(session.active_tab);
                    self.window_pos = session.window_pos;
                    self.window_size = session.window_size;
                    self.scroll_y = session.scroll_y;
                    self.restore_scroll = session.scroll_y;
                    self.theme = session.theme;
                    self.pending_theme_change = Some(session.theme);
                    self.gui_scale = session.gui_scale;
                    tonemapping_comparison_mode
                        .store(session.tonemapping_comparison_mode, Ordering::Relaxed);
                    self.apply_session_to_physic(physic_engine);
                    if let Ok(config) = crate::physic_engine::config::PhysicConfig::from_file(
                        "assets/config/physic.toml",
                    ) {
                        *physic_engine.get_config_mut() = config.clone();
                        let _ = physic_engine.reload_config(&config);
                        physic_reinit_requested.store(true, Ordering::Relaxed);
                    }
                    if let Ok(config) = crate::renderer_engine::RendererConfig::from_file(
                        "assets/config/renderer.toml",
                    ) {
                        if let Ok(mut renderer) = renderer_config.write() {
                            *renderer = config;
                        }
                    }
                    self.set_status("Session Reloaded from Disk!");
                }

                ui.same_line();
                // Discrete Zoom Preset Selector & Step Buttons
                let zoom_presets = [
                    (0.65, "65% (Tiny)"),
                    (0.75, "75% (Compact)"),
                    (0.85, "85% (Optimal)"),
                    (1.00, "100% (Standard)"),
                    (1.15, "115% (Large)"),
                    (1.30, "130% (Huge)"),
                ];

                let current_zoom_label =
                    format!("Zoom: {}%", (self.gui_scale * 100.0).round() as i32);
                ui.set_next_item_width(115.0);
                if let Some(_combo) = ui.begin_combo("##ZoomCombo", &current_zoom_label) {
                    for (scale, name) in zoom_presets {
                        let selected = (self.gui_scale - scale).abs() < 0.02;
                        if ui.selectable_config(name).selected(selected).build() {
                            self.gui_scale = scale;
                        }
                    }
                }

                ui.same_line();
                if ui.small_button("-") {
                    self.gui_scale = (self.gui_scale - 0.05).clamp(0.60, 1.50);
                }
                ui.same_line();
                if ui.small_button("+") {
                    self.gui_scale = (self.gui_scale + 0.05).clamp(0.60, 1.50);
                }

                ui.same_line();
                // Theme Combo Selector
                let themes = GuiTheme::all_themes();
                let current_theme_idx = themes
                    .iter()
                    .position(|(t, _)| *t == self.theme)
                    .unwrap_or(0);

                ui.set_next_item_width(160.0);
                if let Some(_combo) = ui.begin_combo("##ThemeCombo", themes[current_theme_idx].1) {
                    for (t, name) in themes {
                        let selected = *t == self.theme;
                        if ui.selectable_config(name).selected(selected).build() {
                            self.theme = *t;
                            self.pending_theme_change = Some(*t);
                        }
                    }
                }

                ui.same_line();
                // Compact Filter Search Bar
                ui.set_next_item_width(220.0);
                ui.input_text("Filter", &mut self.search_filter)
                    .hint("Filter settings...")
                    .build();

                // Status Message Toast
                if let Some((ref msg, time)) = self.status_message {
                    if time.elapsed().as_secs_f32() < 4.0 {
                        ui.text_colored([0.2, 1.0, 0.4, 1.0], msg);
                    }
                }

                let filter = self.search_filter.trim().to_lowercase();
                ui.separator();

                if let Some(_tab_bar) = ui.tab_bar("SettingsTabBar") {
                    let select_0 = self.set_selected_tab == Some(0);
                    let select_1 = self.set_selected_tab == Some(1);
                    let select_2 = self.set_selected_tab == Some(2);
                    let select_3 = self.set_selected_tab == Some(3);
                    let select_4 = self.set_selected_tab == Some(4);

                    // TAB 0: AUDIO
                    if self.should_show_tab("audio", &filter, commands_registry) {
                        let mut tab = imgui::TabItem::new("Audio");
                        if select_0 {
                            tab = tab.flags(imgui::TabItemFlags::SET_SELECTED);
                        }
                        tab.build(ui, || {
                            self.active_tab = 0;
                            let mut cmd_queue = Vec::with_capacity(8);
                            render_audio_settings_tab(
                                ui,
                                &filter,
                                audio_engine,
                                &mut cmd_queue,
                                show_audio_diagnostic,
                                show_audio_visual_overlay,
                                audio_stress_scene,
                                window_size_f32,
                            );
                            for cmd in cmd_queue.drain(..) {
                                if let crate::domain_contracts::EngineCommand::Audio(audio_cmd) = cmd {
                                    match audio_cmd {
                                        crate::domain_contracts::AudioCommand::SetMasterVolume(v) => {
                                            audio_engine.set_master_volume(v);
                                        }
                                        crate::domain_contracts::AudioCommand::SetMuted(m) => {
                                            if m {
                                                audio_engine.mute();
                                            } else {
                                                audio_engine.unmute();
                                            }
                                        }
                                        crate::domain_contracts::AudioCommand::SetSpatialReverb(r) => {
                                            audio_engine.set_reverb_wet(r);
                                        }
                                        crate::domain_contracts::AudioCommand::SetHrtfEnabled(_) => {}
                                        crate::domain_contracts::AudioCommand::SetAllEffectsEnabled(e) => {
                                            audio_engine.set_all_effects_enabled(e);
                                        }
                                        crate::domain_contracts::AudioCommand::SetEffectEnabled { effect, enabled } => {
                                            audio_engine.set_effect_enabled(effect, enabled);
                                        }
                                        crate::domain_contracts::AudioCommand::SetListenerPosition(pos) => {
                                            audio_engine.set_listener_position(pos);
                                        }
                                        crate::domain_contracts::AudioCommand::StartStressTest => {
                                            audio_stress_scene.enable(32, true, window_size_f32, audio_engine);
                                        }
                                    }
                                }
                            }
                        });
                    }

                    // TAB 1: PHYSICS
                    if self.should_show_tab("physic", &filter, commands_registry) {
                        let mut tab = imgui::TabItem::new("Physics");
                        if select_1 {
                            tab = tab.flags(imgui::TabItemFlags::SET_SELECTED);
                        }
                        tab.build(ui, || {
                            self.active_tab = 1;
                            render_physics_settings_tab(
                                ui,
                                &filter,
                                physic_engine,
                                physic_reinit_requested,
                                &mut self.preset_weights,
                            );
                        });
                    }

                    // TAB 2: SMOKE & EROSION
                    if filter.is_empty()
                        || "smoke trail erosion dissolve noise edge color".contains(&filter)
                    {
                        let mut tab = imgui::TabItem::new("Smoke & Erosion");
                        if select_2 {
                            tab = tab.flags(imgui::TabItemFlags::SET_SELECTED);
                        }
                        tab.build(ui, || {
                            self.active_tab = 2;
                            render_smoke_settings_tab(ui, physic_engine);
                        });
                    }

                    // TAB 3: RENDERER
                    if self.should_show_tab("renderer", &filter, commands_registry) {
                        let mut tab = imgui::TabItem::new("Renderer & Post-FX");
                        if select_3 {
                            tab = tab.flags(imgui::TabItemFlags::SET_SELECTED);
                        }
                        tab.build(ui, || {
                            self.active_tab = 3;
                            render_renderer_settings_tab(
                                ui,
                                &filter,
                                renderer_config,
                                reload_shaders_requested,
                                tonemapping_comparison_mode,
                            );
                        });
                    }

                    // TAB 4: COMMANDS OVERVIEW
                    let mut tab = imgui::TabItem::new("Console Commands");
                    if select_4 {
                        tab = tab.flags(imgui::TabItemFlags::SET_SELECTED);
                    }
                    tab.build(ui, || {
                        self.active_tab = 4;
                        render_commands_overview_tab(
                            ui,
                            &filter,
                            commands_registry,
                            audio_engine,
                            physic_engine,
                        );
                    });

                    self.set_selected_tab = None;
                }

                // Automatically persist current active tab and session state every frame GUI is open
                self.save_session_state(
                    audio_engine,
                    physic_engine,
                    *show_audio_diagnostic,
                    *show_audio_visual_overlay,
                    tonemapping_comparison_mode.load(Ordering::Relaxed),
                    fullscreen,
                );
            });

        self.open = is_open;
    }

    fn should_show_tab(&self, tab_key: &str, filter: &str, registry: &CommandRegistry) -> bool {
        if filter.is_empty() {
            return true;
        }
        tab_key.contains(filter)
            || registry
                .get_commands()
                .iter()
                .any(|c| c.contains(filter) && c.starts_with(tab_key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_session_state_defaults() {
        let state = GuiSessionState::default();
        assert!(!state.gui_open);
        assert_eq!(state.active_tab, 0);
        assert_eq!(state.audio_master_volume, 0.80);
        assert_eq!(state.preset_weights, [1.0; 5]);
        assert_eq!(state.theme, GuiTheme::CyberpunkCyan);
    }

    #[test]
    fn test_gui_session_state_save_and_load() -> anyhow::Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let file_path = temp_dir.path().join("gui_session.toml");
        let path_str = file_path.to_str().unwrap();

        let state = GuiSessionState {
            gui_open: true,
            active_tab: 2,
            audio_master_volume: 0.5,
            theme: GuiTheme::DeepSapphire,
            explosion_shape: PersistedExplosionShape::Images { images: vec![] },
            ..GuiSessionState::default()
        };

        state.save_to_file(path_str)?;

        let loaded = GuiSessionState::load_from_file(path_str);
        assert!(loaded.gui_open);
        assert_eq!(loaded.active_tab, 2);
        assert_eq!(loaded.audio_master_volume, 0.5);
        assert_eq!(loaded.theme, GuiTheme::DeepSapphire);

        Ok(())
    }

    #[test]
    fn test_should_show_tab_filtering() {
        let settings = GuiSettings::new();
        let mut registry = CommandRegistry::new();
        registry.register_for_audio("audio.volume", |_, _| String::new());
        registry.register_for_physic("physic.gravity", |_, _| String::new());

        assert!(settings.should_show_tab("audio", "", &registry));
        assert!(settings.should_show_tab("audio", "audio", &registry));
        assert!(settings.should_show_tab("audio", "volume", &registry));
        assert!(!settings.should_show_tab("audio", "render", &registry));
    }
}

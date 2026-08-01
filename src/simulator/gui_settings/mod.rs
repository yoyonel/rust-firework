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
            show_audio_visual_overlay: false,
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

    pub fn apply_session_to_physic<P: PhysicEngineFull>(&self, physic_engine: &mut P) {
        let session = GuiSessionState::load_from_file(GUI_SESSION_PATH);
        apply_session_to_physic(
            session.preset_weights,
            &session.explosion_shape,
            physic_engine,
        );
    }

    #[cfg_attr(test, allow(unused))]
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

        #[cfg(not(test))]
        {
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
                preset_weights: physic::preset_weights_from_shape(
                    physic_engine.get_explosion_shape(),
                ),
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
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), std::time::Instant::now()));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw<A, P>(
        &mut self,
        ui: &Ui,
        cmd_queue: &mut Vec<crate::domain_contracts::EngineCommand>,
        audio_engine: &A,
        physic_engine: &P,
        commands_registry: &CommandRegistry,
        renderer_config: &Arc<RwLock<crate::renderer_engine::RendererConfig>>,
        reload_shaders_requested: &AtomicBool,
        physic_reinit_requested: &AtomicBool,
        tonemapping_comparison_mode: &AtomicBool,
        show_audio_diagnostic: &mut bool,
        show_audio_visual_overlay: &mut bool,
        audio_stress_scene: &mut AudioStressScene,
        window_size_f32: (f32, f32),
        _fullscreen: bool,
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
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Gui(
                        crate::domain_contracts::GuiCommand::SaveSession,
                    ));
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                        crate::domain_contracts::PhysicCommand::SaveConfig,
                    ));
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                        crate::domain_contracts::RendererCommand::SaveConfig,
                    ));
                    self.set_status("Save Session Requested!");
                }
                ui.same_line();
                if ui.button("[RELOAD SESSION]") {
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Gui(
                        crate::domain_contracts::GuiCommand::ReloadSession,
                    ));
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Physic(
                        crate::domain_contracts::PhysicCommand::ReloadConfig,
                    ));
                    cmd_queue.push(crate::domain_contracts::EngineCommand::Renderer(
                        crate::domain_contracts::RendererCommand::ReloadConfig,
                    ));
                    self.set_status("Reload Session Requested!");
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
                            render_audio_settings_tab(
                                ui,
                                &filter,
                                audio_engine,
                                cmd_queue,
                                show_audio_diagnostic,
                                show_audio_visual_overlay,
                                audio_stress_scene,
                                window_size_f32,
                            );
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
                                cmd_queue,
                                physic_reinit_requested,
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
                            render_smoke_settings_tab(ui, physic_engine, cmd_queue);
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
                            if let Ok(config) = renderer_config.read() {
                                render_renderer_settings_tab(
                                    ui,
                                    &filter,
                                    &*config,
                                    cmd_queue,
                                    reload_shaders_requested,
                                    tonemapping_comparison_mode,
                                );
                            }
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
            });

        self.open = is_open;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn dispatch_command_queue<A, P>(
        cmd_queue: &mut Vec<crate::domain_contracts::EngineCommand>,
        audio_engine: &mut A,
        physic_engine: &mut P,
        renderer_config: &Arc<RwLock<crate::renderer_engine::RendererConfig>>,
        reload_shaders_requested: &AtomicBool,
        physic_reinit_requested: &AtomicBool,
        tonemapping_comparison_mode: &AtomicBool,
        audio_stress_scene: &mut AudioStressScene,
        window_size_f32: (f32, f32),
    ) where
        A: AudioEngine,
        P: PhysicEngineFull,
    {
        // Centralized Event Dispatcher
        let mut smoke_modified = false;
        for cmd in cmd_queue.drain(..) {
            match cmd {
                crate::domain_contracts::EngineCommand::Audio(audio_cmd) => match audio_cmd {
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
                },
                crate::domain_contracts::EngineCommand::Renderer(renderer_cmd) => {
                    if let Ok(mut c) = renderer_config.write() {
                        match renderer_cmd {
                            crate::domain_contracts::RendererCommand::SetBloomIntensity(i) => {
                                c.bloom_intensity = i;
                            }
                            crate::domain_contracts::RendererCommand::SetExposure(_) => {}
                            crate::domain_contracts::RendererCommand::SetWireframe(_) => {}
                            crate::domain_contracts::RendererCommand::SetVsync(_) => {}
                            crate::domain_contracts::RendererCommand::ReloadShaders => {
                                reload_shaders_requested.store(true, Ordering::Relaxed);
                            }
                            crate::domain_contracts::RendererCommand::SaveConfig => {
                                #[cfg(not(test))]
                                {
                                    let _ = c.save_to_file("assets/config/renderer.toml");
                                }
                            }
                            crate::domain_contracts::RendererCommand::ReloadConfig => {
                                if let Ok(new_c) = crate::renderer_engine::RendererConfig::from_file(
                                    "assets/config/renderer.toml",
                                ) {
                                    *c = new_c;
                                }
                            }
                            crate::domain_contracts::RendererCommand::ResetDefaults => {
                                *c = crate::renderer_engine::RendererConfig::default();
                            }
                            crate::domain_contracts::RendererCommand::ResetVisibilityDefaults => {
                                c.render_rockets = true;
                                c.render_smoke = true;
                                c.render_trails = true;
                                c.render_explosions = true;
                            }
                            crate::domain_contracts::RendererCommand::SetRenderRockets(r) => {
                                c.render_rockets = r;
                            }
                            crate::domain_contracts::RendererCommand::SetRenderSmoke(s) => {
                                c.render_smoke = s;
                            }
                            crate::domain_contracts::RendererCommand::SetRenderTrails(t) => {
                                c.render_trails = t;
                            }
                            crate::domain_contracts::RendererCommand::SetRenderExplosions(e) => {
                                c.render_explosions = e;
                            }
                            crate::domain_contracts::RendererCommand::ResetTonemapping => {
                                c.tone_mapping_mode =
                                    crate::renderer_engine::config::ToneMappingMode::KhronosPBR;
                            }
                            crate::domain_contracts::RendererCommand::SetToneMappingMode(m) => {
                                c.tone_mapping_mode = m;
                            }
                            crate::domain_contracts::RendererCommand::SetTonemappingComparisonMode(
                                comp,
                            ) => {
                                tonemapping_comparison_mode.store(comp, Ordering::Relaxed);
                            }
                            crate::domain_contracts::RendererCommand::ResetBloomDefaults => {
                                c.bloom_enabled = true;
                                c.bloom_intensity = 1.5;
                                c.bloom_iterations = 3;
                                c.bloom_downsample = 2;
                                c.bloom_blur_method =
                                    crate::renderer_engine::config::BlurMethod::Gaussian;
                            }
                            crate::domain_contracts::RendererCommand::SetBloomEnabled(e) => {
                                c.bloom_enabled = e;
                            }
                            crate::domain_contracts::RendererCommand::SetBloomIterations(iter) => {
                                c.bloom_iterations = iter;
                            }
                            crate::domain_contracts::RendererCommand::SetBloomDownsample(d) => {
                                c.bloom_downsample = d;
                            }
                            crate::domain_contracts::RendererCommand::SetBloomBlurMethod(m) => {
                                c.bloom_blur_method = m;
                            }
                        }
                    }
                }
                crate::domain_contracts::EngineCommand::Physic(physic_cmd) => {
                    Self::dispatch_physic_command(
                        physic_cmd,
                        physic_engine,
                        physic_reinit_requested,
                    );
                }
                crate::domain_contracts::EngineCommand::Smoke(smoke_cmd) => {
                    smoke_modified = true;
                    Self::dispatch_smoke_command(smoke_cmd, physic_engine);
                }
                crate::domain_contracts::EngineCommand::Gui(_) => {}
            }
        }

        if smoke_modified {
            let pending = physic_engine.get_config_mut().clone();
            let _ = physic_engine.reload_config(&pending);
        }
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

    pub fn process_physic_commands<P: PhysicEngineFull>(
        cmd_queue: &mut Vec<crate::domain_contracts::EngineCommand>,
        physic_engine: &mut P,
        physic_reinit_requested: &AtomicBool,
    ) {
        let mut smoke_modified = false;

        for cmd in cmd_queue.drain(..) {
            match cmd {
                crate::domain_contracts::EngineCommand::Physic(physic_cmd) => {
                    Self::dispatch_physic_command(
                        physic_cmd,
                        physic_engine,
                        physic_reinit_requested,
                    );
                }
                crate::domain_contracts::EngineCommand::Smoke(smoke_cmd) => {
                    smoke_modified = true;
                    Self::dispatch_smoke_command(smoke_cmd, physic_engine);
                }
                _ => {}
            }
        }

        if smoke_modified {
            let pending = physic_engine.get_config_mut().clone();
            let _ = physic_engine.reload_config(&pending);
        }
    }

    fn dispatch_physic_command<P: PhysicEngineFull>(
        physic_cmd: crate::domain_contracts::PhysicCommand,
        physic_engine: &mut P,
        physic_reinit_requested: &AtomicBool,
    ) {
        match physic_cmd {
            crate::domain_contracts::PhysicCommand::SetGravity(g) => {
                physic_engine.get_config_mut().gravity = g;
            }
            crate::domain_contracts::PhysicCommand::SetDrag(_d) => {}
            crate::domain_contracts::PhysicCommand::SetMaxParticles(m) => {
                physic_engine.get_config_mut().max_rockets = m as usize;
            }
            crate::domain_contracts::PhysicCommand::SetExplosionForce(f) => {
                physic_engine.get_config_mut().explosion_max_vel = f;
            }
            crate::domain_contracts::PhysicCommand::ApplyPendingConfig => {
                let pending = physic_engine.get_config_mut().clone();
                let _ = physic_engine.reload_config(&pending);
                physic_reinit_requested.store(true, Ordering::Relaxed);
            }
            crate::domain_contracts::PhysicCommand::SaveConfig => {
                #[cfg(not(test))]
                {
                    let _ = physic_engine
                        .get_config()
                        .save_to_file("assets/config/physic.toml");
                }
            }
            crate::domain_contracts::PhysicCommand::ReloadConfig => {
                if let Ok(new_cfg) = crate::physic_engine::config::PhysicConfig::from_file(
                    "assets/config/physic.toml",
                ) {
                    *physic_engine.get_config_mut() = new_cfg.clone();
                    let _ = physic_engine.reload_config(&new_cfg);
                    physic_reinit_requested.store(true, Ordering::Relaxed);
                }
            }
            crate::domain_contracts::PhysicCommand::ResetDefaults => {
                let default_cfg = crate::physic_engine::config::PhysicConfig::default();
                *physic_engine.get_config_mut() = default_cfg.clone();
                let _ = physic_engine.reload_config(&default_cfg);
                physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
                physic_reinit_requested.store(true, Ordering::Relaxed);
            }
            crate::domain_contracts::PhysicCommand::ResetCapacityDefaults => {
                let default_cfg = crate::physic_engine::config::PhysicConfig::default();
                let cfg_mut = physic_engine.get_config_mut();
                cfg_mut.max_rockets = default_cfg.max_rockets;
                cfg_mut.particles_per_explosion = default_cfg.particles_per_explosion;
                cfg_mut.particles_per_trail = default_cfg.particles_per_trail;
            }
            crate::domain_contracts::PhysicCommand::SetMaxRockets(r) => {
                physic_engine.get_config_mut().max_rockets = r as usize;
            }
            crate::domain_contracts::PhysicCommand::SetParticlesPerExplosion(p) => {
                physic_engine.get_config_mut().particles_per_explosion = p as usize;
            }
            crate::domain_contracts::PhysicCommand::SetParticlesPerTrail(p) => {
                physic_engine.get_config_mut().particles_per_trail = p as usize;
            }
            crate::domain_contracts::PhysicCommand::SetMaxSmokeParticles(m) => {
                physic_engine.get_config_mut().max_smoke_particles = m as usize;
            }
            crate::domain_contracts::PhysicCommand::ResetSpawnDefaults => {
                let default_cfg = crate::physic_engine::config::PhysicConfig::default();
                let cfg_mut = physic_engine.get_config_mut();
                cfg_mut.rocket_interval_mean = default_cfg.rocket_interval_mean;
                cfg_mut.rocket_interval_variation = default_cfg.rocket_interval_variation;
                cfg_mut.rocket_max_next_interval = default_cfg.rocket_max_next_interval;
                cfg_mut.spawn_rocket_margin = default_cfg.spawn_rocket_margin;
                cfg_mut.spawn_rocket_vertical_angle = default_cfg.spawn_rocket_vertical_angle;
                cfg_mut.spawn_rocket_angle_variation = default_cfg.spawn_rocket_angle_variation;
                cfg_mut.spawn_rocket_min_speed = default_cfg.spawn_rocket_min_speed;
                cfg_mut.spawn_rocket_max_speed = default_cfg.spawn_rocket_max_speed;
                cfg_mut.initial_rocket_speed = default_cfg.initial_rocket_speed;
            }
            crate::domain_contracts::PhysicCommand::SetRocketIntervalMean(v) => {
                physic_engine.get_config_mut().rocket_interval_mean = v;
            }
            crate::domain_contracts::PhysicCommand::SetRocketIntervalVariation(v) => {
                physic_engine.get_config_mut().rocket_interval_variation = v;
            }
            crate::domain_contracts::PhysicCommand::SetRocketMaxNextInterval(v) => {
                physic_engine.get_config_mut().rocket_max_next_interval = v;
            }
            crate::domain_contracts::PhysicCommand::SetSpawnRocketMargin(v) => {
                physic_engine.get_config_mut().spawn_rocket_margin = v;
            }
            crate::domain_contracts::PhysicCommand::SetSpawnRocketVerticalAngle(v) => {
                physic_engine.get_config_mut().spawn_rocket_vertical_angle = v;
            }
            crate::domain_contracts::PhysicCommand::SetSpawnRocketAngleVariation(v) => {
                physic_engine.get_config_mut().spawn_rocket_angle_variation = v;
            }
            crate::domain_contracts::PhysicCommand::SetSpawnRocketMinSpeed(v) => {
                physic_engine.get_config_mut().spawn_rocket_min_speed = v;
            }
            crate::domain_contracts::PhysicCommand::SetSpawnRocketMaxSpeed(v) => {
                physic_engine.get_config_mut().spawn_rocket_max_speed = v;
            }
            crate::domain_contracts::PhysicCommand::SetInitialRocketSpeed(v) => {
                physic_engine.get_config_mut().initial_rocket_speed = v;
            }
            crate::domain_contracts::PhysicCommand::ResetForcesDefaults => {
                let default_cfg = crate::physic_engine::config::PhysicConfig::default();
                let cfg_mut = physic_engine.get_config_mut();
                cfg_mut.gravity = default_cfg.gravity;
                cfg_mut.explosion_threshold = default_cfg.explosion_threshold;
                cfg_mut.explosion_min_vel = default_cfg.explosion_min_vel;
                cfg_mut.explosion_max_vel = default_cfg.explosion_max_vel;
            }
            crate::domain_contracts::PhysicCommand::SetExplosionThreshold(v) => {
                physic_engine.get_config_mut().explosion_threshold = v;
            }
            crate::domain_contracts::PhysicCommand::SetExplosionMinVel(v) => {
                physic_engine.get_config_mut().explosion_min_vel = v;
            }
            crate::domain_contracts::PhysicCommand::SetExplosionMaxVel(v) => {
                physic_engine.get_config_mut().explosion_max_vel = v;
            }
            crate::domain_contracts::PhysicCommand::SetExplosionShapeSpherical => {
                physic_engine.set_explosion_shape(crate::physic_engine::ExplosionShape::Spherical);
            }
            crate::domain_contracts::PhysicCommand::ResetAllPresetWeights => {}
            crate::domain_contracts::PhysicCommand::SetPresetWeight { index, weight } => {
                let idx = index as usize;
                if idx < PRESET_DEFINITIONS.len() {
                    let (_, _, path, scale, flight) = PRESET_DEFINITIONS[idx];
                    let _ =
                        physic_engine.load_explosion_image_weighted(path, scale, flight, weight);
                }
            }
            crate::domain_contracts::PhysicCommand::SetPresetSingleShape { index } => {
                let idx = index as usize;
                if idx < PRESET_DEFINITIONS.len() {
                    let (_, _, path, scale, flight) = PRESET_DEFINITIONS[idx];
                    let _ = physic_engine.load_explosion_image(path, scale, flight);
                }
            }
            crate::domain_contracts::PhysicCommand::AddPresetShapeWeighted { index, weight } => {
                let idx = index as usize;
                if idx < PRESET_DEFINITIONS.len() {
                    let (_, _, path, scale, flight) = PRESET_DEFINITIONS[idx];
                    let _ =
                        physic_engine.load_explosion_image_weighted(path, scale, flight, weight);
                }
            }
            crate::domain_contracts::PhysicCommand::DeleteSingleShape => {
                if let crate::physic_engine::ExplosionShape::Image(img) =
                    physic_engine.get_explosion_shape().clone()
                {
                    let _ = physic_engine.remove_explosion_image(&img.file_stem);
                }
            }
            crate::domain_contracts::PhysicCommand::ResetSingleShapeDefaults => {
                if let crate::physic_engine::ExplosionShape::Image(img) =
                    physic_engine.get_explosion_shape().clone()
                {
                    let (def_scale, def_flight) = physic::get_preset_defaults(&img.file_stem);
                    let mut updated = img.clone();
                    updated.scale = def_scale;
                    updated.flight_time = def_flight;
                    physic_engine
                        .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
                }
            }
            crate::domain_contracts::PhysicCommand::SetSingleShapeScale(scale) => {
                if let crate::physic_engine::ExplosionShape::Image(img) =
                    physic_engine.get_explosion_shape().clone()
                {
                    let mut updated = img.clone();
                    updated.scale = scale;
                    physic_engine
                        .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
                }
            }
            crate::domain_contracts::PhysicCommand::SetSingleShapeFlightTime(flight) => {
                if let crate::physic_engine::ExplosionShape::Image(img) =
                    physic_engine.get_explosion_shape().clone()
                {
                    let mut updated = img.clone();
                    updated.flight_time = flight;
                    physic_engine
                        .set_explosion_shape(crate::physic_engine::ExplosionShape::Image(updated));
                }
            }
            crate::domain_contracts::PhysicCommand::DeleteMultiShapeItem(idx) => {
                let idx = idx as usize;
                if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                    physic_engine.get_explosion_shape().clone()
                {
                    if idx < shapes.len() {
                        let stem = shapes[idx].0.file_stem.clone();
                        let _ = physic_engine.remove_explosion_image(&stem);
                    }
                }
            }
            crate::domain_contracts::PhysicCommand::ResetMultiShapeItemDefaults(idx) => {
                let idx = idx as usize;
                if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                    physic_engine.get_explosion_shape().clone()
                {
                    if idx < shapes.len() {
                        let mut updated = shapes.clone();
                        let (def_scale, def_flight) =
                            physic::get_preset_defaults(&updated[idx].0.file_stem);
                        updated[idx].1 = 1.0;
                        updated[idx].0.scale = def_scale;
                        updated[idx].0.flight_time = def_flight;
                        let new_total: f32 = updated.iter().map(|(_, w)| *w).sum();
                        physic_engine.set_explosion_shape(
                            crate::physic_engine::ExplosionShape::MultiImage {
                                shapes: updated,
                                total_weight: new_total,
                            },
                        );
                    }
                }
            }
            crate::domain_contracts::PhysicCommand::SetMultiShapeItemWeight { index, weight } => {
                let index = index as usize;
                if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                    physic_engine.get_explosion_shape().clone()
                {
                    if index < shapes.len() {
                        let mut updated = shapes.clone();
                        updated[index].1 = weight;
                        let new_total: f32 = updated.iter().map(|(_, w)| *w).sum();
                        physic_engine.set_explosion_shape(
                            crate::physic_engine::ExplosionShape::MultiImage {
                                shapes: updated,
                                total_weight: new_total,
                            },
                        );
                    }
                }
            }
            crate::domain_contracts::PhysicCommand::SetMultiShapeItemScale { index, scale } => {
                let index = index as usize;
                if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                    physic_engine.get_explosion_shape().clone()
                {
                    if index < shapes.len() {
                        let mut updated = shapes.clone();
                        updated[index].0.scale = scale;
                        let new_total: f32 = updated.iter().map(|(_, w)| *w).sum();
                        physic_engine.set_explosion_shape(
                            crate::physic_engine::ExplosionShape::MultiImage {
                                shapes: updated,
                                total_weight: new_total,
                            },
                        );
                    }
                }
            }
            crate::domain_contracts::PhysicCommand::SetMultiShapeItemFlightTime {
                index,
                flight_time,
            } => {
                let index = index as usize;
                if let crate::physic_engine::ExplosionShape::MultiImage { shapes, .. } =
                    physic_engine.get_explosion_shape().clone()
                {
                    if index < shapes.len() {
                        let mut updated = shapes.clone();
                        updated[index].0.flight_time = flight_time;
                        let new_total: f32 = updated.iter().map(|(_, w)| *w).sum();
                        physic_engine.set_explosion_shape(
                            crate::physic_engine::ExplosionShape::MultiImage {
                                shapes: updated,
                                total_weight: new_total,
                            },
                        );
                    }
                }
            }
        }
    }

    fn dispatch_smoke_command<P: PhysicEngineFull>(
        smoke_cmd: crate::domain_contracts::SmokeCommand,
        physic_engine: &mut P,
    ) {
        match smoke_cmd {
            crate::domain_contracts::SmokeCommand::SetDensity(d) => {
                physic_engine.get_config_mut().smoke_intensity = d;
            }
            crate::domain_contracts::SmokeCommand::SetDissipation(d) => {
                physic_engine.get_config_mut().smoke_fade_duration = d;
            }
            crate::domain_contracts::SmokeCommand::SetWind(_) => {}
            crate::domain_contracts::SmokeCommand::SetErosionEnabled(e) => {
                physic_engine.get_config_mut().smoke_erosion_enabled = e;
            }
            crate::domain_contracts::SmokeCommand::SetErosionScale(s) => {
                physic_engine.get_config_mut().smoke_erosion_scale = s;
            }
            crate::domain_contracts::SmokeCommand::SetErosionEdgeWidth(w) => {
                physic_engine.get_config_mut().smoke_erosion_edge_width = w;
            }
            crate::domain_contracts::SmokeCommand::SetErosionEdgeColor(c) => {
                let f32_color = [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                ];
                physic_engine.get_config_mut().smoke_erosion_edge_color = f32_color;
            }
            crate::domain_contracts::SmokeCommand::SetFlowDistortionStrength(s) => {
                physic_engine.get_config_mut().flow_distortion_strength = s;
            }
            crate::domain_contracts::SmokeCommand::SetFlowAnimationSpeed(s) => {
                physic_engine.get_config_mut().flow_animation_speed = s;
            }
            crate::domain_contracts::SmokeCommand::SetColorMode(m) => {
                physic_engine.get_config_mut().smoke_color_mode = m;
            }
            crate::domain_contracts::SmokeCommand::SetInheritedColorIntensity(i) => {
                physic_engine
                    .get_config_mut()
                    .smoke_inherited_color_intensity = i;
            }
            crate::domain_contracts::SmokeCommand::SetCustomColor(c) => {
                let f32_color = [
                    c[0] as f32 / 255.0,
                    c[1] as f32 / 255.0,
                    c[2] as f32 / 255.0,
                ];
                physic_engine.get_config_mut().smoke_custom_color = f32_color;
            }
            crate::domain_contracts::SmokeCommand::SetSpawnRate(r) => {
                physic_engine.get_config_mut().smoke_spawn_rate = r;
            }
            crate::domain_contracts::SmokeCommand::SetInitialSize(s) => {
                physic_engine.get_config_mut().smoke_initial_size = s;
            }
            crate::domain_contracts::SmokeCommand::SetGrowthRateMultiplier(g) => {
                physic_engine.get_config_mut().smoke_growth_rate_multiplier = g;
            }
            crate::domain_contracts::SmokeCommand::SetFadeDuration(f) => {
                physic_engine.get_config_mut().smoke_fade_duration = f;
            }
            crate::domain_contracts::SmokeCommand::SetIntensity(i) => {
                physic_engine.get_config_mut().smoke_intensity = i;
            }
            crate::domain_contracts::SmokeCommand::SetMaxSmokeParticles(m) => {
                physic_engine.get_config_mut().max_smoke_particles = m as usize;
            }
            crate::domain_contracts::SmokeCommand::ResetDefaults => {
                let default_cfg = crate::physic_engine::config::PhysicConfig::default();
                let cfg_mut = physic_engine.get_config_mut();
                cfg_mut.smoke_spawn_rate = default_cfg.smoke_spawn_rate;
                cfg_mut.smoke_initial_size = default_cfg.smoke_initial_size;
                cfg_mut.smoke_growth_rate_multiplier = default_cfg.smoke_growth_rate_multiplier;
                cfg_mut.smoke_fade_duration = default_cfg.smoke_fade_duration;
                cfg_mut.max_smoke_particles = default_cfg.max_smoke_particles;
                cfg_mut.smoke_intensity = default_cfg.smoke_intensity;
                cfg_mut.smoke_color_mode = default_cfg.smoke_color_mode;
                cfg_mut.smoke_custom_color = default_cfg.smoke_custom_color;
                cfg_mut.smoke_erosion_enabled = default_cfg.smoke_erosion_enabled;
                cfg_mut.smoke_erosion_scale = default_cfg.smoke_erosion_scale;
                cfg_mut.smoke_erosion_edge_width = default_cfg.smoke_erosion_edge_width;
                cfg_mut.smoke_erosion_edge_color = default_cfg.smoke_erosion_edge_color;
            }
            crate::domain_contracts::SmokeCommand::ApplyPreset(preset_id) => {
                let cfg_mut = physic_engine.get_config_mut();
                match preset_id {
                    0 => {
                        // Fire & Ember
                        cfg_mut.smoke_erosion_edge_width = 0.12;
                        cfg_mut.smoke_erosion_edge_color = [1.0, 0.4, 0.05];
                        cfg_mut.smoke_color_mode =
                            crate::physic_engine::config::SmokeColorMode::Custom;
                        cfg_mut.smoke_custom_color = [0.15, 0.15, 0.15];
                        cfg_mut.smoke_intensity = 0.85;
                    }
                    1 => {
                        // Plasma Blue
                        cfg_mut.smoke_erosion_edge_width = 0.15;
                        cfg_mut.smoke_erosion_edge_color = [0.1, 0.8, 1.0];
                        cfg_mut.smoke_color_mode =
                            crate::physic_engine::config::SmokeColorMode::Custom;
                        cfg_mut.smoke_custom_color = [0.8, 0.9, 1.0];
                        cfg_mut.smoke_intensity = 1.0;
                    }
                    2 => {
                        // Volumetric Cloud
                        cfg_mut.smoke_erosion_edge_width = 0.05;
                        cfg_mut.smoke_erosion_edge_color = [0.75, 0.75, 0.75];
                        cfg_mut.smoke_color_mode =
                            crate::physic_engine::config::SmokeColorMode::Custom;
                        cfg_mut.smoke_custom_color = [0.85, 0.85, 0.85];
                        cfg_mut.smoke_intensity = 0.5;
                    }
                    3 => {
                        // Toxic Plasma
                        cfg_mut.smoke_erosion_edge_width = 0.18;
                        cfg_mut.smoke_erosion_edge_color = [0.2, 1.0, 0.3];
                        cfg_mut.smoke_color_mode =
                            crate::physic_engine::config::SmokeColorMode::Custom;
                        cfg_mut.smoke_custom_color = [0.1, 0.25, 0.1];
                        cfg_mut.smoke_intensity = 0.9;
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physic_engine::PhysicEngine;

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
        registry.register_for_audio("audio.volume", |_, _, _| String::new());
        registry.register_for_physic("physic.gravity", |_, _, _| String::new());

        assert!(settings.should_show_tab("audio", "", &registry));
        assert!(settings.should_show_tab("audio", "audio", &registry));
        assert!(settings.should_show_tab("audio", "volume", &registry));
        assert!(!settings.should_show_tab("audio", "render", &registry));
    }

    struct SpyPhysicEngine {
        inner: crate::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks,
        reload_count: usize,
    }

    impl SpyPhysicEngine {
        fn new() -> Self {
            let config = crate::physic_engine::config::PhysicConfig::default();
            Self {
                inner: crate::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks::new(
                    &config, 800.0,
                ),
                reload_count: 0,
            }
        }
    }

    impl crate::physic_engine::PhysicEngineIterator for SpyPhysicEngine {
        fn for_each_active_particle(
            &self,
            f: &mut dyn FnMut(&crate::physic_engine::particle::Particle),
        ) {
            self.inner.for_each_active_particle(f);
        }
        fn for_each_active_head_not_exploded(
            &self,
            f: &mut dyn FnMut(&crate::physic_engine::particle::Particle),
        ) {
            self.inner.for_each_active_head_not_exploded(f);
        }
        fn for_each_particle_of_type(
            &self,
            particle_type: crate::physic_engine::ParticleType,
            f: &mut dyn FnMut(&crate::physic_engine::particle::Particle),
        ) {
            self.inner.for_each_particle_of_type(particle_type, f);
        }
        fn for_each_smoke_particle(
            &self,
            f: &mut dyn FnMut(&crate::physic_engine::smoke_system::SmokeParticle),
        ) {
            self.inner.for_each_smoke_particle(f);
        }
        fn get_smoke_intensity(&self) -> f32 {
            self.inner.get_smoke_intensity()
        }
        fn get_smoke_erosion_params(&self) -> (bool, f32, f32, [f32; 3]) {
            self.inner.get_smoke_erosion_params()
        }
        fn get_smoke_flow_params(&self) -> (f32, f32) {
            self.inner.get_smoke_flow_params()
        }
    }

    impl crate::physic_engine::PhysicEngine for SpyPhysicEngine {
        fn set_window_width(&mut self, width: f32) {
            self.inner.set_window_width(width);
        }
        fn update(&mut self, dt: f32) -> crate::physic_engine::types::UpdateResult<'_> {
            self.inner.update(dt)
        }
        fn reload_config(&mut self, config: &crate::physic_engine::config::PhysicConfig) -> bool {
            self.reload_count += 1;
            self.inner.reload_config(config)
        }
        fn get_config(&self) -> &crate::physic_engine::config::PhysicConfig {
            self.inner.get_config()
        }
        fn get_config_mut(&mut self) -> &mut crate::physic_engine::config::PhysicConfig {
            self.inner.get_config_mut()
        }
        fn get_pending_config(&self) -> &crate::physic_engine::config::PhysicConfig {
            self.inner.get_pending_config()
        }
        fn set_explosion_shape(&mut self, shape: crate::physic_engine::ExplosionShape) {
            self.inner.set_explosion_shape(shape);
        }
        fn get_explosion_shape(&self) -> &crate::physic_engine::ExplosionShape {
            self.inner.get_explosion_shape()
        }
        fn load_explosion_image(
            &mut self,
            path: &str,
            scale: f32,
            flight_time: f32,
        ) -> Result<(), String> {
            self.inner.load_explosion_image(path, scale, flight_time)
        }
        fn load_explosion_image_weighted(
            &mut self,
            path: &str,
            scale: f32,
            flight_time: f32,
            weight: f32,
        ) -> Result<(), String> {
            self.inner
                .load_explosion_image_weighted(path, scale, flight_time, weight)
        }
        fn set_explosion_image_weight(&mut self, name: &str, weight: f32) -> Result<(), String> {
            self.inner.set_explosion_image_weight(name, weight)
        }
        fn remove_explosion_image(&mut self, name: &str) -> Result<(), String> {
            self.inner.remove_explosion_image(name)
        }
        fn as_physic_engine(&self) -> &dyn crate::physic_engine::PhysicEngine {
            self
        }
    }

    impl crate::physic_engine::PhysicEngineFull for SpyPhysicEngine {}

    #[test]
    fn test_smoke_commands_batched_single_reload() {
        let mut spy_engine = SpyPhysicEngine::new();
        let reinit_requested = AtomicBool::new(false);
        let mut cmd_queue = vec![
            crate::domain_contracts::EngineCommand::Smoke(
                crate::domain_contracts::SmokeCommand::SetDensity(0.95),
            ),
            crate::domain_contracts::EngineCommand::Smoke(
                crate::domain_contracts::SmokeCommand::SetErosionEnabled(true),
            ),
            crate::domain_contracts::EngineCommand::Smoke(
                crate::domain_contracts::SmokeCommand::SetFlowDistortionStrength(0.45),
            ),
            crate::domain_contracts::EngineCommand::Smoke(
                crate::domain_contracts::SmokeCommand::SetSpawnRate(45.0),
            ),
        ];

        GuiSettings::process_physic_commands(&mut cmd_queue, &mut spy_engine, &reinit_requested);

        // Verify single batched reload call
        assert_eq!(
            spy_engine.reload_count, 1,
            "Smoke command batch should trigger reload_config exactly ONCE"
        );
        // Verify no premature physical reinitialization requested
        assert!(
            !reinit_requested.load(Ordering::Relaxed),
            "Smoke commands must NOT request full physical reinitialization"
        );
        // Verify config fields updated properly
        let cfg = spy_engine.get_config();
        assert_eq!(cfg.smoke_intensity, 0.95);
        assert!(cfg.smoke_erosion_enabled);
        assert_eq!(cfg.flow_distortion_strength, 0.45);
        assert_eq!(cfg.smoke_spawn_rate, 45.0);
    }

    #[test]
    fn test_physic_command_set_gravity_pending_no_immediate_reload() {
        let mut spy_engine = SpyPhysicEngine::new();
        let reinit_requested = AtomicBool::new(false);
        let mut cmd_queue = vec![crate::domain_contracts::EngineCommand::Physic(
            crate::domain_contracts::PhysicCommand::SetGravity(-15.0),
        )];

        GuiSettings::process_physic_commands(&mut cmd_queue, &mut spy_engine, &reinit_requested);

        // Verify NO engine reload triggered on pending physics setting mutation
        assert_eq!(
            spy_engine.reload_count, 0,
            "SetGravity should NOT trigger immediate reload_config"
        );
        // Verify reinit flag is NOT set until ApplyPendingConfig
        assert!(
            !reinit_requested.load(Ordering::Relaxed),
            "SetGravity should NOT set reinit_requested"
        );
        // Verify pending config was mutated while active config remains unchanged
        assert_eq!(spy_engine.get_config_mut().gravity, -15.0);
        assert_ne!(spy_engine.get_config().gravity, -15.0);

        // Now push ApplyPendingConfig and verify reload, reinit flag, and active config update
        let mut apply_queue = vec![crate::domain_contracts::EngineCommand::Physic(
            crate::domain_contracts::PhysicCommand::ApplyPendingConfig,
        )];
        GuiSettings::process_physic_commands(&mut apply_queue, &mut spy_engine, &reinit_requested);

        assert_eq!(
            spy_engine.reload_count, 1,
            "ApplyPendingConfig MUST trigger reload_config"
        );
        assert!(
            reinit_requested.load(Ordering::Relaxed),
            "ApplyPendingConfig MUST set reinit_requested"
        );
        assert_eq!(spy_engine.get_config().gravity, -15.0);
    }

    #[test]
    fn test_exhaustive_engine_command_dispatcher_match() {
        // Compile-time check verifying all variants of EngineCommand are covered without wildcard _
        let audio_cmd = crate::domain_contracts::EngineCommand::Audio(
            crate::domain_contracts::AudioCommand::SetMasterVolume(0.5),
        );
        let physic_cmd = crate::domain_contracts::EngineCommand::Physic(
            crate::domain_contracts::PhysicCommand::SetGravity(-9.8),
        );
        let renderer_cmd = crate::domain_contracts::EngineCommand::Renderer(
            crate::domain_contracts::RendererCommand::SetBloomIntensity(1.2),
        );
        let smoke_cmd = crate::domain_contracts::EngineCommand::Smoke(
            crate::domain_contracts::SmokeCommand::SetDensity(0.8),
        );

        for cmd in [audio_cmd, physic_cmd, renderer_cmd, smoke_cmd] {
            match cmd {
                crate::domain_contracts::EngineCommand::Audio(_) => {}
                crate::domain_contracts::EngineCommand::Physic(_) => {}
                crate::domain_contracts::EngineCommand::Renderer(_) => {}
                crate::domain_contracts::EngineCommand::Smoke(_) => {}
                crate::domain_contracts::EngineCommand::Gui(_) => {}
            }
        }
    }

    struct SpyAudioEngine {
        master_volume: std::sync::RwLock<f32>,
        muted: AtomicBool,
        reverb_wet: std::sync::RwLock<f32>,
        listener_pos: std::sync::RwLock<glam::Vec2>,
        dsp_effects: std::sync::atomic::AtomicU32,
    }

    impl SpyAudioEngine {
        fn new() -> Self {
            Self {
                master_volume: std::sync::RwLock::new(0.80),
                muted: AtomicBool::new(false),
                reverb_wet: std::sync::RwLock::new(0.08),
                listener_pos: std::sync::RwLock::new(glam::Vec2::ZERO),
                dsp_effects: std::sync::atomic::AtomicU32::new(
                    crate::audio_engine::effect_flags::DEFAULT_FLAGS,
                ),
            }
        }
    }

    impl AudioEngine for SpyAudioEngine {
        fn play_rocket(&self, _pos: glam::Vec2, _gain: f32) {}
        fn play_rocket_with_id(&self, _id: u64, _pos: glam::Vec2, _gain: f32) {}
        fn play_explosion(&self, _pos: glam::Vec2, _gain: f32) {}
        fn play_explosion_with_id(&self, _id: u64, _pos: glam::Vec2, _gain: f32) {}
        fn start_audio_thread(&mut self, _export_path: Option<&str>) {}
        fn stop_audio_thread(&mut self) {}
        fn set_listener_position(&mut self, pos: glam::Vec2) {
            *self.listener_pos.write().unwrap() = pos;
        }
        fn get_listener_position(&self) -> glam::Vec2 {
            *self.listener_pos.read().unwrap()
        }
        fn mute(&mut self) {
            self.muted.store(true, Ordering::Relaxed);
        }
        fn unmute(&mut self) -> f32 {
            self.muted.store(false, Ordering::Relaxed);
            self.get_master_volume()
        }
        fn is_muted(&self) -> bool {
            self.muted.load(Ordering::Relaxed)
        }
        fn set_effect_enabled(&self, effect: AudioEffect, enabled: bool) {
            let mask = effect as u32;
            if enabled {
                self.dsp_effects.fetch_or(mask, Ordering::Relaxed);
            } else {
                self.dsp_effects.fetch_and(!mask, Ordering::Relaxed);
            }
        }
        fn set_all_effects_enabled(&self, enabled: bool) {
            if enabled {
                self.dsp_effects.store(0xFFFF_FFFF, Ordering::Relaxed);
            } else {
                self.dsp_effects.store(0, Ordering::Relaxed);
            }
        }
        fn get_effect_enabled(&self, effect: AudioEffect) -> bool {
            (self.dsp_effects.load(Ordering::Relaxed) & (effect as u32)) != 0
        }
        fn get_effects_status(&self) -> String {
            String::new()
        }
        fn set_reverb_wet(&self, wet: f32) {
            *self.reverb_wet.write().unwrap() = wet;
        }
        fn get_reverb_wet(&self) -> f32 {
            *self.reverb_wet.read().unwrap()
        }
        fn set_master_volume(&self, volume: f32) {
            *self.master_volume.write().unwrap() = volume;
        }
        fn get_master_volume(&self) -> f32 {
            *self.master_volume.read().unwrap()
        }
        fn as_audio_engine(&self) -> &dyn AudioEngine {
            self
        }
    }

    macro_rules! assert_state_reflection {
        ($cmd_queue:expr, $audio:expr, $physic:expr, $renderer:expr, $reinit:expr, $tonemap:expr, $stress:expr, $command:expr, $reader:expr, $getter_eval:expr, $expected:expr, $desc:expr) => {{
            $cmd_queue.push($command);
            let reload_shaders_dummy = std::sync::atomic::AtomicBool::new(false);
            GuiSettings::dispatch_command_queue(
                &mut $cmd_queue,
                &mut $audio,
                &mut $physic,
                &$renderer,
                &reload_shaders_dummy,
                &$reinit,
                &$tonemap,
                &mut $stress,
                (800.0, 600.0),
            );
            let actual = $getter_eval(&$reader);
            assert_eq!(
                actual, $expected,
                "UI State Feedback Loop failed for '{}': expected {:?}, got {:?}",
                $desc, $expected, actual
            );
        }};
    }

    #[test]
    fn test_ui_state_feedback_loop_audio() {
        use crate::domain_contracts::{AudioCommand, AudioStateReader, EngineCommand};

        let mut spy_audio = SpyAudioEngine::new();
        let mut spy_physic = SpyPhysicEngine::new();
        let renderer_config = Arc::new(RwLock::new(
            crate::renderer_engine::RendererConfig::default(),
        ));
        let reinit_req = AtomicBool::new(false);
        let tonemap_comp = AtomicBool::new(false);
        let mut stress_scene = AudioStressScene::new();
        let mut cmd_queue = Vec::with_capacity(16);

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Audio(AudioCommand::SetMasterVolume(0.42)),
            spy_audio,
            |r: &SpyAudioEngine| r.master_volume(),
            0.42,
            "Audio master_volume"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Audio(AudioCommand::SetMuted(true)),
            spy_audio,
            |r: &SpyAudioEngine| AudioStateReader::is_muted(r),
            true,
            "Audio is_muted true"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Audio(AudioCommand::SetMuted(false)),
            spy_audio,
            |r: &SpyAudioEngine| AudioStateReader::is_muted(r),
            false,
            "Audio is_muted false"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Audio(AudioCommand::SetSpatialReverb(0.35)),
            spy_audio,
            |r: &SpyAudioEngine| r.spatial_reverb(),
            0.35,
            "Audio spatial_reverb"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Audio(AudioCommand::SetEffectEnabled {
                effect: AudioEffect::HrtfBus,
                enabled: true
            }),
            spy_audio,
            |r: &SpyAudioEngine| r.hrtf_enabled(),
            true,
            "Audio hrtf_enabled"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Audio(AudioCommand::SetEffectEnabled {
                effect: AudioEffect::SpatialReverb,
                enabled: true
            }),
            spy_audio,
            |r: &SpyAudioEngine| r.effect_enabled(AudioEffect::SpatialReverb),
            true,
            "Audio effect_enabled SpatialReverb"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Audio(AudioCommand::SetAllEffectsEnabled(false)),
            spy_audio,
            |r: &SpyAudioEngine| r.effect_enabled(AudioEffect::SpatialReverb),
            false,
            "Audio set_all_effects_enabled false"
        );
    }

    #[test]
    fn test_ui_state_feedback_loop_physic() {
        use crate::domain_contracts::{EngineCommand, PhysicCommand, PhysicStateReader};

        let mut spy_audio = SpyAudioEngine::new();
        let mut spy_physic = SpyPhysicEngine::new();
        let renderer_config = Arc::new(RwLock::new(
            crate::renderer_engine::RendererConfig::default(),
        ));
        let reinit_req = AtomicBool::new(false);
        let tonemap_comp = AtomicBool::new(false);
        let mut stress_scene = AudioStressScene::new();
        let mut cmd_queue = Vec::with_capacity(16);

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Physic(PhysicCommand::SetGravity(-15.5)),
            spy_physic,
            |r: &SpyPhysicEngine| r.gravity(),
            -15.5,
            "Physic gravity pending reflection"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Physic(PhysicCommand::SetMaxRockets(250)),
            spy_physic,
            |r: &SpyPhysicEngine| r.max_particles(),
            250,
            "Physic max_particles pending reflection"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Physic(PhysicCommand::SetExplosionMaxVel(120.0)),
            spy_physic,
            |r: &SpyPhysicEngine| r.explosion_force(),
            120.0,
            "Physic explosion_force pending reflection"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Physic(PhysicCommand::SetExplosionShapeSpherical),
            spy_physic,
            |r: &SpyPhysicEngine| r.explosion_shape()
                == &crate::physic_engine::ExplosionShape::Spherical,
            true,
            "Physic explosion_shape Spherical"
        );
    }

    #[test]
    fn test_ui_state_feedback_loop_renderer() {
        use crate::domain_contracts::{EngineCommand, RendererCommand, RendererStateReader};

        let mut spy_audio = SpyAudioEngine::new();
        let mut spy_physic = SpyPhysicEngine::new();
        let renderer_config = Arc::new(RwLock::new(
            crate::renderer_engine::RendererConfig::default(),
        ));
        let reinit_req = AtomicBool::new(false);
        let tonemap_comp = AtomicBool::new(false);
        let mut stress_scene = AudioStressScene::new();
        let mut cmd_queue = Vec::with_capacity(16);

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetBloomIntensity(3.2)),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .bloom_intensity(),
            3.2,
            "Renderer bloom_intensity"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetRenderRockets(false)),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .render_rockets,
            false,
            "Renderer render_rockets"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetRenderSmoke(false)),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .render_smoke,
            false,
            "Renderer render_smoke"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetRenderTrails(false)),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .render_trails,
            false,
            "Renderer render_trails"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetRenderExplosions(false)),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .render_explosions,
            false,
            "Renderer render_explosions"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetToneMappingMode(
                crate::renderer_engine::config::ToneMappingMode::Reinhard
            )),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .tone_mapping_mode,
            crate::renderer_engine::config::ToneMappingMode::Reinhard,
            "Renderer tone_mapping_mode"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetBloomEnabled(false)),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .bloom_enabled,
            false,
            "Renderer bloom_enabled"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetBloomIterations(5)),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .bloom_iterations,
            5,
            "Renderer bloom_iterations"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetBloomDownsample(4)),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .bloom_downsample,
            4,
            "Renderer bloom_downsample"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Renderer(RendererCommand::SetBloomBlurMethod(
                crate::renderer_engine::config::BlurMethod::Kawase
            )),
            renderer_config,
            |r: &Arc<RwLock<crate::renderer_engine::RendererConfig>>| r
                .read()
                .unwrap()
                .config()
                .bloom_blur_method,
            crate::renderer_engine::config::BlurMethod::Kawase,
            "Renderer bloom_blur_method"
        );
    }

    #[test]
    fn test_ui_state_feedback_loop_smoke() {
        use crate::domain_contracts::{EngineCommand, SmokeCommand, SmokeStateReader};

        let mut spy_audio = SpyAudioEngine::new();
        let mut spy_physic = SpyPhysicEngine::new();
        let renderer_config = Arc::new(RwLock::new(
            crate::renderer_engine::RendererConfig::default(),
        ));
        let reinit_req = AtomicBool::new(false);
        let tonemap_comp = AtomicBool::new(false);
        let mut stress_scene = AudioStressScene::new();
        let mut cmd_queue = Vec::with_capacity(16);

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetDensity(0.92)),
            spy_physic,
            |r: &SpyPhysicEngine| r.density(),
            0.92,
            "Smoke density"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetDissipation(4.2)),
            spy_physic,
            |r: &SpyPhysicEngine| r.dissipation(),
            4.2,
            "Smoke dissipation"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetErosionEnabled(true)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_erosion_enabled,
            true,
            "Smoke erosion_enabled"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetErosionScale(2.8)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_erosion_scale,
            2.8,
            "Smoke erosion_scale"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetErosionEdgeWidth(0.35)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_erosion_edge_width,
            0.35,
            "Smoke erosion_edge_width"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetErosionEdgeColor([255, 128, 64])),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_erosion_edge_color,
            [1.0, 128.0 / 255.0, 64.0 / 255.0],
            "Smoke erosion_edge_color"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetFlowDistortionStrength(0.75)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().flow_distortion_strength,
            0.75,
            "Smoke flow_distortion_strength"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetFlowAnimationSpeed(1.4)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().flow_animation_speed,
            1.4,
            "Smoke flow_animation_speed"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetColorMode(
                crate::physic_engine::config::SmokeColorMode::Custom
            )),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_color_mode,
            crate::physic_engine::config::SmokeColorMode::Custom,
            "Smoke smoke_color_mode"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetCustomColor([50, 100, 150])),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_custom_color,
            [50.0 / 255.0, 100.0 / 255.0, 150.0 / 255.0],
            "Smoke smoke_custom_color"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetSpawnRate(55.0)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_spawn_rate,
            55.0,
            "Smoke smoke_spawn_rate"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetInitialSize(18.0)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_initial_size,
            18.0,
            "Smoke smoke_initial_size"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetGrowthRateMultiplier(2.2)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().smoke_growth_rate_multiplier,
            2.2,
            "Smoke smoke_growth_rate_multiplier"
        );

        assert_state_reflection!(
            cmd_queue,
            spy_audio,
            spy_physic,
            renderer_config,
            reinit_req,
            tonemap_comp,
            stress_scene,
            EngineCommand::Smoke(SmokeCommand::SetMaxSmokeParticles(3500)),
            spy_physic,
            |r: &SpyPhysicEngine| r.config().max_smoke_particles,
            3500,
            "Smoke max_smoke_particles"
        );
    }
}

pub mod audio;
pub mod command_handler;
pub mod physic;
pub mod renderer;
pub mod session;
pub mod smoke;
pub mod theme;

#[cfg(test)]
mod tests;

pub use audio::{default_audio_master_volume, render_audio_settings_tab};
pub use command_handler::dispatch_engine_commands;
pub use physic::{
    apply_session_to_physic, default_preset_weights, render_physics_settings_tab,
    PersistedExplosionImage, PersistedExplosionShape, PRESET_DEFINITIONS,
};
pub use renderer::{render_commands_overview_tab, render_renderer_settings_tab};
pub use session::GuiSessionState;
pub use smoke::render_smoke_settings_tab;
pub use theme::{
    apply_theme_to_context, GuiTheme, COLOR_ALERT, COLOR_COMMAND_NAME, COLOR_HEADER, COLOR_SUCCESS,
    COLOR_TEXT_HINT, COLOR_TEXT_MUTED, COLOR_TITLE, COLOR_WARNING, DEFAULT_GUI_SCALE,
    GUI_SCALE_MAX, GUI_SCALE_MIN, GUI_SCALE_STEP, ZOOM_PRESETS,
};

use crate::audio_engine::effect_flags::AudioEffect;
use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::simulator::audio_stress_scene::AudioStressScene;
use crate::utils::command_console::CommandRegistry;
use imgui::Ui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

#[cfg(test)]
pub(crate) static IMGUI_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    pub smoke_preview_max_zoom: f32,
    pub show_geometry_trimming: bool,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl GuiSettings {
    pub fn new() -> Self {
        let session_path = crate::utils::config_path::get_gui_session_path();
        let session = GuiSessionState::load_from_file(&session_path);
        let result = Self {
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
            smoke_preview_max_zoom: session.smoke_preview_max_zoom,
            show_geometry_trimming: session.show_geometry_trimming,
        };
        // Sync static AtomicBool from persisted session state
        smoke::SHOW_GEOMETRY_TRIMMING.store(
            result.show_geometry_trimming,
            std::sync::atomic::Ordering::Relaxed,
        );
        result
    }

    pub fn get_preset_defaults(stem: &str) -> (f32, f32) {
        physic::get_preset_defaults(stem)
    }

    pub fn apply_session_from_path_to_audio<A: AudioEngine>(
        &self,
        path: &std::path::Path,
        audio_engine: &mut A,
        show_audio_diagnostic: &mut bool,
        show_audio_visual_overlay: &mut bool,
    ) {
        let session = GuiSessionState::load_from_file(path);
        session.apply_session_from_path_to_audio(
            path,
            audio_engine,
            show_audio_diagnostic,
            show_audio_visual_overlay,
        );
    }

    pub fn apply_session_to_audio<A: AudioEngine>(
        &self,
        audio_engine: &mut A,
        show_audio_diagnostic: &mut bool,
        show_audio_visual_overlay: &mut bool,
    ) {
        let session_path = crate::utils::config_path::get_gui_session_path();
        self.apply_session_from_path_to_audio(
            &session_path,
            audio_engine,
            show_audio_diagnostic,
            show_audio_visual_overlay,
        );
    }

    pub fn apply_session_to_physic<P: PhysicEngineFull>(&self, physic_engine: &mut P) {
        let session_path = crate::utils::config_path::get_gui_session_path();
        let session = GuiSessionState::load_from_file(&session_path);
        session.apply_session_to_physic(physic_engine);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn save_session_state_to_path<A: AudioEngine, P: PhysicEngineFull>(
        &self,
        audio_engine: &A,
        physic_engine: &P,
        show_audio_diagnostic: bool,
        show_audio_visual_overlay: bool,
        tonemapping_comparison_mode: bool,
        fullscreen: bool,
        path: &std::path::Path,
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
            preset_weights: physic::preset_weights_from_shape(physic_engine.get_explosion_shape()),
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
            smoke_preview_max_zoom: self.smoke_preview_max_zoom,
            show_geometry_trimming: smoke::SHOW_GEOMETRY_TRIMMING
                .load(std::sync::atomic::Ordering::Relaxed),
        };

        if let Err(e) = session.save_to_file(path) {
            eprintln!(
                "Failed to save session state to {}: {:?}",
                path.display(),
                e
            );
        }
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
        if crate::utils::config_path::is_config_save_enabled() {
            let path = crate::utils::config_path::get_gui_session_path();
            self.save_session_state_to_path(
                audio_engine,
                physic_engine,
                show_audio_diagnostic,
                show_audio_visual_overlay,
                tonemapping_comparison_mode,
                fullscreen,
                &path,
            );
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
            imgui::Condition::FirstUseEver
        } else {
            imgui::Condition::Always
        };
        let size_cond = if self.window_size.is_some() {
            imgui::Condition::FirstUseEver
        } else {
            imgui::Condition::Always
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
                let current_zoom_label =
                    format!("Zoom: {}%", (self.gui_scale * 100.0).round() as i32);
                ui.set_next_item_width(ui.current_font_size() * 8.5);
                if let Some(_combo) = ui.begin_combo("##ZoomCombo", &current_zoom_label) {
                    for (scale, name) in ZOOM_PRESETS {
                        let selected = (self.gui_scale - scale).abs() < 0.02;
                        if ui.selectable_config(name).selected(selected).build() {
                            self.gui_scale = scale;
                        }
                    }
                }

                ui.same_line();
                if ui.small_button("-") {
                    self.gui_scale =
                        (self.gui_scale - GUI_SCALE_STEP).clamp(GUI_SCALE_MIN, GUI_SCALE_MAX);
                }
                ui.same_line();
                if ui.small_button("+") {
                    self.gui_scale =
                        (self.gui_scale + GUI_SCALE_STEP).clamp(GUI_SCALE_MIN, GUI_SCALE_MAX);
                }

                ui.same_line();
                // Theme Combo Selector
                let themes = GuiTheme::all_themes();
                let current_theme_idx = themes
                    .iter()
                    .position(|(t, _)| *t == self.theme)
                    .unwrap_or(0);

                ui.set_next_item_width(ui.current_font_size() * 12.0);
                if let Some(_combo) = ui.begin_combo("##ThemeCombo", themes[current_theme_idx].1) {
                    for (t, name) in themes {
                        let selected = *t == self.theme;
                        if ui.selectable_config(name).selected(selected).build() {
                            self.theme = *t;
                            self.pending_theme_change = Some(*t);
                            self.set_status(format!("Applied theme: {:?}", *t));
                        }
                    }
                }

                ui.separator();

                // Filter & Search Input
                ui.set_next_item_width(ui.current_font_size() * 15.0);
                ui.input_text("Search / Filter", &mut self.search_filter)
                    .build();
                ui.same_line();
                if ui.small_button("Clear") {
                    self.search_filter.clear();
                }

                if let Some((msg, created)) = &self.status_message {
                    if created.elapsed().as_secs_f32() < 4.0 {
                        ui.same_line();
                        ui.text_colored(COLOR_SUCCESS, msg);
                    }
                }

                ui.spacing();

                let select_0 = self.set_selected_tab == Some(0);
                let select_1 = self.set_selected_tab == Some(1);
                let select_2 = self.set_selected_tab == Some(2);
                let select_3 = self.set_selected_tab == Some(3);
                let select_4 = self.set_selected_tab == Some(4);

                let filter = self.search_filter.to_lowercase();

                if let Some(_tab_bar) = ui.tab_bar("gui_settings_tabs") {
                    // TAB 0: AUDIO
                    if self.should_show_tab("audio", &filter, commands_registry) {
                        let mut tab = imgui::TabItem::new("Audio System");
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
                                &mut self.smoke_preview_max_zoom,
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
                            render_smoke_settings_tab(
                                ui,
                                physic_engine,
                                cmd_queue,
                                &mut self.smoke_preview_max_zoom,
                            );
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
                                    let _ = c.save_to_file(
                                        crate::utils::config_path::get_renderer_config_path(),
                                    );
                                }
                            }
                            crate::domain_contracts::RendererCommand::ReloadConfig => {
                                if let Ok(new_c) = crate::renderer_engine::RendererConfig::from_file(
                                    crate::utils::config_path::get_renderer_config_path(),
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
                    command_handler::dispatch_engine_commands(
                        &mut vec![crate::domain_contracts::EngineCommand::Physic(physic_cmd)],
                        physic_engine,
                        physic_reinit_requested,
                    );
                }
                crate::domain_contracts::EngineCommand::Smoke(smoke_cmd) => {
                    smoke_modified = true;
                    command_handler::dispatch_engine_commands(
                        &mut vec![crate::domain_contracts::EngineCommand::Smoke(smoke_cmd)],
                        physic_engine,
                        physic_reinit_requested,
                    );
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
        command_handler::dispatch_engine_commands(
            cmd_queue,
            physic_engine,
            physic_reinit_requested,
        );
    }
}

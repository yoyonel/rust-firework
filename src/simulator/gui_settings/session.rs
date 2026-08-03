use crate::audio_engine::effect_flags::AudioEffect;
use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::simulator::gui_settings::audio::default_audio_master_volume;
use crate::simulator::gui_settings::physic::{
    apply_session_to_physic, default_preset_weights, PersistedExplosionImage,
    PersistedExplosionShape, PRESET_DEFINITIONS,
};
use crate::simulator::gui_settings::theme::{GuiTheme, DEFAULT_GUI_SCALE};
use serde::{Deserialize, Serialize};

use crate::audio_engine::constants as audio_constants;

fn default_gui_scale() -> f32 {
    DEFAULT_GUI_SCALE
}

// GUI_PERSIST: gui.layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiSessionState {
    pub gui_open: bool,
    pub active_tab: usize,
    pub search_filter: String,
    // GUI_PERSIST: audio.diagnostics
    pub show_audio_diagnostic: bool,
    pub show_audio_visual_overlay: bool,
    // GUI_PERSIST: audio.output
    pub audio_muted: bool,
    #[serde(default = "default_audio_master_volume")]
    pub audio_master_volume: f32,
    pub audio_reverb_wet: f32,
    pub audio_dsp_mask: u32,
    // GUI_PERSIST: physics.preset_weights
    #[serde(default = "default_preset_weights")]
    pub preset_weights: [f32; 5],
    // GUI_PERSIST: physics.explosion_shape
    #[serde(default, skip_serializing_if = "PersistedExplosionShape::is_spherical")]
    pub explosion_shape: PersistedExplosionShape,
    #[serde(default, rename = "active_shape_name", skip_serializing)]
    pub(crate) legacy_active_shape_name: String,
    // GUI_PERSIST: renderer.comparison
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
    // GUI_PERSIST: gui.theme
    #[serde(default)]
    pub theme: GuiTheme,
    // GUI_PERSIST: gui.scale
    #[serde(default = "default_gui_scale")]
    pub gui_scale: f32,
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_smoke_preview_max_zoom")]
    pub smoke_preview_max_zoom: f32,
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_show_geometry_trimming")]
    pub show_geometry_trimming: bool,
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_smoke_preview_rocket_color")]
    pub smoke_preview_rocket_color: [f32; 3],
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_smoke_preview_simulated_speed")]
    pub smoke_preview_simulated_speed: f32,
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_smoke_preview_simulated_angle_offset")]
    pub smoke_preview_simulated_angle_offset: f32,
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_smoke_preview_zoom")]
    pub smoke_preview_zoom: f32,
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_smoke_preview_pan_x")]
    pub smoke_preview_pan_x: f32,
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_smoke_preview_pan_y")]
    pub smoke_preview_pan_y: f32,
    // GUI_PERSIST: gui.layout
    #[serde(default = "default_smoke_preview_rot_z")]
    pub smoke_preview_rot_z: f32,
}

fn default_smoke_preview_max_zoom() -> f32 {
    crate::physic_engine::constants::DEFAULT_SMOKE_PREVIEW_MAX_ZOOM
}

fn default_show_geometry_trimming() -> bool {
    true
}

fn default_smoke_preview_rocket_color() -> [f32; 3] {
    crate::physic_engine::constants::DEFAULT_SMOKE_PREVIEW_ROCKET_COLOR
}

fn default_smoke_preview_simulated_speed() -> f32 {
    crate::physic_engine::constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_SPEED
}

fn default_smoke_preview_simulated_angle_offset() -> f32 {
    crate::physic_engine::constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET
}

fn default_smoke_preview_zoom() -> f32 {
    1.0
}

fn default_smoke_preview_pan_x() -> f32 {
    0.0
}

fn default_smoke_preview_pan_y() -> f32 {
    0.0
}

fn default_smoke_preview_rot_z() -> f32 {
    0.0
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
            audio_master_volume: audio_constants::DEFAULT_GLOBAL_GAIN,
            audio_reverb_wet: audio_constants::REVERB_DEFAULT_WET_GAIN,
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
            gui_scale: DEFAULT_GUI_SCALE,
            smoke_preview_max_zoom: crate::physic_engine::constants::DEFAULT_SMOKE_PREVIEW_MAX_ZOOM,
            show_geometry_trimming: true,
            smoke_preview_rocket_color:
                crate::physic_engine::constants::DEFAULT_SMOKE_PREVIEW_ROCKET_COLOR,
            smoke_preview_simulated_speed:
                crate::physic_engine::constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_SPEED,
            smoke_preview_simulated_angle_offset:
                crate::physic_engine::constants::DEFAULT_SMOKE_PREVIEW_SIMULATED_ANGLE_OFFSET,
            smoke_preview_zoom: 1.0,
            smoke_preview_pan_x: 0.0,
            smoke_preview_pan_y: 0.0,
            smoke_preview_rot_z: 0.0,
        }
    }
}

impl GuiSessionState {
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> Self {
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
            .filter(|(_, preset)| active == "multi" || active == preset.stem)
            .map(|(index, preset)| PersistedExplosionImage {
                file_stem: preset.stem.to_string(),
                scale: preset.default_scale,
                flight_time: preset.default_flight_time,
                weight: self.preset_weights[index],
            })
            .collect();
        if !images.is_empty() {
            self.explosion_shape = PersistedExplosionShape::Images { images };
        }
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> anyhow::Result<()> {
        let path = path.as_ref();
        let content = toml::to_string_pretty(self)?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn apply_session_from_path_to_audio<A: AudioEngine>(
        &self,
        path: &std::path::Path,
        audio_engine: &mut A,
        show_audio_diagnostic: &mut bool,
        show_audio_visual_overlay: &mut bool,
    ) {
        let session = Self::load_from_file(path);
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
        apply_session_to_physic(self.preset_weights, &self.explosion_shape, physic_engine);
    }
}

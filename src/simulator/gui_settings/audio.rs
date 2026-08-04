use super::theme::{COLOR_ALERT, COLOR_HEADER};
use crate::audio_engine::constants as audio_constants;
use crate::audio_engine::effect_flags::AudioEffect;
use crate::domain_contracts::{AudioCommand, AudioStateReader, EngineCommand};
use crate::simulator::audio_stress_scene::AudioStressScene;
use imgui::Ui;

pub fn default_audio_master_volume() -> f32 {
    audio_constants::DEFAULT_GLOBAL_GAIN
}

#[allow(clippy::too_many_arguments)]
pub fn render_audio_settings_tab(
    ui: &Ui,
    filter: &str,
    state: &impl AudioStateReader,
    cmd_queue: &mut Vec<EngineCommand>,
    show_audio_diagnostic: &mut bool,
    show_audio_visual_overlay: &mut bool,
    audio_stress_scene: &mut AudioStressScene,
    _window_size_f32: (f32, f32),
) {
    ui.spacing();
    ui.text_colored(COLOR_HEADER, "=== GLOBAL AUDIO CONTROLS ===");

    ui.same_line();
    if ui.button("[RESET AUDIO DEFAULTS]") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            audio_constants::DEFAULT_GLOBAL_GAIN,
        )));
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMuted(false)));
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetSpatialReverb(
            audio_constants::REVERB_DEFAULT_WET_GAIN,
        )));
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetAllEffectsEnabled(
            true,
        )));
        *show_audio_diagnostic = false;
        *show_audio_visual_overlay = true;
    }

    let font_sz = ui.current_font_size();
    let mut master_vol = state.master_volume();
    ui.set_next_item_width(font_sz * 14.0);
    if ui
        .slider_config(
            "Master Volume (`audio.volume`)##master_vol_slider",
            audio_constants::SLIDER_VOLUME_MIN,
            audio_constants::SLIDER_VOLUME_MAX,
        )
        .display_format("%.3f")
        .build(&mut master_vol)
    {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            master_vol,
        )));
    }
    ui.same_line();
    if ui.small_button("Reset Vol") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            audio_constants::DEFAULT_GLOBAL_GAIN,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text("Reset Master Volume to default 0.80 (80%)");
    }

    // Quick Volume Presets
    ui.text("Volume Presets:");
    ui.same_line();
    if ui.small_button("Mute (0%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            audio_constants::PRESET_VOL_MUTE,
        )));
    }
    ui.same_line();
    if ui.small_button("Low (25%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            audio_constants::PRESET_VOL_LOW,
        )));
    }
    ui.same_line();
    if ui.small_button("Medium (50%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            audio_constants::PRESET_VOL_MEDIUM,
        )));
    }
    ui.same_line();
    if ui.small_button("Default (80%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            audio_constants::PRESET_VOL_DEFAULT,
        )));
    }
    ui.same_line();
    if ui.small_button("Full (100%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            audio_constants::PRESET_VOL_FULL,
        )));
    }
    ui.same_line();
    if ui.small_button("Boost (150%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMasterVolume(
            audio_constants::PRESET_VOL_BOOST,
        )));
    }

    ui.spacing();
    // Master Mute Toggle (reads live is_muted status)
    let mut mute_state = state.is_muted();
    if ui.checkbox(
        "Mute Audio (`audio.mute` / `audio.unmute`)",
        &mut mute_state,
    ) {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMuted(mute_state)));
    }
    ui.same_line();
    if ui.button("Mute") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMuted(true)));
    }
    ui.same_line();
    if ui.button("Unmute") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetMuted(false)));
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(
        COLOR_HEADER,
        "=== SPATIAL REVERB WET MIX (`audio.reverb_wet`) ===",
    );

    let mut reverb_wet = state.spatial_reverb();
    ui.set_next_item_width(font_sz * 14.0);
    if ui.slider(
        "Reverb Wet Gain",
        audio_constants::SLIDER_REVERB_MIN,
        audio_constants::SLIDER_REVERB_MAX,
        &mut reverb_wet,
    ) {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetSpatialReverb(
            reverb_wet,
        )));
    }
    if ui.is_item_hovered() {
        ui.tooltip_text(
            "Adjust spatial reverb wet mix gain (0.00 = Dry, 0.08 = Default, 1.00 = 100% Echo)",
        );
    }

    // Quick Reverb Presets & Reset
    ui.text("Presets:");
    ui.same_line();
    if ui.button("Dry (0%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetSpatialReverb(
            audio_constants::PRESET_REVERB_DRY,
        )));
    }
    ui.same_line();
    if ui.button("Default (8%) [Reset]") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetSpatialReverb(
            audio_constants::PRESET_REVERB_DEFAULT,
        )));
    }
    ui.same_line();
    if ui.button("Medium (20%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetSpatialReverb(
            audio_constants::PRESET_REVERB_MEDIUM,
        )));
    }
    ui.same_line();
    if ui.button("Cathedral (50%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetSpatialReverb(
            audio_constants::PRESET_REVERB_CATHEDRAL,
        )));
    }
    ui.same_line();
    if ui.button("Full Wet (100%)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetSpatialReverb(
            audio_constants::PRESET_REVERB_FULL_WET,
        )));
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(
        COLOR_HEADER,
        "=== DSP EFFECTS MATRIX (`audio.fx`, `audio.fx_all`) ===",
    );

    if ui.button("[ON] Enable All DSP Effects (`audio.fx_all on`)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetAllEffectsEnabled(
            true,
        )));
    }
    ui.same_line();
    if ui.button("[OFF] Disable All DSP Effects (`audio.fx_all off`)") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetAllEffectsEnabled(
            false,
        )));
    }
    ui.same_line();
    if ui.button("[RESET DSP EFFECTS]") {
        cmd_queue.push(EngineCommand::Audio(AudioCommand::SetAllEffectsEnabled(
            true,
        )));
    }

    ui.spacing();
    ui.text("Individual DSP Effects:");

    for (name, fx) in AudioEffect::all_names() {
        if !filter.is_empty() && !name.contains(filter) {
            continue;
        }

        let mut enabled = state.effect_enabled(*fx);
        if ui.checkbox(*name, &mut enabled) {
            cmd_queue.push(EngineCommand::Audio(AudioCommand::SetEffectEnabled {
                effect: *fx,
                enabled,
            }));
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(*name);
        }
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(COLOR_HEADER, "=== AUDIO DIAGNOSTICS & MONITORING ===");

    ui.checkbox(
        "Display Audio Diagnostic Monitor (F3)",
        show_audio_diagnostic,
    );
    ui.checkbox(
        "Display Visual Overlay (Waves / Beams / Badges)",
        show_audio_visual_overlay,
    );

    if audio_stress_scene.enabled {
        ui.text_colored(COLOR_ALERT, "[ACTIVE] Audio Stress Test Scene is ACTIVE");
        if ui.button("Stop Stress Test Scene") {
            audio_stress_scene.enabled = false;
        }
    } else if ui.button("Start Audio Stress Test Scene (32 sources)") {
        *show_audio_diagnostic = true;
        cmd_queue.push(EngineCommand::Audio(AudioCommand::StartStressTest));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    struct MockAudioState {
        volume: f32,
        muted: bool,
        reverb: f32,
        hrtf: bool,
    }

    impl AudioStateReader for MockAudioState {
        fn master_volume(&self) -> f32 {
            self.volume
        }
        fn is_muted(&self) -> bool {
            self.muted
        }
        fn spatial_reverb(&self) -> f32 {
            self.reverb
        }
        fn hrtf_enabled(&self) -> bool {
            self.hrtf
        }
        fn effect_enabled(&self, _effect: AudioEffect) -> bool {
            true
        }
    }

    #[test]
    fn test_default_audio_master_volume() {
        assert_eq!(
            default_audio_master_volume(),
            audio_constants::DEFAULT_GLOBAL_GAIN
        );
    }

    #[test]
    fn test_mock_audio_state_reader() {
        let state = MockAudioState {
            volume: 0.75,
            muted: true,
            reverb: 0.12,
            hrtf: true,
        };

        assert_eq!(state.master_volume(), 0.75);
        assert!(state.is_muted());
        assert_eq!(state.spatial_reverb(), 0.12);
        assert!(state.hrtf_enabled());
        assert!(state.effect_enabled(AudioEffect::Binaural));
    }

    #[test]
    #[serial]
    fn test_render_audio_settings_tab_pure_function_execution() {
        let state = MockAudioState {
            volume: 0.80,
            muted: false,
            reverb: 0.08,
            hrtf: true,
        };
        let mut cmd_queue: Vec<EngineCommand> = Vec::with_capacity(16);
        let mut show_diagnostic = false;
        let mut show_overlay = true;
        let mut stress_scene = AudioStressScene::new();

        let _guard = crate::simulator::gui_settings::IMGUI_TEST_MUTEX
            .lock()
            .unwrap();
        let mut imgui_ctx = imgui::Context::create();
        imgui_ctx.set_ini_filename(None);
        imgui_ctx.fonts().build_rgba32_texture();
        imgui_ctx.io_mut().display_size = [800.0, 600.0];

        let ui = imgui_ctx.frame();

        render_audio_settings_tab(
            ui,
            "",
            &state,
            &mut cmd_queue,
            &mut show_diagnostic,
            &mut show_overlay,
            &mut stress_scene,
            (800.0, 600.0),
        );

        assert!(cmd_queue.capacity() >= 16);
    }
}

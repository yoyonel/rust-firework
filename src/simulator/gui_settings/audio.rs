use crate::audio_engine::effect_flags::AudioEffect;
use crate::audio_engine::AudioEngine;
use crate::simulator::audio_stress_scene::AudioStressScene;
use imgui::Ui;

pub fn default_audio_master_volume() -> f32 {
    0.80
}

pub fn render_audio_settings_tab<A: AudioEngine>(
    ui: &Ui,
    filter: &str,
    audio_engine: &mut A,
    show_audio_diagnostic: &mut bool,
    show_audio_visual_overlay: &mut bool,
    audio_stress_scene: &mut AudioStressScene,
    window_size_f32: (f32, f32),
) {
    ui.spacing();
    ui.text_colored([0.4, 0.8, 1.0, 1.0], "=== GLOBAL AUDIO CONTROLS ===");

    ui.same_line();
    if ui.button("[RESET AUDIO DEFAULTS]") {
        audio_engine.set_master_volume(0.80);
        audio_engine.unmute();
        audio_engine.set_reverb_wet(0.08);
        audio_engine.set_all_effects_enabled(true);
        *show_audio_diagnostic = false;
        *show_audio_visual_overlay = true;
    }

    // Master Volume Slider & Reset
    let mut master_vol = audio_engine.get_master_volume();
    ui.set_next_item_width(200.0);
    if ui
        .slider_config(
            "Master Volume (`audio.volume`)##master_vol_slider",
            0.0,
            2.0,
        )
        .display_format("%.3f")
        .build(&mut master_vol)
    {
        audio_engine.set_master_volume(master_vol);
    }
    ui.same_line();
    if ui.small_button("Reset Vol") {
        audio_engine.set_master_volume(0.80);
    }
    if ui.is_item_hovered() {
        ui.tooltip_text("Reset Master Volume to default 0.80 (80%)");
    }

    // Quick Volume Presets
    ui.text("Volume Presets:");
    ui.same_line();
    if ui.small_button("Mute (0%)") {
        audio_engine.set_master_volume(0.0);
    }
    ui.same_line();
    if ui.small_button("Low (25%)") {
        audio_engine.set_master_volume(0.25);
    }
    ui.same_line();
    if ui.small_button("Medium (50%)") {
        audio_engine.set_master_volume(0.50);
    }
    ui.same_line();
    if ui.small_button("Default (80%)") {
        audio_engine.set_master_volume(0.80);
    }
    ui.same_line();
    if ui.small_button("Full (100%)") {
        audio_engine.set_master_volume(1.00);
    }
    ui.same_line();
    if ui.small_button("Boost (150%)") {
        audio_engine.set_master_volume(1.50);
    }

    ui.spacing();
    // Master Mute Toggle (reads live is_muted status)
    let mut mute_state = audio_engine.is_muted();
    if ui.checkbox(
        "Mute Audio (`audio.mute` / `audio.unmute`)",
        &mut mute_state,
    ) {
        if mute_state {
            audio_engine.mute();
        } else {
            audio_engine.unmute();
        }
    }
    ui.same_line();
    if ui.button("Mute") {
        audio_engine.mute();
    }
    ui.same_line();
    if ui.button("Unmute") {
        audio_engine.unmute();
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(
        [0.4, 0.8, 1.0, 1.0],
        "=== SPATIAL REVERB WET MIX (`audio.reverb_wet`) ===",
    );

    let mut reverb_wet = audio_engine.get_reverb_wet();
    ui.set_next_item_width(200.0);
    if ui.slider("Reverb Wet Gain", 0.0, 1.0, &mut reverb_wet) {
        audio_engine.set_reverb_wet(reverb_wet);
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
        audio_engine.set_reverb_wet(0.0);
    }
    ui.same_line();
    if ui.button("Default (8%) [Reset]") {
        audio_engine.set_reverb_wet(0.08);
    }
    ui.same_line();
    if ui.button("Medium (20%)") {
        audio_engine.set_reverb_wet(0.20);
    }
    ui.same_line();
    if ui.button("Cathedral (50%)") {
        audio_engine.set_reverb_wet(0.50);
    }
    ui.same_line();
    if ui.button("Full Wet (100%)") {
        audio_engine.set_reverb_wet(1.0);
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(
        [0.4, 0.8, 1.0, 1.0],
        "=== DSP EFFECTS MATRIX (`audio.fx`, `audio.fx_all`) ===",
    );

    if ui.button("[ON] Enable All DSP Effects (`audio.fx_all on`)") {
        audio_engine.set_all_effects_enabled(true);
    }
    ui.same_line();
    if ui.button("[OFF] Disable All DSP Effects (`audio.fx_all off`)") {
        audio_engine.set_all_effects_enabled(false);
    }
    ui.same_line();
    if ui.button("[RESET DSP EFFECTS]") {
        audio_engine.set_all_effects_enabled(true);
    }

    ui.spacing();
    ui.text("Individual DSP Effects:");

    // Display grid of all registered Audio Effects
    for (name, fx) in AudioEffect::all_names() {
        if !filter.is_empty() && !name.contains(filter) {
            continue;
        }

        let mut enabled = audio_engine.get_effect_enabled(*fx);
        let label = format!("{} (`audio.fx {}`)", name, name);
        if ui.checkbox(&label, &mut enabled) {
            audio_engine.set_effect_enabled(*fx, enabled);
        }
        if ui.is_item_hovered() {
            ui.tooltip_text(format!("Toggle DSP effect: {}", name));
        }
    }

    ui.spacing();
    ui.separator();
    ui.text_colored(
        [0.4, 0.8, 1.0, 1.0],
        "=== AUDIO DIAGNOSTICS & MONITORING ===",
    );

    ui.checkbox(
        "Display Audio Diagnostic Monitor (F3)",
        show_audio_diagnostic,
    );
    ui.checkbox(
        "Display Visual Overlay (Waves / Beams / Badges)",
        show_audio_visual_overlay,
    );

    if audio_stress_scene.enabled {
        ui.text_colored(
            [1.0, 0.8, 0.0, 1.0],
            "[ACTIVE] Audio Stress Test Scene is ACTIVE",
        );
        if ui.button("Stop Stress Test Scene") {
            audio_stress_scene.enabled = false;
        }
    } else if ui.button("Start Audio Stress Test Scene (32 sources)") {
        *show_audio_diagnostic = true;
        audio_stress_scene.enable(32, true, window_size_f32, audio_engine);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_audio_master_volume() {
        assert_eq!(default_audio_master_volume(), 0.80);
    }
}

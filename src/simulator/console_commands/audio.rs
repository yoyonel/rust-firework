use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::renderer_engine::RendererEngine;
use crate::window_engine::WindowEngine;
use crate::Simulator;

impl<R, P, A, W> Simulator<R, P, A, W>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
    W: WindowEngine,
{
    pub(crate) fn register_audio_commands(&mut self) {
        self.commands_registry
            .register_for_audio("audio.mute", |engine, _| {
                engine.mute();
                "Audio muted".to_string()
            });

        self.commands_registry
            .register_for_audio("audio.unmute", |engine, _| {
                engine.unmute();
                "Audio unmuted".to_string()
            });

        // --- Commandes de contrôle des effets DSP ---

        // audio.fx <effect_name> <on|off>
        // Toggle un effet DSP à chaud. Lock-free, sans overhead sur le thread CPAL.
        self.commands_registry
            .register_for_audio("audio.fx", |engine, input| {
                use crate::audio_engine::effect_flags::AudioEffect;
                let parts: Vec<&str> = input.split_whitespace().collect();
                match parts.as_slice() {
                    [_, effect_name, state] => {
                        if let Ok(fx) = effect_name.parse::<AudioEffect>() {
                            let enabled = matches!(*state, "on" | "1" | "true");
                            engine.set_effect_enabled(fx, enabled);
                            format!(
                                "Effect '{}' -> {}",
                                effect_name,
                                if enabled { "ON ✅" } else { "OFF ❌" }
                            )
                        } else {
                            let names: Vec<&str> =
                                AudioEffect::all_names().iter().map(|(n, _)| *n).collect();
                            format!(
                                "Unknown effect '{}'. Available: {}",
                                effect_name,
                                names.join(", ")
                            )
                        }
                    }
                    _ => format!(
                        "Usage: audio.fx <effect_name> <on|off>\nAvailable effects: {}",
                        AudioEffect::all_names()
                            .iter()
                            .map(|(n, _)| *n)
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                }
            });

        // Autocomplétion statique : noms des effets disponibles
        self.commands_registry.register_args(
            "audio.fx",
            crate::audio_engine::effect_flags::AudioEffect::all_names()
                .iter()
                .map(|(n, _)| *n)
                .collect(),
        );
        self.commands_registry.register_hint(
            "audio.fx",
            "<effect_name> <on|off> — Toggle a DSP effect at runtime",
        );

        // Valeur courante affichée en bleu dans la console lors de l'autocomplétion
        self.commands_registry
            .register_current_value("audio.fx", |audio, _| audio.get_effects_status());

        // audio.fx_all — Active ou désactive tous les effets DSP en même temps
        self.commands_registry
            .register_for_audio("audio.fx_all", |engine, input| {
                let parts: Vec<&str> = input.split_whitespace().collect();
                match parts.as_slice() {
                    [_, state] => {
                        let enabled = matches!(*state, "on" | "1" | "true");
                        engine.set_all_effects_enabled(enabled);
                        format!(
                            "All DSP effects -> {}",
                            if enabled { "ON ✅" } else { "OFF ❌" }
                        )
                    }
                    _ => "Usage: audio.fx_all <on|off>".to_string(),
                }
            });
        self.commands_registry
            .register_args("audio.fx_all", vec!["on", "off"]);
        self.commands_registry.register_hint(
            "audio.fx_all",
            "<on|off> — Enable or disable all DSP effects at runtime",
        );

        // audio.fx_status — Affiche l'état de tous les effets DSP
        self.commands_registry
            .register_for_audio("audio.fx_status", |engine, _| {
                format!("DSP Effects:\n  {}", engine.get_effects_status())
            });
        self.commands_registry.register_hint(
            "audio.fx_status",
            "List all DSP effects and their current state",
        );

        // audio.reverb_wet <gain 0.0..1.0> — Ajuste ou affiche le gain wet de la réverbération
        self.commands_registry
            .register_for_audio("audio.reverb_wet", |engine, input| {
                let parts: Vec<&str> = input.split_whitespace().collect();
                match parts.as_slice() {
                    [_, value_str] => {
                        if let Ok(val) = value_str.parse::<f32>() {
                            let clamped = val.clamp(0.0, 1.0);
                            engine.set_reverb_wet(clamped);
                            format!(
                                "Spatial Reverb Wet Gain -> {:.2} ({:.0}%)",
                                clamped,
                                clamped * 100.0
                            )
                        } else {
                            "Valeur invalide. Attendu : nombre flottant entre 0.0 et 1.0 (ex: 0.08)".to_string()
                        }
                    }
                    _ => {
                        let current = engine.get_reverb_wet();
                        format!(
                            "Spatial Reverb Wet Gain = {:.2} ({:.0}%)\n\
                            • 0.00 (0%)   : Signal pur sec (Dry). Aucune réverbération.\n\
                            • 0.08 (8%)   : [Par défaut] Écho d'espace extérieur subtil & naturel.\n\
                            • 0.20 (20%)  : Réverbération moyenne (espace semi-fermé / vallonné).\n\
                            • 0.50 (50%)  : Écho très fort (salle de concert / cathédrale).\n\
                            • 1.00 (100%) : 100% signal réverbéré (écho noyé).\n\
                            Usage: audio.reverb_wet <0.0..1.0>",
                            current,
                            current * 100.0
                        )
                    }
                }
            });
        self.commands_registry.register_hint(
            "audio.reverb_wet",
            "<0.0..1.0> — Set or view Spatial Reverb wet mix gain (Default: 0.08)",
        );
        self.commands_registry
            .register_current_value("audio.reverb_wet", |audio, _| {
                format!("{:.2}", audio.get_reverb_wet())
            });
    }
}

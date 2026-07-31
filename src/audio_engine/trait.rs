use crate::audio_engine::effect_flags::AudioEffect;
use glam::Vec2;

pub trait AudioEngine {
    fn play_rocket(&self, pos: Vec2, gain: f32);
    fn play_rocket_with_id(&self, id: u64, pos: Vec2, gain: f32);
    fn play_explosion(&self, pos: Vec2, gain: f32);
    fn play_explosion_with_id(&self, id: u64, pos: Vec2, gain: f32);
    fn start_audio_thread(&mut self, export_path: Option<&str>);
    fn stop_audio_thread(&mut self);

    // Getter/Setter
    fn set_listener_position(&mut self, pos: Vec2);
    fn get_listener_position(&self) -> Vec2;

    fn mute(&mut self);
    fn unmute(&mut self) -> f32;
    fn is_muted(&self) -> bool {
        false
    }

    // --- Contrôle des effets DSP à chaud ---

    /// Active ou désactive un effet DSP. Opération lock-free, safe depuis le main thread.
    fn set_effect_enabled(&self, effect: AudioEffect, enabled: bool);

    /// Active ou désactive tous les effets DSP en même temps. Opération lock-free.
    fn set_all_effects_enabled(&self, enabled: bool);

    /// Retourne `true` si l'effet est actuellement activé.
    fn get_effect_enabled(&self, effect: AudioEffect) -> bool;

    /// Retourne une chaîne listant tous les effets et leur état courant (pour la console).
    fn get_effects_status(&self) -> String;

    /// Récupère les événements de diagnostic de debug accumulés par le moteur audio.
    fn pop_debug_events(&self, _buf: &mut Vec<crate::audio_engine::types::AudioDebugEvent>) {}

    /// Récupère la distance d'atténuation maximale configurée.
    fn get_max_distance(&self) -> f32 {
        1000.0
    }

    /// Définir le gain wet de la réverbération spatiale (0.00 à 1.00).
    fn set_reverb_wet(&self, _wet: f32) {}

    /// Obtenir le gain wet de la réverbération spatiale.
    fn get_reverb_wet(&self) -> f32 {
        0.08
    }

    /// Définir le volume général du rendu audio (0.00 à 2.00). Opération lock-free.
    fn set_master_volume(&self, _volume: f32) {}

    /// Obtenir le volume général du rendu audio (0.00 à 2.00).
    fn get_master_volume(&self) -> f32 {
        0.8
    }

    /// Obtenir le dernier volume non nul sauvegardé (pour la persistance GUI en mode mute).
    fn get_saved_master_volume(&self) -> f32 {
        self.get_master_volume()
    }

    fn as_audio_engine(&self) -> &dyn AudioEngine;
}

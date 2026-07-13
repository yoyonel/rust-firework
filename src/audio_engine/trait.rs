use crate::audio_engine::effect_flags::AudioEffect;
use glam::Vec2;

pub trait AudioEngine {
    fn play_rocket(&self, pos: Vec2, gain: f32);
    fn play_rocket_with_id(&self, id: u64, pos: Vec2, gain: f32);
    fn play_explosion(&self, pos: Vec2, gain: f32);
    fn start_audio_thread(&mut self, export_path: Option<&str>);
    fn stop_audio_thread(&mut self);

    // Getter/Setter
    fn set_listener_position(&mut self, pos: Vec2);
    fn get_listener_position(&self) -> Vec2;

    fn mute(&mut self);
    fn unmute(&mut self) -> f32;

    // --- Contrôle des effets DSP à chaud ---

    /// Active ou désactive un effet DSP. Opération lock-free, safe depuis le main thread.
    fn set_effect_enabled(&self, effect: AudioEffect, enabled: bool);

    /// Active ou désactive tous les effets DSP en même temps. Opération lock-free.
    fn set_all_effects_enabled(&self, enabled: bool);

    /// Retourne `true` si l'effet est actuellement activé.
    fn get_effect_enabled(&self, effect: AudioEffect) -> bool;

    /// Retourne une chaîne listant tous les effets et leur état courant (pour la console).
    fn get_effects_status(&self) -> String;

    fn as_audio_engine(&self) -> &dyn AudioEngine;
}

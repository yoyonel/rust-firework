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

    fn as_audio_engine(&self) -> &dyn AudioEngine;
}

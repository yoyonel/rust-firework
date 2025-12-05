pub trait AudioEngine {
    fn play_rocket(&self, pos: (f32, f32), gain: f32);
    fn play_explosion(&self, pos: (f32, f32), gain: f32);
    fn start_audio_thread(&mut self, export_path: Option<&str>);
    fn stop_audio_thread(&mut self);

    // Getter/Setter
    fn set_listener_position(&mut self, pos: (f32, f32));
    fn get_listener_position(&self) -> (f32, f32);

    fn mute(&mut self);
    fn unmute(&mut self) -> f32;

    fn as_audio_engine(&self) -> &dyn AudioEngine;
}

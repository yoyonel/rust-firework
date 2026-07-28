pub mod audio;
pub mod physic;
pub mod renderer;

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
    pub fn init_console_commands(&mut self) {
        self.register_audio_commands();
        self.register_physic_commands();
        self.register_renderer_base_commands();
        self.register_bloom_commands();
        self.register_tonemapping_commands();
    }
}

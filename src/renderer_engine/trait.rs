use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::renderer_engine::command_console::CommandRegistry;

use anyhow::Result;

pub trait RendererEngine {
    fn run_loop<P: PhysicEngineFull, A: AudioEngine>(
        &mut self,
        physic: &mut P,
        audio: &mut A,
        commands_registry: &CommandRegistry,
    ) -> Result<()>;
    fn close(&mut self);
}

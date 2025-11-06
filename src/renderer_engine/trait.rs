use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngine;
use anyhow::Result;

pub trait RendererEngine {
    fn run_loop<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        physic: &mut P,
        audio: &mut A,
    ) -> Result<()>;
    fn close(&mut self);
}

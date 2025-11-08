use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngine;
use crate::renderer_engine::RendererEngine;

pub struct Simulator<R, P, A>
where
    R: RendererEngine,
    P: PhysicEngine,
    A: AudioEngine,
{
    renderer_engine: R,
    physic_engine: P,
    audio_engine: A,
}

impl<R, P, A> Simulator<R, P, A>
where
    R: RendererEngine,
    P: PhysicEngine,
    A: AudioEngine,
{
    pub fn new(renderer_engine: R, physic_engine: P, audio_engine: A) -> Self {
        Self {
            renderer_engine,
            physic_engine,
            audio_engine,
        }
    }

    pub fn run(&mut self, export_path: Option<&str>) -> anyhow::Result<()> {
        self.audio_engine.start_audio_thread(export_path);

        // On passe les références mutables des moteurs au Renderer
        self.renderer_engine
            .run_loop(&mut self.physic_engine, &mut self.audio_engine)?;

        Ok(())
    }

    pub fn close(&mut self) {
        self.renderer_engine.close();
        self.physic_engine.close();
        self.audio_engine.stop_audio_thread();
    }

    pub fn renderer_engine(&self) -> &R {
        &self.renderer_engine
    }

    pub fn physic_engine(&self) -> &P {
        &self.physic_engine
    }

    pub fn audio_engine(&self) -> &A {
        &self.audio_engine
    }
}

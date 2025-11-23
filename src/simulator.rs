use crate::audio_engine::AudioEngine;
use crate::physic_engine::{PhysicEngine, PhysicEngineFull};
use crate::renderer_engine::command_console::CommandRegistry;
use crate::renderer_engine::RendererEngine;

pub struct Simulator<R, P, A>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
{
    renderer_engine: R,
    physic_engine: P,
    pub audio_engine: A,
    pub commands_registry: CommandRegistry,
}

impl<R, P, A> Simulator<R, P, A>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
{
    pub fn new(renderer_engine: R, physic_engine: P, audio_engine: A) -> Self {
        Self {
            renderer_engine,
            physic_engine,
            audio_engine,
            commands_registry: CommandRegistry::new(),
        }
    }

    pub fn run(&mut self, export_path: Option<&str>) -> anyhow::Result<()> {
        self.audio_engine.start_audio_thread(export_path);

        // On passe les références mutables des moteurs au Renderer
        self.renderer_engine.run_loop(
            &mut self.physic_engine,
            &mut self.audio_engine,
            &self.commands_registry,
        )?;

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

impl<R, P, A> Simulator<R, P, A>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
{
    pub fn init_console_commands(&mut self) {
        // Commande "mute"
        self.commands_registry.register_for_audio(
            "audio.mute",
            |engine: &mut dyn AudioEngine, _args| {
                engine.mute();
                "Audio muted".to_string()
            },
        );

        // Tu pourrais ajouter d'autres commandes ici (unmute, volume, etc.)
        self.commands_registry.register_for_audio(
            "audio.unmute",
            |engine: &mut dyn AudioEngine, _args| {
                engine.unmute();
                "Audio unmuted".to_string()
            },
        );

        self.commands_registry
            // register_physic est ici une méthode qui stocke la closure pour
            // exécution future.
            .register_for_physic("physic.config", |engine: &mut dyn PhysicEngine, _args| {
                // <-- LE CAST À L'INTÉRIEUR DE LA CLOSURE
                // Le moteur passé ici n'est que la partie Dyn Compatible.
                // Or, get_config() est bien dans PhysicEngine (maintenant Dyn Compatible).
                format!("{:#?}", engine.get_config())
            });
    }
}

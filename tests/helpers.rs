use fireworks_sim::audio_engine::AudioEngine;
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::PhysicEngine;
use fireworks_sim::physic_engine::{particle::Particle, types::UpdateResult};
use fireworks_sim::renderer_engine::RendererEngine;
use std::cell::RefCell;

#[allow(unused)]
pub struct DummyAudio;

impl AudioEngine for DummyAudio {
    fn get_listener_position(&self) -> (f32, f32) {
        (0.0, 0.0)
    }
    fn set_listener_position(&mut self, _pos: (f32, f32)) {}
    fn play_rocket(&self, _pos: (f32, f32), _gain: f32) {}
    fn play_explosion(&self, _pos: (f32, f32), _gain: f32) {}
    fn start_audio_thread(&mut self, _export_path: Option<&str>) {}
    fn stop_audio_thread(&mut self) {}
}

pub struct DummyPhysic;
impl PhysicEngine for DummyPhysic {
    fn update(&mut self, _dt: f32) -> UpdateResult<'_> {
        UpdateResult {
            new_rocket: None,
            triggered_explosions: &[],
        }
    }
    fn close(&mut self) {}
    fn set_window_width(&mut self, _width: f32) {}
    fn iter_active_particles<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
        // Crée un itérateur vide, compatible avec la signature
        std::iter::empty()
    }
    fn iter_active_heads<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
        // Crée un itérateur vide, compatible avec la signature
        std::iter::empty()
    }
    fn reload_config(&mut self, _config: &PhysicConfig) -> bool {
        false
    }
}

#[allow(dead_code)]
pub struct DummyRenderer;
#[allow(dead_code)]
impl RendererEngine for DummyRenderer {
    fn run_loop<P: PhysicEngine, A: AudioEngine>(
        &mut self,
        _physic: &mut P,
        _audio: &mut A,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn close(&mut self) {
        println!("Closing renderer...");
    }
}

#[allow(dead_code)]
pub struct LoggingRenderer {
    pub log: Vec<String>,
}

impl fireworks_sim::renderer_engine::RendererEngine for LoggingRenderer {
    fn run_loop<
        P: fireworks_sim::physic_engine::PhysicEngine,
        A: fireworks_sim::audio_engine::AudioEngine,
    >(
        &mut self,
        physic: &mut P,
        audio: &mut A,
    ) -> anyhow::Result<()> {
        self.log.push("run_loop called".into());
        // Simule des appels au physic et audio engine
        physic.update(0.016);
        audio.play_rocket((0.0, 0.0), 0.6);
        Ok(())
    }

    fn close(&mut self) {
        self.log.push("close called".into());
    }
}

#[allow(dead_code)]
pub struct LoggingAudio {
    pub log: RefCell<Vec<String>>,
}

impl LoggingAudio {
    // pub fn new() -> Self {
    //     Self {
    //         log: RefCell::new(vec![]),
    //     }
    // }
}

impl fireworks_sim::audio_engine::AudioEngine for LoggingAudio {
    fn get_listener_position(&self) -> (f32, f32) {
        (0.0, 0.0)
    }

    fn set_listener_position(&mut self, _pos: (f32, f32)) {
        self.log
            .borrow_mut()
            .push("set_listener_position called".into());
    }

    fn play_rocket(&self, _pos: (f32, f32), _gain: f32) {
        self.log.borrow_mut().push("play_rocket called".into());
    }

    fn play_explosion(&self, _pos: (f32, f32), _gain: f32) {
        self.log.borrow_mut().push("play_explosion called".into());
    }

    fn start_audio_thread(&mut self, _export_path: Option<&str>) {
        self.log
            .borrow_mut()
            .push("start_audio_thread called".into());
    }

    fn stop_audio_thread(&mut self) {
        self.log
            .borrow_mut()
            .push("stop_audio_thread called".into());
    }
}

#[allow(dead_code)]
pub struct LoggingPhysic {
    pub log: Vec<String>,
}

impl fireworks_sim::physic_engine::PhysicEngine for LoggingPhysic {
    fn update(&mut self, _dt: f32) -> UpdateResult<'_> {
        self.log.push("update called".into());
        UpdateResult {
            new_rocket: None,
            triggered_explosions: &[],
        }
    }
    fn close(&mut self) {
        self.log.push("close called".into());
    }
    fn set_window_width(&mut self, _width: f32) {
        self.log.push("set_width called".into());
    }
    fn iter_active_particles<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
        // Crée un itérateur vide, compatible avec la signature
        std::iter::empty()
    }
    fn iter_active_heads<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
        // Crée un itérateur vide, compatible avec la signature
        std::iter::empty()
    }
    fn reload_config(&mut self, _config: &PhysicConfig) -> bool {
        false
    }
}

use fireworks_sim::audio_engine::AudioEngine;
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::particle::Particle;
use fireworks_sim::physic_engine::types::UpdateResult;
use fireworks_sim::physic_engine::{PhysicEngine, PhysicEngineFull, PhysicEngineIterator};
use fireworks_sim::renderer_engine::command_console::CommandRegistry;
use fireworks_sim::renderer_engine::RendererEngine;
use std::cell::RefCell;
use std::rc::Rc;

// --- Shared Types ---
#[allow(dead_code)]
pub type SharedLog = Rc<RefCell<Vec<String>>>;

// --- Dummy Mocks (Minimal implementation, no logging) ---

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
    fn mute(&mut self) {}
    fn unmute(&mut self) -> f32 {
        1.0
    }
}

#[allow(dead_code)]
pub struct DummyPhysic {
    pub config: PhysicConfig,
}

impl Default for DummyPhysic {
    fn default() -> Self {
        Self {
            config: PhysicConfig::default(),
        }
    }
}

impl PhysicEngine for DummyPhysic {
    fn update(&mut self, _dt: f32) -> UpdateResult<'_> {
        UpdateResult {
            new_rocket: None,
            triggered_explosions: &[],
        }
    }
    fn close(&mut self) {}
    fn set_window_width(&mut self, _width: f32) {}
    fn reload_config(&mut self, _config: &PhysicConfig) -> bool {
        false
    }
    fn get_config(&self) -> &PhysicConfig {
        &self.config
    }
}

impl PhysicEngineIterator for DummyPhysic {
    fn iter_active_particles<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
        std::iter::empty()
    }
    fn iter_active_heads_not_exploded<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
        std::iter::empty()
    }
}

impl PhysicEngineFull for DummyPhysic {}

#[allow(dead_code)]
pub struct DummyRenderer;
#[allow(dead_code)]
impl RendererEngine for DummyRenderer {
    fn run_loop<P: PhysicEngineFull, A: AudioEngine>(
        &mut self,
        _physic: &mut P,
        _audio: &mut A,
        _registry: &CommandRegistry,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn close(&mut self) {
        println!("Closing renderer...");
    }
}

// --- Test Mocks (Logging + Failure Injection) ---

#[allow(dead_code)]
pub struct TestAudio {
    pub log: SharedLog,
    pub fail_on_start: bool,
}

#[allow(dead_code)]
impl TestAudio {
    pub fn new(log: SharedLog) -> Self {
        Self {
            log,
            fail_on_start: false,
        }
    }
}

impl AudioEngine for TestAudio {
    fn start_audio_thread(&mut self, _export_path: Option<&str>) {
        self.log.borrow_mut().push("audio.start".into());
        if self.fail_on_start {
            panic!("AudioEngine failed at start_audio_thread");
        }
    }
    fn stop_audio_thread(&mut self) {
        self.log.borrow_mut().push("audio.stop".into());
    }
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
    fn mute(&mut self) {
        self.log.borrow_mut().push("mute called".into());
    }
    fn unmute(&mut self) -> f32 {
        self.log.borrow_mut().push("unmute called".into());
        1.0
    }
}

#[allow(dead_code)]
pub struct TestPhysic {
    pub log: SharedLog,
    pub config: PhysicConfig,
    pub fail_on_update: bool,
}

#[allow(dead_code)]
impl TestPhysic {
    pub fn new(log: SharedLog) -> Self {
        Self {
            log,
            config: PhysicConfig::default(),
            fail_on_update: false,
        }
    }
}

impl PhysicEngine for TestPhysic {
    fn update(&mut self, _dt: f32) -> UpdateResult<'_> {
        self.log.borrow_mut().push("physic.update".into());
        if self.fail_on_update {
            panic!("PhysicEngine failed during update");
        }
        UpdateResult {
            new_rocket: None,
            triggered_explosions: &[],
        }
    }
    fn set_window_width(&mut self, _width: f32) {
        self.log.borrow_mut().push("physic.set_width".into());
    }
    fn close(&mut self) {
        self.log.borrow_mut().push("physic.close".into());
    }
    fn reload_config(&mut self, _config: &PhysicConfig) -> bool {
        false
    }
    fn get_config(&self) -> &PhysicConfig {
        &self.config
    }
}

impl PhysicEngineIterator for TestPhysic {
    fn iter_active_particles<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
        std::iter::empty()
    }
    fn iter_active_heads_not_exploded<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
        std::iter::empty()
    }
}

impl PhysicEngineFull for TestPhysic {}

#[allow(dead_code)]
pub struct TestRenderer {
    pub log: SharedLog,
    pub fail_on_run_loop: bool,
}

#[allow(dead_code)]
impl TestRenderer {
    pub fn new(log: SharedLog) -> Self {
        Self {
            log,
            fail_on_run_loop: false,
        }
    }
}

impl RendererEngine for TestRenderer {
    fn run_loop<P: PhysicEngineFull, A: AudioEngine>(
        &mut self,
        physic: &mut P,
        audio: &mut A,
        _registry: &CommandRegistry,
    ) -> anyhow::Result<()> {
        self.log.borrow_mut().push("renderer.run_loop.start".into());
        if self.fail_on_run_loop {
            return Err(anyhow::anyhow!("RendererEngine simulated failure"));
        }

        // Simule une frame
        physic.update(0.016);
        audio.play_rocket((0.0, 0.0), 1.0);

        self.log.borrow_mut().push("renderer.run_loop.end".into());
        Ok(())
    }

    fn close(&mut self) {
        self.log.borrow_mut().push("renderer.close".into());
    }
}

// Legacy Logging structs (kept for compatibility if needed, but Test* structs are preferred)
// We can alias them or reimplement them if we want to avoid breaking changes immediately,
// but since we are refactoring, we will encourage using Test* structs.
// For now, I'll remove the old Logging* structs to force migration and cleanliness.

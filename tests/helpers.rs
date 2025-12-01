use fireworks_sim::audio_engine::AudioEngine;
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::particle::Particle;
use fireworks_sim::physic_engine::types::UpdateResult;
use fireworks_sim::physic_engine::{
    ParticleType, PhysicEngine, PhysicEngineFull, PhysicEngineIterator,
};
use fireworks_sim::renderer_engine::BloomPass;
use fireworks_sim::renderer_engine::RendererEngine;

use anyhow::Result;
use fireworks_sim::window_engine::{ImguiSystem, WindowEngine, WindowEvents};
use glfw::{CursorMode, WindowMode};
use std::cell::RefCell;
use std::rc::Rc;

// --- Shared Types ---
#[allow(dead_code)]
pub type SharedLog = Rc<RefCell<Vec<String>>>;

// --- Dummy Window Engine ---

#[allow(dead_code)]
pub struct DummyWindowEngine;

impl WindowEngine for DummyWindowEngine {
    fn init(_width: i32, _height: i32, _title: &str) -> Result<Self> {
        Ok(Self)
    }

    fn poll_events(&mut self) {}
    fn swap_buffers(&mut self) {}
    fn should_close(&self) -> bool {
        false
    }
    fn set_should_close(&mut self, _value: bool) {}
    fn get_size(&self) -> (i32, i32) {
        (800, 600)
    }
    fn get_pos(&self) -> (i32, i32) {
        (0, 0)
    }
    fn is_fullscreen(&self) -> bool {
        false
    }
    fn set_monitor(
        &mut self,
        _mode: WindowMode,
        _xpos: i32,
        _ypos: i32,
        _width: u32,
        _height: u32,
        _refresh_rate: Option<u32>,
    ) {
    }
    fn set_cursor_mode(&mut self, _mode: CursorMode) {}
    fn make_current(&mut self) {}
    fn get_glfw(&self) -> &glfw::Glfw {
        panic!("DummyWindowEngine does not have a real GLFW instance")
    }
    fn get_window_mut(&mut self) -> &mut glfw::PWindow {
        panic!("DummyWindowEngine does not have a real window")
    }
    fn get_events(&self) -> &WindowEvents {
        panic!("DummyWindowEngine does not have real events")
    }
    fn get_imgui_system_mut(&mut self) -> &mut ImguiSystem {
        panic!("DummyWindowEngine does not have a real imgui system")
    }
    fn get_window_and_imgui_mut(&mut self) -> (&mut glfw::PWindow, &mut ImguiSystem) {
        panic!("DummyWindowEngine does not have real window/imgui")
    }
}

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
    pub particles: Vec<Particle>,
}

impl Default for DummyPhysic {
    fn default() -> Self {
        Self {
            config: PhysicConfig::default(),
            particles: Vec::new(),
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
    fn iter_active_particles<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(self.particles.iter())
    }
    fn iter_active_heads_not_exploded<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(self.particles.iter())
    }
    fn iter_particles_by_type<'a>(
        &'a self,
        particle_type: ParticleType,
    ) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(
            self.particles
                .iter()
                .filter(move |p| p.particle_type == particle_type),
        )
    }
}

impl PhysicEngineFull for DummyPhysic {}

#[allow(dead_code)]
pub struct DummyRenderer;
#[allow(dead_code)]
impl RendererEngine for DummyRenderer {
    fn render_frame<P: PhysicEngineIterator>(&mut self, _physic: &P) -> usize {
        0
    }
    fn set_window_size(&mut self, _width: i32, _height: i32) {}
    fn recreate_buffers(&mut self, _max_particles: usize) {}
    fn reload_shaders(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn close(&mut self) {
        println!("Closing renderer...");
    }
    fn bloom_pass_mut(&mut self) -> &mut BloomPass {
        panic!("DummyRenderer does not have a real bloom pass")
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
    fn iter_active_particles<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(std::iter::empty())
    }
    fn iter_active_heads_not_exploded<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(std::iter::empty())
    }
    fn iter_particles_by_type<'a>(
        &'a self,
        _particle_type: ParticleType,
    ) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
        Box::new(std::iter::empty())
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
    fn render_frame<P: PhysicEngineIterator>(&mut self, _physic: &P) -> usize {
        self.log.borrow_mut().push("renderer.render_frame".into());
        if self.fail_on_run_loop {
            panic!("RendererEngine simulated failure");
        }
        0
    }
    fn set_window_size(&mut self, _width: i32, _height: i32) {
        self.log
            .borrow_mut()
            .push("renderer.set_window_size".into());
    }
    fn recreate_buffers(&mut self, _max_particles: usize) {
        self.log
            .borrow_mut()
            .push("renderer.recreate_buffers".into());
    }
    fn reload_shaders(&mut self) -> Result<(), String> {
        self.log.borrow_mut().push("renderer.reload_shaders".into());
        Ok(())
    }
    fn close(&mut self) {
        self.log.borrow_mut().push("renderer.close".into());
    }
    fn bloom_pass_mut(&mut self) -> &mut fireworks_sim::renderer_engine::BloomPass {
        panic!("TestRenderer does not have a real BloomPass")
    }
}

// Legacy Logging structs (kept for compatibility if needed, but Test* structs are preferred)
// We can alias them or reimplement them if we want to avoid breaking changes immediately,
// but since we are refactoring, we will encourage using Test* structs.
// For now, I'll remove the old Logging* structs to force migration and cleanliness.

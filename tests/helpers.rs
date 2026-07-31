use crossbeam_channel::Sender; // NOUVEAU
use fireworks_sim::audio_engine::effect_flags::AudioEffect;
use fireworks_sim::audio_engine::{AudioEngine, DopplerEvent}; // MODIFIÉ : ajout de DopplerEvent
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::explosion_shape::ExplosionShape;
use fireworks_sim::physic_engine::particle::Particle;
use fireworks_sim::physic_engine::types::UpdateResult;
use fireworks_sim::physic_engine::{
    ParticleType, PhysicEngine, PhysicEngineFull, PhysicEngineIterator,
};
use fireworks_sim::renderer_engine::RendererEngine;

use anyhow::Result;
use fireworks_sim::window_engine::{ImguiSystem, WindowEngine, WindowEvents};
use glfw::{Context, CursorMode, WindowMode};
use std::cell::RefCell;
use std::rc::Rc;

// --- Shared Types ---
#[allow(dead_code)]
pub type SharedLog = Rc<RefCell<Vec<String>>>;

// --- Dummy Window Engine ---

#[allow(dead_code)]
pub struct DummyWindowEngine {
    pub window: glfw::PWindow,
    pub events: WindowEvents,
    pub glfw: std::mem::ManuallyDrop<glfw::Glfw>,
}

impl Default for DummyWindowEngine {
    fn default() -> Self {
        let mut glfw = glfw::init(glfw::fail_on_errors)
            .ok()
            .or_else(|| glfw::init(glfw::log_errors).ok())
            .expect("DummyWindowEngine requires GLFW context for WindowEvents");

        glfw.window_hint(glfw::WindowHint::Visible(false));
        let (mut window, events) = glfw
            .create_window(1, 1, "dummy", glfw::WindowMode::Windowed)
            .expect("DummyWindowEngine requires window creation");
        window.make_current();
        gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

        Self {
            window,
            events,
            glfw: std::mem::ManuallyDrop::new(glfw),
        }
    }
}

impl WindowEngine for DummyWindowEngine {
    fn init(_width: i32, _height: i32, _title: &str) -> Result<Self> {
        Ok(Self::default())
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
        &self.glfw
    }
    fn get_window_mut(&mut self) -> &mut glfw::PWindow {
        &mut self.window
    }
    fn get_events(&self) -> &WindowEvents {
        &self.events
    }
    fn get_imgui_system_mut(&mut self) -> &mut ImguiSystem {
        panic!("DummyWindowEngine does not have a real imgui system")
    }
    fn get_window_and_imgui_mut(&mut self) -> (&mut glfw::PWindow, &mut ImguiSystem) {
        panic!("DummyWindowEngine does not have real window/imgui")
    }
}

use glam::Vec2;

// --- Dummy Mocks (Minimal implementation, no logging) ---

#[allow(unused)]
pub struct DummyAudio;

impl AudioEngine for DummyAudio {
    fn get_listener_position(&self) -> Vec2 {
        Vec2::ZERO
    }
    fn set_listener_position(&mut self, _pos: Vec2) {}
    fn play_rocket(&self, _pos: Vec2, _gain: f32) {}
    fn play_rocket_with_id(&self, _id: u64, _pos: Vec2, _gain: f32) {}
    fn play_explosion(&self, _pos: Vec2, _gain: f32) {}
    fn play_explosion_with_id(&self, _id: u64, _pos: Vec2, _gain: f32) {}
    fn start_audio_thread(&mut self, _export_path: Option<&str>) {}
    fn stop_audio_thread(&mut self) {}
    fn mute(&mut self) {}
    fn unmute(&mut self) -> f32 {
        1.0
    }
    fn as_audio_engine(&self) -> &dyn AudioEngine {
        self
    }
    fn set_effect_enabled(&self, _: AudioEffect, _: bool) {}
    fn set_all_effects_enabled(&self, _: bool) {}
    fn get_effect_enabled(&self, _: AudioEffect) -> bool {
        true
    }
    fn get_effects_status(&self) -> String {
        "Binaural: ON".to_string()
    }
}

#[allow(dead_code)]
#[derive(Default)]
pub struct DummyPhysic {
    pub config: PhysicConfig,
    pub particles: Vec<Particle>,
    pub explosion_shape: ExplosionShape,
}

impl PhysicEngine for DummyPhysic {
    fn update(&mut self, _dt: f32) -> UpdateResult<'_> {
        UpdateResult {
            new_rocket: None,
            triggered_explosions: &[],
            triggered_explosion_ids: &[],
            anticipated_rocket_launch: None,
            anticipated_explosions: &[],
        }
    }
    fn set_doppler_sender(&mut self, _sender: Sender<DopplerEvent>) {}
    fn close(&mut self) {}
    fn set_window_width(&mut self, _width: f32) {}
    fn reload_config(&mut self, _config: &PhysicConfig) -> bool {
        false
    }
    fn get_config(&self) -> &PhysicConfig {
        &self.config
    }
    fn get_config_mut(&mut self) -> &mut PhysicConfig {
        &mut self.config
    }
    fn set_explosion_shape(&mut self, shape: ExplosionShape) {
        self.explosion_shape = shape;
    }
    fn get_explosion_shape(&self) -> &ExplosionShape {
        &self.explosion_shape
    }
    fn load_explosion_image(
        &mut self,
        _path: &str,
        _scale: f32,
        _flight_time: f32,
    ) -> Result<(), String> {
        Ok(()) // Mock: always succeeds
    }
    fn load_explosion_image_weighted(
        &mut self,
        _path: &str,
        _scale: f32,
        _flight_time: f32,
        _weight: f32,
    ) -> Result<(), String> {
        Ok(()) // Mock: always succeeds
    }
    fn set_explosion_image_weight(&mut self, _name: &str, _weight: f32) -> Result<(), String> {
        Ok(()) // Mock: always succeeds
    }

    fn as_physic_engine(&self) -> &dyn PhysicEngine {
        self
    }
}

impl PhysicEngineIterator for DummyPhysic {
    fn for_each_active_particle(&self, f: &mut dyn FnMut(&Particle)) {
        for p in &self.particles {
            f(p);
        }
    }
    fn for_each_active_head_not_exploded(&self, f: &mut dyn FnMut(&Particle)) {
        for p in &self.particles {
            f(p);
        }
    }
    fn for_each_particle_of_type(&self, particle_type: ParticleType, f: &mut dyn FnMut(&Particle)) {
        for p in &self.particles {
            if p.particle_type == particle_type {
                f(p);
            }
        }
    }
}

impl PhysicEngineFull for DummyPhysic {}

#[allow(dead_code)]
pub struct DummyRenderer {
    pub bloom_pass: fireworks_sim::renderer_engine::BloomPass,
}

#[allow(dead_code)]
impl DummyRenderer {
    pub fn new() -> Self {
        Self {
            bloom_pass: fireworks_sim::renderer_engine::BloomPass::new_dummy(),
        }
    }
}

impl Default for DummyRenderer {
    fn default() -> Self {
        Self::new()
    }
}

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
    fn bloom_pass_mut(&mut self) -> &mut fireworks_sim::renderer_engine::BloomPass {
        &mut self.bloom_pass
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
    fn get_listener_position(&self) -> Vec2 {
        Vec2::ZERO
    }
    fn set_listener_position(&mut self, _pos: Vec2) {
        self.log
            .borrow_mut()
            .push("set_listener_position called".into());
    }
    fn play_rocket(&self, _pos: Vec2, _gain: f32) {
        self.log.borrow_mut().push("play_rocket called".into());
    }
    fn play_rocket_with_id(&self, _id: u64, _pos: Vec2, _gain: f32) {
        self.log
            .borrow_mut()
            .push("play_rocket_with_id called".into());
    }
    fn play_explosion(&self, _pos: Vec2, _gain: f32) {
        self.log.borrow_mut().push("play_explosion called".into());
    }
    fn play_explosion_with_id(&self, _id: u64, _pos: Vec2, _gain: f32) {
        self.log
            .borrow_mut()
            .push("play_explosion_with_id called".into());
    }
    fn mute(&mut self) {
        self.log.borrow_mut().push("mute called".into());
    }
    fn unmute(&mut self) -> f32 {
        self.log.borrow_mut().push("audio.unmute".into());
        1.0
    }
    fn as_audio_engine(&self) -> &dyn AudioEngine {
        self
    }
    fn set_effect_enabled(&self, effect: AudioEffect, enabled: bool) {
        self.log.borrow_mut().push(format!(
            "set_effect_enabled called: {:?} = {}",
            effect, enabled
        ));
    }
    fn set_all_effects_enabled(&self, enabled: bool) {
        self.log
            .borrow_mut()
            .push(format!("set_all_effects_enabled called: {}", enabled));
    }
    fn get_effect_enabled(&self, _effect: AudioEffect) -> bool {
        self.log
            .borrow_mut()
            .push("get_effect_enabled called".into());
        true
    }
    fn get_effects_status(&self) -> String {
        self.log
            .borrow_mut()
            .push("get_effects_status called".into());
        "test_all_on".to_string()
    }
}

#[allow(dead_code)]
pub struct TestPhysic {
    pub log: SharedLog,
    pub config: PhysicConfig,
    pub pending_config: PhysicConfig,
    pub fail_on_update: bool,
    pub explosion_shape: ExplosionShape,
}

#[allow(dead_code)]
impl TestPhysic {
    pub fn new(log: SharedLog) -> Self {
        Self {
            log,
            config: PhysicConfig::default(),
            pending_config: PhysicConfig::default(),
            fail_on_update: false,
            explosion_shape: ExplosionShape::default(),
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
            triggered_explosion_ids: &[],
            anticipated_rocket_launch: None,
            anticipated_explosions: &[],
        }
    }
    fn set_doppler_sender(&mut self, _sender: Sender<DopplerEvent>) {
        self.log
            .borrow_mut()
            .push("physic.set_doppler_sender".into());
    }
    fn set_window_width(&mut self, _width: f32) {
        self.log.borrow_mut().push("physic.set_width".into());
    }
    fn close(&mut self) {
        self.log.borrow_mut().push("physic.close".into());
    }
    fn reload_config(&mut self, config: &PhysicConfig) -> bool {
        self.config = config.clone();
        self.pending_config = config.clone();
        false
    }
    fn get_config(&self) -> &PhysicConfig {
        &self.config
    }
    fn get_config_mut(&mut self) -> &mut PhysicConfig {
        &mut self.pending_config
    }
    fn set_explosion_shape(&mut self, shape: ExplosionShape) {
        self.explosion_shape = shape;
    }
    fn get_explosion_shape(&self) -> &ExplosionShape {
        &self.explosion_shape
    }
    fn load_explosion_image(
        &mut self,
        _path: &str,
        _scale: f32,
        _flight_time: f32,
    ) -> Result<(), String> {
        self.log
            .borrow_mut()
            .push("physic.load_explosion_image".into());
        Ok(()) // Mock: always succeeds
    }
    fn load_explosion_image_weighted(
        &mut self,
        _path: &str,
        _scale: f32,
        _flight_time: f32,
        _weight: f32,
    ) -> Result<(), String> {
        self.log
            .borrow_mut()
            .push("physic.load_explosion_image_weighted".into());
        Ok(()) // Mock: always succeeds
    }
    fn set_explosion_image_weight(&mut self, _name: &str, _weight: f32) -> Result<(), String> {
        self.log
            .borrow_mut()
            .push("physic.set_explosion_image_weight".into());
        Ok(()) // Mock: always succeeds
    }
    fn as_physic_engine(&self) -> &dyn PhysicEngine {
        self
    }
}

impl PhysicEngineIterator for TestPhysic {
    fn for_each_active_particle(&self, _f: &mut dyn FnMut(&Particle)) {}
    fn for_each_active_head_not_exploded(&self, _f: &mut dyn FnMut(&Particle)) {}
    fn for_each_particle_of_type(
        &self,
        _particle_type: ParticleType,
        _f: &mut dyn FnMut(&Particle),
    ) {
    }
}

impl PhysicEngineFull for TestPhysic {}

#[allow(dead_code)]
pub struct TestRenderer {
    pub log: SharedLog,
    pub fail_on_run_loop: bool,
    pub bloom_pass: fireworks_sim::renderer_engine::BloomPass,
}

#[allow(dead_code)]
impl TestRenderer {
    pub fn new(log: SharedLog) -> Self {
        Self {
            log,
            fail_on_run_loop: false,
            bloom_pass: fireworks_sim::renderer_engine::BloomPass::new_dummy(),
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
        &mut self.bloom_pass
    }
}

// Legacy Logging structs (kept for compatibility if needed, but Test* structs are preferred)
// We can alias them or reimplement them if we want to avoid breaking changes immediately,
// but since we are refactoring, we will encourage using Test* structs.
// For now, I'll remove the old Logging* structs to force migration and cleanliness.

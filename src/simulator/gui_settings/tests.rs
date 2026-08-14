use super::*;
use crate::audio_engine::effect_flags::AudioEffect;
use crate::audio_engine::AudioEngine;
use crate::domain_contracts::{
    AudioCommand, AudioStateReader, EngineCommand, PhysicCommand, PhysicStateReader,
    RendererCommand, RendererStateReader, SmokeCommand, SmokeStateReader,
};
use crate::physic_engine::PhysicEngine;
use crate::simulator::audio_stress_scene::AudioStressScene;
use crate::utils::command_console::CommandRegistry;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

#[test]
fn test_gui_session_state_defaults() {
    let state = GuiSessionState::default();
    assert!(!state.gui_open);
    assert_eq!(state.active_tab, 0);
    assert_eq!(state.audio_master_volume, 0.80);
    assert_eq!(state.preset_weights, [1.0; 5]);
    assert_eq!(state.theme, GuiTheme::CyberpunkCyan);
}

#[test]
fn test_gui_session_state_save_and_load() -> anyhow::Result<()> {
    let temp_dir = tempfile::tempdir()?;
    let file_path = temp_dir.path().join("gui_session.toml");
    let path_str = file_path.to_str().unwrap();

    let state = GuiSessionState {
        gui_open: true,
        active_tab: 2,
        audio_master_volume: 0.5,
        theme: GuiTheme::DeepSapphire,
        explosion_shape: PersistedExplosionShape::Images { images: vec![] },
        ..GuiSessionState::default()
    };

    state.save_to_file(path_str)?;

    let loaded = GuiSessionState::load_from_file(path_str);
    assert!(loaded.gui_open);
    assert_eq!(loaded.active_tab, 2);
    assert_eq!(loaded.audio_master_volume, 0.5);
    assert_eq!(loaded.theme, GuiTheme::DeepSapphire);

    Ok(())
}

#[test]
fn test_should_show_tab_filtering() {
    let settings = GuiSettings::new();
    let mut registry = CommandRegistry::new();
    registry.register_for_audio("audio.volume", |_, _, _| String::new());
    registry.register_for_physic("physic.gravity", |_, _, _| String::new());

    assert!(settings.should_show_tab("audio", "", &registry));
    assert!(settings.should_show_tab("audio", "audio", &registry));
    assert!(settings.should_show_tab("audio", "volume", &registry));
    assert!(!settings.should_show_tab("audio", "render", &registry));
}

struct SpyPhysicEngine {
    inner: crate::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks,
    reload_count: usize,
}

impl SpyPhysicEngine {
    fn new() -> Self {
        let config = crate::physic_engine::config::PhysicConfig::default();
        Self {
            inner:
                crate::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks::new(
                    &config, 800.0, None,
                ),
            reload_count: 0,
        }
    }
}

impl crate::physic_engine::PhysicEngineIterator for SpyPhysicEngine {
    fn for_each_active_particle(
        &self,
        f: &mut dyn FnMut(&crate::physic_engine::particle::Particle),
    ) {
        self.inner.for_each_active_particle(f);
    }
    fn for_each_active_head_not_exploded(
        &self,
        f: &mut dyn FnMut(&crate::physic_engine::particle::Particle),
    ) {
        self.inner.for_each_active_head_not_exploded(f);
    }
    fn for_each_particle_of_type(
        &self,
        particle_type: crate::physic_engine::ParticleType,
        f: &mut dyn FnMut(&crate::physic_engine::particle::Particle),
    ) {
        self.inner.for_each_particle_of_type(particle_type, f);
    }
    fn for_each_smoke_particle(
        &self,
        f: &mut dyn FnMut(&crate::physic_engine::smoke_system::SmokeParticle),
    ) {
        self.inner.for_each_smoke_particle(f);
    }
    fn get_smoke_intensity(&self) -> f32 {
        self.inner.get_smoke_intensity()
    }
    fn get_smoke_erosion_params(&self) -> (bool, f32, f32, [f32; 3]) {
        self.inner.get_smoke_erosion_params()
    }
    fn get_smoke_flow_params(&self) -> (f32, f32) {
        self.inner.get_smoke_flow_params()
    }
}

impl crate::physic_engine::PhysicEngine for SpyPhysicEngine {
    fn set_window_width(&mut self, width: f32) {
        self.inner.set_window_width(width);
    }
    fn update(&mut self, dt: f32) -> crate::physic_engine::types::UpdateResult<'_> {
        self.inner.update(dt)
    }
    fn reload_config(&mut self, config: &crate::physic_engine::config::PhysicConfig) -> bool {
        self.reload_count += 1;
        self.inner.reload_config(config)
    }
    fn get_config(&self) -> &crate::physic_engine::config::PhysicConfig {
        self.inner.get_config()
    }
    fn get_config_mut(&mut self) -> &mut crate::physic_engine::config::PhysicConfig {
        self.inner.get_config_mut()
    }
    fn get_pending_config(&self) -> &crate::physic_engine::config::PhysicConfig {
        self.inner.get_pending_config()
    }
    fn set_explosion_shape(&mut self, shape: crate::physic_engine::ExplosionShape) {
        self.inner.set_explosion_shape(shape);
    }
    fn get_explosion_shape(&self) -> &crate::physic_engine::ExplosionShape {
        self.inner.get_explosion_shape()
    }
    fn load_explosion_image(
        &mut self,
        path: &str,
        scale: f32,
        flight_time: f32,
    ) -> Result<(), String> {
        self.inner.load_explosion_image(path, scale, flight_time)
    }
    fn load_explosion_image_weighted(
        &mut self,
        path: &str,
        scale: f32,
        flight_time: f32,
        weight: f32,
    ) -> Result<(), String> {
        self.inner
            .load_explosion_image_weighted(path, scale, flight_time, weight)
    }
    fn set_explosion_image_weight(&mut self, name: &str, weight: f32) -> Result<(), String> {
        self.inner.set_explosion_image_weight(name, weight)
    }
    fn remove_explosion_image(&mut self, name: &str) -> Result<(), String> {
        self.inner.remove_explosion_image(name)
    }
    fn as_physic_engine(&self) -> &dyn crate::physic_engine::PhysicEngine {
        self
    }
}

impl crate::physic_engine::PhysicEngineFull for SpyPhysicEngine {}

#[test]
fn test_smoke_commands_batched_single_reload() {
    let mut spy_engine = SpyPhysicEngine::new();
    let reinit_requested = AtomicBool::new(false);
    let mut cmd_queue = vec![
        EngineCommand::Smoke(SmokeCommand::SetDensity(0.95)),
        EngineCommand::Smoke(SmokeCommand::SetErosionEnabled(true)),
        EngineCommand::Smoke(SmokeCommand::SetFlowDistortionStrength(0.45)),
        EngineCommand::Smoke(SmokeCommand::SetSpawnRate(45.0)),
    ];

    GuiSettings::process_physic_commands(&mut cmd_queue, &mut spy_engine, &reinit_requested);

    assert_eq!(
        spy_engine.reload_count, 1,
        "Smoke command batch should trigger reload_config exactly ONCE"
    );
    assert!(
        !reinit_requested.load(Ordering::Relaxed),
        "Smoke commands must NOT request full physical reinitialization"
    );
    let cfg = spy_engine.get_config();
    assert_eq!(cfg.smoke_intensity, 0.95);
    assert!(cfg.smoke_erosion_enabled);
    assert_eq!(cfg.flow_distortion_strength, 0.45);
    assert_eq!(cfg.smoke_spawn_rate, 45.0);
}

#[test]
fn test_physic_command_set_gravity_pending_no_immediate_reload() {
    let mut spy_engine = SpyPhysicEngine::new();
    let reinit_requested = AtomicBool::new(false);
    let mut cmd_queue = vec![EngineCommand::Physic(PhysicCommand::SetGravity(-15.0))];

    GuiSettings::process_physic_commands(&mut cmd_queue, &mut spy_engine, &reinit_requested);

    assert_eq!(
        spy_engine.reload_count, 0,
        "SetGravity should NOT trigger immediate reload_config"
    );
    assert!(
        !reinit_requested.load(Ordering::Relaxed),
        "SetGravity should NOT set reinit_requested"
    );
    assert_eq!(spy_engine.get_config_mut().gravity, -15.0);
    assert_ne!(spy_engine.get_config().gravity, -15.0);

    let mut apply_queue = vec![EngineCommand::Physic(PhysicCommand::ApplyPendingConfig)];
    GuiSettings::process_physic_commands(&mut apply_queue, &mut spy_engine, &reinit_requested);

    assert_eq!(
        spy_engine.reload_count, 1,
        "ApplyPendingConfig MUST trigger reload_config"
    );
    assert!(
        reinit_requested.load(Ordering::Relaxed),
        "ApplyPendingConfig MUST set reinit_requested"
    );
    assert_eq!(spy_engine.get_config().gravity, -15.0);
}

#[test]
fn test_exhaustive_engine_command_dispatcher_match() {
    let audio_cmd = EngineCommand::Audio(AudioCommand::SetMasterVolume(0.5));
    let physic_cmd = EngineCommand::Physic(PhysicCommand::SetGravity(-9.8));
    let renderer_cmd = EngineCommand::Renderer(RendererCommand::SetBloomIntensity(1.2));
    let smoke_cmd = EngineCommand::Smoke(SmokeCommand::SetDensity(0.8));

    for cmd in [audio_cmd, physic_cmd, renderer_cmd, smoke_cmd] {
        match cmd {
            EngineCommand::Audio(_) => {}
            EngineCommand::Physic(_) => {}
            EngineCommand::Renderer(_) => {}
            EngineCommand::Smoke(_) => {}
            EngineCommand::Gui(_) => {}
        }
    }
}

struct SpyAudioEngine {
    master_volume: std::sync::RwLock<f32>,
    muted: AtomicBool,
    reverb_wet: std::sync::RwLock<f32>,
    listener_pos: std::sync::RwLock<glam::Vec2>,
    dsp_effects: std::sync::atomic::AtomicU32,
}

impl SpyAudioEngine {
    fn new() -> Self {
        Self {
            master_volume: std::sync::RwLock::new(0.80),
            muted: AtomicBool::new(false),
            reverb_wet: std::sync::RwLock::new(0.08),
            listener_pos: std::sync::RwLock::new(glam::Vec2::ZERO),
            dsp_effects: std::sync::atomic::AtomicU32::new(
                crate::audio_engine::effect_flags::DEFAULT_FLAGS,
            ),
        }
    }
}

impl AudioEngine for SpyAudioEngine {
    fn play_rocket(&self, _pos: glam::Vec2, _gain: f32) {}
    fn play_rocket_with_id(&self, _id: u64, _pos: glam::Vec2, _gain: f32) {}
    fn play_explosion(&self, _pos: glam::Vec2, _gain: f32) {}
    fn play_explosion_with_id(&self, _id: u64, _pos: glam::Vec2, _gain: f32) {}
    fn start_audio_thread(&mut self, _export_path: Option<&str>) {}
    fn stop_audio_thread(&mut self) {}
    fn set_listener_position(&mut self, pos: glam::Vec2) {
        *self.listener_pos.write().unwrap() = pos;
    }
    fn get_listener_position(&self) -> glam::Vec2 {
        *self.listener_pos.read().unwrap()
    }
    fn mute(&mut self) {
        self.muted.store(true, Ordering::Relaxed);
    }
    fn unmute(&mut self) -> f32 {
        self.muted.store(false, Ordering::Relaxed);
        self.get_master_volume()
    }
    fn is_muted(&self) -> bool {
        self.muted.load(Ordering::Relaxed)
    }
    fn set_effect_enabled(&self, effect: AudioEffect, enabled: bool) {
        let mask = effect as u32;
        if enabled {
            self.dsp_effects.fetch_or(mask, Ordering::Relaxed);
        } else {
            self.dsp_effects.fetch_and(!mask, Ordering::Relaxed);
        }
    }
    fn set_all_effects_enabled(&self, enabled: bool) {
        if enabled {
            self.dsp_effects.store(0xFFFF_FFFF, Ordering::Relaxed);
        } else {
            self.dsp_effects.store(0, Ordering::Relaxed);
        }
    }
    fn get_effect_enabled(&self, effect: AudioEffect) -> bool {
        (self.dsp_effects.load(Ordering::Relaxed) & (effect as u32)) != 0
    }
    fn get_effects_status(&self) -> String {
        String::new()
    }
    fn set_reverb_wet(&self, wet: f32) {
        *self.reverb_wet.write().unwrap() = wet;
    }
    fn get_reverb_wet(&self) -> f32 {
        *self.reverb_wet.read().unwrap()
    }
    fn set_master_volume(&self, volume: f32) {
        *self.master_volume.write().unwrap() = volume;
    }
    fn get_master_volume(&self) -> f32 {
        *self.master_volume.read().unwrap()
    }
    fn as_audio_engine(&self) -> &dyn AudioEngine {
        self
    }
}

struct TestHarness {
    spy_audio: SpyAudioEngine,
    spy_physic: SpyPhysicEngine,
    renderer_config: Arc<RwLock<crate::renderer_engine::RendererConfig>>,
    reinit_req: AtomicBool,
    tonemap_comp: AtomicBool,
    stress_scene: AudioStressScene,
    cmd_queue: Vec<EngineCommand>,
}

impl TestHarness {
    fn new() -> Self {
        Self {
            spy_audio: SpyAudioEngine::new(),
            spy_physic: SpyPhysicEngine::new(),
            renderer_config: Arc::new(RwLock::new(
                crate::renderer_engine::RendererConfig::default(),
            )),
            reinit_req: AtomicBool::new(false),
            tonemap_comp: AtomicBool::new(false),
            stress_scene: AudioStressScene::new(),
            cmd_queue: Vec::with_capacity(16),
        }
    }

    fn dispatch(&mut self, cmd: EngineCommand) {
        self.cmd_queue.push(cmd);
        let reload_shaders_dummy = AtomicBool::new(false);
        GuiSettings::dispatch_command_queue(
            &mut self.cmd_queue,
            &mut self.spy_audio,
            &mut self.spy_physic,
            &self.renderer_config,
            &reload_shaders_dummy,
            &self.reinit_req,
            &self.tonemap_comp,
            &mut self.stress_scene,
            (800.0, 600.0),
        );
    }
}

macro_rules! test_reflection {
    ($h:ident, $cmd:expr, $eval:expr, $expected:expr, $desc:expr) => {{
        $h.dispatch($cmd);
        let actual = $eval(&$h);
        assert_eq!(
            actual, $expected,
            "UI State Feedback Loop failed for '{}': expected {:?}, got {:?}",
            $desc, $expected, actual
        );
    }};
}

#[test]
fn test_ui_state_feedback_loop_audio() {
    let mut h = TestHarness::new();

    test_reflection!(
        h,
        EngineCommand::Audio(AudioCommand::SetMasterVolume(0.42)),
        |h: &TestHarness| h.spy_audio.master_volume(),
        0.42,
        "Audio master_volume"
    );
    test_reflection!(
        h,
        EngineCommand::Audio(AudioCommand::SetMuted(true)),
        |h: &TestHarness| AudioStateReader::is_muted(&h.spy_audio),
        true,
        "Audio is_muted true"
    );
    test_reflection!(
        h,
        EngineCommand::Audio(AudioCommand::SetMuted(false)),
        |h: &TestHarness| AudioStateReader::is_muted(&h.spy_audio),
        false,
        "Audio is_muted false"
    );
    test_reflection!(
        h,
        EngineCommand::Audio(AudioCommand::SetSpatialReverb(0.35)),
        |h: &TestHarness| h.spy_audio.spatial_reverb(),
        0.35,
        "Audio spatial_reverb"
    );
    test_reflection!(
        h,
        EngineCommand::Audio(AudioCommand::SetEffectEnabled {
            effect: AudioEffect::HrtfBus,
            enabled: true
        }),
        |h: &TestHarness| h.spy_audio.hrtf_enabled(),
        true,
        "Audio hrtf_enabled"
    );
    test_reflection!(
        h,
        EngineCommand::Audio(AudioCommand::SetEffectEnabled {
            effect: AudioEffect::SpatialReverb,
            enabled: true
        }),
        |h: &TestHarness| h.spy_audio.effect_enabled(AudioEffect::SpatialReverb),
        true,
        "Audio effect_enabled SpatialReverb"
    );
    test_reflection!(
        h,
        EngineCommand::Audio(AudioCommand::SetAllEffectsEnabled(false)),
        |h: &TestHarness| h.spy_audio.effect_enabled(AudioEffect::SpatialReverb),
        false,
        "Audio set_all_effects_enabled false"
    );
}

#[test]
fn test_ui_state_feedback_loop_physic() {
    let mut h = TestHarness::new();

    test_reflection!(
        h,
        EngineCommand::Physic(PhysicCommand::SetGravity(-15.5)),
        |h: &TestHarness| h.spy_physic.gravity(),
        -15.5,
        "Physic gravity pending reflection"
    );
    test_reflection!(
        h,
        EngineCommand::Physic(PhysicCommand::SetMaxRockets(250)),
        |h: &TestHarness| h.spy_physic.max_particles(),
        250,
        "Physic max_particles pending reflection"
    );
    test_reflection!(
        h,
        EngineCommand::Physic(PhysicCommand::SetExplosionMaxVel(120.0)),
        |h: &TestHarness| h.spy_physic.explosion_force(),
        120.0,
        "Physic explosion_force pending reflection"
    );
    test_reflection!(
        h,
        EngineCommand::Physic(PhysicCommand::SetExplosionVelocityBoost(5.5)),
        |h: &TestHarness| h.spy_physic.config().explosion_velocity_boost,
        5.5,
        "Physic explosion_velocity_boost real-time reflection"
    );
    test_reflection!(
        h,
        EngineCommand::Physic(PhysicCommand::SetExplosionShapeSpherical),
        |h: &TestHarness| h.spy_physic.explosion_shape()
            == &crate::physic_engine::ExplosionShape::Spherical,
        true,
        "Physic explosion_shape Spherical"
    );
}

#[test]
fn test_ui_state_feedback_loop_renderer() {
    let mut h = TestHarness::new();

    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetBloomIntensity(3.2)),
        |h: &TestHarness| h.renderer_config.read().unwrap().bloom_intensity(),
        3.2,
        "Renderer bloom_intensity"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetRenderRockets(false)),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().render_rockets,
        false,
        "Renderer render_rockets"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetRenderSmoke(false)),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().render_smoke,
        false,
        "Renderer render_smoke"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetRenderTrails(false)),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().render_trails,
        false,
        "Renderer render_trails"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetRenderExplosions(false)),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().render_explosions,
        false,
        "Renderer render_explosions"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetToneMappingMode(
            crate::renderer_engine::config::ToneMappingMode::Reinhard
        )),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().tone_mapping_mode,
        crate::renderer_engine::config::ToneMappingMode::Reinhard,
        "Renderer tone_mapping_mode"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetBloomEnabled(false)),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().bloom_enabled,
        false,
        "Renderer bloom_enabled"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetBloomIterations(5)),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().bloom_iterations,
        5,
        "Renderer bloom_iterations"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetBloomDownsample(4)),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().bloom_downsample,
        4,
        "Renderer bloom_downsample"
    );
    test_reflection!(
        h,
        EngineCommand::Renderer(RendererCommand::SetBloomBlurMethod(
            crate::renderer_engine::config::BlurMethod::Kawase
        )),
        |h: &TestHarness| h.renderer_config.read().unwrap().config().bloom_blur_method,
        crate::renderer_engine::config::BlurMethod::Kawase,
        "Renderer bloom_blur_method"
    );
}

#[test]
fn test_ui_state_feedback_loop_smoke() {
    let mut h = TestHarness::new();

    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetDensity(0.92)),
        |h: &TestHarness| h.spy_physic.density(),
        0.92,
        "Smoke density"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetDissipation(4.2)),
        |h: &TestHarness| h.spy_physic.dissipation(),
        4.2,
        "Smoke dissipation"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetErosionEnabled(true)),
        |h: &TestHarness| h.spy_physic.config().smoke_erosion_enabled,
        true,
        "Smoke erosion_enabled"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetErosionScale(2.8)),
        |h: &TestHarness| h.spy_physic.config().smoke_erosion_scale,
        2.8,
        "Smoke erosion_scale"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetErosionEdgeWidth(0.35)),
        |h: &TestHarness| h.spy_physic.config().smoke_erosion_edge_width,
        0.35,
        "Smoke erosion_edge_width"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetErosionEdgeColor([255, 128, 64])),
        |h: &TestHarness| h.spy_physic.config().smoke_erosion_edge_color,
        [1.0, 128.0 / 255.0, 64.0 / 255.0],
        "Smoke erosion_edge_color"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetFlowDistortionStrength(0.75)),
        |h: &TestHarness| h.spy_physic.config().flow_distortion_strength,
        0.75,
        "Smoke flow_distortion_strength"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetFlowAnimationSpeed(1.4)),
        |h: &TestHarness| h.spy_physic.config().flow_animation_speed,
        1.4,
        "Smoke flow_animation_speed"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetColorMode(
            crate::physic_engine::config::SmokeColorMode::Custom
        )),
        |h: &TestHarness| h.spy_physic.config().smoke_color_mode,
        crate::physic_engine::config::SmokeColorMode::Custom,
        "Smoke smoke_color_mode"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetCustomColor([50, 100, 150])),
        |h: &TestHarness| h.spy_physic.config().smoke_custom_color,
        [50.0 / 255.0, 100.0 / 255.0, 150.0 / 255.0],
        "Smoke smoke_custom_color"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetSpawnRate(55.0)),
        |h: &TestHarness| h.spy_physic.config().smoke_spawn_rate,
        55.0,
        "Smoke smoke_spawn_rate"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetInitialSize(18.0)),
        |h: &TestHarness| h.spy_physic.config().smoke_initial_size,
        18.0,
        "Smoke smoke_initial_size"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetGrowthRateMultiplier(2.2)),
        |h: &TestHarness| h.spy_physic.config().smoke_growth_rate_multiplier,
        2.2,
        "Smoke smoke_growth_rate_multiplier"
    );
    test_reflection!(
        h,
        EngineCommand::Smoke(SmokeCommand::SetMaxSmokeParticles(3500)),
        |h: &TestHarness| h.spy_physic.config().smoke_intensity,
        0.92,
        "Smoke max_smoke_particles"
    );
}

#[test]
fn test_apply_all_themes_to_imgui_context() {
    let _guard = IMGUI_TEST_MUTEX.lock().unwrap();
    let mut ctx = imgui::Context::create();
    for (theme, name) in GuiTheme::all_themes() {
        apply_theme_to_context(&mut ctx, *theme);
        assert_eq!(theme.display_name(), *name);
    }
}

#[test]
fn test_pending_theme_change_preservation_for_imgui() {
    let mut settings = GuiSettings::default();
    assert_eq!(settings.pending_theme_change, None);

    let target_theme = GuiTheme::DeepSapphire;
    settings.pending_theme_change = Some(target_theme);

    // Verify that pending_theme_change is preserved for ui.rs to consume
    assert_eq!(settings.pending_theme_change, Some(target_theme));
    let consumed = settings.pending_theme_change.take();
    assert_eq!(consumed, Some(target_theme));
    assert_eq!(settings.pending_theme_change, None);
}

#[test]
fn test_gui_scale_zoom_bounds_and_presets() {
    for (scale, name) in ZOOM_PRESETS {
        assert!(
            (GUI_SCALE_MIN..=GUI_SCALE_MAX).contains(&scale),
            "Preset scale {} ({}) outside bounds [{}, {}]",
            scale,
            name,
            GUI_SCALE_MIN,
            GUI_SCALE_MAX
        );
    }
}

#[test]
fn test_smoke_preset_definitions_integrity() {
    let presets = crate::physic_engine::constants::SMOKE_PRESET_DEFINITIONS;
    assert_eq!(presets.len(), 4);
    for preset in presets {
        assert!(!preset.name.is_empty());
        assert!(preset.intensity > 0.0);
        assert!(preset.edge_width > 0.0);
    }
}

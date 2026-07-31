#![allow(clippy::field_reassign_with_default)]

use fireworks_sim::physic_engine::{
    config::{PhysicConfig, SmokeColorMode},
    physic_engine_generational_arena::{PhysicEngineFireworks, PhysicEngineTestHelpers},
    smoke_system::{
        ParticleLifecycle, ParticleOpacity, ParticleSizing, SmokeParticle, SmokeSystem,
    },
    ParticleType, PhysicEngine, PhysicEngineIterator,
};
use glam::{Vec2, Vec3 as Color};

#[test]
fn test_smoke_system_emission_and_lifecycle() {
    let mut config = PhysicConfig::default();
    config.smoke_spawn_rate = 60.0;
    config.smoke_initial_size = 10.0;
    config.smoke_growth_rate_multiplier = 1.2;
    config.smoke_fade_duration = 0.5;
    config.smoke_intensity = 0.5;

    let mut system = SmokeSystem::new(100);
    let mut rng = rand::rng();

    let rocket_pos = Vec2::new(100.0, 200.0);
    let rocket_vel = Vec2::new(0.0, 300.0);
    let rocket_color = Color::new(1.0, 0.2, 0.3);

    // Initial state: all inactive
    let mut count = 0;
    system.for_each_active(&mut |_| count += 1);
    assert_eq!(count, 0);

    // Emit single smoke particle
    system.emit(rocket_pos, rocket_vel, rocket_color, &config, &mut rng);

    count = 0;
    let mut active_particle: Option<SmokeParticle> = None;
    system.for_each_active(&mut |p| {
        count += 1;
        active_particle = Some(*p);
    });
    assert_eq!(count, 1);

    let p = active_particle.unwrap();
    assert!(p.active);
    assert!(p.opacity.initial_alpha > 0.0);
    assert!((p.sizing.initial_size - config.smoke_initial_size).abs() <= 2.5);

    // Advance lifecycle by half duration
    system.update(0.25, &config);
    count = 0;
    let mut mid_particle: Option<SmokeParticle> = None;
    system.for_each_active(&mut |p| {
        count += 1;
        mid_particle = Some(*p);
    });
    assert_eq!(count, 1);

    let mid_p = mid_particle.unwrap();
    assert!(mid_p.sizing.current_size > mid_p.sizing.initial_size);
    assert!(mid_p.opacity.alpha < p.opacity.initial_alpha);

    // Advance past total lifetime -> particle deactivates
    system.update(0.8, &config);
    count = 0;
    system.for_each_active(&mut |_| count += 1);
    assert_eq!(count, 0);
}

#[test]
fn test_smoke_particle_conversion_to_particle() {
    let p = SmokeParticle {
        pos: Vec2::new(50.0, 150.0),
        vel: Vec2::new(1.0, 2.0),
        color: Color::new(0.9, 0.4, 0.1),
        rocket_color: Color::new(0.9, 0.4, 0.1),
        sizing: ParticleSizing {
            initial_size: 8.0,
            current_size: 12.0,
            growth_rate: 1.0,
        },
        opacity: ParticleOpacity {
            initial_alpha: 0.5,
            alpha: 0.25,
        },
        lifecycle: ParticleLifecycle {
            age: 0.25,
            max_life: 0.5,
        },
        rotation: 0.78,
        active: true,
    };

    let converted = p.to_particle();
    assert_eq!(converted.particle_type, ParticleType::Smoke);
    assert_eq!(converted.pos, Vec2::new(50.0, 150.0));
    assert_eq!(converted.color, Color::new(0.9, 0.4, 0.1));
    assert_eq!(converted.size, 12.0);
    assert_eq!(converted.angle, 0.78);
    assert!(converted.active);
}

#[test]
fn test_smoke_color_mode_selection() {
    let mut system = SmokeSystem::new(10);
    let mut rng = rand::rng();

    // Mode 1: RocketColor (inherited)
    let mut config = PhysicConfig::default();
    config.smoke_color_mode = SmokeColorMode::RocketColor;
    let rocket_color = Color::new(0.2, 0.9, 0.4);

    system.emit(
        Vec2::new(10.0, 10.0),
        Vec2::new(0.0, 100.0),
        rocket_color,
        &config,
        &mut rng,
    );

    let mut emitted_color = Color::ZERO;
    system.for_each_active(&mut |p| emitted_color = p.color);
    assert_eq!(emitted_color, rocket_color);

    system.clear();

    // Mode 2: Custom color
    config.smoke_color_mode = SmokeColorMode::Custom;
    config.smoke_custom_color = [0.1, 0.5, 0.9];

    system.emit(
        Vec2::new(10.0, 10.0),
        Vec2::new(0.0, 100.0),
        rocket_color,
        &config,
        &mut rng,
    );

    system.for_each_active(&mut |p| emitted_color = p.color);
    assert_eq!(emitted_color, Color::new(0.1, 0.5, 0.9));
}

#[test]
fn test_physic_engine_smoke_emission_at_rocket_base() {
    let mut config = PhysicConfig::default();
    config.smoke_spawn_rate = 60.0;
    config.smoke_intensity = 0.8;

    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);
    assert_eq!(engine.get_smoke_intensity(), 0.8);

    engine.force_next_launch();
    for _ in 0..10 {
        engine.update(0.016);
    }

    let mut smoke_count = 0;
    let mut positions = Vec::new();
    engine.for_each_particle_of_type(ParticleType::Smoke, &mut |p| {
        smoke_count += 1;
        positions.push(p.pos);
    });

    assert!(smoke_count > 0, "Active ascending rocket should emit smoke");
    for pos in positions {
        assert!(
            pos.x != 0.0 || pos.y != 0.0,
            "Smoke particle positions should be valid"
        );
    }
}

#[test]
fn test_physic_engine_smoke_config_reload() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 1920.0);

    let mut new_config = config.clone();
    new_config.max_smoke_particles = 1024;
    new_config.smoke_intensity = 1.2;
    new_config.smoke_color_mode = SmokeColorMode::Custom;

    let _reinit = engine.reload_config(&new_config);
    assert_eq!(engine.get_config().smoke_intensity, 1.2);
    assert_eq!(engine.get_smoke_intensity(), 1.2);
    assert_eq!(engine.get_config().smoke_color_mode, SmokeColorMode::Custom);
}

#[test]
fn test_smoke_dynamic_settings_update_in_realtime() {
    let mut system = SmokeSystem::new(10);
    let mut rng = rand::rng();
    let mut config = PhysicConfig::default();
    config.smoke_color_mode = SmokeColorMode::Custom;
    config.smoke_custom_color = [1.0, 0.0, 0.0];
    config.smoke_growth_rate_multiplier = 1.0;

    system.emit(
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 50.0),
        Color::ONE,
        &config,
        &mut rng,
    );

    // Initial update
    system.update(0.1, &config);

    let mut current_color = Color::ZERO;
    system.for_each_active(&mut |p| current_color = p.color);
    assert_eq!(current_color, Color::new(1.0, 0.0, 0.0));

    // Dynamic real-time update of custom color and capacity without re-initialization
    config.smoke_custom_color = [0.0, 1.0, 0.5];
    config.max_smoke_particles = 50;
    system.update(0.1, &config);

    assert_eq!(system.particles.len(), 50);
    system.for_each_active(&mut |p| current_color = p.color);
    assert_eq!(current_color, Color::new(0.0, 1.0, 0.5));
}

#[test]
fn test_physic_engine_smoke_erosion_params_and_toggle() {
    let mut config = PhysicConfig::default();
    config.smoke_erosion_enabled = false;
    config.smoke_erosion_scale = 1.5;
    config.smoke_erosion_edge_width = 0.25;
    config.smoke_erosion_edge_color = [0.1, 0.8, 0.9];

    let engine = PhysicEngineFireworks::new(&config, 1920.0);
    let (enabled, scale, edge_width, edge_color) = engine.get_smoke_erosion_params();

    assert!(!enabled);
    assert_eq!(scale, 1.5);
    assert_eq!(edge_width, 0.25);
    assert_eq!(edge_color, [0.1, 0.8, 0.9]);
}

#[test]
fn test_physic_engine_smoke_flow_params() {
    let mut config = PhysicConfig::default();
    config.flow_distortion_strength = 0.28;
    config.flow_animation_speed = 1.75;

    let engine = PhysicEngineFireworks::new(&config, 1920.0);
    let (strength, speed) = engine.get_smoke_flow_params();

    assert_eq!(strength, 0.28);
    assert_eq!(speed, 1.75);
}

#[test]
fn test_smoke_inherited_color_intensity() {
    let mut system = SmokeSystem::new(10);
    let mut rng = rand::rng();
    let mut config = PhysicConfig::default();
    config.smoke_color_mode = SmokeColorMode::RocketColor;
    config.smoke_inherited_color_intensity = 0.5;

    let base_rocket_col = Color::new(0.8, 0.6, 0.4);

    system.emit(
        Vec2::new(0.0, 0.0),
        Vec2::new(0.0, 50.0),
        base_rocket_col,
        &config,
        &mut rng,
    );

    let mut current_color = Color::ZERO;
    system.for_each_active(&mut |p| current_color = p.color);
    assert_eq!(current_color, base_rocket_col * 0.5);

    // Update in real-time
    config.smoke_inherited_color_intensity = 1.5;
    system.update(0.1, &config);

    system.for_each_active(&mut |p| current_color = p.color);
    assert_eq!(current_color, base_rocket_col * 1.5);
}

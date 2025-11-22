use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::particles_pools::ParticlesPoolsForRockets;
use fireworks_sim::physic_engine::rocket::Rocket;
use rand::SeedableRng;

// ==================================
// 1. Tests de random_color
// ==================================

#[test]
fn test_random_color_in_valid_range() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);

    // Générer plusieurs couleurs
    for _ in 0..100 {
        let color = rocket.color;

        // Vérifier que les composantes RGB sont dans [0.5, 1.0]
        assert!(
            color.x >= 0.5 && color.x <= 1.0,
            "Red component out of range: {}",
            color.x
        );
        assert!(
            color.y >= 0.5 && color.y <= 1.0,
            "Green component out of range: {}",
            color.y
        );
        assert!(
            color.z >= 0.5 && color.z <= 1.0,
            "Blue component out of range: {}",
            color.z
        );
        assert_eq!(color.w, 1.0, "Alpha should always be 1.0");

        // Réinitialiser pour générer une nouvelle couleur
        rocket.reset(&PhysicConfig::default(), 1920.0);
    }
}

// ==================================
// 2. Tests de random_vel
// ==================================

#[test]
fn test_random_vel_respects_config() {
    let mut config = PhysicConfig::default();
    config.spawn_rocket_vertical_angle = std::f32::consts::FRAC_PI_2; // π/2 (vertical)
    config.spawn_rocket_angle_variation = 0.3; // ±0.3 rad
    config.spawn_rocket_min_speed = 350.0;
    config.spawn_rocket_max_speed = 500.0;

    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);

    for _ in 0..100 {
        rocket.reset(&config, 1920.0);
        let vel = rocket.vel;
        let speed = vel.length();

        // Vérifier que la vitesse est dans la plage
        assert!(
            speed >= config.spawn_rocket_min_speed && speed <= config.spawn_rocket_max_speed,
            "Speed out of range: {}",
            speed
        );

        // Vérifier que l'angle est approximativement vertical (±variation)
        // Note: cette vérification est approximative car l'angle dépend de la direction
        assert!(
            vel.y > 0.0,
            "Rocket should be going upward (positive y velocity)"
        );
    }
}

#[test]
fn test_random_vel_different_configs() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    // Config 1: Vitesse lente
    let mut config_slow = PhysicConfig::default();
    config_slow.spawn_rocket_min_speed = 100.0;
    config_slow.spawn_rocket_max_speed = 200.0;

    let mut rocket = Rocket::new(&mut rng);
    rocket.reset(&config_slow, 1920.0);
    let speed_slow = rocket.vel.length();
    assert!(speed_slow >= 100.0 && speed_slow <= 200.0);

    // Config 2: Vitesse rapide
    let mut config_fast = PhysicConfig::default();
    config_fast.spawn_rocket_min_speed = 600.0;
    config_fast.spawn_rocket_max_speed = 800.0;

    rocket.reset(&config_fast, 1920.0);
    let speed_fast = rocket.vel.length();
    assert!(speed_fast >= 600.0 && speed_fast <= 800.0);
}

// ==================================
// 3. Tests de remove_inactive_rockets
// ==================================

#[test]
fn test_remove_inactive_rockets_when_all_particles_inactive() {
    let config = PhysicConfig::default();
    let mut pools = ParticlesPoolsForRockets::new(
        config.max_rockets,
        config.particles_per_explosion,
        config.particles_per_trail,
    );

    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);
    rocket.reset(&config, 1920.0);

    // Simuler jusqu'à l'explosion (augmenter le nombre de frames)
    for _ in 0..500 {
        rocket.update(0.016, &mut pools, &config);
        if rocket.exploded {
            break;
        }
    }

    assert!(rocket.exploded, "Rocket should have exploded");
    assert!(
        rocket.active,
        "Rocket should still be active after explosion"
    );

    // Simuler jusqu'à ce que toutes les particules soient inactives
    for _ in 0..500 {
        rocket.update(0.016, &mut pools, &config);
        if !rocket.active {
            break;
        }
    }

    // La fusée devrait être désactivée
    assert!(
        !rocket.active,
        "Rocket should be inactive when all particles are inactive"
    );
}

#[test]
fn test_remove_inactive_rockets_stays_active_with_active_particles() {
    let config = PhysicConfig::default();
    let mut pools = ParticlesPoolsForRockets::new(
        config.max_rockets,
        config.particles_per_explosion,
        config.particles_per_trail,
    );

    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);
    rocket.reset(&config, 1920.0);

    // Simuler jusqu'à l'explosion (augmenter le nombre de frames)
    for _ in 0..500 {
        rocket.update(0.016, &mut pools, &config);
        if rocket.exploded {
            break;
        }
    }

    assert!(rocket.exploded, "Rocket should have exploded");

    // Juste après l'explosion, il devrait y avoir des particules actives
    rocket.update(0.016, &mut pools, &config);
    assert!(
        rocket.active,
        "Rocket should stay active with active particles"
    );
}

// ==================================
// 4. Tests de update_head_particle
// ==================================

#[test]
fn test_update_head_particle_position_matches_rocket() {
    let config = PhysicConfig::default();
    let mut pools = ParticlesPoolsForRockets::new(
        config.max_rockets,
        config.particles_per_explosion,
        config.particles_per_trail,
    );

    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);
    rocket.reset(&config, 1920.0);

    // Simuler quelques frames
    for _ in 0..10 {
        rocket.update(0.016, &mut pools, &config);

        let head = rocket.head_particle();

        // La position de la tête devrait correspondre à la position de la fusée
        assert_eq!(head.pos, rocket.pos);
        assert_eq!(head.vel, rocket.vel);
        assert_eq!(head.color, rocket.color);
        assert!(head.active);
    }
}

#[test]
fn test_update_head_particle_angle_calculation() {
    let config = PhysicConfig::default();
    let mut pools = ParticlesPoolsForRockets::new(
        config.max_rockets,
        config.particles_per_explosion,
        config.particles_per_trail,
    );

    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);
    rocket.reset(&config, 1920.0);

    // Avant le premier update, la fusée monte
    rocket.update(0.016, &mut pools, &config);
    let head = rocket.head_particle();

    // L'angle devrait être défini (non NaN)
    assert!(head.angle.is_finite(), "Angle should be finite");
}

#[test]
fn test_update_head_particle_with_zero_velocity() {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let rocket = Rocket::new(&mut rng);

    // Fusée avec vélocité nulle (cas par défaut)
    let head = rocket.head_particle();

    // L'angle devrait être 0.0 quand la vélocité est nulle
    assert_eq!(head.angle, 0.0, "Angle should be 0.0 with zero velocity");
}

// ==================================
// 5. Tests de reset
// ==================================

#[test]
fn test_reset_reinitializes_rocket_state() {
    let config = PhysicConfig::default();
    let mut pools = ParticlesPoolsForRockets::new(
        config.max_rockets,
        config.particles_per_explosion,
        config.particles_per_trail,
    );

    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);
    rocket.reset(&config, 1920.0);

    // Simuler jusqu'à l'explosion (augmenter le nombre de frames)
    for _ in 0..500 {
        rocket.update(0.016, &mut pools, &config);
        if rocket.exploded {
            break;
        }
    }

    assert!(rocket.exploded);

    // Reset
    rocket.reset(&config, 1920.0);

    // Vérifier que l'état est réinitialisé
    assert!(rocket.active, "Rocket should be active after reset");
    assert!(
        !rocket.exploded,
        "Rocket should not be exploded after reset"
    );
    assert_eq!(rocket.pos.y, 0.0, "Rocket should start at y=0");
    assert!(rocket.vel.y > 0.0, "Rocket should be moving upward");
    assert!(rocket.explosion_particle_indices.is_none());
    assert!(rocket.trail_particle_indices.is_none());
}

#[test]
fn test_reset_respects_window_width() {
    let config = PhysicConfig::default();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);

    for window_width in [800.0, 1920.0, 3840.0] {
        let mut rocket = Rocket::new(&mut rng);
        rocket.reset(&config, window_width);

        // La position x devrait être dans les marges
        assert!(
            rocket.pos.x >= config.spawn_rocket_margin,
            "Rocket x position should be >= margin"
        );
        assert!(
            rocket.pos.x <= window_width - config.spawn_rocket_margin,
            "Rocket x position should be <= window_width - margin"
        );
    }
}

// ==================================
// 6. Tests d'intégration
// ==================================

#[test]
fn test_rocket_full_lifecycle() {
    let config = PhysicConfig::default();
    let mut pools = ParticlesPoolsForRockets::new(
        config.max_rockets,
        config.particles_per_explosion,
        config.particles_per_trail,
    );

    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    let mut rocket = Rocket::new(&mut rng);
    rocket.reset(&config, 1920.0);

    let mut exploded_frame = None;
    let mut deactivated_frame = None;

    // Simuler jusqu'à désactivation complète
    for frame in 0..1000 {
        rocket.update(0.016, &mut pools, &config);

        if rocket.exploded && exploded_frame.is_none() {
            exploded_frame = Some(frame);
        }

        if !rocket.active {
            deactivated_frame = Some(frame);
            break;
        }
    }

    assert!(exploded_frame.is_some(), "Rocket should explode");
    assert!(deactivated_frame.is_some(), "Rocket should deactivate");
    assert!(
        deactivated_frame.unwrap() > exploded_frame.unwrap(),
        "Deactivation should happen after explosion"
    );
}

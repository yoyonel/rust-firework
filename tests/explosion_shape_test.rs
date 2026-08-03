use fireworks_sim::physic_engine::explosion_shape::ImageShape;
use glam::Vec2;
use image::{ImageBuffer, Luma};

/// Teste la création d'une ImageShape à partir d'une image générée en mémoire
#[test]
fn test_image_shape_loading() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path_buf = temp_dir.path().join("test_shape_loading.png");
    let path = path_buf.to_str().unwrap();

    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(10, 10);
    img.put_pixel(5, 5, Luma([255]));
    img.save(path).unwrap();

    let shape = ImageShape::from_image(path, 100, 50.0, 1.0).expect("Failed to load image shape");

    assert_eq!(shape.scale(), 50.0);
    assert_eq!(shape.flight_time(), 1.0);
    assert!(shape.sample_count() > 0);
}

/// Teste la logique de rotation des cibles
#[test]
fn test_target_position_rotation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path_buf = temp_dir.path().join("test_rotation.png");
    let path = path_buf.to_str().unwrap();

    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(100, 100);
    img.put_pixel(50, 50, Luma([255]));
    img.put_pixel(80, 50, Luma([255]));
    img.save(path).unwrap();

    let shape = ImageShape::from_image(path, 10, 100.0, 1.0).unwrap();

    let center = Vec2::ZERO;

    let pos_no_rot = shape.get_target_position_rotated(0, center, 1.0, 0.0);
    let pos_rot_90 = shape.get_target_position_rotated(0, center, 0.0, 1.0);

    assert!(
        (pos_no_rot.length() - pos_rot_90.length()).abs() < 0.001,
        "Distance should be preserved"
    );

    assert!((pos_rot_90.x + pos_no_rot.y).abs() < 0.001);
    assert!((pos_rot_90.y - pos_no_rot.x).abs() < 0.001);
}

/// Teste le calcul de la vitesse initiale (balistique)
#[test]
fn test_ballistic_calculation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let path_buf = temp_dir.path().join("test_ballistic.png");
    let path = path_buf.to_str().unwrap();

    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(10, 10);
    img.put_pixel(5, 5, Luma([255]));
    img.save(path).unwrap();

    let flight_time = 2.0;
    let shape = ImageShape::from_image(path, 1, 100.0, flight_time).unwrap();

    let start = Vec2::ZERO;
    let target = Vec2::new(100.0, 0.0); // Cible à 100m sur X
    let gravity = Vec2::ZERO; // Pas de gravité pour simplifier

    let v0 = shape.compute_initial_velocity(start, target, gravity);

    // v = d / t = 100 / 2 = 50
    assert!((v0.x - 50.0).abs() < 0.001);
    assert!((v0.y - 0.0).abs() < 0.001);

    // Avec gravité (0, -10)
    let gravity = Vec2::new(0.0, -10.0);
    let v0_g = shape.compute_initial_velocity(start, target, gravity);

    // v0 = (d - 0.5*g*t^2) / t
    // v0_y = (0 - 0.5*(-10)*4) / 2 = (20) / 2 = 10
    // On doit tirer vers le haut pour compenser la chute
    assert!((v0_g.x - 50.0).abs() < 0.001);
    assert!((v0_g.y - 10.0).abs() < 0.001);
}

/// Teste l'ajout cumulatif et la suppression de formes d'explosion (MultiImage)
#[test]
fn test_multi_image_shape_addition_and_deletion() {
    use fireworks_sim::physic_engine::explosion_shape::ExplosionShape;
    use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
    use fireworks_sim::physic_engine::PhysicEngine;

    let config = fireworks_sim::physic_engine::config::PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 800.0);

    // Initialement Spherical
    match engine.get_explosion_shape() {
        ExplosionShape::Spherical => {}
        _ => panic!("Expected Spherical initial shape"),
    }

    // Chargement de la première forme avec un poids de 1.0 (Heart)
    let _ = engine.load_explosion_image_weighted(
        "assets/textures/explosion_shapes/heart.png",
        150.0,
        1.5,
        1.0,
    );

    match engine.get_explosion_shape() {
        ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } => {
            assert_eq!(shapes.len(), 1);
            assert_eq!(shapes[0].0.file_stem, "heart");
            assert_eq!(*total_weight, 1.0);
        }
        _ => panic!("Expected MultiImage after loading weighted heart shape"),
    }

    // Ajout d'une 2ème forme avec un poids de 2.0 (Star)
    let _ = engine.load_explosion_image_weighted(
        "assets/textures/explosion_shapes/star.png",
        180.0,
        1.5,
        2.0,
    );

    match engine.get_explosion_shape() {
        ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } => {
            assert_eq!(shapes.len(), 2);
            assert_eq!(*total_weight, 3.0);
        }
        _ => panic!("Expected 2 MultiImage shapes"),
    }

    // Modification du poids de "heart" à 3.0
    let _ = engine.set_explosion_image_weight("heart", 3.0);
    match engine.get_explosion_shape() {
        ExplosionShape::MultiImage { total_weight, .. } => {
            assert_eq!(*total_weight, 5.0);
        }
        _ => panic!("Expected MultiImage total weight 5.0"),
    }

    // Suppression de la forme "star"
    let _ = engine.remove_explosion_image("star");
    match engine.get_explosion_shape() {
        ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } => {
            assert_eq!(shapes.len(), 1);
            assert_eq!(shapes[0].0.file_stem, "heart");
            assert_eq!(*total_weight, 3.0);
        }
        _ => panic!("Expected 1 MultiImage shape after deleting star"),
    }

    // Suppression de la dernière forme "heart" -> doit repasser automatiquement en Spherical
    let _ = engine.remove_explosion_image("heart");
    match engine.get_explosion_shape() {
        ExplosionShape::Spherical => {}
        _ => panic!("Expected Spherical shape after removing all shapes"),
    }
}

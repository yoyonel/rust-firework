use fireworks_sim::physic_engine::explosion_shape::ImageShape;
use glam::Vec2;
use image::{ImageBuffer, Luma};

/// Teste la création d'une ImageShape à partir d'une image générée en mémoire
#[test]
fn test_image_shape_loading() {
    // Crée une image 10x10 avec un seul pixel blanc au centre (5, 5)
    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(10, 10);
    img.put_pixel(5, 5, Luma([255]));

    // Sauvegarde temporaire pour le test (ImageShape charge depuis un fichier)
    let path = "test_shape_loading.png";
    img.save(path).unwrap();

    let shape = ImageShape::from_image(path, 100, 50.0, 1.0).expect("Failed to load image shape");

    // Nettoyage
    let _ = std::fs::remove_file(path);

    assert_eq!(shape.scale(), 50.0);
    assert_eq!(shape.flight_time(), 1.0);
    assert!(shape.sample_count() > 0);
}

/// Teste la logique de rotation des cibles
#[test]
fn test_target_position_rotation() {
    // Simule une shape manuellement (champs privés, mais on teste via les méthodes publiques si possible)
    // Comme on ne peut pas instancier ImageShape directement (champs privés ou crate externe),
    // on va utiliser from_image sur une image simple : 3 pixels alignés horizontalement

    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(100, 100);
    // Centre
    img.put_pixel(50, 50, Luma([255]));
    // Droite
    img.put_pixel(80, 50, Luma([255]));

    let path = "test_rotation.png";
    img.save(path).unwrap();

    // Scale 100.0
    let shape = ImageShape::from_image(path, 10, 100.0, 1.0).unwrap();
    let _ = std::fs::remove_file(path);

    let center = Vec2::ZERO;

    // Pas de rotation (angle 0 => cos=1, sin=0)
    let pos_no_rot = shape.get_target_position_rotated(0, center, 1.0, 0.0);

    // Rotation 90 degrés (PI/2 => cos=0, sin=1)
    let pos_rot_90 = shape.get_target_position_rotated(0, center, 0.0, 1.0);

    // Vérifie que la distance au centre est conservée
    assert!(
        (pos_no_rot.length() - pos_rot_90.length()).abs() < 0.001,
        "Distance should be preserved"
    );

    // Vérifie la rotation (x devient -y, y devient x)
    // Note: get_target_position_rotated applique explicement:
    // x' = x*cos - y*sin
    // y' = x*sin + y*cos
    // Si pos_no_rot est (x, y), pos_rot_90 doit être (-y, x)
    assert!((pos_rot_90.x + pos_no_rot.y).abs() < 0.001);
    assert!((pos_rot_90.y - pos_no_rot.x).abs() < 0.001);
}

/// Teste le calcul de la vitesse initiale (balistique)
#[test]
fn test_ballistic_calculation() {
    // Création d'une image dummy
    let mut img = ImageBuffer::<Luma<u8>, Vec<u8>>::new(10, 10);
    img.put_pixel(5, 5, Luma([255]));
    let path = "test_ballistic.png";
    img.save(path).unwrap();

    let flight_time = 2.0;
    let shape = ImageShape::from_image(path, 1, 100.0, flight_time).unwrap();
    let _ = std::fs::remove_file(path);

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

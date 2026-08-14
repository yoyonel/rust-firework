#[cfg(test)]
mod tests {
    use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
    use fireworks_sim::physic_engine::{ExplosionShape, PhysicConfig, PhysicEngine};

    #[test]
    fn test_load_and_weight_management() {
        // Assuming default window width 800.0 or similar; exact value doesn't matter for this test
        let mut engine = PhysicEngineFireworks::new(&PhysicConfig::default(), 800.0, None);
        let dummy_path_1 = "assets/textures/explosion_shapes/heart.png";
        let dummy_path_2 = "assets/textures/explosion_shapes/star.png";

        // 1. Initial State: Spherical
        assert!(matches!(
            engine.get_explosion_shape(),
            ExplosionShape::Spherical
        ));

        // 2. Load first image (weighted)
        // Note: load_explosion_image_weighted actually loads the file.
        // We need existing files or we should mock the loader.
        // The real loader reads files. We assume assets exist in the repo.
        // If not, we might fail or need to mock ImageShape::from_image.
        // Assuming asset files exist based on file listing earlier.

        let res = engine.load_explosion_image_weighted(dummy_path_1, 150.0, 1.5, 1.0);
        assert!(res.is_ok(), "Failed to load heart.png: {:?}", res.err());

        // Check if switched to MultiImage
        if let ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } = engine.get_explosion_shape()
        {
            assert_eq!(shapes.len(), 1);
            assert_eq!(shapes[0].0.file_stem, "heart");
            assert!((total_weight - 1.0).abs() < 0.001);
            assert!((shapes[0].1 - 1.0).abs() < 0.001);
        } else {
            panic!("Expected MultiImage after first weighted load");
        }

        // 3. Load second image
        let res = engine.load_explosion_image_weighted(dummy_path_2, 150.0, 1.5, 3.0);
        assert!(res.is_ok(), "Failed to load star.png: {:?}", res.err());

        if let ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } = engine.get_explosion_shape()
        {
            assert_eq!(shapes.len(), 2);
            assert_eq!(shapes[1].0.file_stem, "star");
            // Total weight should be 1.0 + 3.0 = 4.0
            assert!((total_weight - 4.0).abs() < 0.001);
        } else {
            panic!("Expected MultiImage after second weighted load");
        }

        // 4. Update weight of first image
        let res = engine.set_explosion_image_weight("heart", 5.0);
        assert!(res.is_ok());

        if let ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } = engine.get_explosion_shape()
        {
            // Heart is shapes[0]
            assert_eq!(shapes[0].0.file_stem, "heart");
            assert!((shapes[0].1 - 5.0).abs() < 0.001);

            // Star is shapes[1] (weight 3.0)
            assert!((shapes[1].1 - 3.0).abs() < 0.001);

            // Total: 5.0 + 3.0 = 8.0
            assert!((total_weight - 8.0).abs() < 0.001);
        } else {
            panic!("Expected MultiImage");
        }

        // 5. Update non-existent image
        let res = engine.set_explosion_image_weight("unknown", 1.0);
        assert!(res.is_err());

        // 6. Reload existing image (Upsert behavior check)
        // Loading heart again with weight 2.0. Should not add duplicate, just update.
        let res = engine.load_explosion_image_weighted(dummy_path_1, 150.0, 1.5, 2.0);
        assert!(res.is_ok());

        if let ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } = engine.get_explosion_shape()
        {
            assert_eq!(shapes.len(), 2, "Should still have 2 shapes, not 3");
            let heart_entry = shapes.iter().find(|(s, _)| s.file_stem == "heart").unwrap();
            assert!(
                (heart_entry.1 - 2.0).abs() < 0.001,
                "Heart weight should be updated to 2.0"
            );

            // Total: 2.0 (heart) + 3.0 (star) = 5.0
            assert!((total_weight - 5.0).abs() < 0.001);
        } else {
            panic!("Expected MultiImage");
        }

        // 7. Remove image by setting weight <= 0
        // Remove heart
        let res = engine.load_explosion_image_weighted(dummy_path_1, 150.0, 1.5, 0.0);
        assert!(res.is_ok());

        if let ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } = engine.get_explosion_shape()
        {
            assert_eq!(shapes.len(), 1, "Should have 1 shape left");
            assert_eq!(shapes[0].0.file_stem, "star");
            assert!((total_weight - 3.0).abs() < 0.001);
        } else {
            panic!("Expected MultiImage after removing one shape");
        }

        // 8. Remove last image -> Revert to Spherical
        let res = engine.load_explosion_image_weighted(dummy_path_2, 150.0, 1.5, -1.0); // Test negative too
        assert!(res.is_ok());

        assert!(
            matches!(engine.get_explosion_shape(), ExplosionShape::Spherical),
            "Should revert to Spherical when last shape is removed"
        );
    }
}

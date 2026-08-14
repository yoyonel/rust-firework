use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::physic_engine::{ExplosionShape, PhysicEngine};
use fireworks_sim::simulator::gui_settings::physic::{
    apply_session_to_physic, preset_weights_from_shape, PRESET_DEFINITIONS,
};
use fireworks_sim::simulator::gui_settings::PersistedExplosionShape;

#[test]
fn test_shape_cumulativity_add_inactive_shape_with_zero_weight_slider() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);

    // 1. Initial State: Set single Heart shape
    let heart = &PRESET_DEFINITIONS[0];
    engine
        .load_explosion_image(heart.path, heart.default_scale, heart.default_flight_time)
        .expect("Heart image should load");

    // Verify initial shape is single Image(heart)
    match engine.get_explosion_shape() {
        ExplosionShape::Image(img) => {
            assert_eq!(img.file_stem, "heart");
        }
        _ => panic!("Expected single Image(heart) shape"),
    }

    // 2. Simulate GUI preset weights calculation when single image is active
    let preset_weights = preset_weights_from_shape(engine.get_explosion_shape());
    assert_eq!(preset_weights[0], 1.0); // Heart active
    assert_eq!(preset_weights[2], 0.0); // Smiley inactive (weight 0.0 in UI)

    // 3. User clicks +Add next to Smiley (index 2) when UI weight slider displays 0.0
    let smiley = &PRESET_DEFINITIONS[2];

    // Simulate command processing in gui_settings/mod.rs: effective_weight is max(1.0) if slider was 0.0
    let effective_weight = if preset_weights[2] <= 0.0 {
        1.0
    } else {
        preset_weights[2]
    };
    engine
        .load_explosion_image_weighted(
            smiley.path,
            smiley.default_scale,
            smiley.default_flight_time,
            effective_weight,
        )
        .expect("Smiley image should load weighted");

    // 4. Verify shape promoted to MultiImage containing BOTH Heart and Smiley!
    match engine.get_explosion_shape() {
        ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } => {
            assert_eq!(shapes.len(), 2, "Expected 2 active shapes after +Add");
            assert_eq!(shapes[0].0.file_stem, "heart");
            assert_eq!(shapes[0].1, 1.0, "Heart weight should be 1.0");
            assert_eq!(shapes[1].0.file_stem, "smiley");
            assert_eq!(shapes[1].1, 1.0, "Smiley weight should be 1.0");
            assert_eq!(*total_weight, 2.0f32, "Total weight should be 2.0");
        }
        _ => panic!("Expected MultiImage shape with 2 active items"),
    }
}

#[test]
fn test_shape_cumulativity_batch_add_all_five_presets() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);

    // Add all 5 presets cumulatively
    for (i, preset) in PRESET_DEFINITIONS.iter().enumerate() {
        let weight = (i + 1) as f32; // Weights: 1.0, 2.0, 3.0, 4.0, 5.0
        engine
            .load_explosion_image_weighted(
                preset.path,
                preset.default_scale,
                preset.default_flight_time,
                weight,
            )
            .expect("Preset image should load");
    }

    // Verify all 5 shapes are active in MultiImage
    match engine.get_explosion_shape() {
        ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } => {
            assert_eq!(shapes.len(), 5, "Expected all 5 preset shapes active");
            let expected_stems = ["heart", "star", "smiley", "note", "ring"];
            for (idx, (shape, w)) in shapes.iter().enumerate() {
                assert_eq!(shape.file_stem, expected_stems[idx]);
                assert_eq!(*w, (idx + 1) as f32);
            }
            assert_eq!(*total_weight, 15.0f32, "Total weight should be 15.0");
        }
        _ => panic!("Expected MultiImage with 5 shapes"),
    }
}

#[test]
fn test_shape_weight_slider_removal_and_readdition() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);

    // 1. Add Heart and Star
    let heart = &PRESET_DEFINITIONS[0];
    let star = &PRESET_DEFINITIONS[1];

    engine
        .load_explosion_image_weighted(
            heart.path,
            heart.default_scale,
            heart.default_flight_time,
            1.0,
        )
        .unwrap();
    engine
        .load_explosion_image_weighted(star.path, star.default_scale, star.default_flight_time, 1.0)
        .unwrap();

    if let ExplosionShape::MultiImage { shapes, .. } = engine.get_explosion_shape() {
        assert_eq!(shapes.len(), 2);
    } else {
        panic!("Expected MultiImage");
    }

    // 2. Drag Star weight slider to 0.0 -> removal request
    engine
        .load_explosion_image_weighted(star.path, star.default_scale, star.default_flight_time, 0.0)
        .unwrap();

    // 3. Verify Star removed, Heart remains
    match engine.get_explosion_shape() {
        ExplosionShape::MultiImage { shapes, .. } => {
            assert_eq!(shapes.len(), 1);
            assert_eq!(shapes[0].0.file_stem, "heart");
        }
        ExplosionShape::Image(img) => {
            assert_eq!(img.file_stem, "heart");
        }
        _ => panic!("Expected heart shape remaining"),
    }

    // 4. Re-add Star via +Add (effective_weight 1.0)
    engine
        .load_explosion_image_weighted(star.path, star.default_scale, star.default_flight_time, 1.0)
        .unwrap();

    match engine.get_explosion_shape() {
        ExplosionShape::MultiImage { shapes, .. } => {
            assert_eq!(shapes.len(), 2);
        }
        _ => panic!("Expected 2 shapes after re-adding Star"),
    }
}

#[test]
fn test_persisted_multi_image_shape_session_roundtrip() {
    let config = PhysicConfig::default();
    let mut engine = PhysicEngineFireworks::new(&config, 800.0, None);

    // 1. Build multi-image shape: Heart (1.5) + Ring (2.5)
    let heart = &PRESET_DEFINITIONS[0];
    let ring = &PRESET_DEFINITIONS[4];
    engine
        .load_explosion_image_weighted(
            heart.path,
            heart.default_scale,
            heart.default_flight_time,
            1.5,
        )
        .unwrap();
    engine
        .load_explosion_image_weighted(ring.path, ring.default_scale, ring.default_flight_time, 2.5)
        .unwrap();

    // 2. Persist to PersistedExplosionShape
    let persisted = PersistedExplosionShape::from_engine(engine.get_explosion_shape());
    match &persisted {
        PersistedExplosionShape::Images { images } => {
            assert_eq!(images.len(), 2);
            assert_eq!(images[0].file_stem, "heart");
            assert_eq!(images[0].weight, 1.5);
            assert_eq!(images[1].file_stem, "ring");
            assert_eq!(images[1].weight, 2.5);
        }
        _ => panic!("Expected PersistedExplosionShape::Images"),
    }

    // 3. Restore to clean engine using apply_session_to_physic
    let mut restored_engine = PhysicEngineFireworks::new(&config, 800.0, None);
    let weights = preset_weights_from_shape(engine.get_explosion_shape());
    apply_session_to_physic(weights, &persisted, &mut restored_engine);

    // 4. Verify restored engine has both shapes with identical weights
    match restored_engine.get_explosion_shape() {
        ExplosionShape::MultiImage {
            shapes,
            total_weight,
        } => {
            assert_eq!(shapes.len(), 2);
            assert_eq!(shapes[0].0.file_stem, "heart");
            assert_eq!(shapes[0].1, 1.5);
            assert_eq!(shapes[1].0.file_stem, "ring");
            assert_eq!(shapes[1].1, 2.5);
            assert_eq!(*total_weight, 4.0f32);
        }
        _ => panic!("Expected MultiImage restored in engine"),
    }
}

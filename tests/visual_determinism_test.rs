#[cfg(feature = "interactive_tests")]
mod visual_determinism_tests {
    use fireworks_sim::{
        audio_engine::audio_event::doppler_queue::DopplerQueue,
        audio_engine::config::AudioConfig,
        audio_engine::FireworksAudio3D,
        physic_engine::config::PhysicConfig,
        physic_engine::physic_engine_generational_arena::PhysicEngineFireworks,
        renderer_engine::renderer::Renderer,
        simulator::Simulator,
        window_engine::{GlfwWindowEngine, WindowEngine},
        PhysicEngine,
    };
    use image::RgbaImage;
    use std::fs;
    use std::io::Cursor;
    use std::path::Path;

    fn get_rendered_image_hash(seed: u64, frames: u64) -> u64 {
        let window_width = 800;
        let window_height = 600;

        let mut window_engine =
            GlfwWindowEngine::init(window_width, window_height, "visual-determinism")
                .expect("Failed to create headless GL context");

        let mut config = PhysicConfig::default();
        config.max_rockets = 50;
        config.rocket_interval_mean = 0.05;
        config.rocket_interval_variation = 0.0;
        config.particles_per_explosion = 100;
        config.particles_per_trail = 50;

        let renderer_engine =
            Renderer::new(window_width, window_height, &config).expect("Failed to create renderer");

        let mut physic_engine =
            PhysicEngineFireworks::new(&config, window_width as f32, Some(seed));

        let doppler_queue = DopplerQueue::new();
        physic_engine.set_doppler_sender(doppler_queue.sender.clone());

        let audio_config = AudioConfig::default().to_engine_config(config.max_rockets);
        let audio_engine =
            FireworksAudio3D::new(audio_config).expect("Failed to create audio engine");

        let mut simulator =
            Simulator::new(renderer_engine, physic_engine, audio_engine, window_engine);
        // Fixed dt
        simulator.fixed_dt = Some(1.0 / 60.0);
        simulator.max_frames = Some(frames);

        simulator.run(None).expect("Simulator run failed");

        let mut pixels = vec![0u8; (window_width * window_height * 4) as usize];
        unsafe {
            gl::ReadBuffer(gl::FRONT);
            gl::ReadPixels(
                0,
                0,
                window_width,
                window_height,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                pixels.as_mut_ptr() as *mut _,
            );
        }

        simulator.close();

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        pixels.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn test_visual_determinism_same_seed_same_pixels() {
        let seed = 42;
        let frames = 60;
        let hash1 = get_rendered_image_hash(seed, frames);
        let hash2 = get_rendered_image_hash(seed, frames);
        assert_eq!(
            hash1, hash2,
            "Identical runs with same seed must render same pixels"
        );
    }
}

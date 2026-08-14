/// Tests d'intégration OpenGL headless : vérifient que `recreate_buffers()` applique
/// correctement la séquence unmap→delete→reallocate et laisse les structures dans
/// un état cohérent.
///
/// Ces tests verrouillent le fix du bug "DeleteBuffers sans UnmapBuffer préalable"
/// (comportement indéfini selon la spec ARB_buffer_storage) en s'assurant que :
///   1. `mapped_ptr` est non-null après création ET après recreate.
///   2. `mapped_ptr` est null après `close()`.
///   3. Plusieurs cycles de `recreate_buffers()` ne crashent pas (double-delete, driver UB).
///
/// # Contexte du bug
/// `close()` appelait correctement `UnmapBuffer` avant `DeleteBuffers`, mais
/// `recreate_buffers()` ne le faisait pas. La refacto `release_particle_buffers()`
/// partage désormais ce code entre les deux chemins.
#[cfg(feature = "interactive_tests")]
mod renderer_recreate_buffers_tests {
    use fireworks_sim::physic_engine::PhysicConfig;
    use fireworks_sim::renderer_engine::{RendererGraphics, RendererGraphicsInstanced};
    use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};

    const TEST_TEXTURE_PATH: &str =
        "assets/textures/04ddeae2-7367-45f1-87e0-361d1d242630_scaled.png";

    fn init_headless_gl() -> GlfwWindowEngine {
        GlfwWindowEngine::init(1, 1, "headless-test").expect("Failed to create headless GL context")
    }

    // =========================================================================
    // RendererGraphics (point rendering)
    // =========================================================================

    /// Vérifie que `mapped_ptr` est valide après création et reste non-null après recreate.
    /// Vérifie qu'il est null après close() — preuve que l'unmap a bien eu lieu.
    #[test]
    fn test_renderer_graphics_recreate_buffers_ptr_invariant() {
        let mut window = init_headless_gl();

        let mut r = RendererGraphics::new(64);

        assert!(
            !r.mapped_ptr.is_null(),
            "mapped_ptr must be non-null right after construction"
        );

        unsafe { r.recreate_buffers(128) };
        assert!(
            !r.mapped_ptr.is_null(),
            "mapped_ptr must be non-null after recreate_buffers (new mapping established)"
        );

        unsafe { r.close() };
        assert!(
            r.mapped_ptr.is_null(),
            "mapped_ptr must be null after close() — proof that UnmapBuffer was called"
        );

        window.close_imgui();
    }

    /// Vérifie que plusieurs cycles successifs de recreate_buffers ne crashent pas
    /// (détecte les double-delete et les régressions d'UB OpenGL).
    #[test]
    fn test_renderer_graphics_multiple_recreate_cycles() {
        let mut window = init_headless_gl();

        let mut r = RendererGraphics::new(32);

        for size in [64usize, 128, 256, 64, 32] {
            unsafe { r.recreate_buffers(size) };
            assert!(
                !r.mapped_ptr.is_null(),
                "mapped_ptr must remain non-null after recreate_buffers({size})"
            );
            assert_eq!(
                r.max_particles_on_gpu, size,
                "max_particles_on_gpu must reflect new size after recreate"
            );
        }

        unsafe { r.close() };
        assert!(r.mapped_ptr.is_null());

        window.close_imgui();
    }

    // =========================================================================
    // RendererGraphicsInstanced (quad instanced rendering)
    // =========================================================================

    /// Même invariant mapped_ptr pour RendererGraphicsInstanced.
    #[test]
    fn test_renderer_graphics_instanced_recreate_buffers_ptr_invariant() {
        let mut window = init_headless_gl();

        let texture_data =
            fireworks_sim::renderer_engine::utils::texture::load_image_data_from_disk(
                TEST_TEXTURE_PATH,
            );
        let mut r = RendererGraphicsInstanced::new(
            64,
            fireworks_sim::physic_engine::ParticleType::Rocket,
            &texture_data,
        );

        assert!(
            r.is_mapped(),
            "mapped_ptr must be non-null right after construction"
        );

        unsafe { r.recreate_buffers(128) };
        assert!(
            r.is_mapped(),
            "mapped_ptr must be non-null after recreate_buffers"
        );

        unsafe { r.close() };
        assert!(!r.is_mapped(), "mapped_ptr must be null after close()");

        window.close_imgui();
    }

    /// Cycles multiples pour RendererGraphicsInstanced.
    #[test]
    fn test_renderer_graphics_instanced_multiple_recreate_cycles() {
        let mut window = init_headless_gl();

        let texture_data =
            fireworks_sim::renderer_engine::utils::texture::load_image_data_from_disk(
                TEST_TEXTURE_PATH,
            );
        let mut r = RendererGraphicsInstanced::new(
            32,
            fireworks_sim::physic_engine::ParticleType::Rocket,
            &texture_data,
        );

        for size in [64usize, 128, 64] {
            unsafe { r.recreate_buffers(size) };
            assert!(
                r.is_mapped(),
                "mapped_ptr must remain non-null after recreate_buffers({size})"
            );
            assert_eq!(r.max_particles(), size);
        }

        unsafe { r.close() };
        assert!(!r.is_mapped());

        window.close_imgui();
    }

    // =========================================================================
    // Scénario E2E : Renderer haut-niveau (via physic.apply)
    // =========================================================================

    /// Simule le chemin complet déclenché par `physic.apply` :
    ///   PhysicConfig change → Renderer::recreate_buffers(new_max) → close().
    #[test]
    fn test_renderer_high_level_recreate_after_config_change() {
        use fireworks_sim::renderer_engine::renderer::Renderer;
        use fireworks_sim::renderer_engine::RendererEngine;

        let mut window =
            GlfwWindowEngine::init(800, 600, "test-recreate").expect("Failed to create window");

        let mut renderer =
            Renderer::new(800, 600, &PhysicConfig::default()).expect("Failed to create Renderer");

        // Simule physic.apply avec max_rockets * particles_per_explosion changés
        renderer.recreate_buffers(128 * 512);

        // Deuxième cycle (régression double-recreate)
        renderer.recreate_buffers(64 * 256);

        renderer.close();
        window.close_imgui();
    }
}

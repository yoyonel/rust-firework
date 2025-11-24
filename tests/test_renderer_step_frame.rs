use fireworks_sim::physic_engine::PhysicConfig;
use fireworks_sim::renderer_engine::renderer::Renderer;
mod helpers;
use helpers::DummyPhysic;

use fireworks_sim::audio_engine::FireworksAudio3D;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::renderer_engine::RendererEngine;
use fireworks_sim::Simulator;

#[test]
fn test_renderer_step_frame_coverage() {
    let mut physic = DummyPhysic::default();

    // 1. Init Window (Hidden)
    let (_glfw, _window, _events, _imgui, _cursor_data) =
        Simulator::<
            fireworks_sim::renderer_engine::Renderer,
            PhysicEngineFireworks,
            FireworksAudio3D,
        >::init_window(800, 600, "Test Renderer")
        .expect("Failed to init window");

    // 2. Create Renderer
    let mut renderer =
        Renderer::new(800, 600, &PhysicConfig::default()).expect("Failed to create Renderer");

    // Ajout de particules pour tester le rendu
    physic
        .particles
        .push(fireworks_sim::physic_engine::particle::Particle::default());

    // Test recreate_buffers coverage (instead of reload_config)
    renderer.recreate_buffers(1000);

    // ✅ On appelle render_frame directement pour couvrir tout
    renderer.render_frame(&physic);

    // Vérifie qu'on peut fermer correctement
    renderer.close();
}

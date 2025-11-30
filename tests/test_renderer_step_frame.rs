#![cfg(feature = "interactive_tests")]

use fireworks_sim::physic_engine::PhysicConfig;
use fireworks_sim::renderer_engine::renderer::Renderer;
use fireworks_sim::renderer_engine::RendererEngine;
use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
mod helpers;
use helpers::DummyPhysic;

#[test]
#[ignore] // Segfaults in headless environment
fn test_renderer_step_frame() -> Result<(), Box<dyn std::error::Error>> {
    let mut physic = DummyPhysic::default();

    // 1. Init Window (Hidden)
    let _window_engine =
        GlfwWindowEngine::init(800, 600, "Test Renderer").expect("Failed to init window");

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

    Ok(())
}

#![cfg(feature = "interactive_tests")]

use fireworks_sim::renderer_engine::renderer::Renderer;
use fireworks_sim::renderer_engine::RendererEngine;
use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
mod helpers;
use fireworks_sim::physic_engine::PhysicConfig;
use helpers::DummyPhysic;

#[test]
#[ignore] // Segfaults in headless environment
fn test_renderer_render_frame() -> Result<(), Box<dyn std::error::Error>> {
    let physic = DummyPhysic::default();

    // 1. Init Window (Hidden) - Keep it alive to maintain OpenGL context
    let _window_engine = GlfwWindowEngine::init(800, 600, "Test Renderer")?;

    // 2. Create Renderer
    let mut renderer = Renderer::new(800, 600, &PhysicConfig::default())?;

    // 3. Render a frame
    let particles_count = renderer.render_frame(&physic);

    // Check something?
    // With DummyPhysic, 0 particles.
    assert_eq!(particles_count, 0);

    // 4. Close
    renderer.close();

    // Window is dropped here automatically
    Ok(())
}

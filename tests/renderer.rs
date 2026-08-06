#![cfg(feature = "interactive_tests")]

use fireworks_sim::renderer_engine::renderer::Renderer;
use fireworks_sim::renderer_engine::RendererEngine;
use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
mod helpers;
use fireworks_sim::physic_engine::PhysicConfig;
use helpers::DummyPhysic;

#[test]
fn test_renderer_render_frame() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("🔍 Step 1: Creating DummyPhysic");
    let physic = DummyPhysic::default();

    eprintln!("🔍 Step 2: Initializing GlfwWindowEngine");
    // 1. Init Window (Hidden) - Keep it alive to maintain OpenGL context
    let mut window_engine = GlfwWindowEngine::init(800, 600, "Test Renderer")?;
    eprintln!("✅ Window engine initialized");

    eprintln!("🔍 Step 3: Creating Renderer");
    // 2. Create Renderer
    let mut renderer = Renderer::new(800, 600, &PhysicConfig::default())?;
    eprintln!("✅ Renderer created");

    eprintln!("🔍 Step 4: Rendering frame");
    // 3. Render a frame
    let particles_count = renderer.render_frame(&physic, 0.0);
    eprintln!("✅ Frame rendered, particles: {}", particles_count);

    // Check something?
    // With DummyPhysic, 0 particles.
    assert_eq!(particles_count, 0);

    eprintln!("🔍 Step 5: Closing renderer");
    // 4. Close
    renderer.close();
    eprintln!("✅ Renderer closed");

    eprintln!("🔍 Step 6: Dropping renderer explicitly");
    drop(renderer);
    eprintln!("✅ Renderer dropped");

    eprintln!("🔍 Step 7: Closing ImGui explicitly");
    // Explicitly close ImGui to prevent SIGSEGV during window destruction
    window_engine.close_imgui();
    eprintln!("✅ ImGui closed");

    eprintln!("🔍 Step 8: About to drop window engine");
    // Keep window_engine alive until here
    drop(window_engine);
    eprintln!("✅ Window engine dropped");

    Ok(())
}

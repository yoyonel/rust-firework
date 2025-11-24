use fireworks_sim::renderer_engine::renderer::Renderer;
mod helpers;
use fireworks_sim::physic_engine::PhysicConfig;
use helpers::DummyPhysic;

use fireworks_sim::audio_engine::FireworksAudio3D;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::renderer_engine::RendererEngine;
use fireworks_sim::Simulator;

#[test]
fn test_renderer_render_frame() {
    let physic = DummyPhysic::default();

    // 1. Init Window (Hidden)
    // We use the concrete types for init_window because it's a static method on Simulator<R, P, A>
    // But actually init_window doesn't depend on R, P, A types for logic, but it's defined in the impl block.
    // So we need to specify types. We can use the real types or mocks if they satisfy bounds.
    // DummyRenderer satisfies RendererEngine.
    // DummyPhysic satisfies PhysicEngineFull.
    // DummyAudio satisfies AudioEngine.
    let (_glfw, _window, _events, _imgui) = Simulator::<
        fireworks_sim::renderer_engine::Renderer,
        PhysicEngineFireworks,
        FireworksAudio3D,
    >::init_window(800, 600, "Test Renderer")
    .expect("Failed to init window");

    // 2. Create Renderer
    let mut renderer =
        Renderer::new(800, 600, &PhysicConfig::default()).expect("Failed to create Renderer");

    // 3. Render a frame
    // We don't need the loop, just one call
    let particles_count = renderer.render_frame(&physic);

    // Check something?
    // With DummyPhysic, 0 particles.
    assert_eq!(particles_count, 0);

    // 4. Close
    renderer.close();

    // Window is dropped here
}

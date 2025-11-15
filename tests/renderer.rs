use fireworks_sim::renderer_engine::renderer::Renderer;
mod helpers;
use fireworks_sim::physic_engine::PhysicConfig;
use helpers::{DummyAudio, DummyPhysic};

#[test]
fn test_renderer_run_loop_one_frame() {
    let mut audio = DummyAudio;
    let mut physic = DummyPhysic;

    let mut renderer = Renderer::new(800, 600, "Test Renderer", &PhysicConfig::default().clone())
        .expect("Failed to create Renderer");

    // Force la fermeture de la fenêtre après 1 frame
    renderer
        .window
        .as_mut()
        .expect("Window not initialized")
        .set_should_close(true);

    // Appelle run_loop : la boucle doit s'arrêter immédiatement
    renderer
        .run_loop(&mut physic, &mut audio)
        .expect("run_loop failed");

    // Ferme correctement
    renderer.close();
}

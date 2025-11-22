use fireworks_sim::physic_engine::PhysicConfig;
use fireworks_sim::renderer_engine::renderer::Renderer;
mod helpers;
use helpers::DummyPhysic;

#[test]
fn test_renderer_step_frame_coverage() {
    let mut physic = DummyPhysic::default();
    // Renderer minimal (pas de fenêtre réelle pour test)
    let mut renderer =
        Renderer::new(800, 600, "Test Renderer", &PhysicConfig::default().clone()).unwrap();

    // Forcer la fermeture immédiate pour éviter boucle infinie
    renderer.window.as_mut().unwrap().set_should_close(true);

    // ✅ On appelle step_frame directement pour couvrir tout
    unsafe {
        renderer.render_frame(&mut physic);
    }

    // Vérifie qu'on peut fermer correctement
    renderer.close();
}

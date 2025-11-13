use fireworks_sim::renderer_engine::renderer::Renderer;
mod helpers;
use helpers::DummyPhysic;

#[test]
fn test_renderer_step_frame_coverage() {
    let mut physic = DummyPhysic;
    // Renderer minimal (pas de fenêtre réelle pour test)
    let mut renderer = Renderer::new(800, 600, "Test Renderer", 0).unwrap();

    // Forcer la fermeture immédiate pour éviter boucle infinie
    renderer.window.as_mut().unwrap().set_should_close(true);

    // ✅ On appelle step_frame directement pour couvrir tout
    renderer.render_frame(&mut physic);

    // Vérifie qu'on peut fermer correctement
    renderer.close();
}

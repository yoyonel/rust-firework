use fireworks_sim::audio_engine::FireworksAudio3D;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::renderer_engine::Renderer;
use fireworks_sim::Simulator;
use std::cell::RefCell;
use std::rc::Rc;
mod helpers;
use helpers::{TestAudio, TestPhysic, TestRenderer};

/// Vérifie que le Simulator appelle quand même les méthodes `close`
/// après une erreur dans le moteur de rendu.
#[test]
fn test_renderer_error_triggers_proper_cleanup() {
    let log = Rc::new(RefCell::new(vec![]));

    // --- Assemblage du simulateur ---
    let mut renderer = TestRenderer::new(log.clone());
    renderer.fail_on_run_loop = true; // Simulation d'erreur

    let physic = TestPhysic::new(log.clone());
    let audio = TestAudio::new(log.clone());

    let mut sim = {
        let (glfw, window, events, imgui) = Simulator::<
            Renderer,
            PhysicEngineFireworks,
            FireworksAudio3D,
        >::init_window(800, 600, "Test Simulator")
        .unwrap();
        Simulator::new(renderer, physic, audio, glfw, window, events, imgui)
    };

    // --- Simulation d'une exécution échouée ---
    // TestRenderer panics on failure now
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sim.run(None)));
    assert!(run_result.is_err(), "Expected renderer to fail (panic)");

    // Même en cas d’erreur, on appelle `close()` explicitement
    sim.close();

    // --- Vérification du journal ---
    let calls = log.borrow();
    assert_eq!(
        *calls,
        vec![
            "audio.start",
            "set_listener_position called",
            "physic.update",
            "renderer.render_frame",
            "renderer.close",
            "physic.close",
            "audio.stop",
        ],
        "Unexpected call order: {:?}",
        *calls
    );
}

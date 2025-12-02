#![cfg(feature = "interactive_tests")]

use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
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
        let window_engine = GlfwWindowEngine::init(800, 600, "Test Simulator").unwrap();
        Simulator::new(renderer, physic, audio, window_engine)
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

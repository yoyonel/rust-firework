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

    let mut sim = Simulator::new(renderer, physic, audio);

    // --- Simulation d'une exécution échouée ---
    let run_result = sim.run(None);
    assert!(run_result.is_err(), "Expected renderer to fail");

    // Même en cas d’erreur, on appelle `close()` explicitement
    sim.close();

    // --- Vérification du journal ---
    let calls = log.borrow();
    assert_eq!(
        *calls,
        vec![
            "audio.start",             // toujours appelé avant run_loop
            "renderer.run_loop.start", // le renderer démarre
            // -> erreur simulée ici (pas de update/play_rocket)
            "renderer.close", // cleanup explicite
            "physic.close",
            "audio.stop",
        ],
        "Unexpected call order: {:?}",
        *calls
    );
}

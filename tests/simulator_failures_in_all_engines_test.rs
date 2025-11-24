use fireworks_sim::audio_engine::FireworksAudio3D;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::renderer_engine::Renderer;
use fireworks_sim::Simulator;
use std::cell::RefCell;
use std::rc::Rc;
mod helpers;
use helpers::{TestAudio, TestPhysic, TestRenderer};

#[derive(Clone, Copy, Debug)]
enum EngineFailure {
    Renderer,
    Physic,
    Audio,
}

fn run_failure_test(failure: EngineFailure) {
    let log = Rc::new(RefCell::new(vec![]));

    // --- Configuration des moteurs ---
    let mut renderer = TestRenderer::new(log.clone());
    if let EngineFailure::Renderer = failure {
        renderer.fail_on_run_loop = true;
    }

    let mut physic = TestPhysic::new(log.clone());
    if let EngineFailure::Physic = failure {
        physic.fail_on_update = true;
    }

    let mut audio = TestAudio::new(log.clone());
    if let EngineFailure::Audio = failure {
        audio.fail_on_start = true;
    }

    let mut sim = {
        let (glfw, window, events, imgui) = Simulator::<
            Renderer,
            PhysicEngineFireworks,
            FireworksAudio3D,
        >::init_window(800, 600, "Test Simulator")
        .unwrap();
        Simulator::new(renderer, physic, audio, glfw, window, events, imgui)
    };

    // --- Exécution & vérification ---
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sim.run(None)));

    // On tolère deux cas :
    // - panic (p.ex. Audio/Physic)
    // - Err (p.ex. Renderer)
    assert!(
        result.is_err() || result.as_ref().unwrap().is_err(),
        "Expected simulator.run() to fail for {:?}",
        failure
    );

    // Fermeture manuelle pour vérifier le nettoyage
    sim.close();

    // --- Vérification du log ---
    let calls = log.borrow();
    println!("[{:?}] => {:?}", failure, *calls);

    assert!(
        calls.contains(&"renderer.close".into()),
        "renderer.close() not called after {:?} failure",
        failure
    );
    assert!(
        calls.contains(&"physic.close".into()),
        "physic.close() not called after {:?} failure",
        failure
    );
    assert!(
        calls.contains(&"audio.stop".into()),
        "audio.stop() not called after {:?} failure",
        failure
    );
}

#[test]
fn test_failures_in_all_engines() {
    for failure in [
        EngineFailure::Renderer,
        EngineFailure::Physic,
        EngineFailure::Audio,
    ] {
        run_failure_test(failure);
    }
}

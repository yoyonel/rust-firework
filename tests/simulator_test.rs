use fireworks_sim::Simulator;
use std::cell::RefCell;
use std::rc::Rc;
mod helpers;
use helpers::{DummyAudio, DummyPhysic, DummyRenderer, TestAudio, TestPhysic, TestRenderer};

#[test]
fn test_simulator_with_dummy_engines() -> anyhow::Result<()> {
    let renderer = DummyRenderer;
    let audio = DummyAudio;
    let physic = DummyPhysic::default();
    let mut simulator = Simulator::new(renderer, physic, audio);
    simulator.run(None).unwrap();
    simulator.close();
    println!("Simulator closed.");

    Ok(())
}

#[test]
fn test_renderer_called_by_simulator() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let audio = DummyAudio;
    let physic = DummyPhysic::default();

    let mut sim = Simulator::new(renderer, physic, audio);
    sim.run(None).unwrap();
    sim.close();

    assert_eq!(
        *log.borrow(),
        vec![
            "renderer.run_loop.start",
            "renderer.run_loop.end",
            "renderer.close"
        ]
    );
}

#[test]
fn test_audio_called_by_renderer() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let audio = TestAudio::new(log.clone());
    let physic = DummyPhysic::default();

    let mut sim = Simulator::new(renderer, physic, audio);
    sim.run(None).unwrap();
    sim.close();

    // On vérifie que le Renderer a bien appelé start_audio_thread
    // Note: TestRenderer appelle aussi play_rocket
    let calls = log.borrow();
    assert!(calls.contains(&"audio.start".into()));
    assert!(calls.contains(&"play_rocket called".into()));
    assert!(calls.contains(&"audio.stop".into()));
}

#[test]
fn test_physic_called_by_renderer() {
    let log = Rc::new(RefCell::new(vec![]));
    let renderer = TestRenderer::new(log.clone());
    let audio = DummyAudio;
    let physic = TestPhysic::new(log.clone());

    let mut sim = Simulator::new(renderer, physic, audio);
    sim.run(None).unwrap();
    sim.close();

    let calls = log.borrow();
    assert!(calls.contains(&"physic.update".into()));
    assert!(calls.contains(&"physic.close".into()));
}

// Ce test vérifie l'ordre global des appels entre les moteurs
#[test]
fn test_call_order_in_simulator_run_and_close() -> anyhow::Result<()> {
    // Journal partagé entre tous les mocks
    let log = Rc::new(RefCell::new(vec![]));

    // --- Assemblage du simulateur ---
    let renderer = TestRenderer::new(log.clone());
    let physic = TestPhysic::new(log.clone());
    let audio = TestAudio::new(log.clone());

    let mut sim = Simulator::new(renderer, physic, audio);

    // --- Exécution du simulateur ---
    sim.run(None)?;
    sim.close();

    // --- Vérification de l’ordre des appels ---
    let calls = log.borrow();
    assert_eq!(
        *calls,
        vec![
            // --- Phase de run ---
            "audio.start",
            "renderer.run_loop.start",
            "physic.update",
            "play_rocket called", // Appelé par TestRenderer
            "renderer.run_loop.end",
            // --- Phase de close ---
            "renderer.close",
            "physic.close",
            "audio.stop",
        ]
    );

    Ok(())
}

use fireworks_sim::audio_engine::AudioEngine;
use fireworks_sim::physic_engine::{
    config::PhysicConfig, particle::Particle, rocket::Rocket, types::UpdateResult, PhysicEngine,
};
use fireworks_sim::renderer_engine::RendererEngine;
use fireworks_sim::Simulator;
use std::cell::RefCell;
use std::rc::Rc;
mod helpers; // inclut tests/helpers.rs comme sous-module
use helpers::{
    DummyAudio, DummyPhysic, DummyRenderer, LoggingAudio, LoggingPhysic, LoggingRenderer,
};

#[test]
fn test_simulator_with_dummy_engines() -> anyhow::Result<()> {
    let renderer = DummyRenderer;
    let audio = DummyAudio;
    let physic = DummyPhysic;
    let mut simulator = Simulator::new(renderer, physic, audio);
    simulator.run(None).unwrap();
    simulator.close();
    println!("Simulator closed.");

    Ok(())
}

#[test]
fn test_renderer_called_by_simulator() {
    let renderer = LoggingRenderer { log: vec![] };
    let audio = DummyAudio;
    let physic = DummyPhysic;

    let mut sim = Simulator::new(renderer, physic, audio);
    sim.run(None).unwrap();
    sim.close();

    assert_eq!(
        sim.renderer_engine().log,
        vec!["run_loop called", "close called"]
    );
}

#[test]
fn test_audio_called_by_renderer() {
    let renderer = LoggingRenderer { log: vec![] };
    let audio = LoggingAudio {
        log: RefCell::new(vec![]),
    };
    let physic = LoggingPhysic { log: vec![] };

    let mut sim = Simulator::new(renderer, physic, audio);
    sim.run(None).unwrap();
    sim.close();

    // On vérifie que le Renderer a bien appelé start_audio_thread
    assert_eq!(
        sim.audio_engine().log.borrow().as_slice(),
        &[
            "start_audio_thread called",
            "play_rocket called",
            "stop_audio_thread called"
        ]
    );
}

#[test]
fn test_physic_called_by_renderer() {
    let renderer = LoggingRenderer { log: vec![] };
    let audio = LoggingAudio {
        log: RefCell::new(vec![]),
    };
    let physic = LoggingPhysic { log: vec![] };

    let mut sim = Simulator::new(renderer, physic, audio);
    sim.run(None).unwrap();
    sim.close();

    // On vérifie que le Renderer a bien appelé update et set_width
    assert_eq!(
        sim.physic_engine().log,
        vec!["update called", "close called"]
    );
}

// Ce test vérifie l'ordre global des appels entre les moteurs
#[test]
fn test_call_order_in_simulator_run_and_close() -> anyhow::Result<()> {
    // Journal partagé entre tous les mocks
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));

    // --- Mock du moteur Audio ---
    struct OrderedAudio {
        log: Rc<RefCell<Vec<String>>>,
    }
    impl AudioEngine for OrderedAudio {
        fn start_audio_thread(&mut self, _export_path: Option<&str>) {
            self.log.borrow_mut().push("audio.start".into());
        }
        fn stop_audio_thread(&mut self) {
            self.log.borrow_mut().push("audio.stop".into());
        }
        fn get_listener_position(&self) -> (f32, f32) {
            (0.0, 0.0)
        }
        fn set_listener_position(&mut self, _pos: (f32, f32)) {}
        fn play_rocket(&self, _pos: (f32, f32), _gain: f32) {}
        fn play_explosion(&self, _pos: (f32, f32), _gain: f32) {}
    }

    // --- Mock du moteur Physique ---
    struct OrderedPhysic {
        log: Rc<RefCell<Vec<String>>>,
    }
    impl PhysicEngine for OrderedPhysic {
        fn update(&mut self, _dt: f32) -> UpdateResult<'_> {
            self.log.borrow_mut().push("physic.update".into());
            UpdateResult {
                new_rocket: None,
                explosions: &[],
            }
        }
        fn set_window_width(&mut self, _width: f32) {
            self.log.borrow_mut().push("physic.set_width".into());
        }
        fn active_particles<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Particle> + 'a> {
            Box::new(std::iter::empty())
        }
        fn active_rockets<'a>(&'a self) -> Box<dyn Iterator<Item = &'a Rocket> + 'a> {
            Box::new(std::iter::empty())
        }
        fn close(&mut self) {
            self.log.borrow_mut().push("physic.close".into());
        }
        fn reload_config(&mut self, _config: &PhysicConfig) -> bool {
            false
        }
    }

    // --- Mock du moteur de rendu ---
    struct OrderedRenderer {
        log: Rc<RefCell<Vec<String>>>,
    }
    impl RendererEngine for OrderedRenderer {
        fn run_loop<P: PhysicEngine, A: AudioEngine>(
            &mut self,
            physic: &mut P,
            audio: &mut A,
        ) -> anyhow::Result<()> {
            self.log.borrow_mut().push("renderer.run_loop.start".into());
            physic.update(0.016); // Simule une frame
            audio.play_rocket((0.0, 0.0), 1.0); // Simule un son
            self.log.borrow_mut().push("renderer.run_loop.end".into());
            Ok(())
        }

        fn close(&mut self) {
            self.log.borrow_mut().push("renderer.close".into());
        }
    }

    // --- Assemblage du simulateur ---
    let renderer = OrderedRenderer { log: log.clone() };
    let physic = OrderedPhysic { log: log.clone() };
    let audio = OrderedAudio { log: log.clone() };

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
            "renderer.run_loop.end",
            // --- Phase de close ---
            "renderer.close",
            "physic.close",
            "audio.stop",
        ]
    );

    Ok(())
}

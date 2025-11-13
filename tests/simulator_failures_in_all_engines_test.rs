use fireworks_sim::{
    audio_engine::AudioEngine,
    physic_engine::{
        config::PhysicConfig, particle::Particle, rocket::Rocket, types::UpdateResult, PhysicEngine,
    },
    renderer_engine::RendererEngine,
    Simulator,
};

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug)]
enum EngineFailure {
    Renderer,
    Physic,
    Audio,
}

fn run_failure_test(failure: EngineFailure) {
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));

    // --- Mock Audio ---
    struct MockAudio {
        log: Rc<RefCell<Vec<String>>>,
        fail: bool,
    }
    impl AudioEngine for MockAudio {
        fn start_audio_thread(&mut self, _export_path: Option<&str>) {
            self.log.borrow_mut().push("audio.start".into());
            if self.fail {
                panic!("AudioEngine failed at start_audio_thread");
            }
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

    // --- Mock Physic ---
    struct MockPhysic {
        log: Rc<RefCell<Vec<String>>>,
        fail: bool,
    }
    impl PhysicEngine for MockPhysic {
        fn update(&mut self, _dt: f32) -> UpdateResult<'_> {
            self.log.borrow_mut().push("physic.update".into());
            if self.fail {
                panic!("PhysicEngine failed during update");
            }
            UpdateResult {
                new_rocket: None,
                triggered_explosions: &[],
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

    // --- Mock Renderer ---
    struct MockRenderer {
        log: Rc<RefCell<Vec<String>>>,
        fail: bool,
    }
    impl RendererEngine for MockRenderer {
        fn run_loop<P: PhysicEngine, A: AudioEngine>(
            &mut self,
            physic: &mut P,
            _audio: &mut A,
        ) -> anyhow::Result<()> {
            self.log.borrow_mut().push("renderer.run_loop".into());
            if self.fail {
                return Err(anyhow::anyhow!("RendererEngine simulated failure"));
            }
            // Appelle `update()` pour simuler une frame
            let _ = physic.update(0.016);
            Ok(())
        }
        fn close(&mut self) {
            self.log.borrow_mut().push("renderer.close".into());
        }
    }

    // --- Création des moteurs avec la panne ciblée ---
    let renderer = MockRenderer {
        log: log.clone(),
        fail: matches!(failure, EngineFailure::Renderer),
    };
    let physic = MockPhysic {
        log: log.clone(),
        fail: matches!(failure, EngineFailure::Physic),
    };
    let audio = MockAudio {
        log: log.clone(),
        fail: matches!(failure, EngineFailure::Audio),
    };

    let mut sim = Simulator::new(renderer, physic, audio);

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

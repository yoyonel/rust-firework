use fireworks_sim::audio_engine::AudioEngine;
use fireworks_sim::physic_engine::{
    config::PhysicConfig, particle::Particle, types::UpdateResult, PhysicEngine,
};

use fireworks_sim::renderer_engine::RendererEngine;
use fireworks_sim::Simulator;
use std::cell::RefCell;
use std::rc::Rc;

/// Vérifie que le Simulator appelle quand même les méthodes `close`
/// après une erreur dans le moteur de rendu.
#[test]
fn test_renderer_error_triggers_proper_cleanup() {
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));

    // --- Mock du moteur Audio ---
    struct ErrorAudio {
        log: Rc<RefCell<Vec<String>>>,
    }
    impl AudioEngine for ErrorAudio {
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
    struct ErrorPhysic {
        log: Rc<RefCell<Vec<String>>>,
    }
    impl PhysicEngine for ErrorPhysic {
        fn update(&mut self, _dt: f32) -> UpdateResult<'_> {
            self.log.borrow_mut().push("physic.update".into());
            UpdateResult {
                new_rocket: None,
                triggered_explosions: &[],
            }
        }
        fn set_window_width(&mut self, _width: f32) {
            self.log.borrow_mut().push("physic.set_width".into());
        }
        fn iter_active_particles<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
            // Crée un itérateur vide, compatible avec la signature
            std::iter::empty()
        }
        fn iter_active_heads_not_exploded<'a>(&'a self) -> impl Iterator<Item = &'a Particle> + 'a {
            std::iter::empty()
        }
        fn close(&mut self) {
            self.log.borrow_mut().push("physic.close".into());
        }
        fn reload_config(&mut self, _config: &PhysicConfig) -> bool {
            false
        }
    }

    // --- Mock du moteur Renderer (qui simule une erreur) ---
    struct FailingRenderer {
        log: Rc<RefCell<Vec<String>>>,
    }
    impl RendererEngine for FailingRenderer {
        fn run_loop<P: PhysicEngine, A: AudioEngine>(
            &mut self,
            _physic: &mut P,
            _audio: &mut A,
        ) -> anyhow::Result<()> {
            self.log.borrow_mut().push("renderer.run_loop.start".into());
            Err(anyhow::anyhow!("Simulated renderer failure"))
        }

        fn close(&mut self) {
            self.log.borrow_mut().push("renderer.close".into());
        }
    }

    // --- Assemblage du simulateur ---
    let renderer = FailingRenderer { log: log.clone() };
    let physic = ErrorPhysic { log: log.clone() };
    let audio = ErrorAudio { log: log.clone() };

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
            // -> erreur simulée ici
            "renderer.close", // cleanup explicite
            "physic.close",
            "audio.stop",
        ],
        "Unexpected call order: {:?}",
        *calls
    );
}

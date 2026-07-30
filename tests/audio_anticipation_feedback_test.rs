use fireworks_sim::audio_engine::types::{AudioDebugEvent, AudioSoundType};
use fireworks_sim::audio_engine::{AudioEffect, AudioEngine};
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::Simulator;
use glam::Vec2;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod helpers;
use helpers::{DummyRenderer, DummyWindowEngine};

// A mock audio engine that simulates a constant, configurable transit latency
// and records debug events to feed back to the simulator.
struct MockFeedbackAudio {
    events: Arc<Mutex<Vec<AudioDebugEvent>>>,
    next_request_id: std::sync::atomic::AtomicU64,
    simulated_transit_delay_ms: f32,
}

impl MockFeedbackAudio {
    fn new(simulated_transit_delay_ms: f32) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            next_request_id: std::sync::atomic::AtomicU64::new(1),
            simulated_transit_delay_ms,
        }
    }
}

impl AudioEngine for MockFeedbackAudio {
    fn play_rocket(&self, _pos: Vec2, _gain: f32) {}

    fn play_rocket_with_id(&self, id: u64, _pos: Vec2, _gain: f32) {
        let req_id = self
            .next_request_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let sent_at = Instant::now();

        let mut evts = self.events.lock().unwrap();
        evts.push(AudioDebugEvent::Sent {
            request_id: req_id,
            sound_type: AudioSoundType::Rocket,
            entity_id: id,
            sent_at,
        });

        // Simulate the sound rendering starting after exactly simulated_transit_delay_ms
        let started_at =
            sent_at + Duration::from_secs_f32(self.simulated_transit_delay_ms / 1000.0);
        evts.push(AudioDebugEvent::Started {
            request_id: req_id,
            started_at,
            voice_index: 0,
        });
    }

    fn play_explosion(&self, _pos: Vec2, _gain: f32) {}

    fn play_explosion_with_id(&self, id: u64, _pos: Vec2, _gain: f32) {
        let req_id = self
            .next_request_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let sent_at = Instant::now();

        let mut evts = self.events.lock().unwrap();
        evts.push(AudioDebugEvent::Sent {
            request_id: req_id,
            sound_type: AudioSoundType::Explosion,
            entity_id: id,
            sent_at,
        });

        // Simulate the sound rendering starting after exactly simulated_transit_delay_ms
        let started_at =
            sent_at + Duration::from_secs_f32(self.simulated_transit_delay_ms / 1000.0);
        evts.push(AudioDebugEvent::Started {
            request_id: req_id,
            started_at,
            voice_index: 1,
        });
    }

    fn pop_debug_events(&self, buf: &mut Vec<AudioDebugEvent>) {
        let mut evts = self.events.lock().unwrap();
        buf.append(&mut *evts);
    }

    fn start_audio_thread(&mut self, _export_path: Option<&str>) {}
    fn stop_audio_thread(&mut self) {}
    fn set_listener_position(&mut self, _pos: Vec2) {}
    fn get_listener_position(&self) -> Vec2 {
        Vec2::ZERO
    }
    fn mute(&mut self) {}
    fn unmute(&mut self) -> f32 {
        1.0
    }
    fn set_effect_enabled(&self, _effect: AudioEffect, _enabled: bool) {}
    fn set_all_effects_enabled(&self, _enabled: bool) {}
    fn get_effect_enabled(&self, _effect: AudioEffect) -> bool {
        true
    }
    fn get_effects_status(&self) -> String {
        "Mock".to_string()
    }
    fn as_audio_engine(&self) -> &dyn AudioEngine {
        self
    }
}

#[test]
fn test_audio_anticipation_feedback_loop() -> anyhow::Result<()> {
    // 1. Initialiser une configuration physique avec des valeurs d'anticipation de départ à 10 ms
    let config = PhysicConfig {
        audio_launch_anticipation_ms: 10.0,
        audio_explosion_anticipation_ms: 10.0,
        rocket_interval_mean: 0.025,
        rocket_interval_variation: 0.0,
        rocket_max_next_interval: 0.025,
        spawn_rocket_min_speed: 50.0,
        spawn_rocket_max_speed: 60.0,
        gravity: -200.0,
        explosion_threshold: 40.0,
        ..Default::default()
    };

    let physic_engine = PhysicEngineFireworks::new(&config, 800.0);

    // 2. Simuler un retard audio matériel fixe de 15 ms
    let simulated_transit = 15.0; // ms
    let audio_engine = MockFeedbackAudio::new(simulated_transit);

    let renderer_engine = DummyRenderer::default();
    let window_engine = DummyWindowEngine::default();

    let mut simulator = Simulator::new(renderer_engine, physic_engine, audio_engine, window_engine);

    // Activer l'affichage diagnostique pour que process_audio_debug_events soit appelé à chaque step
    simulator.show_audio_diagnostic = true;

    // Récupérer les valeurs d'anticipation initiales
    let initial_launch_anticip = simulator.get_physic_config().audio_launch_anticipation_ms;
    let initial_explosion_anticip = simulator
        .get_physic_config()
        .audio_explosion_anticipation_ms;
    assert_eq!(initial_launch_anticip, 10.0);
    assert_eq!(initial_explosion_anticip, 10.0);

    println!(
        "Start: launch_anticip = {:.2} ms, explosion_anticip = {:.2} ms",
        initial_launch_anticip, initial_explosion_anticip
    );

    // 3. Faire tourner la boucle de simulation pendant 1000 frames de 2 ms (2.0 secondes de simulation au total)
    // À chaque pas, des requêtes audio vont partir et le feedback loop va ajuster les paramètres
    let dt = 0.002; // 2 ms frame time
    for _ in 0..1000 {
        // Simuler le step temporel du simulateur (qui traite aussi les événements audio maintenant)
        simulator.step_custom_dt(dt);
        // Attendre 2 ms pour que le temps réel Instant::now() s'écoule de manière cohérente avec le temps physique dt
        std::thread::sleep(Duration::from_millis(2));
    }

    // Récupérer les valeurs finales d'anticipation
    let final_launch_anticip = simulator.get_physic_config().audio_launch_anticipation_ms;
    let final_explosion_anticip = simulator
        .get_physic_config()
        .audio_explosion_anticipation_ms;

    println!(
        "End: launch_anticip = {:.2} ms, explosion_anticip = {:.2} ms",
        final_launch_anticip, final_explosion_anticip
    );
    println!(
        "Launches tracked: {}, Explosions tracked: {}",
        simulator.sync_launch_count, simulator.sync_explosion_count
    );

    // 4. Vérifier que la boucle de rétroaction est active et a fait évoluer les paramètres de départ
    assert!(
        final_launch_anticip != 10.0,
        "Launch anticipation should have evolved from its initial 10.0 ms"
    );
    assert!(
        final_explosion_anticip != 10.0,
        "Explosion anticipation should have evolved from its initial 10.0 ms"
    );

    // 5. Vérifier que les moyennes de désynchronisation de l'affichage diagnostic tendent vers 0.0 ms
    let (avg_launch_sync, avg_explosion_sync) = simulator.get_average_syncs_test_helper();
    println!(
        "Average Launch Sync: {:.3} ms, Average Explosion Sync: {:.3} ms",
        avg_launch_sync, avg_explosion_sync
    );

    assert!(
        avg_launch_sync.abs() < 1.5,
        "Average launch sync ({:.3} ms) should be very close to 0 ms",
        avg_launch_sync
    );
    assert!(
        avg_explosion_sync.abs() < 1.5,
        "Average explosion sync ({:.3} ms) should be very close to 0 ms",
        avg_explosion_sync
    );

    Ok(())
}

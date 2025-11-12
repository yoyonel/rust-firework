// Ici on importe depuis la crate lib complète
use anyhow::Result;
use log::info;
use std::{cmp, env, path::PathBuf};

// use fireworks_sim::audio_engine::audio_event::doppler_queue::DopplerQueue;
use fireworks_sim::audio_engine::settings::AudioEngineSettings;
use fireworks_sim::audio_engine::{FireworksAudio3D, FireworksAudioConfig};
use fireworks_sim::physic_engine::config::PhysicConfig;
// use fireworks_sim::physic_engine::physic_engine_static_aos::PhysicEngineFireworks;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::Simulator;

use fireworks_sim::renderer_engine::renderer::Renderer;

fn main() -> Result<()> {
    env_logger::init();

    info!("🚀 Starting Fireworks Simulator...");

    // TODO: mettre en place un vrai gestionnaire de configurations (avec traits) !
    let physic_config = PhysicConfig::from_file("assets/config/physic.toml").unwrap_or_default();
    info!("Physic config loaded:\n{:#?}", physic_config);

    // --------------------------
    // Gestion du chemin d'export audio
    // --------------------------
    let export_path = std::env::args()
        .nth(1) // priorité à l'argument CLI
        .map(PathBuf::from)
        .or_else(|| env::var("FIREWORKS_AUDIO_EXPORT").ok().map(PathBuf::from));

    if let Some(path) = &export_path {
        info!("Audio export path set to: {}", path.display());
    }

    // --------------------------
    // Initialisation des moteurs
    // --------------------------
    // Paramètres audio par défaut
    let audio_settings = AudioEngineSettings::default();
    // let doppler_queue = DopplerQueue::new();
    let audio_config = FireworksAudioConfig {
        // TODO: meilleur gestion des chemins (assets), avec une lib (python) style pathlib
        rocket_path: "assets/sounds/rocket.wav".into(),
        explosion_path: "assets/sounds/explosion.wav".into(),
        // TODO: afficher visuellement la position de l'auditeur
        listener_pos: (0.0, 0.0),
        // TODO: faudrait étudier l'influence de ce paramètre et les types de valeurs qu'on peut utiliser (et dans quel intérêt)
        sample_rate: 48000,
        // TODO: étudier l'influence sonore (qualité du rendu) et de performance de ce paramètre block_size
        block_size: 512,
        // limité à 32 voix, si MAX_ROCKETS "grand", évite le bordel sonore (effet mitraille très désagréable)
        max_voices: cmp::min(32, physic_config.max_rockets),
        settings: audio_settings.clone(),
        // doppler_receiver: Some(doppler_queue.receiver.clone()),
        // doppler_states: Vec::new(),
        // export_in_wav: true,
    };
    let audio_engine = FireworksAudio3D::new(audio_config);

    let window_width = 1024;

    let physic_engine = PhysicEngineFireworks::new(&physic_config, window_width as f32);

    let renderer_engine = Renderer::new(
        window_width,
        800,
        "Fireworks Simulator",
        physic_engine.max_particles(),
        // Some(doppler_queue.sender.clone()),
        false,
    )?;

    // ----------------------------
    // Initialisation du simulateur
    // ----------------------------
    info!("🚀 Starting Fireworks Simulator...");
    let mut simulator = Simulator::new(renderer_engine, physic_engine, audio_engine);
    let _ = simulator.run(export_path.as_ref().map(|p| p.to_str().unwrap()));
    simulator.close();

    Ok(())
}

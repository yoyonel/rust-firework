// Ici on importe depuis la crate lib complète
use anyhow::Result;
use fireworks_sim::audio_engine::audio_event::doppler_queue::DopplerQueue;
use fireworks_sim::audio_engine::config::AudioConfig;
use fireworks_sim::audio_engine::FireworksAudio3D;
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::renderer_engine::renderer::Renderer;
use fireworks_sim::utils::show_rust_core_dependencies;
use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
use fireworks_sim::PhysicEngine;
use fireworks_sim::Simulator;
use log::info;
use std::{env, path::PathBuf};

/// Main entry point for the Fireworks Simulator application.
fn main() -> Result<()> {
    env_logger::init();

    info!("🚀 Starting Fireworks Simulator...");

    show_rust_core_dependencies();

    // TODO: mettre en place un vrai gestionnaire de configurations (avec traits) !
    let physic_config = PhysicConfig::from_file("assets/config/physic.toml").unwrap_or_default();
    info!("Physic config loaded:\n{:#?}", physic_config);

    let audio_file_config = AudioConfig::from_file("assets/config/audio.toml").unwrap_or_default();
    info!("Audio config loaded:\n{:#?}", audio_file_config);

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
    let doppler_queue = DopplerQueue::new();
    // Paramètres audio par défaut
    let mut audio_config = audio_file_config.to_engine_config(physic_config.max_rockets);
    audio_config.doppler_receiver = Some(doppler_queue.receiver.clone());
    let audio_engine = FireworksAudio3D::new(audio_config)?;

    let window_width = 1024;
    let window_height = 800;

    // 1. Init Window & Context
    let window_engine = GlfwWindowEngine::init(window_width, window_height, "Fireworks Simulator")?;

    #[cfg(feature = "tracy")]
    {
        tracy_client::Client::start();
        log::info!("📊 Tracy + Fibers + OpenGL activés");
    }

    // 2. Init Renderer (now that GL context is ready)
    let renderer_engine = Renderer::new(window_width, window_height, &physic_config)?;

    let mut physic_engine = PhysicEngineFireworks::new(&physic_config, window_width as f32);
    physic_engine.set_doppler_sender(doppler_queue.sender.clone());

    // 3. Init Simulator
    info!("🚀 Starting Fireworks Simulator...");
    let mut simulator = Simulator::new(renderer_engine, physic_engine, audio_engine, window_engine);

    simulator.init_console_commands();
    let _ = simulator.run(
        export_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
    );
    simulator.close();

    Ok(())
}

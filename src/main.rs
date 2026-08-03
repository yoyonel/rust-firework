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

    // Auto-détection du dossier de travail : si "assets" n'existe pas en local mais existe chez le parent, on s'y déplace.
    if !std::path::Path::new("assets").exists() {
        if let Ok(current) = std::env::current_dir() {
            if let Some(parent) = current.parent() {
                if parent.join("assets").exists() {
                    let _ = std::env::set_current_dir(parent);
                }
            }
        }
    }

    let physic_path = fireworks_sim::utils::config_path::get_physic_config_path();
    let physic_config = PhysicConfig::from_file(&physic_path).unwrap_or_default();
    info!("Physic config loaded:\n{:#?}", physic_config);

    let audio_path = fireworks_sim::utils::config_path::get_audio_config_path();
    let audio_file_config = AudioConfig::from_file(&audio_path).unwrap_or_default();
    info!("Audio config loaded:\n{:#?}", audio_file_config);

    // --------------------------
    // Gestion du chemin d'export audio
    // --------------------------
    let export_path = std::env::args()
        .nth(1) // priorité à l'argument CLI
        .filter(|arg| !arg.starts_with('-')) // ne pas confondre les flags avec un chemin de fichier
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

    // =========================================================================
    // 🎙️ MODE PROFILING : HEADLESS AUDIO STRESS TEST (PERF + HOTSPOT)
    // =========================================================================
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--headless-audio-stress") {
        let duration_secs: u64 = args.get(pos + 1).and_then(|s| s.parse().ok()).unwrap_or(10); // 10 secondes par défaut

        info!(
            "🎧 [PERF MODE] Démarrage du stress-test audio pour {} secondes...",
            duration_secs
        );
        info!("   (Zéro rendu GLFW/OpenGL, saturation des voix actives)");

        use std::thread;
        use std::time::{Duration, Instant};

        // 1. Initialisation du moteur audio (avec le nombre max de fusées de la physique)
        let audio_config = audio_file_config.to_engine_config(physic_config.max_rockets);
        let mut audio_engine = FireworksAudio3D::new(audio_config)?;
        audio_engine.start_audio_thread(None);

        // 2. Boucle de stress-test dans le thread principal (simule 60 FPS de requêtes)
        let start_time = Instant::now();
        let dt = Duration::from_millis(16); // ~60 FPS
        let mut angle = 0.0_f32;

        while start_time.elapsed().as_secs() < duration_secs {
            angle += 0.05;

            // Simule des fusées en mouvement circulaire rapide autour de l'auditeur
            for i in 0..8 {
                let r = 50.0 + (i as f32 * 20.0);
                let a = angle + (i as f32 * std::f32::consts::FRAC_PI_4);
                let pos = glam::Vec2::new(a.cos() * r, a.sin() * r);

                // Exécute les méthodes réelles et disponibles sur votre moteur
                audio_engine.play_rocket(pos, 0.7);
                if i % 4 == 0 {
                    audio_engine.play_explosion(pos, 1.0);
                }
            }

            thread::sleep(dt);
        }

        audio_engine.stop_audio_thread();
        info!("🏁 [PERF MODE] Stress-test terminé avec succès.");
        return Ok(());
    }

    let mut audio_stress_sources = None;
    let mut randomize_stress_positions = false;
    if let Some(pos) = args.iter().position(|a| a == "--audio-stress-scene") {
        let num_sources: usize = args.get(pos + 1).and_then(|s| s.parse().ok()).unwrap_or(32);
        audio_stress_sources = Some(num_sources);
        randomize_stress_positions = args.iter().any(|a| a == "--randomize-stress-positions");
        info!(
            "🎧 [STRESS TEST SCENE] Starting interactive audio stress-test with {} virtual sources (randomize positions: {})...",
            num_sources,
            randomize_stress_positions
        );
    }

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

    if let Some(n) = audio_stress_sources {
        simulator.set_doppler_sender(doppler_queue.sender.clone());
        simulator.enable_audio_stress_scene(n, randomize_stress_positions);
    }

    simulator.init_console_commands();
    let _ = simulator.run(
        export_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
    );
    simulator.close();

    Ok(())
}

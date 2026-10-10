use anyhow::Result;
use fireworks_sim::audio_engine::audio_event::doppler_queue::DopplerQueue;
use fireworks_sim::audio_engine::config::AudioConfig;
use fireworks_sim::audio_engine::FireworksAudio3D;
use fireworks_sim::cli::CliArgs;
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::PhysicEngineFireworks;
use fireworks_sim::renderer_engine::renderer::Renderer;
use fireworks_sim::simulator::audio_stress_scene::run_headless_audio_stress;
use fireworks_sim::utils::show_rust_core_dependencies;
use fireworks_sim::window_engine::GlfwWindowEngine;
use fireworks_sim::{PhysicEngine, Simulator, WindowEngine};
use log::info;

/// Auto-détection du dossier de travail : si "assets" n'existe pas en local mais existe chez le parent, on s'y déplace.
fn ensure_assets_dir() {
    if !std::path::Path::new("assets").exists() {
        if let Ok(current) = std::env::current_dir() {
            if let Some(parent) = current.parent() {
                if parent.join("assets").exists() {
                    let _ = std::env::set_current_dir(parent);
                }
            }
        }
    }
}

/// Charge les configurations physique et audio depuis les fichiers de configuration.
fn load_configs() -> (PhysicConfig, AudioConfig) {
    let physic_path = fireworks_sim::utils::config_path::get_physic_config_path();
    let physic_config = PhysicConfig::from_file(&physic_path).unwrap_or_default();
    info!("Physic config loaded:\n{:#?}", physic_config);

    let audio_path = fireworks_sim::utils::config_path::get_audio_config_path();
    let audio_file_config = AudioConfig::from_file(&audio_path).unwrap_or_default();
    info!("Audio config loaded:\n{:#?}", audio_file_config);

    (physic_config, audio_file_config)
}

/// Initialise les 4 moteurs (Fenêtre, Rendu, Physique, Audio) et exécute la boucle de simulation.
fn run_simulator(
    cli: CliArgs,
    physic_config: PhysicConfig,
    audio_file_config: AudioConfig,
) -> Result<()> {
    let doppler_queue = DopplerQueue::new();
    let mut audio_config = audio_file_config.to_engine_config(physic_config.max_rockets);
    audio_config.doppler_receiver = Some(doppler_queue.receiver.clone());
    let audio_engine = FireworksAudio3D::new(audio_config)?;

    let window_width = 1024;
    let window_height = 800;

    let window_engine = GlfwWindowEngine::init(window_width, window_height, "Fireworks Simulator")?;

    #[cfg(feature = "tracy")]
    {
        tracy_client::Client::start();
        info!("📊 Tracy + Fibers + OpenGL activés");
    }

    let renderer_engine = Renderer::new(window_width, window_height, &physic_config)?;

    let mut physic_engine =
        PhysicEngineFireworks::new(&physic_config, window_width as f32, cli.deterministic_seed);
    physic_engine.set_doppler_sender(doppler_queue.sender.clone());

    info!("🚀 Starting Fireworks Simulator...");
    let mut simulator = Simulator::new(renderer_engine, physic_engine, audio_engine, window_engine);
    simulator.config = fireworks_sim::simulator::SimConfig {
        max_frames: cli.max_frames,
        fixed_dt: cli.fixed_dt,
        timeout_secs: cli.timeout_secs,
        disable_audio: cli.disable_audio,
    };

    if let Some(n) = cli.audio_stress_sources {
        simulator.set_doppler_sender(doppler_queue.sender.clone());
        simulator.enable_audio_stress_scene(n, cli.randomize_stress_positions);
    }

    simulator.init_console_commands();
    let _ = simulator.run(
        cli.export_path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
    );
    simulator.close();

    Ok(())
}

/// Point d'entrée principal de l'application Fireworks Simulator.
fn main() -> Result<()> {
    env_logger::init();
    info!("🚀 Starting Fireworks Simulator...");
    show_rust_core_dependencies();
    ensure_assets_dir();

    let cli = CliArgs::parse();
    cli.log_overrides();
    let (physic_config, audio_file_config) = load_configs();

    if let Some(duration) = cli.headless_audio_stress {
        return run_headless_audio_stress(duration, &audio_file_config, physic_config.max_rockets);
    }

    run_simulator(cli, physic_config, audio_file_config)
}

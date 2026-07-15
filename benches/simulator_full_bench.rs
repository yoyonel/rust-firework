use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use fireworks_sim::audio_engine::audio_event::doppler_queue::DopplerQueue;
use fireworks_sim::audio_engine::config::AudioConfig;
use fireworks_sim::audio_engine::FireworksAudio3D;
use fireworks_sim::physic_engine::config::PhysicConfig;
use fireworks_sim::physic_engine::physic_engine_generational_arena::{
    PhysicEngineFireworks, PhysicEngineTestHelpers,
};
use fireworks_sim::renderer_engine::renderer::Renderer;
use fireworks_sim::window_engine::{GlfwWindowEngine, WindowEngine};
use fireworks_sim::{PhysicEngine, Simulator};

fn bench_simulator_scaling(c: &mut Criterion) {
    // Activer le mode headless pour éviter d'ouvrir une fenêtre physique
    std::env::set_var("FIREWORKS_BENCH", "1");

    let audio_file_config = AudioConfig::default();

    let mut group = c.benchmark_group("simulator/frame_step_scaling");
    group.sample_size(30); // Réduire à 30 échantillons car les très hauts volumes prennent du temps

    // Tester des charges soutenues réalistes allant de faibles à extrêmes (jusqu'à 4000 fusées actives)
    for n_rockets in [10, 50, 200, 1000, 4000] {
        let mut physic_config = PhysicConfig::default();
        // Une fusée vit environ 3.0 secondes. Pour maintenir un état stationnaire de N fusées,
        // l'intervalle moyen d'apparition doit être d'environ 3.0 / N secondes.
        let interval = 3.0 / n_rockets as f32;
        physic_config.rocket_interval_mean = interval;
        physic_config.rocket_interval_variation = interval * 0.75;
        physic_config.rocket_max_next_interval = interval;

        // Limiter le nombre max de fusées dans l'arène au besoin
        physic_config.max_rockets = n_rockets * 2;

        // Initialisation des ressources
        let doppler_queue = DopplerQueue::new();
        let mut audio_config = audio_file_config.to_engine_config(physic_config.max_rockets);
        audio_config.doppler_receiver = Some(doppler_queue.receiver.clone());
        let mut audio_engine =
            FireworksAudio3D::new(audio_config).expect("Failed to create audio engine");

        audio_engine.start_audio_thread(None);

        let window_width = 1024;
        let window_height = 800;
        let window_engine =
            GlfwWindowEngine::init(window_width, window_height, "Fireworks Benchmark")
                .expect("Failed to init window");
        let renderer_engine = Renderer::new(window_width, window_height, &physic_config)
            .expect("Failed to init renderer");
        let mut physic_engine = PhysicEngineFireworks::new(&physic_config, window_width as f32);
        physic_engine.set_doppler_sender(doppler_queue.sender.clone());

        // Pré-générer une charge de travail initiale stable (n_rockets fusées actives)
        for _ in 0..n_rockets {
            physic_engine.force_next_launch();
            physic_engine.update(0.016);
        }

        let mut simulator =
            Simulator::new(renderer_engine, physic_engine, audio_engine, window_engine);
        simulator.init_console_commands();

        group.bench_with_input(
            BenchmarkId::from_parameter(n_rockets),
            &n_rockets,
            |b, _| {
                b.iter(|| {
                    let res = simulator.step();
                    black_box(res);
                });
            },
        );

        // Nettoyage après benchmark de cette configuration
        simulator.close();
    }

    group.finish();
}

criterion_group!(benches, bench_simulator_scaling);
criterion_main!(benches);

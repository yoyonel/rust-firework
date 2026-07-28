use fireworks_sim::audio_engine::audio_event::doppler_queue::DopplerQueue;
use fireworks_sim::audio_engine::effect_flags::AudioEffect;
use fireworks_sim::audio_engine::types::{AudioDebugEvent, AudioSoundType};
use fireworks_sim::audio_engine::FireworksAudio3D;
use fireworks_sim::AudioEngine;
use rand::Rng;
use std::thread;
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    println!("🧪 [STRESS PROFILE 128] Starting headless profiler with 128 sources...");

    let doppler_queue = DopplerQueue::new();
    let mut config =
        fireworks_sim::audio_engine::config::AudioConfig::default().to_engine_config(128);
    config.max_voices = 128;
    config.doppler_receiver = Some(doppler_queue.receiver.clone());

    let mut audio_engine = FireworksAudio3D::new(config)?;
    audio_engine.start_audio_thread(None);

    // Enforce target effects
    audio_engine.set_effect_enabled(AudioEffect::SpatialBus, true);
    audio_engine.set_effect_enabled(AudioEffect::HrtfBus, true);
    audio_engine.set_effect_enabled(AudioEffect::SpatialReverb, true);
    audio_engine.set_listener_position(glam::Vec2::new(512.0, 384.0));

    println!("DSP Status: {}", audio_engine.get_effects_status());

    let num_sources = 128;
    struct Source {
        id: u64,
        angle: f32,
        angular_speed: f32,
        radius: f32,
        target_radius: f32,
        radius_speed: f32,
        pos: glam::Vec2,
        sound_type: AudioSoundType,
        active_request_id: Option<u64>,
    }

    let center = glam::Vec2::new(512.0, 384.0);
    let max_r = 300.0;

    let mut sources = Vec::new();
    let mut rng = rand::rng();
    for i in 0..num_sources {
        let angle = rng.random::<f32>() * 2.0 * std::f32::consts::PI;
        let dir = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
        let angular_speed = (rng.random::<f32>() * 0.65 + 0.15) * dir;
        let radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
        let target_radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
        let speed = rng.random::<f32>() * 50.0 + 15.0;
        let radius_speed = if target_radius > radius {
            speed
        } else {
            -speed
        };

        let pos = center + glam::Vec2::new(radius * angle.cos(), radius * angle.sin());
        sources.push(Source {
            id: (i + 1) as u64,
            angle,
            angular_speed,
            radius,
            target_radius,
            radius_speed,
            pos,
            sound_type: AudioSoundType::Rocket,
            active_request_id: None,
        });
    }

    // Bootstrap
    for s in &sources {
        audio_engine.play_rocket_with_id(s.id, s.pos, 0.7);
    }

    let start_time = Instant::now();
    let duration = Duration::from_secs(5);
    let dt = 0.016_f32;

    let mut total_blocks = 0u64;
    let mut underrun_blocks = 0u64;
    let mut sum_elapsed_us = 0u64;
    let mut max_elapsed_us = 0u64;
    let mut budget_us = 5333u64;
    let mut active_voices = 0;

    let mut debug_events_buf = Vec::new();

    while start_time.elapsed() < duration {
        let frame_start = Instant::now();

        // Move sources
        let mut rng = rand::rng();
        for source in &mut sources {
            let to_target = source.target_radius - source.radius;
            if to_target.abs() < 5.0 {
                source.target_radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
                let speed = rng.random::<f32>() * 50.0 + 15.0;
                source.radius_speed = if source.target_radius > source.radius {
                    speed
                } else {
                    -speed
                };
            } else {
                source.radius += source.radius_speed * dt;
            }

            source.angle += source.angular_speed * dt;
            source.pos = center
                + glam::Vec2::new(
                    source.radius * source.angle.cos(),
                    source.radius * source.angle.sin(),
                );

            // Velocity (derivative)
            let vx = source.radius_speed * source.angle.cos()
                - source.radius * source.angular_speed * source.angle.sin();
            let vy = source.radius_speed * source.angle.sin()
                + source.radius * source.angular_speed * source.angle.cos();
            let vel = glam::Vec2::new(vx, vy);

            let _ = doppler_queue
                .sender
                .send(fireworks_sim::audio_engine::DopplerEvent {
                    id: source.id,
                    pos: source.pos,
                    vel,
                    gain: 1.0,
                    timestamp: Instant::now(),
                });
        }

        // Process debug events
        debug_events_buf.clear();
        audio_engine.pop_debug_events(&mut debug_events_buf);
        for event in &debug_events_buf {
            match event {
                AudioDebugEvent::Sent {
                    request_id,
                    entity_id,
                    ..
                } => {
                    if *entity_id > 0 && *entity_id <= num_sources as u64 {
                        sources[(*entity_id - 1) as usize].active_request_id = Some(*request_id);
                    }
                }
                AudioDebugEvent::Completed { request_id, .. }
                | AudioDebugEvent::Dropped { request_id, .. } => {
                    if let Some(source) = sources
                        .iter_mut()
                        .find(|s| s.active_request_id == Some(*request_id))
                    {
                        source.active_request_id = None;
                        match source.sound_type {
                            AudioSoundType::Rocket => {
                                source.sound_type = AudioSoundType::Explosion;
                                audio_engine.play_explosion_with_id(source.id, source.pos, 1.0);
                            }
                            AudioSoundType::Explosion => {
                                source.sound_type = AudioSoundType::Rocket;
                                source.angle = rng.random::<f32>() * 2.0 * std::f32::consts::PI;
                                let dir = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                                source.angular_speed = (rng.random::<f32>() * 0.65 + 0.15) * dir;
                                source.radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
                                source.target_radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
                                let speed = rng.random::<f32>() * 50.0 + 15.0;
                                source.radius_speed = if source.target_radius > source.radius {
                                    speed
                                } else {
                                    -speed
                                };
                                source.pos = center
                                    + glam::Vec2::new(
                                        source.radius * source.angle.cos(),
                                        source.radius * source.angle.sin(),
                                    );
                                audio_engine.play_rocket_with_id(source.id, source.pos, 0.7);
                            }
                        }
                    }
                }
                AudioDebugEvent::BlockProcessed {
                    elapsed_us,
                    budget_us: b_us,
                    active_voices: av,
                } => {
                    total_blocks += 1;
                    sum_elapsed_us += *elapsed_us;
                    max_elapsed_us = max_elapsed_us.max(*elapsed_us);
                    budget_us = *b_us;
                    active_voices = *av;
                }
                AudioDebugEvent::Underrun { .. } => {
                    underrun_blocks += 1;
                }
                _ => {}
            }
        }

        let elapsed_frame = frame_start.elapsed();
        let target_frame = Duration::from_millis(16);
        if elapsed_frame < target_frame {
            thread::sleep(target_frame - elapsed_frame);
        }
    }

    audio_engine.stop_audio_thread();

    let avg_us = sum_elapsed_us.checked_div(total_blocks).unwrap_or(0);
    let load_pct = (avg_us as f64 / budget_us as f64) * 100.0;
    println!("\n📊 [STRESS PROFILE RESULTS]");
    println!("  - Active Voices: {}", active_voices);
    println!("  - Block Budget: {} us", budget_us);
    println!("  - CPU Render Avg: {} us ({:.2}%)", avg_us, load_pct);
    println!("  - CPU Render Max: {} us", max_elapsed_us);
    println!(
        "  - ALSA Underruns (CPU budget overflows): {} / {} blocks ({:.2}%)",
        underrun_blocks,
        total_blocks,
        (underrun_blocks as f64 / total_blocks as f64) * 100.0
    );

    Ok(())
}

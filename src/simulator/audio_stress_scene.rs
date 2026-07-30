use crate::audio_engine::{
    types::{AudioDebugEvent, AudioSoundType},
    AudioEngine, DopplerEvent,
};
use crate::renderer_engine::{CircleGPUData, CircleGPURenderer};
use crossbeam_channel::Sender;
use glam::Vec2;
use rand::Rng;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct VirtualSource {
    pub id: u64,
    pub pos: Vec2,
    pub angle: f32,
    pub angular_speed: f32,
    pub radius: f32,
    pub target_radius: f32,
    pub radius_speed: f32,
    pub sound_type: AudioSoundType,
    pub active_request_id: Option<u64>,
    pub initial_angle: f32,
    pub initial_angular_speed: f32,
    pub initial_radius: f32,
    pub initial_target_radius: f32,
    pub initial_radius_speed: f32,
}

pub struct AudioStressScene {
    pub enabled: bool,
    pub num_sources: usize,
    pub randomize_positions: bool,
    pub sources: Vec<VirtualSource>,
    pub doppler_sender: Option<Sender<DopplerEvent>>,
    pub avg_render_us: u64,
    pub max_render_us: u64,
    pub budget_us: u64,
    pub underruns: u64,
    pub total_blocks: u64,
    pub active_voices: usize,
    pub last_report: Instant,
    pub last_doppler: Instant,
    pub circle_renderer: Option<CircleGPURenderer>,
}

impl AudioStressScene {
    pub fn new() -> Self {
        Self {
            enabled: false,
            num_sources: 0,
            randomize_positions: false,
            sources: Vec::new(),
            doppler_sender: None,
            avg_render_us: 0,
            max_render_us: 0,
            budget_us: 5333,
            underruns: 0,
            total_blocks: 0,
            active_voices: 0,
            last_report: Instant::now(),
            last_doppler: Instant::now(),
            circle_renderer: None,
        }
    }

    pub fn set_doppler_sender(&mut self, sender: Sender<DopplerEvent>) {
        self.doppler_sender = Some(sender);
    }

    pub fn enable(
        &mut self,
        num_sources: usize,
        randomize_positions: bool,
        window_size_f32: (f32, f32),
        audio_engine: &mut impl AudioEngine,
    ) {
        self.enabled = true;
        self.num_sources = num_sources;
        self.randomize_positions = randomize_positions;

        if self.circle_renderer.is_none() && gl::GenVertexArrays::is_loaded() {
            self.circle_renderer = Some(CircleGPURenderer::new());
        }

        let center = Vec2::new(window_size_f32.0 * 0.5, window_size_f32.1 * 0.5);
        let max_r = (window_size_f32.0.min(window_size_f32.1) * 0.4).max(100.0);

        audio_engine.set_listener_position(center);

        // Activer par défaut le Spatial Bus, Hrtf Bus et Spatial Reverb
        use crate::audio_engine::effect_flags::AudioEffect;
        audio_engine.set_effect_enabled(AudioEffect::SpatialBus, true);
        audio_engine.set_effect_enabled(AudioEffect::HrtfBus, true);
        audio_engine.set_effect_enabled(AudioEffect::SpatialReverb, true);

        let mut sources = Vec::with_capacity(num_sources);
        let mut rng = rand::rng();
        for i in 0..num_sources {
            let angle = rng.random::<f32>() * 2.0 * std::f32::consts::PI;
            let dir = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
            let angular_speed = (rng.random::<f32>() * 0.65 + 0.15) * dir;

            let radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
            let target_radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
            let speed = rng.random::<f32>() * 50.0 + 15.0; // 15 to 65 px/s
            let radius_speed = if target_radius > radius {
                speed
            } else {
                -speed
            };

            let pos = center + Vec2::new(radius * angle.cos(), radius * angle.sin());

            sources.push(VirtualSource {
                id: (i + 1) as u64,
                pos,
                angle,
                angular_speed,
                radius,
                target_radius,
                radius_speed,
                sound_type: AudioSoundType::Rocket,
                active_request_id: None,
                initial_angle: angle,
                initial_angular_speed: angular_speed,
                initial_radius: radius,
                initial_target_radius: target_radius,
                initial_radius_speed: radius_speed,
            });
        }

        self.sources = sources;

        // Bootstrap sounds
        for source in &self.sources {
            audio_engine.play_rocket_with_id(source.id, source.pos, 0.7);
        }
    }

    pub fn update(
        &mut self,
        delta: f32,
        window_size_f32: (f32, f32),
        audio_engine: &mut impl AudioEngine,
        audio_events_buf: &mut Vec<AudioDebugEvent>,
    ) {
        let center = Vec2::new(window_size_f32.0 * 0.5, window_size_f32.1 * 0.5);
        let max_r = (window_size_f32.0.min(window_size_f32.1) * 0.4).max(100.0);

        let mut rng = rand::rng();

        // 1. Move virtual sources
        for source in &mut self.sources {
            let to_target = source.target_radius - source.radius;
            if to_target.abs() < 5.0 {
                if self.randomize_positions {
                    source.target_radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
                    let speed = rng.random::<f32>() * 50.0 + 15.0;
                    source.radius_speed = if source.target_radius > source.radius {
                        speed
                    } else {
                        -speed
                    };
                } else {
                    // Oscillate deterministically between initial_radius and initial_target_radius
                    if (source.target_radius - source.initial_target_radius).abs() < 1e-3 {
                        source.target_radius = source.initial_radius;
                    } else {
                        source.target_radius = source.initial_target_radius;
                    }
                    let speed = source.initial_radius_speed.abs();
                    source.radius_speed = if source.target_radius > source.radius {
                        speed
                    } else {
                        -speed
                    };
                }
            } else {
                source.radius += source.radius_speed * delta;
            }

            source.angle += source.angular_speed * delta;
            source.pos = center
                + Vec2::new(
                    source.radius * source.angle.cos(),
                    source.radius * source.angle.sin(),
                );
        }

        // Send DopplerEvents at 144 Hz max AND only for sources actively playing in the audio engine
        let send_doppler =
            self.last_doppler.elapsed() >= std::time::Duration::from_secs_f64(1.0 / 144.0);
        if send_doppler {
            self.last_doppler = Instant::now();
            for source in &self.sources {
                if source.active_request_id.is_some() {
                    // Velocity (derivative)
                    let vx = source.radius_speed * source.angle.cos()
                        - source.radius * source.angular_speed * source.angle.sin();
                    let vy = source.radius_speed * source.angle.sin()
                        + source.radius * source.angular_speed * source.angle.cos();
                    let vel = Vec2::new(vx, vy);

                    if let Some(tx) = &self.doppler_sender {
                        let _ = tx.send(DopplerEvent {
                            id: source.id,
                            pos: source.pos,
                            vel,
                            gain: 1.0,
                            timestamp: Instant::now(),
                        });
                    }
                }
            }
        }

        // 2. Process debug events for state machine and stats
        audio_events_buf.clear();
        audio_engine.pop_debug_events(audio_events_buf);

        let mut block_count = 0u64;
        let mut underrun_count = 0u64;
        let mut sum_us = 0u64;
        let mut max_us = 0u64;
        let mut av_voices = 0;
        let mut budget = self.budget_us;

        for event in audio_events_buf {
            match event {
                AudioDebugEvent::Sent {
                    request_id,
                    entity_id,
                    ..
                } => {
                    if *entity_id > 0 && *entity_id <= self.num_sources as u64 {
                        self.sources[(*entity_id - 1) as usize].active_request_id =
                            Some(*request_id);
                    }
                }
                AudioDebugEvent::Completed { request_id, .. }
                | AudioDebugEvent::Dropped { request_id, .. } => {
                    if let Some(source) = self
                        .sources
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
                                if self.randomize_positions {
                                    source.angle = rng.random::<f32>() * 2.0 * std::f32::consts::PI;
                                    let dir = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
                                    source.angular_speed =
                                        (rng.random::<f32>() * 0.65 + 0.15) * dir;
                                    source.radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
                                    source.target_radius =
                                        rng.random::<f32>() * (max_r - 80.0) + 80.0;
                                    let speed = rng.random::<f32>() * 50.0 + 15.0;
                                    source.radius_speed = if source.target_radius > source.radius {
                                        speed
                                    } else {
                                        -speed
                                    };
                                    source.pos = center
                                        + Vec2::new(
                                            source.radius * source.angle.cos(),
                                            source.radius * source.angle.sin(),
                                        );
                                }

                                source.pos = center
                                    + Vec2::new(
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
                    budget_us,
                    active_voices,
                } => {
                    block_count += 1;
                    sum_us += *elapsed_us;
                    max_us = max_us.max(*elapsed_us);
                    budget = *budget_us;
                    av_voices = *active_voices;
                }
                AudioDebugEvent::Underrun { .. } => {
                    underrun_count += 1;
                }
                _ => {}
            }
        }

        // Accumulate statistics
        if block_count > 0 {
            self.total_blocks += block_count;
            self.underruns += underrun_count;
            self.avg_render_us = (self.avg_render_us * 9 + sum_us / block_count) / 10;
            self.max_render_us = self.max_render_us.max(max_us);
            self.budget_us = budget;
            self.active_voices = av_voices;
        }

        // Reset max occasionally (every 2 seconds) to keep it fresh
        if self.last_report.elapsed() >= std::time::Duration::from_secs(2) {
            self.max_render_us = max_us;
            self.last_report = Instant::now();
        }
    }

    pub fn draw(&mut self, window_size_f32: (f32, f32), audio_engine: &impl AudioEngine) {
        if let Some(renderer) = &mut self.circle_renderer {
            let center = Vec2::new(window_size_f32.0 * 0.5, window_size_f32.1 * 0.5);

            let mut orbits = Vec::with_capacity(self.sources.len() + 3);
            let mut discs = Vec::with_capacity(3 + self.sources.len() * 2);

            // 1. Virtual sound sphere (drawn as outline)
            let max_dist = audio_engine.get_max_distance();
            orbits.push(CircleGPUData {
                center: [center.x, center.y],
                radius: max_dist,
                color: [0.0, 0.8, 1.0, 0.15],
                thickness: 0.0,
            });

            // 2. Listener outer ring (outline)
            orbits.push(CircleGPUData {
                center: [center.x, center.y],
                radius: 12.0,
                color: [0.0, 1.0, 0.0, 0.8],
                thickness: 0.0,
            });

            // 3. Listener center dot (filled)
            discs.push(CircleGPUData {
                center: [center.x, center.y],
                radius: 4.0,
                color: [0.0, 1.0, 0.0, 1.0],
                thickness: 0.0,
            });

            // 4. Source orbits (drawn as extremely cheap LINE_LOOP outlines)
            for source in &self.sources {
                orbits.push(CircleGPUData {
                    center: [center.x, center.y],
                    radius: source.radius,
                    color: [1.0, 1.0, 1.0, 0.04],
                    thickness: 0.0,
                });

                // 5. Source representations (discs / rings)
                match source.sound_type {
                    AudioSoundType::Rocket => {
                        discs.push(CircleGPUData {
                            center: [source.pos.x, source.pos.y],
                            radius: 5.0,
                            color: [1.0, 0.9, 0.0, 0.8],
                            thickness: 0.0,
                        });
                    }
                    AudioSoundType::Explosion => {
                        // Filled red inner dot
                        discs.push(CircleGPUData {
                            center: [source.pos.x, source.pos.y],
                            radius: 8.0,
                            color: [1.0, 0.2, 0.0, 0.9],
                            thickness: 0.0,
                        });
                        // Wireframe orange outer ring (drawn as outline)
                        orbits.push(CircleGPUData {
                            center: [source.pos.x, source.pos.y],
                            radius: 20.0,
                            color: [1.0, 0.4, 0.0, 0.3],
                            thickness: 0.0,
                        });
                    }
                }
            }

            unsafe {
                renderer.draw(&orbits, &discs);
            }
        }
    }

    pub fn build_imgui_window(&mut self, ui: &imgui::Ui, audio_engine: &impl AudioEngine) {
        // 1. Draw virtual sources on the background draw list
        let draw_list = ui.get_background_draw_list();
        let window_width = ui.io().display_size[0];
        let window_height = ui.io().display_size[1];

        let center_x = window_width * 0.5;
        let center_y = window_height * 0.5;

        let label = "🎧 Listener (Centre)";
        let text_size = ui.calc_text_size(label);
        draw_list.add_text(
            [center_x - text_size[0] * 0.5, center_y - 35.0],
            [0.0, 1.0, 0.0, 1.0],
            label,
        );

        // 2. Custom ImGui window for stress stats & controls
        ui.window("Audio Stress Scene Monitor")
            .size(
                [window_width * 0.45, window_height * 0.50],
                imgui::Condition::FirstUseEver,
            )
            .position(
                [window_width * 0.53, window_height * 0.45],
                imgui::Condition::FirstUseEver,
            )
            .resizable(true)
            .collapsible(true)
            .build(|| {
                ui.text("=== INTERACTIVE AUDIO STRESS PROFILE ===");
                ui.separator();

                ui.text(format!(
                    "Virtual Sources: {}  |  Active DSP Voices: {}",
                    self.num_sources, self.active_voices
                ));

                let load_percent = (self.avg_render_us as f64 / self.budget_us as f64) * 100.0;
                ui.text(format!(
                    "Block Budget: {} us  |  CPU Render Avg: {} us ({:.1}%)",
                    self.budget_us, self.avg_render_us, load_percent
                ));

                ui.text(format!("CPU Render Max: {} us", self.max_render_us));

                let underrun_ratio = if self.total_blocks > 0 {
                    (self.underruns as f64 / self.total_blocks as f64) * 100.0
                } else {
                    0.0
                };

                if self.underruns > 0 {
                    ui.text_colored(
                        [1.0, 0.2, 0.2, 1.0],
                        format!(
                            "⚠️ ALSA UNDERRUNS (CPU OVERFLOWS): {} / {} blocks ({:.2}%)",
                            self.underruns, self.total_blocks, underrun_ratio
                        ),
                    );
                } else {
                    ui.text_colored(
                        [0.0, 1.0, 0.0, 1.0],
                        "   ALSA Underruns: None (Healthy DSP pipeline)",
                    );
                }

                ui.separator();
                ui.text("=== INTERACTIVE DSP EFFECTS CONTROL ===");

                // Toggle Buttons for Effects
                use crate::audio_engine::effect_flags::AudioEffect;

                fn toggle_effect_btn<A: AudioEngine>(
                    ui: &imgui::Ui,
                    engine: &A,
                    effect: AudioEffect,
                    name: &str,
                ) {
                    let current = engine.get_effect_enabled(effect);
                    let label = if current {
                        format!("Disable {}", name)
                    } else {
                        format!("Enable {}", name)
                    };

                    if ui.button(label) {
                        engine.set_effect_enabled(effect, !current);
                    }
                }

                toggle_effect_btn(
                    ui,
                    audio_engine,
                    AudioEffect::HrtfBus,
                    "HRTF (Binaural Bus)",
                );
                ui.same_line();
                toggle_effect_btn(ui, audio_engine, AudioEffect::SpatialBus, "Spatial Bus");

                toggle_effect_btn(
                    ui,
                    audio_engine,
                    AudioEffect::SpatialReverb,
                    "Spatial Reverb",
                );
                ui.same_line();
                toggle_effect_btn(ui, audio_engine, AudioEffect::Doppler, "Doppler Effect");

                toggle_effect_btn(
                    ui,
                    audio_engine,
                    AudioEffect::Binaural,
                    "Legacy Direct Binaural",
                );
                ui.same_line();
                toggle_effect_btn(
                    ui,
                    audio_engine,
                    AudioEffect::Panning,
                    "Legacy Direct Panning",
                );

                ui.separator();
                ui.text("Current DSP Status flags:");
                ui.text_wrapped(audio_engine.get_effects_status());
            });
    }
}

impl Default for AudioStressScene {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_source_determinism() {
        let center = Vec2::new(512.0, 400.0);
        let max_r = 400.0;

        let mut source = VirtualSource {
            id: 1,
            pos: center + Vec2::new(100.0, 0.0),
            angle: 0.0,
            angular_speed: 0.5,
            radius: 100.0,
            target_radius: 120.0,
            radius_speed: 10.0,
            sound_type: AudioSoundType::Rocket,
            active_request_id: None,
            initial_angle: 0.0,
            initial_angular_speed: 0.5,
            initial_radius: 100.0,
            initial_target_radius: 120.0,
            initial_radius_speed: 10.0,
        };

        let mut rng = rand::rng();

        // Test 1: Deterministic oscillation instead of random target_radius change when randomize is false
        let mut source_rand = source.clone();

        // Setup close to target radius so it triggers the target radius change condition
        source.radius = 119.9;
        let to_target = source.target_radius - source.radius;
        assert!(to_target.abs() < 5.0);

        // Run the update logic for deterministic case
        {
            let to_target = source.target_radius - source.radius;
            if to_target.abs() < 5.0 {
                // Should oscillate to initial_radius
                if (source.target_radius - source.initial_target_radius).abs() < 1e-3 {
                    source.target_radius = source.initial_radius;
                } else {
                    source.target_radius = source.initial_target_radius;
                }
                let speed = source.initial_radius_speed.abs();
                source.radius_speed = if source.target_radius > source.radius {
                    speed
                } else {
                    -speed
                };
            }
        }

        assert_eq!(source.target_radius, 100.0);
        assert_eq!(source.radius_speed, -10.0); // moving backwards

        // Test 2: Random target_radius change when randomize is true
        source_rand.radius = 119.9;
        {
            let to_target = source_rand.target_radius - source_rand.radius;
            if to_target.abs() < 5.0 {
                source_rand.target_radius = rng.random::<f32>() * (max_r - 80.0) + 80.0;
            }
        }
        // It should have randomized target_radius instead of oscillating to 100.0
        assert_ne!(source_rand.target_radius, 100.0);
    }

    #[test]
    fn test_audio_stress_scene_lifecycle() {
        let mut scene = AudioStressScene::new();
        assert!(!scene.enabled);
        assert_eq!(scene.num_sources, 0);

        let (tx, rx) = crossbeam_channel::unbounded();
        scene.set_doppler_sender(tx);
        assert!(scene.doppler_sender.is_some());
        drop(rx);
    }
}

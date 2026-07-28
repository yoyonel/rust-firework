use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::renderer_engine::RendererEngine;
use crate::utils::Fullscreen;
use crate::window_engine::WindowEngine;
use crate::Simulator;
use glfw::{Action, Key, WindowMode};
use log::info;
use std::time::Instant;

impl<R, P, A, W> Simulator<R, P, A, W>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
    W: WindowEngine,
{
    pub(crate) fn process_audio_debug_events(&mut self) {
        self.audio_events_buf.clear();
        self.audio_engine
            .pop_debug_events(&mut self.audio_events_buf);

        let mut events = std::mem::take(&mut self.audio_events_buf);
        for evt in events.drain(..) {
            match evt {
                crate::audio_engine::types::AudioDebugEvent::Sent {
                    request_id,
                    sound_type,
                    entity_id,
                    sent_at,
                } => {
                    self.audio_debug_records.push_back(
                        crate::audio_engine::types::AudioDebugRecord {
                            request_id,
                            sound_type,
                            entity_id,
                            sent_at,
                            received_at: None,
                            started_at: None,
                            dropped_at: None,
                            completed_at: None,
                            status: crate::audio_engine::types::AudioPlayStatus::Sent,
                            voice_index: None,
                            drop_reason: None,
                        },
                    );
                    if self.audio_debug_records.len() > 100 {
                        self.audio_debug_records.pop_front();
                    }
                    match sound_type {
                        crate::audio_engine::types::AudioSoundType::Rocket => {
                            self.audio_sent_rocket += 1;
                        }
                        crate::audio_engine::types::AudioSoundType::Explosion => {
                            self.audio_sent_explosion += 1;
                        }
                    }
                }
                crate::audio_engine::types::AudioDebugEvent::Received {
                    request_id,
                    received_at,
                } => {
                    if let Some(rec) = self
                        .audio_debug_records
                        .iter_mut()
                        .find(|r| r.request_id == request_id)
                    {
                        rec.received_at = Some(received_at);
                        rec.status = crate::audio_engine::types::AudioPlayStatus::Received;
                        let latency = received_at.duration_since(rec.sent_at);
                        self.latency_dispatch_sum += latency;
                        self.latency_dispatch_count += 1;
                        match rec.sound_type {
                            crate::audio_engine::types::AudioSoundType::Rocket => {
                                self.audio_received_rocket += 1;
                            }
                            crate::audio_engine::types::AudioSoundType::Explosion => {
                                self.audio_received_explosion += 1;
                            }
                        }
                    }
                }
                crate::audio_engine::types::AudioDebugEvent::Started {
                    request_id,
                    started_at,
                    voice_index,
                } => {
                    if let Some(rec) = self
                        .audio_debug_records
                        .iter_mut()
                        .find(|r| r.request_id == request_id)
                    {
                        rec.started_at = Some(started_at);
                        rec.voice_index = Some(voice_index);
                        rec.status = crate::audio_engine::types::AudioPlayStatus::Playing;
                        let latency = started_at.duration_since(rec.sent_at);
                        self.latency_play_sum += latency;
                        self.latency_play_count += 1;
                        match rec.sound_type {
                            crate::audio_engine::types::AudioSoundType::Rocket => {
                                self.audio_played_rocket += 1;
                                // Sync metrics calculation for Rocket launch
                                if let Some(spawn_time) =
                                    self.phys_launch_times.remove(&rec.entity_id)
                                {
                                    let diff_ms = if started_at >= spawn_time {
                                        started_at.duration_since(spawn_time).as_secs_f32() * 1000.0
                                    } else {
                                        spawn_time.duration_since(started_at).as_secs_f32()
                                            * -1000.0
                                    };
                                    self.sync_launch_sum += diff_ms as f64;
                                    self.sync_launch_count += 1;
                                    self.profiler.record_metric("sync_launch_ms", diff_ms);

                                    // Ajustement dynamique
                                    self.adjust_launch_anticipation_ms(diff_ms);
                                } else {
                                    self.audio_start_launch_times
                                        .insert(rec.entity_id, started_at);
                                }
                            }
                            crate::audio_engine::types::AudioSoundType::Explosion => {
                                self.audio_played_explosion += 1;
                                // Sync metrics calculation for Explosion
                                if let Some(explode_time) =
                                    self.phys_explosion_times.remove(&rec.entity_id)
                                {
                                    let diff_ms = if started_at >= explode_time {
                                        started_at.duration_since(explode_time).as_secs_f32()
                                            * 1000.0
                                    } else {
                                        explode_time.duration_since(started_at).as_secs_f32()
                                            * -1000.0
                                    };
                                    self.sync_explosion_sum += diff_ms as f64;
                                    self.sync_explosion_count += 1;
                                    self.profiler.record_metric("sync_explosion_ms", diff_ms);

                                    // Ajustement dynamique
                                    self.adjust_explosion_anticipation_ms(diff_ms);
                                } else {
                                    self.audio_start_explosion_times
                                        .insert(rec.entity_id, started_at);
                                }
                            }
                        }
                    }
                }
                crate::audio_engine::types::AudioDebugEvent::Dropped {
                    request_id,
                    dropped_at,
                    reason,
                } => {
                    if let Some(rec) = self
                        .audio_debug_records
                        .iter_mut()
                        .find(|r| r.request_id == request_id)
                    {
                        rec.dropped_at = Some(dropped_at);
                        rec.drop_reason = Some(reason);
                        rec.status = crate::audio_engine::types::AudioPlayStatus::Dropped;

                        // Avoid memory leaks in tracking tables if sound is dropped
                        self.phys_launch_times.remove(&rec.entity_id);
                        self.phys_explosion_times.remove(&rec.entity_id);
                        self.audio_start_launch_times.remove(&rec.entity_id);
                        self.audio_start_explosion_times.remove(&rec.entity_id);

                        match rec.sound_type {
                            crate::audio_engine::types::AudioSoundType::Rocket => {
                                self.audio_dropped_rocket += 1;
                            }
                            crate::audio_engine::types::AudioSoundType::Explosion => {
                                self.audio_dropped_explosion += 1;
                            }
                        }
                        log::warn!(
                            "⚠️ AUDIO DROPPED: request #{} ({:?}) for entity {} was dropped: {}",
                            request_id,
                            rec.sound_type,
                            rec.entity_id,
                            reason
                        );
                    }
                }
                crate::audio_engine::types::AudioDebugEvent::Completed {
                    request_id,
                    completed_at,
                } => {
                    if let Some(rec) = self
                        .audio_debug_records
                        .iter_mut()
                        .find(|r| r.request_id == request_id)
                    {
                        rec.completed_at = Some(completed_at);
                        rec.status = crate::audio_engine::types::AudioPlayStatus::Completed;
                        match rec.sound_type {
                            crate::audio_engine::types::AudioSoundType::Rocket => {
                                self.audio_completed_rocket += 1;
                            }
                            crate::audio_engine::types::AudioSoundType::Explosion => {
                                self.audio_completed_explosion += 1;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        self.audio_events_buf = events;
    }

    pub(crate) fn handle_window_events(&mut self) -> (bool, bool) {
        let mut reload_config = false;
        let mut reload_shaders = false;

        self.window_engine.poll_events();
        let events: Vec<_> = glfw::flush_messages(self.window_engine.get_events()).collect();

        for (_, event) in events {
            match event {
                glfw::WindowEvent::FramebufferSize(w, h) => self.handle_resize(w, h),
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    self.window_engine.set_should_close(true);
                }
                glfw::WindowEvent::Key(Key::R, _, Action::Press, _) if !self.console.open => {
                    reload_config = true;
                }
                glfw::WindowEvent::Key(Key::S, _, Action::Press, _) if !self.console.open => {
                    reload_shaders = true;
                }
                glfw::WindowEvent::Key(Key::F11, _, Action::Press, _) => {
                    self.toggle_fullscreen();
                }
                glfw::WindowEvent::Key(Key::F3, _, Action::Press, _) => {
                    self.show_audio_diagnostic = !self.show_audio_diagnostic;
                    self.update_cursor_mode();
                }
                glfw::WindowEvent::Key(Key::GraveAccent, _, Action::Press, _) => {
                    self.toggle_console();
                }
                _ => {}
            }

            // ImGui Input Handling
            let is_key_event = matches!(
                event,
                glfw::WindowEvent::Key(_, _, _, _) | glfw::WindowEvent::Char(_)
            );

            if self.console.open || self.show_audio_diagnostic || !is_key_event {
                let imgui_system = self.window_engine.get_imgui_system_mut();
                imgui_system
                    .glfw
                    .handle_event(&mut imgui_system.context, &event);
            }
        }

        (reload_config, reload_shaders)
    }

    fn handle_resize(&mut self, w: i32, h: i32) {
        self.renderer_engine.set_window_size(w, h);
        self.window_size_f32 = (w as f32, h as f32);
        self.physic_engine.set_window_width(w as f32);
        let listener_pos = if self.audio_stress_scene.enabled {
            glam::Vec2::new(w as f32 / 2.0, h as f32 / 2.0)
        } else {
            glam::Vec2::new(w as f32 / 2.0, 0.0)
        };
        self.audio_engine.set_listener_position(listener_pos);
    }

    fn toggle_fullscreen(&mut self) {
        if self.window_engine.is_fullscreen() {
            self.window_engine.set_monitor(
                WindowMode::Windowed,
                self.window_last_pos.0,
                self.window_last_pos.1,
                self.window_last_size.0 as u32,
                self.window_last_size.1 as u32,
                None,
            );
            self.window_size = self.window_last_size;
            self.window_size_f32 = (
                self.window_last_size.0 as f32,
                self.window_last_size.1 as f32,
            );
            info!(
                "🖥️ Window resized: {} x {}",
                self.window_size.0, self.window_size.1
            );
        } else {
            self.window_last_pos = self.window_engine.get_pos();
            self.window_last_size = self.window_engine.get_size();

            let mut glfw = self.window_engine.get_glfw().clone();
            let window = self.window_engine.get_window_mut();
            glfw.with_primary_monitor(|_, primary_monitor| {
                if let Some(mon) = primary_monitor {
                    if let Some(video_mode) = mon.get_video_mode() {
                        window.set_fullscreen(mon);
                        self.window_size = (video_mode.width as i32, video_mode.height as i32);
                        self.window_size_f32 =
                            (self.window_size.0 as f32, self.window_size.1 as f32);
                        info!(
                            "🖥️ Fullscreen: {} x {}",
                            self.window_size.0, self.window_size.1
                        );
                    } else {
                        info!("⚠️ Could not get monitor video mode, staying windowed");
                    }
                }
            });
        }
    }

    fn toggle_console(&mut self) {
        self.console.open = !self.console.open;
        if self.console.open {
            self.console.focus_previous_widget = true;
        }
        self.update_cursor_mode();
    }

    fn update_cursor_mode(&mut self) {
        let cursor_mode = if self.console.open || self.show_audio_diagnostic {
            glfw::CursorMode::Normal
        } else {
            glfw::CursorMode::Disabled
        };
        self.window_engine.set_cursor_mode(cursor_mode);
    }

    pub(crate) fn apply_reload_requests(&mut self, reload_config: bool, reload_shaders: bool) {
        if reload_config {
            self.reload_config();
        }

        let atomic_reload = self
            .reload_shaders_requested
            .load(std::sync::atomic::Ordering::Relaxed);

        if reload_shaders || atomic_reload {
            if atomic_reload {
                self.reload_shaders_requested
                    .store(false, std::sync::atomic::Ordering::Relaxed);
            }
            self.reload_shaders();
        }

        if self
            .physic_reinit_requested
            .swap(false, std::sync::atomic::Ordering::Relaxed)
        {
            let config = self.physic_engine.get_config().clone();
            let new_max =
                config.max_rockets * (config.particles_per_explosion + config.particles_per_trail);
            self.renderer_engine.recreate_buffers(new_max);
            self.console
                .log("-> Engines (physic + renderer) re-synchronized");
        }
    }

    pub(crate) fn sync_renderer_config(&mut self) {
        // Apply Bloom Parameters from Config
        if let Ok(config) = self.renderer_config.read() {
            self.renderer_engine.sync_bloom_config(&config);
        }

        // Sync comparison mode with BloomPass
        let comparison_active = self
            .tonemapping_comparison_mode
            .load(std::sync::atomic::Ordering::Relaxed);
        self.renderer_engine.bloom_pass_mut().comparison_mode = comparison_active;
    }

    pub(crate) fn update_frame_timing(&mut self) -> f32 {
        let now = Instant::now();
        let delta = now.duration_since(self.last_time).as_secs_f32();
        self.last_time = now;
        self.frames += 1;

        // Instant FPS for sampling
        let fps = if delta > 0.0 { 1.0 / delta } else { 0.0 };

        if self.sampler.should_sample(delta) {
            self.sampled_fps.push(fps);
        }

        // Calculate averages
        let alpha = 0.15;
        self.fps_avg = alpha * fps + (1.0 - alpha) * self.fps_avg;

        let n_frames = 100;
        self.fps_avg_iter = (self.fps_avg_iter * (n_frames - 1) as f32 + fps) / n_frames as f32;

        delta
    }
}

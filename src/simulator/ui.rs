use crate::audio_engine::AudioEngine;
use crate::physic_engine::PhysicEngineFull;
use crate::renderer_engine::RendererEngine;
use crate::window_engine::WindowEngine;
use crate::Simulator;

macro_rules! ui_text {
    ($ui:expr, $($arg:tt)*) => {
        let mut buf = [0u8; 256];
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        if std::io::Write::write_fmt(&mut cursor, format_args!($($arg)*)).is_ok() {
            let pos = cursor.position() as usize;
            if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                $ui.text(s);
            }
        }
    };
}

macro_rules! ui_text_colored {
    ($ui:expr, $color:expr, $($arg:tt)*) => {
        let mut buf = [0u8; 256];
        let mut cursor = std::io::Cursor::new(&mut buf[..]);
        if std::io::Write::write_fmt(&mut cursor, format_args!($($arg)*)).is_ok() {
            let pos = cursor.position() as usize;
            if let Ok(s) = std::str::from_utf8(&buf[..pos]) {
                $ui.text_colored($color, s);
            }
        }
    };
}

impl<R, P, A, W> Simulator<R, P, A, W>
where
    R: RendererEngine,
    P: PhysicEngineFull,
    A: AudioEngine,
    W: WindowEngine,
{
    pub(crate) fn render_ui(&mut self) {
        let comparison_active = self
            .tonemapping_comparison_mode
            .load(std::sync::atomic::Ordering::Relaxed);

        if !self.console.open
            && !comparison_active
            && !self.show_audio_diagnostic
            && !self.gui_settings.open
        {
            return;
        }

        if let Some(new_theme) = self.gui_settings.pending_theme_change.take() {
            let (_, imgui_system) = self.window_engine.get_window_and_imgui_mut();
            crate::simulator::gui_settings::apply_theme_to_context(
                &mut imgui_system.context,
                new_theme,
            );
        }

        let (window, imgui_system) = self.window_engine.get_window_and_imgui_mut();
        let ui = imgui_system.glfw.frame(window, &mut imgui_system.context);

        // Draw comparison labels (background)
        if comparison_active {
            let (positions, labels) = self
                .renderer_engine
                .bloom_pass_mut()
                .get_comparison_grid_info();
            let draw_list = ui.get_background_draw_list();

            for ((x, y, _w, _h), &label) in positions.iter().zip(labels.iter()) {
                let text_x = x + 10.0;
                let text_y = y + 10.0;
                let text_size = ui.calc_text_size(label);
                let padding = 5.0;

                draw_list
                    .add_rect(
                        [text_x - padding, text_y - padding],
                        [
                            text_x + text_size[0] + padding,
                            text_y + text_size[1] + padding,
                        ],
                        [0.0, 0.0, 0.0, 0.8],
                    )
                    .filled(true)
                    .build();
                draw_list.add_text([text_x, text_y], [1.0, 1.0, 1.0, 1.0], label);
            }
        }

        // Draw console (foreground)
        if self.console.open {
            self.console.draw(
                ui,
                &mut self.audio_engine,
                &mut self.physic_engine,
                &self.commands_registry,
            );
        }

        // NOUVEAU: Fenêtre ImGui de diagnostic Audio (indépendante de la console, toggle via F3)
        if self.show_audio_diagnostic {
            if self.audio_stress_scene.enabled {
                self.audio_stress_scene
                    .build_imgui_window(ui, &self.audio_engine);
            } else {
                {
                    // NOUVEAU : Dessiner l'indicateur graphique de l'auditeur (Listener) en arrière-plan
                    let draw_list = ui.get_background_draw_list();
                    let window_width = ui.io().display_size[0];
                    let window_height = ui.io().display_size[1];

                    let listener_x = window_width * 0.5;
                    let listener_y = window_height;

                    // Icône de casque (dessin vectoriel)
                    draw_list
                        .add_circle([listener_x, listener_y - 20.0], 12.0, [0.0, 1.0, 0.0, 0.8])
                        .thickness(2.0)
                        .build();
                    draw_list
                        .add_circle([listener_x, listener_y - 20.0], 4.0, [0.0, 1.0, 0.0, 1.0])
                        .filled(true)
                        .build();

                    let label = "Listener (Sol / Centre)";
                    let text_size = ui.calc_text_size(label);
                    draw_list.add_text(
                        [listener_x - text_size[0] * 0.5, listener_y - 45.0],
                        [0.0, 1.0, 0.0, 1.0],
                        label,
                    );

                    // Cercle indicatif de la zone de volume max (ref_distance = 50px)
                    draw_list
                        .add_circle([listener_x, listener_y], 50.0, [0.0, 0.8, 1.0, 0.35])
                        .thickness(1.5)
                        .build();
                    draw_list.add_text(
                        [listener_x + 55.0, listener_y - 20.0],
                        [0.0, 0.8, 1.0, 0.7],
                        "Volume Max (50px)",
                    );

                    // Cercle de la zone d'atténuation (max_distance réelle récupérée dynamiquement)
                    let max_dist = self.audio_engine.get_max_distance();
                    draw_list
                        .add_circle([listener_x, listener_y], max_dist, [1.0, 0.5, 0.0, 0.15])
                        .thickness(1.5)
                        .build();

                    let text_y = (listener_y - max_dist).max(10.0);
                    let mut buf = [0u8; 64];
                    let mut cursor = std::io::Cursor::new(&mut buf[..]);
                    use std::io::Write;
                    let _ = write!(cursor, "Zone d'attenuation (max {}px)", max_dist as u32);
                    let pos = cursor.position() as usize;
                    if let Ok(label_text) = std::str::from_utf8(&buf[..pos]) {
                        draw_list.add_text(
                            [listener_x + 10.0, text_y],
                            [1.0, 0.5, 0.0, 0.5],
                            label_text,
                        );
                    }
                }

                // ── Indicateur B: Labels flottants animés pour les évènements audio actifs ──
                if self.show_audio_visual_overlay {
                    let draw_list = ui.get_background_draw_list();
                    let win_h = ui.io().display_size[1];

                    for evt in &self.audio_event_pool {
                        let t = (evt.age / evt.kind.ttl_secs()).clamp(0.0, 1.0);
                        // Fade: fast attack, smooth decay
                        let alpha = {
                            let a = if t < 0.05 { t / 0.05 } else { 1.0 };
                            let b = if t > 0.5 { 1.0 - (t - 0.5) / 0.5 } else { 1.0 };
                            a * b
                        };
                        // Float upward (= decreasing screen Y) as the event ages.
                        // Physics Y is bottom-up; ImGui Y is top-down → flip.
                        let drift_y = evt.age * 55.0;
                        let label_x = evt.pos.x - 28.0;
                        let label_y = win_h - evt.pos.y - drift_y - 18.0;

                        let (label, color) = match evt.kind {
                            crate::renderer_engine::AudioEventKind::Launch => {
                                ("LAUNCH", [0.15_f32, 1.0, 0.4, alpha])
                            }
                            crate::renderer_engine::AudioEventKind::Explosion => {
                                ("BOOM", [1.0_f32, 0.45, 0.05, alpha])
                            }
                        };

                        draw_list.add_text([label_x, label_y], color, label);
                    }
                }

                // ── Indicateur D: Badge ID persistant sur chaque fusée en vol ──
                if self.show_audio_visual_overlay {
                    let draw_list = ui.get_background_draw_list();
                    let win_h = ui.io().display_size[1];
                    let mut rocket_index = 0usize;
                    self.physic_engine.for_each_active_head_not_exploded(
                        &mut |p: &crate::physic_engine::Particle| {
                            rocket_index += 1;
                            // Physics Y is bottom-up; flip to ImGui screen coords (top-down).
                            let badge_x = p.pos.x + 10.0;
                            let badge_y = win_h - p.pos.y - 14.0;
                            // Small dark badge background
                            draw_list
                                .add_rect(
                                    [badge_x - 2.0, badge_y - 2.0],
                                    [badge_x + 36.0, badge_y + 14.0],
                                    [0.0_f32, 0.0, 0.0, 0.55],
                                )
                                .rounding(3.0)
                                .filled(true)
                                .build();
                            // Rocket ID text
                            let mut buf = [0u8; 12];
                            let mut cursor = std::io::Cursor::new(&mut buf[..]);
                            use std::io::Write;
                            let _ = write!(cursor, "#{}", rocket_index);
                            let pos = cursor.position() as usize;
                            if let Ok(txt) = std::str::from_utf8(&buf[..pos]) {
                                draw_list.add_text(
                                    [badge_x, badge_y],
                                    [1.0_f32, 1.0, 1.0, 0.90],
                                    txt,
                                );
                            }
                        },
                    );
                }

                let window_width = ui.io().display_size[0];
                let window_height = ui.io().display_size[1];
                ui.window("Audio Diagnostic Monitor")
                    .size([window_width * 0.45, window_height * 0.45], imgui::Condition::FirstUseEver)
                    .position([window_width * 0.53, window_height * 0.52], imgui::Condition::FirstUseEver)
                    .resizable(true)
                    .collapsible(true)
                    .build(|| {
                        ui.text("=== AUDIO ENGINE REAL-TIME DIAGNOSTIC ===");
                        ui.separator();

                        ui.checkbox(
                            "Afficher les indicateurs visuels (ondes/faisceaux/badges)",
                            &mut self.show_audio_visual_overlay,
                        );
                        ui.separator();

                        // Statistiques globales
                        ui_text!(
                            ui,
                            "Rockets: Sent: {}, Received: {}, Played: {}, Dropped: {}, Completed: {}",
                            self.audio_sent_rocket,
                            self.audio_received_rocket,
                            self.audio_played_rocket,
                            self.audio_dropped_rocket,
                            self.audio_completed_rocket
                        );

                        ui_text!(
                            ui,
                            "Explosions: Sent: {}, Received: {}, Played: {}, Dropped: {}, Completed: {}",
                            self.audio_sent_explosion,
                            self.audio_received_explosion,
                            self.audio_played_explosion,
                            self.audio_dropped_explosion,
                            self.audio_completed_explosion
                        );

                        ui.separator();
                        ui.text("=== LATENCY QUANTIFICATION ===");

                        let avg_dispatch = if self.latency_dispatch_count > 0 {
                            self.latency_dispatch_sum.as_secs_f64() * 1000.0 / self.latency_dispatch_count as f64
                        } else {
                            0.0
                        };

                        let avg_play = if self.latency_play_count > 0 {
                            self.latency_play_sum.as_secs_f64() * 1000.0 / self.latency_play_count as f64
                        } else {
                            0.0
                        };

                        let avg_sync_launch = if self.sync_launch_count > 0 {
                            self.sync_launch_sum / self.sync_launch_count as f64
                        } else {
                            0.0
                        };

                        let avg_sync_explosion = if self.sync_explosion_count > 0 {
                            self.sync_explosion_sum / self.sync_explosion_count as f64
                        } else {
                            0.0
                        };

                        ui_text!(ui, "Avg thread transit latency: {:.3} ms", avg_dispatch);
                        ui_text!(ui, "Avg render-to-audio-start latency: {:.3} ms", avg_play);
                        ui_text!(ui, "Avg physical-to-audio launch sync: {:.3} ms", avg_sync_launch);
                        ui_text!(ui, "Avg physical-to-audio explosion sync: {:.3} ms", avg_sync_explosion);

                        ui.separator();
                        ui.text("=== DYNAMIC ANTICIPATION TUNING ===");
                        let config = self.physic_engine.get_config();

                        ui.text("Launch anticipation:");
                        ui.same_line();
                        ui_text!(ui, "{:.2} ms", config.audio_launch_anticipation_ms);
                        ui.same_line();
                        match self.launch_trend_dir {
                            1 => { ui_text_colored!(ui, [0.0, 1.0, 0.0, 1.0], " (⬆️ HAUSSE/UP)"); }      // Green
                            -1 => { ui_text_colored!(ui, [1.0, 0.0, 0.0, 1.0], " (⬇️ BAISSE/DOWN)"); }   // Red
                            _ => { ui_text_colored!(ui, [0.7, 0.7, 0.7, 1.0], " (Stable)"); }           // Grey
                        }

                        ui.text("Explosion anticipation:");
                        ui.same_line();
                        ui_text!(ui, "{:.2} ms", config.audio_explosion_anticipation_ms);
                        ui.same_line();
                        match self.launch_trend_dir {
                            1 => { ui_text_colored!(ui, [0.0, 1.0, 0.0, 1.0], " (⬆️ HAUSSE/UP)"); }      // Green
                            -1 => { ui_text_colored!(ui, [1.0, 0.0, 0.0, 1.0], " (⬇️ BAISSE/DOWN)"); }   // Red
                            _ => { ui_text_colored!(ui, [0.7, 0.7, 0.7, 1.0], " (Stable)"); }           // Grey
                        }

                        // Warning indicator if anything dropped
                        let total_dropped = self.audio_dropped_rocket + self.audio_dropped_explosion;
                        if total_dropped > 0 {
                            ui_text_colored!(ui, [1.0, 0.0, 0.0, 1.0], "⚠️ CRITICAL: {} SOUNDS DROPPED!", total_dropped);
                        } else {
                            ui.text_colored([0.0, 1.0, 0.0, 1.0], "   All sounds successfully dispatched and mixed");
                        }

                        ui.separator();
                        ui.text("=== RECENT AUDIO EVENT LOG ===");

                        ui.child_window("RecentLogChild")
                            .size([0.0, 180.0])
                            .build(|| {
                                for r in self.audio_debug_records.iter().rev().take(15) {
                                    let type_str = match r.sound_type {
                                        crate::audio_engine::types::AudioSoundType::Rocket => "ROCKET",
                                        crate::audio_engine::types::AudioSoundType::Explosion => "EXPLOSION",
                                    };
                                    let status_color = match r.status {
                                        crate::audio_engine::types::AudioPlayStatus::Sent => [0.7, 0.7, 0.7, 1.0],
                                        crate::audio_engine::types::AudioPlayStatus::Received => [0.2, 0.6, 1.0, 1.0],
                                        crate::audio_engine::types::AudioPlayStatus::Playing => [0.0, 1.0, 0.0, 1.0],
                                        crate::audio_engine::types::AudioPlayStatus::Dropped => [1.0, 0.0, 0.0, 1.0],
                                        crate::audio_engine::types::AudioPlayStatus::Completed => [0.5, 0.5, 0.5, 1.0],
                                    };

                                    let transit_ms = r.received_at.map(|t| t.duration_since(r.sent_at).as_secs_f64() * 1000.0);
                                    let start_ms = r.started_at.map(|t| t.duration_since(r.sent_at).as_secs_f64() * 1000.0);

                                    ui_text!(
                                        ui,
                                        "#{:<3} {:<9} (id:{:<2}) - ",
                                        r.request_id, type_str, r.entity_id
                                    );
                                    ui.same_line();
                                    ui_text_colored!(ui, status_color, "{:?}", r.status);

                                    if r.status == crate::audio_engine::types::AudioPlayStatus::Dropped {
                                        if let Some(reason) = r.drop_reason {
                                            ui.same_line();
                                            ui_text!(ui, "({})", reason);
                                        }
                                    } else {
                                        if let Some(t_ms) = transit_ms {
                                            ui.same_line();
                                            ui_text!(ui, " | Transit: {:.2}ms", t_ms);
                                        }
                                        if let Some(s_ms) = start_ms {
                                            ui.same_line();
                                            ui_text!(ui, " | Render-to-start: {:.2}ms", s_ms);
                                        }
                                    }
                                }
                            });
                    });
            }
        }

        // NOUVEAU: Control Panel ImGui GUI (Settings audio, physic, renderer) - toggle via F4
        if self.gui_settings.open {
            self.gui_settings.draw(
                ui,
                &mut self.audio_engine,
                &mut self.physic_engine,
                &self.commands_registry,
                &self.renderer_config,
                &self.reload_shaders_requested,
                &self.physic_reinit_requested,
                &self.tonemapping_comparison_mode,
                &mut self.show_audio_diagnostic,
                &mut self.show_audio_visual_overlay,
                &mut self.audio_stress_scene,
                self.window_size_f32,
            );
        }

        // Finalize ImGui Draw
        let (win, sys) = self.window_engine.get_window_and_imgui_mut();
        sys.glfw.draw(&mut sys.context, win);
    }
}

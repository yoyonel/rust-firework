// Dans src/audio_engine/dsp_processor.rs

use crate::audio_engine::constants;
use crate::audio_engine::effect_flags::{fx_enabled, AudioEffect, AudioEffectFlags};
use crate::audio_engine::types::{PlayRequest, Voice};
use crate::audio_engine::{AudioBlock, DopplerEvent, SafeWavWriter};
use crate::profiler::Profiler;
use crate::AudioEngineSettings;
use crossbeam_channel::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct DspProcessor {
    pub voices: Vec<Voice>,
    pub play_rx: Receiver<PlayRequest>,
    pub doppler_rx: Option<Receiver<DopplerEvent>>,
    pub garbage_tx: Sender<Arc<Vec<[f32; 2]>>>,
    pub settings: AudioEngineSettings,
    pub listener_pos: Arc<crate::audio_engine::types::AtomicVec2>,
    pub sample_rate: u32,
    pub export_writer: Option<Arc<Mutex<SafeWavWriter>>>,
    pub block_index: u64,
    pub acc: Vec<[f32; 2]>,
    /// Bus spatial 2D : composante omnidirectionnelle W
    pub bus_w: Vec<f32>,
    /// Bus spatial 2D : composante directionnelle X (Droite/Gauche)
    pub bus_x: Vec<f32>,
    /// Buffer de travail pré-alloué pour l'exportation WAV (évite tout Vec::new dans le thread CPAL)
    pub export_buffer: Vec<[f32; 2]>,
    pub last_log: Instant,
    pub log_interval: Duration,
    /// Masque atomique des effets DSP. Lu une seule fois par `process_block`.
    pub effect_flags: Arc<AudioEffectFlags>,
    /// Réverbération spatiale globale FDN / Schroeder sur le bus accumulé O(1)
    pub spatial_reverb: crate::audio_engine::SpatialReverb,
    /// Décodeur HRTF binaural par convolution FFT Overlap-Save sur le bus spatial
    pub hrtf_convolver: crate::audio_engine::HrtfConvolver,
    /// Canal de debug pour notifier le thread principal des événements audio
    pub debug_tx: Option<Sender<crate::audio_engine::types::AudioDebugEvent>>,
}

#[inline(always)]
fn compute_distance_attenuation(
    settings: &AudioEngineSettings,
    distance: f32,
    fx_mask: u32,
) -> f32 {
    if fx_enabled(fx_mask, AudioEffect::DistanceAtten) {
        let ref_distance = crate::audio_engine::constants::REFERENCE_DISTANCE_METERS;
        let max_distance = settings.max_distance().max(ref_distance + 1.0);
        if distance <= ref_distance {
            1.0
        } else if distance >= max_distance {
            0.0
        } else {
            let raw_att = ref_distance / distance;
            let fade = (max_distance - distance) / (max_distance - ref_distance);
            raw_att * fade
        }
    } else {
        1.0
    }
}

#[inline(always)]
fn compute_lowpass_alpha(
    settings: &AudioEngineSettings,
    sample_rate: u32,
    distance: f32,
    fx_mask: u32,
) -> f32 {
    if fx_enabled(fx_mask, AudioEffect::LowPassFilter) {
        let fc = (settings.f_min()
            + (settings.f_max() - settings.f_min())
                * (-settings.distance_alpha() * distance).exp())
        .clamp(settings.f_min(), settings.f_max());
        let dt = 1.0 / sample_rate as f32;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * fc);
        dt / (rc + dt)
    } else {
        1.0
    }
}

#[inline(always)]
fn interpolate_mono_sample(slice: &[[f32; 2]], pos: f64) -> f32 {
    if pos < 0.0 {
        return 0.0;
    }
    let index = pos as usize;
    let total_len = slice.len();
    if index >= total_len {
        return 0.0;
    }

    let s0 = &slice[index];
    let sample0 = (s0[0] + s0[1]) * constants::STEREO_MIX_FACTOR;
    let sample1 = if index + 1 < total_len {
        let s1 = &slice[index + 1];
        (s1[0] + s1[1]) * constants::STEREO_MIX_FACTOR
    } else {
        0.0
    };
    let fraction = (pos - index as f64) as f32;
    sample0 + fraction * (sample1 - sample0)
}

#[inline(always)]
fn apply_fade_in_out(
    sample: f32,
    index: usize,
    total_len: usize,
    fade_in_samples: usize,
    fade_out_samples: usize,
    inv_fade_in: f32,
    inv_fade_out: f32,
) -> f32 {
    if index < fade_in_samples {
        sample * (index as f32 * inv_fade_in)
    } else if total_len - index < fade_out_samples {
        sample * ((total_len - index) as f32 * inv_fade_out)
    } else {
        sample
    }
}

#[inline(always)]
fn apply_iir_lowpass(current_sample: f32, prev_filtered: f32, alpha: f32) -> f32 {
    let mut filtered = prev_filtered + alpha * (current_sample - prev_filtered);
    if filtered.abs() < 1e-15 {
        filtered = 0.0;
    }
    filtered
}

#[inline(always)]
fn fast_tanh(x: f32) -> f32 {
    let x2 = x * x;
    let num = x * (15.0 + x2);
    let den = 15.0 + 6.0 * x2;
    (num / den).clamp(-1.0, 1.0)
}

impl DspProcessor {
    /// Point d'entrée principal du callback CPAL
    #[inline(always)]
    pub fn process_block(&mut self, data: &mut [f32], global_gain: f32, profiler: &Profiler) {
        let start_time = Instant::now();
        let _audio_frame_guard = profiler.measure("audio_frame");
        let frames = data.len() / 2;

        if frames > self.acc.len() {
            log::error!("Buffer under-allocated! Requested {} frames", frames);
            return;
        }

        // Lecture unique du masque des effets pour tout le bloc (1 atomic load, ~1 cycle).
        // Toutes les fonctions appelées en aval utilisent ce snapshot — pas de re-lecture.
        let fx_mask = self.effect_flags.load();

        // 1. Nettoyage du buffer d'accumulation
        self.acc[..frames].fill([0.0; 2]);

        // 2. Traitements Lock-Free
        self.consume_requests(profiler);
        self.process_doppler(fx_mask, profiler);

        // 3. Rendu DSP (Isolé dans Hotspot via #[inline(never)])
        self.process_dsp(frames, fx_mask, profiler);

        // 3.5. Réverbération spatiale globale O(1) sur le bus accumulé
        if fx_enabled(fx_mask, AudioEffect::SpatialReverb) {
            let _reverb_guard = profiler.measure("spatial_reverb");
            self.spatial_reverb.process_block(&mut self.acc, frames);
        }

        // 4. Finalisation et monitoring (Soft clipping isolé dans Hotspot)
        self.write_cpal_buffer(data, frames, global_gain, fx_mask, profiler);
        self.export_wav(data, frames);
        self.log_metrics(profiler);

        let elapsed_us = start_time.elapsed().as_micros() as u64;
        let budget_us = ((frames as f64 / self.sample_rate as f64) * 1_000_000.0) as u64;

        if let Some(debug_tx) = &self.debug_tx {
            let active_voices = self.voices.iter().filter(|v| v.active).count();
            let _ = debug_tx.try_send(
                crate::audio_engine::types::AudioDebugEvent::BlockProcessed {
                    elapsed_us,
                    budget_us,
                    active_voices,
                },
            );

            if elapsed_us > budget_us {
                log::warn!(
                    "⚠️ CPU Audio Underrun detected: block took {} us (budget: {} us)",
                    elapsed_us,
                    budget_us
                );
                if let Err(e) =
                    debug_tx.try_send(crate::audio_engine::types::AudioDebugEvent::Underrun {
                        elapsed_us,
                        budget_us,
                    })
                {
                    log::error!("Failed to send Underrun event: {:?}", e);
                }
            }
        }
    }

    #[inline(always)]
    fn consume_requests(&mut self, profiler: &Profiler) {
        crate::tracy_zone!("audio::consume_requests", 0x00FF00);

        let fx_mask = self.effect_flags.load();
        let listener_pos = self.listener_pos.load();

        while let Ok(req) = self.play_rx.try_recv() {
            if let Some(debug_tx) = &self.debug_tx {
                let _ = debug_tx.try_send(crate::audio_engine::types::AudioDebugEvent::Received {
                    request_id: req.request_id,
                    received_at: Instant::now(),
                });
            }

            let mut selected_voice_idx = None;
            let mut steal_reason = None;

            // 1. Chercher une voix inactive
            if let Some((idx, _)) = self.voices.iter().enumerate().find(|(_, v)| !v.active) {
                selected_voice_idx = Some(idx);
            } else {
                // 2. Stratégie de Voice Stealing (vol de voix)
                // Calculer l'atténuation spatiale pour la nouvelle requête (pre-attenuation fix)
                let d = req.pos - listener_pos;
                let distance = d.length().max(1e-6);
                let att = compute_distance_attenuation(&self.settings, distance, fx_mask);

                let req_priority = match req.sound_type {
                    crate::audio_engine::types::AudioSoundType::Explosion => 2.0,
                    crate::audio_engine::types::AudioSoundType::Rocket => 1.0,
                };
                let req_volume = req.gain * att * req_priority;

                // Trouver la voix active la plus silencieuse (gain et atténuation spatiale pondérés par priorité de son)
                let mut min_volume = f32::MAX;
                let mut quietest_idx = None;

                for (idx, v) in self.voices.iter().enumerate() {
                    let v_priority = match v.sound_type {
                        crate::audio_engine::types::AudioSoundType::Explosion => 2.0,
                        crate::audio_engine::types::AudioSoundType::Rocket => 1.0,
                    };
                    let v_d = v.world_pos - listener_pos;
                    let v_distance = v_d.length().max(1e-6);
                    let v_att = compute_distance_attenuation(&self.settings, v_distance, fx_mask);
                    let v_gain = if v.current_gains == [0.0, 0.0] {
                        v.user_gain
                    } else {
                        v.current_gains[0].abs().max(v.current_gains[1].abs())
                    }
                    .max(0.001);
                    let volume = v_gain * v_att * v_priority;
                    if volume < min_volume {
                        min_volume = volume;
                        quietest_idx = Some(idx);
                    }
                }

                if let Some(idx) = quietest_idx {
                    // On ne vole la voix que si le nouveau son demandé est plus fort que le son actif le plus silencieux
                    if req_volume > min_volume {
                        let stolen_req_id = self.voices[idx].request_id;
                        selected_voice_idx = Some(idx);
                        steal_reason = Some(stolen_req_id);
                    }
                }
            }

            if let Some(voice_idx) = selected_voice_idx {
                // Si on a volé une voix active, on notifie son drop avec le motif approprié
                if let Some(stolen_id) = steal_reason {
                    if let Some(debug_tx) = &self.debug_tx {
                        let _ = debug_tx.try_send(
                            crate::audio_engine::types::AudioDebugEvent::Dropped {
                                request_id: stolen_id,
                                dropped_at: std::time::Instant::now(),
                                reason: "Voice stolen (quieter)",
                            },
                        );
                    }
                }

                let v = &mut self.voices[voice_idx];
                // Éviter tout drop d'Arc dans le thread CPAL lors du vol d'une voix active
                if let Some(dead_arc) = v.data.take() {
                    let _ = self.garbage_tx.try_send(dead_arc);
                }
                v.reset_from_request(&req);
                let now = Instant::now();
                let latency = now.duration_since(req.sent_at);
                profiler.record_metric("audio latency", latency);
                crate::tracy_plot!("Audio: Latency (ms)", latency.as_secs_f64() * 1000.0);

                if let Some(debug_tx) = &self.debug_tx {
                    let _ =
                        debug_tx.try_send(crate::audio_engine::types::AudioDebugEvent::Started {
                            request_id: req.request_id,
                            started_at: now,
                            voice_index: voice_idx,
                        });
                }
            } else {
                // Pas de voix libre et aucune voix active plus silencieuse que le nouveau son
                if let Some(debug_tx) = &self.debug_tx {
                    let _ =
                        debug_tx.try_send(crate::audio_engine::types::AudioDebugEvent::Dropped {
                            request_id: req.request_id,
                            dropped_at: Instant::now(),
                            reason: "No inactive voice available",
                        });
                }
            }
        }

        let nb_actives_voices = self.voices.iter().filter(|v| v.active).count();
        profiler.record_metric("nb_actives_voices", nb_actives_voices);
        crate::tracy_plot!("Audio: Active Voices", nb_actives_voices as f64);
    }

    #[inline(always)]
    fn process_doppler(&mut self, fx_mask: u32, profiler: &Profiler) {
        // Court-circuit si le Doppler est désactivé globalement
        if !fx_enabled(fx_mask, AudioEffect::Doppler) {
            // Remise à 1.0 du taux de lecture pour les voix dynamiques, pour éviter
            // tout artefact si le Doppler est réactivé plus tard.
            for v in self.voices.iter_mut().filter(|v| v.active && v.is_dynamic) {
                v.playback_rate = 1.0;
            }
            return;
        }
        if let Some(doppler_rx) = &self.doppler_rx {
            crate::tracy_zone!("audio::process_doppler", 0x00AAFF);
            let mut events_received_in_block = 0;

            while let Ok(event) = doppler_rx.try_recv() {
                events_received_in_block += 1;

                if let Some(v) = self
                    .voices
                    .iter_mut()
                    .find(|v| v.active && v.is_dynamic && v.id == event.id)
                {
                    v.world_pos = event.pos;
                    v.velocity = event.vel;

                    let d = v.world_pos - self.listener_pos.load();
                    let dist = d.length().max(0.001);

                    let dir = -d / dist;
                    let v_radial = v.velocity.dot(dir);

                    let c = 343.0_f32;
                    let denominator = c - v_radial;
                    v.playback_rate = if denominator <= 0.0 {
                        4.0
                    } else {
                        (c / denominator).clamp(0.25, 4.0)
                    };

                    crate::tracy_plot!("Audio: Doppler Rate (alpha)", v.playback_rate as f64);
                }
            }
            profiler.record_metric("doppler_events", events_received_in_block);
            crate::tracy_plot!(
                "Audio: Doppler Events/Block",
                events_received_in_block as f64
            );
        }
    }

    #[inline(always)]
    fn process_dsp(&mut self, frames: usize, fx_mask: u32, profiler: &Profiler) {
        if fx_enabled(fx_mask, AudioEffect::SpatialBus) {
            self.process_dsp_spatial_bus(frames, fx_mask, profiler);
        } else {
            self.process_dsp_legacy(frames, fx_mask, profiler);
        }
    }

    /// 🎯 BOÎTE HOTSPOT 1B : Rendu ultra-rapide par Bus Spatial 2D (Ambisonics 2D / Harmoniques Circulaires W, X, Y)
    /// Pré-accumule les sources dans un bus 3 canaux ultra-léger (3 mults/sample) avant de décoder une seule fois en Stéréo.
    #[inline(never)]
    fn process_dsp_spatial_bus(&mut self, frames: usize, fx_mask: u32, profiler: &Profiler) {
        let _guard = profiler.measure("process_active_voices_bus");
        crate::tracy_zone!("audio::process_dsp_spatial_bus", 0xAA00FF);

        if frames > self.bus_w.len() || frames > self.bus_x.len() {
            log::error!(
                "Buffer bus_w/bus_x under-allocated! Requested {} frames, available {}",
                frames,
                self.bus_w.len()
            );
            return;
        }

        let bus_w_slice = &mut self.bus_w[..frames];
        let bus_x_slice = &mut self.bus_x[..frames];

        bus_w_slice.fill(0.0);
        bus_x_slice.fill(0.0);

        let listener_pos = self.listener_pos.load();
        let use_fade = fx_enabled(fx_mask, AudioEffect::FadeInOut);

        for v in self.voices.iter_mut() {
            if !v.active || v.data.is_none() {
                continue;
            }

            let d = v.world_pos - listener_pos;
            let distance = d.length().max(1e-6);

            // Direction 2D normalisée (x = panoramique horizontal droite/gauche)
            let dir_x = d.x / distance;

            let att = compute_distance_attenuation(&self.settings, distance, fx_mask);
            let filter_a =
                compute_lowpass_alpha(&self.settings, self.sample_rate, distance, fx_mask);
            v.filter_a = filter_a;

            let slice_ref = v.data.as_ref().expect("Voice data should exist");
            let total_len = slice_ref.len();

            let mut prev_mono = v.filter_state[0];
            let rate = v.playback_rate as f64;
            let voice_gain = v.user_gain * att;

            let w_weight = voice_gain * std::f32::consts::FRAC_1_SQRT_2;
            let x_weight = voice_gain * dir_x;

            let fade_in = v.fade_in_samples;
            let fade_out = v.fade_out_samples;
            let active_fade = use_fade && (fade_in > 0 || fade_out > 0);
            let apply_lpf = filter_a < 0.9999;

            if (rate - 1.0).abs() < 1e-6 {
                // 🚀 Fast Path (rate == 1.0) : Pas de LERP fractionnaire
                let start_idx = v.pos as usize;
                let count = if start_idx >= total_len {
                    0
                } else {
                    (total_len - start_idx).min(frames)
                };

                if count > 0 {
                    let sample_slice = &slice_ref[start_idx..start_idx + count];
                    let w_out = &mut bus_w_slice[..count];
                    let x_out = &mut bus_x_slice[..count];

                    let inv_fade_in = constants::compute_fade_reciprocal(fade_in);
                    let inv_fade_out = constants::compute_fade_reciprocal(fade_out);

                    if !active_fade && !apply_lpf {
                        // Boucle 100% vectorisable par le compilateur (SIMD 8-wide AVX2)
                        for i in 0..count {
                            let s = sample_slice[i];
                            let sample = (s[0] + s[1]) * constants::STEREO_MIX_FACTOR;
                            w_out[i] += sample * w_weight;
                            x_out[i] += sample * x_weight;
                        }
                    } else {
                        for i in 0..count {
                            let index = start_idx + i;
                            let s = sample_slice[i];
                            let mut sample = (s[0] + s[1]) * constants::STEREO_MIX_FACTOR;
                            if active_fade {
                                sample = apply_fade_in_out(
                                    sample,
                                    index,
                                    total_len,
                                    fade_in,
                                    fade_out,
                                    inv_fade_in,
                                    inv_fade_out,
                                );
                            }
                            if apply_lpf {
                                sample = apply_iir_lowpass(sample, prev_mono, filter_a);
                                prev_mono = sample;
                            }
                            w_out[i] += sample * w_weight;
                            x_out[i] += sample * x_weight;
                        }
                    }
                    v.pos += count as f64;
                }
            } else {
                // Path fallback : Vitesse variable (Doppler / Pitch shift avec LERP)
                let inv_fade_in = constants::compute_fade_reciprocal(fade_in);
                let inv_fade_out = constants::compute_fade_reciprocal(fade_out);

                for i in 0..frames {
                    let current_pos = v.pos;
                    let index = current_pos as usize;

                    if index >= total_len {
                        break;
                    }

                    let mut sample = interpolate_mono_sample(slice_ref, current_pos);

                    if active_fade {
                        sample = apply_fade_in_out(
                            sample,
                            index,
                            total_len,
                            fade_in,
                            fade_out,
                            inv_fade_in,
                            inv_fade_out,
                        );
                    }

                    if apply_lpf {
                        sample = apply_iir_lowpass(sample, prev_mono, filter_a);
                    }
                    prev_mono = sample;

                    bus_w_slice[i] += sample * w_weight;
                    bus_x_slice[i] += sample * x_weight;

                    v.pos += rate;
                }
            }

            v.filter_state[0] = prev_mono;
            v.current_gains[0] = voice_gain;
            v.current_gains[1] = voice_gain;

            if v.pos as usize >= total_len {
                v.active = false;
                if let Some(dead_arc) = v.data.take() {
                    let _ = self.garbage_tx.try_send(dead_arc);
                }
                if let Some(debug_tx) = &self.debug_tx {
                    let _ =
                        debug_tx.try_send(crate::audio_engine::types::AudioDebugEvent::Completed {
                            request_id: v.request_id,
                            completed_at: Instant::now(),
                        });
                }
            }
        }

        // Décodage final du Bus Spatial (W, X) vers la sortie Stéréo (L, R)
        if fx_enabled(fx_mask, AudioEffect::HrtfBus) {
            let _hrtf_guard = profiler.measure("hrtf_bus_convolver");
            self.hrtf_convolver
                .process_bus(bus_w_slice, bus_x_slice, &mut self.acc, frames);
        } else {
            let frac_1_sqrt2 = std::f32::consts::FRAC_1_SQRT_2;
            let acc_slice = &mut self.acc[..frames];
            for i in 0..frames {
                let w = bus_w_slice[i];
                let x = bus_x_slice[i];
                acc_slice[i][0] = w - frac_1_sqrt2 * x;
                acc_slice[i][1] = w + frac_1_sqrt2 * x;
            }
        }
    }

    /// 🎯 BOÎTE HOTSPOT 1A : Traitement DSP classique legacy par bloc et par voix (LERP + 3D Binaural)
    /// L'annotation #[inline(never)] garantit que ce bloc sera visible individuellement dans perf.
    #[inline(never)]
    fn process_dsp_legacy(&mut self, frames: usize, fx_mask: u32, profiler: &Profiler) {
        let _guard = profiler.measure("process_active_voices");
        crate::tracy_zone!("audio::process_dsp", 0xAA00FF);

        for v in self.voices.iter_mut() {
            if !v.active || v.data.is_none() {
                continue;
            }

            // 1. Paramètres physiques 3D de base
            let d = v.world_pos - self.listener_pos.load();
            let distance = d.length().max(1e-6);

            let filter_a =
                compute_lowpass_alpha(&self.settings, self.sample_rate, distance, fx_mask);
            v.filter_a = filter_a;

            let slice_ref = v.data.as_ref().expect("Voice data should exist");
            let total_len = slice_ref.len();

            let mut prev_l = v.filter_state[0];
            let mut prev_r = v.filter_state[1];
            let rate = v.playback_rate as f64;

            // 2. Calcul du Binaural ou Panning (Une fois par bloc !)
            let params = crate::audio_engine::binaural_processing::calculate_spatial_params_2d(
                d,
                &self.settings,
                fx_mask,
            );

            let itd_l_samples = params.itd_left_sec * self.sample_rate as f32;
            let itd_r_samples = params.itd_right_sec * self.sample_rate as f32;

            let target_gain_l = v.user_gain * params.gain_left;
            let target_gain_r = v.user_gain * params.gain_right;

            // Initialisation des gains selon que GainLerp est actif ou non
            let (start_gain_l, start_gain_r, gain_step_l, gain_step_r) =
                if fx_enabled(fx_mask, AudioEffect::GainLerp) {
                    let s_l = v.current_gains[0];
                    let s_r = v.current_gains[1];
                    (
                        s_l,
                        s_r,
                        (target_gain_l - s_l) / frames as f32,
                        (target_gain_r - s_r) / frames as f32,
                    )
                } else {
                    // Bypass : on applique directement les gains cibles sans rampe
                    (target_gain_l, target_gain_r, 0.0, 0.0)
                };

            // Initialisation des ITDs si c'est le début du son
            if v.current_itd == [0.0, 0.0] {
                v.current_itd = [itd_l_samples, itd_r_samples];
            }
            let start_itd_l = v.current_itd[0];
            let start_itd_r = v.current_itd[1];
            let itd_step_l = (itd_l_samples - start_itd_l) / frames as f32;
            let itd_step_r = (itd_r_samples - start_itd_r) / frames as f32;

            let inv_fade_in = constants::compute_fade_reciprocal(v.fade_in_samples);
            let inv_fade_out = constants::compute_fade_reciprocal(v.fade_out_samples);

            // 3. Boucle de Mixage : Lecture spatiale directe (Zéro Buffer Intermédiaire)
            for (i, frame) in self.acc[..frames].iter_mut().enumerate() {
                let current_pos = v.pos;
                let index = current_pos as usize;

                if index >= total_len {
                    break;
                }

                let current_itd_l = start_itd_l + itd_step_l * i as f32;
                let current_itd_r = start_itd_r + itd_step_r * i as f32;

                let mut l = interpolate_mono_sample(slice_ref, current_pos - current_itd_l as f64);
                let mut r = interpolate_mono_sample(slice_ref, current_pos - current_itd_r as f64);

                if fx_enabled(fx_mask, AudioEffect::FadeInOut) {
                    l = apply_fade_in_out(
                        l,
                        index,
                        total_len,
                        v.fade_in_samples,
                        v.fade_out_samples,
                        inv_fade_in,
                        inv_fade_out,
                    );
                    r = apply_fade_in_out(
                        r,
                        index,
                        total_len,
                        v.fade_in_samples,
                        v.fade_out_samples,
                        inv_fade_in,
                        inv_fade_out,
                    );
                }

                l = apply_iir_lowpass(l, prev_l, filter_a);
                r = apply_iir_lowpass(r, prev_r, filter_a);

                prev_l = l;
                prev_r = r;

                let current_g_l = start_gain_l + gain_step_l * i as f32;
                let current_g_r = start_gain_r + gain_step_r * i as f32;
                frame[0] += l * current_g_l;
                frame[1] += r * current_g_r;

                v.pos += rate;
            }

            v.filter_state[0] = prev_l;
            v.filter_state[1] = prev_r;
            v.current_gains[0] = target_gain_l;
            v.current_gains[1] = target_gain_r;
            v.current_itd[0] = itd_l_samples;
            v.current_itd[1] = itd_r_samples;

            if v.pos as usize >= total_len {
                v.active = false;
                if let Some(dead_arc) = v.data.take() {
                    let _ = self.garbage_tx.try_send(dead_arc);
                }
                if let Some(debug_tx) = &self.debug_tx {
                    let _ =
                        debug_tx.try_send(crate::audio_engine::types::AudioDebugEvent::Completed {
                            request_id: v.request_id,
                            completed_at: Instant::now(),
                        });
                }
            }
        }
    }

    /// 🎯 BOÎTE HOTSPOT 2 : Soft Clipping & Écriture CPAL
    /// L'annotation #[inline(never)] permet de séparer le coût mathématique du mixage DSP dans perf.
    #[inline(never)]
    fn write_cpal_buffer(
        &mut self,
        data: &mut [f32],
        frames: usize,
        global_gain: f32,
        fx_mask: u32,
        profiler: &Profiler,
    ) {
        profiler.profile_block("write_cpal_buffer", || {
            crate::tracy_zone!("audio::soft_clipping", 0xFF5500);
            let acc_slice = &self.acc[..frames];
            let data_slice = &mut data[..frames * 2];

            if fx_enabled(fx_mask, AudioEffect::Normalization) {
                // Saturation douce via fast_tanh polynomial Padé (évite tout appel libc::tanhf)
                for i in 0..frames {
                    let sample = acc_slice[i];
                    data_slice[2 * i] = fast_tanh(sample[0] * global_gain);
                    data_slice[2 * i + 1] = fast_tanh(sample[1] * global_gain);
                }
            } else {
                // Bypass : pas de gain global, clampage linéaire simple [-1.0, 1.0]
                for i in 0..frames {
                    let sample = acc_slice[i];
                    data_slice[2 * i] = sample[0].clamp(-1.0, 1.0);
                    data_slice[2 * i + 1] = sample[1].clamp(-1.0, 1.0);
                }
            }
        });
    }

    #[inline(always)]
    fn export_wav(&mut self, data: &[f32], frames: usize) {
        if let Some(writer_arc) = &self.export_writer {
            self.export_buffer.clear();
            for i in 0..frames {
                self.export_buffer.push([data[2 * i], data[2 * i + 1]]);
            }

            let block = AudioBlock {
                index: self.block_index,
                frames: self.export_buffer.clone(),
            };
            self.block_index += 1;

            if let Ok(writer) = writer_arc.try_lock() {
                writer.push_block(block);
            }
        }
    }

    #[inline(always)]
    fn log_metrics(&mut self, profiler: &Profiler) {
        if self.last_log.elapsed() >= self.log_interval {
            crate::log_metrics!(profiler);
            self.last_log = Instant::now();
        }
    }
}

#[cfg(test)]
mod tests;

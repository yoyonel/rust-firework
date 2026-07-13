// Dans src/audio_engine/dsp_processor.rs

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
    pub listener_pos: glam::Vec2,
    pub sample_rate: u32,
    pub export_writer: Option<Arc<Mutex<SafeWavWriter>>>,
    pub block_index: u64,
    pub acc: Vec<[f32; 2]>,
    pub last_log: Instant,
    pub log_interval: Duration,
    /// Masque atomique des effets DSP. Lu une seule fois par `process_block`.
    pub effect_flags: Arc<AudioEffectFlags>,
}

impl DspProcessor {
    /// Point d'entrée principal du callback CPAL
    #[inline(always)]
    pub fn process_block(&mut self, data: &mut [f32], global_gain: f32, profiler: &Profiler) {
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

        // 4. Finalisation et monitoring (Soft clipping isolé dans Hotspot)
        self.write_cpal_buffer(data, frames, global_gain, fx_mask, profiler);
        self.export_wav(data, frames);
        self.log_metrics(profiler);
    }

    #[inline(always)]
    fn consume_requests(&mut self, profiler: &Profiler) {
        crate::tracy_zone!("audio::consume_requests", 0x00FF00);

        while let Ok(req) = self.play_rx.try_recv() {
            if let Some(v) = self.voices.iter_mut().find(|v| !v.active) {
                v.reset_from_request(&req);
                let latency = Instant::now().duration_since(req.sent_at);
                profiler.record_metric("audio latency", latency);
                crate::tracy_plot!("Audio: Latency (ms)", latency.as_secs_f64() * 1000.0);
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

                    let d = v.world_pos - self.listener_pos;
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

    /// 🎯 BOÎTE HOTSPOT 1 : Traitement DSP par bloc et par voix (LERP + 3D Binaural)
    /// L'annotation #[inline(never)] garantit que ce bloc sera visible individuellement dans perf.
    #[inline(never)]
    fn process_dsp(&mut self, frames: usize, fx_mask: u32, profiler: &Profiler) {
        let _guard = profiler.measure("process_active_voices");
        crate::tracy_zone!("audio::process_dsp", 0xAA00FF);

        for v in self.voices.iter_mut() {
            if !v.active || v.data.is_none() {
                continue;
            }

            // 1. Paramètres physiques 3D de base
            let d = v.world_pos - self.listener_pos;
            let distance = d.length().max(1e-6);

            // Filtre passe-bas dynamique (conditionnel via fx_mask)
            let filter_a = if fx_enabled(fx_mask, AudioEffect::LowPassFilter) {
                let fc = (self.settings.f_min()
                    + (self.settings.f_max() - self.settings.f_min())
                        * (-self.settings.distance_alpha() * distance).exp())
                .clamp(self.settings.f_min(), self.settings.f_max());
                let dt = 1.0 / self.sample_rate as f32;
                let rc = 1.0 / (2.0 * std::f32::consts::PI * fc);
                dt / (rc + dt)
            } else {
                // Bypass : a=1 → y[n] = y[n-1] + 1*(x - y[n-1]) = x (pass-through)
                1.0
            };
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

            // 3. Boucle de Mixage : Lecture spatiale directe (Zéro Buffer Intermédiaire)
            for (i, frame) in self.acc[..frames].iter_mut().enumerate() {
                let current_pos = v.pos;
                let index = current_pos as usize;

                if index >= total_len {
                    break;
                }

                // 🎯 L'ASTUCE ABSOLUE : Fonction de lecture qui recule dans le temps (ITD)
                let interpolate = |pos: f64| -> f32 {
                    if pos < 0.0 {
                        return 0.0;
                    } // Silence avant que le son n'atteigne l'oreille
                    let idx = pos as usize;
                    if idx >= total_len {
                        return 0.0;
                    }

                    // Somme mono de la source stéréo originale
                    let s0 = (slice_ref[idx][0] + slice_ref[idx][1]) * 0.5;
                    let s1 = if idx + 1 < total_len {
                        (slice_ref[idx + 1][0] + slice_ref[idx + 1][1]) * 0.5
                    } else {
                        0.0
                    };
                    let frac = (pos - idx as f64) as f32;
                    s0 + frac * (s1 - s0)
                };

                let current_itd_l = start_itd_l + itd_step_l * i as f32;
                let current_itd_r = start_itd_r + itd_step_r * i as f32;

                // On lit directement le signal aux deux positions temporelles (Gauche / Droite)
                let mut l = interpolate(current_pos - current_itd_l as f64);
                let mut r = interpolate(current_pos - current_itd_r as f64);

                // Application des Fades (conditionnelle)
                if fx_enabled(fx_mask, AudioEffect::FadeInOut) {
                    if index < v.fade_in_samples {
                        let alpha = index as f32 / v.fade_in_samples as f32;
                        l *= alpha;
                        r *= alpha;
                    } else {
                        let rem = total_len - index;
                        if rem < v.fade_out_samples {
                            let alpha = rem as f32 / v.fade_out_samples as f32;
                            l *= alpha;
                            r *= alpha;
                        }
                    }
                }

                // Filtre IIR Passe-bas (conditionnel — filter_a=1.0 si désactivé = pass-through)
                l = prev_l + filter_a * (l - prev_l);
                r = prev_r + filter_a * (r - prev_r);

                if l.abs() < 1e-15 {
                    l = 0.0;
                }
                if r.abs() < 1e-15 {
                    r = 0.0;
                }

                prev_l = l;
                prev_r = r;

                // Sommation directe avec gains interpolés (LERP) dans le buffer CPAL final
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
            }
        }
    }

    /// 🎯 BOÎTE HOTSPOT 2 : Soft Clipping & Écriture CPAL
    /// L'annotation #[inline(never)] permet de séparer le coût mathématique (.tanh()) du mixage DSP dans perf.
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
            if fx_enabled(fx_mask, AudioEffect::Normalization) {
                // Saturation douce via tanh (limiteur doux, évite le hard clipping) et application du gain global
                for (i, sample) in self.acc.iter_mut().take(frames).enumerate() {
                    data[2 * i] = (sample[0] * global_gain).tanh();
                    data[2 * i + 1] = (sample[1] * global_gain).tanh();
                }
            } else {
                // Bypass : pas de gain global, clampage linéaire simple [-1.0, 1.0]
                for (i, sample) in self.acc.iter_mut().take(frames).enumerate() {
                    data[2 * i] = sample[0].clamp(-1.0, 1.0);
                    data[2 * i + 1] = sample[1].clamp(-1.0, 1.0);
                }
            }
        });
    }

    #[inline(always)]
    fn export_wav(&mut self, data: &[f32], frames: usize) {
        if let Some(writer_arc) = &self.export_writer {
            let mut frames_vec = Vec::with_capacity(frames);
            for i in 0..frames {
                frames_vec.push([data[2 * i], data[2 * i + 1]]);
            }

            let block = AudioBlock {
                index: self.block_index,
                frames: frames_vec,
            };
            self.block_index += 1;

            writer_arc
                .lock()
                .expect("Failed to lock writer")
                .push_block(block);
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
mod tests {
    use std::f32::consts::PI;

    /// Génère un buffer stéréo contenant une onde sinusoïdale pure de fréquence donnée.
    fn generate_sine_wave(
        freq_hz: f32,
        sample_rate: u32,
        duration_samples: usize,
    ) -> Vec<[f32; 2]> {
        (0..duration_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let val = (2.0 * PI * freq_hz * t).sin();
                [val, val]
            })
            .collect()
    }

    #[test]
    fn test_phase_continuity_across_block_boundaries() {
        use crate::audio_engine::types::Voice;
        use crate::profiler::Profiler;
        use crate::AudioEngineSettings;
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        use crate::audio_engine::effect_flags::AudioEffectFlags;
        let sample_rate = 48_000;
        let block_size = 256;
        let total_blocks = 4;
        let total_samples = block_size * total_blocks;

        // 1. Source : Sinusoïde pure à 440 Hz (La fondamental)
        let source_audio = generate_sine_wave(440.0, sample_rate, total_samples + 100);
        let source_arc = Arc::new(source_audio);

        let (_play_tx, play_rx) = crossbeam_channel::unbounded();
        let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();
        let settings = AudioEngineSettings::default();

        // 2. Traitement Mode A : Un seul bloc continu de 1024 échantillons (La Vérité Terrain)
        let mut dsp_ref = super::DspProcessor {
            voices: vec![Voice {
                id: 1,
                active: true,
                data: Some(source_arc.clone()),
                pos: 0.0,
                playback_rate: 1.0,
                is_dynamic: true,
                world_pos: glam::Vec2::new(0.0, 10.0), // Devant le listener
                velocity: glam::Vec2::ZERO,
                fade_in_samples: 0,
                fade_out_samples: 0,
                filter_state: [0.0, 0.0],
                filter_a: 0.0,
                user_gain: 1.0,
                current_gains: [0.99, 0.99],
                target_gains: [0.99, 0.99],
                current_itd: [0.0, 0.0],
                target_itd: [0.0, 0.0],
            }],
            play_rx: play_rx.clone(),
            doppler_rx: None,
            garbage_tx: garbage_tx.clone(),
            settings: settings.clone(),
            listener_pos: glam::Vec2::ZERO,
            sample_rate,
            export_writer: None,
            block_index: 0,
            acc: vec![[0.0; 2]; total_samples],
            last_log: Instant::now(),
            log_interval: Duration::from_secs(1),
            effect_flags: AudioEffectFlags::new_all_enabled(),
        };

        let profiler = Profiler::new(1000);
        let fx_mask_ref = dsp_ref.effect_flags.load();
        dsp_ref.process_dsp(total_samples, fx_mask_ref, &profiler);
        let reference_output = dsp_ref.acc.clone();

        // 3. Traitement Mode B : 4 blocs successifs de 256 échantillons
        let mut dsp_chunk = super::DspProcessor {
            voices: vec![Voice {
                id: 1,
                active: true,
                data: Some(source_arc),
                pos: 0.0,
                playback_rate: 1.0,
                is_dynamic: true,
                world_pos: glam::Vec2::new(0.0, 10.0),
                velocity: glam::Vec2::ZERO,
                fade_in_samples: 0,
                fade_out_samples: 0,
                filter_state: [0.0, 0.0],
                filter_a: 0.0,
                user_gain: 1.0,
                current_gains: [0.99, 0.99],
                target_gains: [0.99, 0.99],
                current_itd: [0.0, 0.0],
                target_itd: [0.0, 0.0],
            }],
            play_rx,
            doppler_rx: None,
            garbage_tx,
            settings,
            listener_pos: glam::Vec2::ZERO,
            sample_rate,
            export_writer: None,
            block_index: 0,
            acc: vec![[0.0; 2]; block_size],
            last_log: Instant::now(),
            log_interval: Duration::from_secs(1),
            effect_flags: AudioEffectFlags::new_all_enabled(),
        };

        let mut chunked_output = Vec::with_capacity(total_samples);
        for _ in 0..total_blocks {
            dsp_chunk.acc.fill([0.0; 2]);
            let fx_mask_chunk = dsp_chunk.effect_flags.load();
            dsp_chunk.process_dsp(block_size, fx_mask_chunk, &profiler);
            chunked_output.extend_from_slice(&dsp_chunk.acc);
        }

        // 4. Vérification de l'équivalence parfaite et de la continuité de phase aux frontières
        for i in 0..total_samples {
            let ref_l = reference_output[i][0];
            let chunk_l = chunked_output[i][0];
            let diff = (ref_l - chunk_l).abs();

            assert!(
                diff < 1e-5,
                "Discontinuité détectée à l'échantillon {} ! Attendu: {}, Obtenu: {}, Diff: {}",
                i,
                ref_l,
                chunk_l,
                diff
            );

            // Vérification spécifique aux frontières de blocs (ex: 255 -> 256)
            if i > 0 && i % block_size == 0 {
                let slope_before = chunked_output[i][0] - chunked_output[i - 1][0];
                let ref_slope = reference_output[i][0] - reference_output[i - 1][0];

                assert!(
                    (slope_before - ref_slope).abs() < 1e-5,
                    "Rupture de pente (Phase Jump) à la frontière du bloc (Index {}) ! La dérivée n'est pas continue.",
                    i
                );
            }
        }
    }

    #[test]
    fn test_dsp_bypass_distance_attenuation() {
        use crate::audio_engine::binaural_processing::calculate_spatial_params_2d;
        use crate::audio_engine::effect_flags::{AudioEffect, DEFAULT_FLAGS};
        use crate::AudioEngineSettings;
        use glam::Vec2;

        let settings = AudioEngineSettings::default();
        let diff = Vec2::new(0.0, 100.0); // Source straight ahead, far away

        // 1. With distance attenuation active
        let params_active = calculate_spatial_params_2d(diff, &settings, DEFAULT_FLAGS);
        assert!(params_active.gain_left < 1.0);
        assert!(params_active.gain_right < 1.0);

        // 2. Without distance attenuation active
        let mut fx_mask = DEFAULT_FLAGS;
        fx_mask &= !(AudioEffect::DistanceAtten as u32);
        let params_bypass = calculate_spatial_params_2d(diff, &settings, fx_mask);
        assert_eq!(params_bypass.gain_left, 1.0);
        assert_eq!(params_bypass.gain_right, 1.0);
    }

    #[test]
    fn test_dsp_bypass_binaural_and_panning() {
        use crate::audio_engine::binaural_processing::calculate_spatial_params_2d;
        use crate::audio_engine::effect_flags::{AudioEffect, DEFAULT_FLAGS};
        use crate::AudioEngineSettings;
        use glam::Vec2;

        let settings = AudioEngineSettings {
            use_binaural: true,
            ..AudioEngineSettings::default()
        };
        let diff = Vec2::new(100.0, 0.0); // Source fully on the right side

        // 1. With Binaural active: left ear should be quieter than right ear
        let params_binaural = calculate_spatial_params_2d(diff, &settings, DEFAULT_FLAGS);
        assert!(params_binaural.gain_left < params_binaural.gain_right);

        // 2. Disable Binaural but keep Panning: left ear should be quieter (standard panning)
        let mut fx_mask = DEFAULT_FLAGS;
        fx_mask &= !(AudioEffect::Binaural as u32);
        let params_pan = calculate_spatial_params_2d(diff, &settings, fx_mask);
        assert!(params_pan.gain_left < params_pan.gain_right);

        // 3. Disable both Binaural and Panning: gains should be equal (flat center mono)
        fx_mask &= !(AudioEffect::Panning as u32);
        let params_bypass = calculate_spatial_params_2d(diff, &settings, fx_mask);
        assert_eq!(params_bypass.gain_left, params_bypass.gain_right);
    }

    #[test]
    fn test_dsp_bypass_doppler() {
        use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
        use crate::audio_engine::types::Voice;
        use crate::profiler::Profiler;
        use crate::AudioEngineSettings;
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        let sample_rate = 48_000;
        let block_size = 256;
        let source_audio = generate_sine_wave(440.0, sample_rate, block_size);
        let source_arc = Arc::new(source_audio);
        let (_play_tx, play_rx) = crossbeam_channel::unbounded();
        let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

        let mut dsp = super::DspProcessor {
            voices: vec![Voice {
                id: 1,
                active: true,
                data: Some(source_arc),
                pos: 0.0,
                playback_rate: 2.0, // Pre-configured moving doppler rate
                is_dynamic: true,
                world_pos: glam::Vec2::new(0.0, 10.0),
                velocity: glam::Vec2::new(0.0, -100.0), // Moving fast toward listener
                fade_in_samples: 0,
                fade_out_samples: 0,
                filter_state: [0.0, 0.0],
                filter_a: 0.0,
                user_gain: 1.0,
                current_gains: [1.0, 1.0],
                target_gains: [1.0, 1.0],
                current_itd: [0.0, 0.0],
                target_itd: [0.0, 0.0],
            }],
            play_rx,
            doppler_rx: None,
            garbage_tx,
            settings: AudioEngineSettings::default(),
            listener_pos: glam::Vec2::ZERO,
            sample_rate,
            export_writer: None,
            block_index: 0,
            acc: vec![[0.0; 2]; block_size],
            last_log: Instant::now(),
            log_interval: Duration::from_secs(1),
            effect_flags: AudioEffectFlags::new_all_enabled(),
        };

        // Disable Doppler
        dsp.effect_flags.set(AudioEffect::Doppler, false);
        let fx_mask = dsp.effect_flags.load();

        let profiler = Profiler::new(1000);
        dsp.process_doppler(fx_mask, &profiler);

        // Doppler processed but disabled -> playback_rate should be reset to 1.0
        assert_eq!(dsp.voices[0].playback_rate, 1.0);
    }

    #[test]
    fn test_dsp_bypass_normalization() {
        use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
        use crate::profiler::Profiler;
        use std::time::{Duration, Instant};

        let sample_rate = 48_000;
        let block_size = 8;
        let (_play_tx, play_rx) = crossbeam_channel::unbounded();
        let (garbage_tx, _garbage_rx) = crossbeam_channel::unbounded();

        let mut dsp = super::DspProcessor {
            voices: vec![],
            play_rx,
            doppler_rx: None,
            garbage_tx,
            settings: crate::AudioEngineSettings::default(),
            listener_pos: glam::Vec2::ZERO,
            sample_rate,
            export_writer: None,
            block_index: 0,
            acc: vec![[1.5, -2.0]; block_size], // Acc holds values that exceed 1.0
            last_log: Instant::now(),
            log_interval: Duration::from_secs(1),
            effect_flags: AudioEffectFlags::new_all_enabled(),
        };

        let mut output_data = vec![0.0; block_size * 2];
        let profiler = Profiler::new(1000);

        // 1. With Normalization enabled (GainStage & SoftClip)
        let fx_mask = dsp.effect_flags.load();
        dsp.write_cpal_buffer(&mut output_data, block_size, 0.8, fx_mask, &profiler); // global_gain = 0.8
                                                                                      // Soft clipping via tanh + global gain should smoothly compress
        assert!(output_data[0] < 1.0 && output_data[0] > 0.0);
        assert!(output_data[1] > -1.0 && output_data[1] < 0.0);
        assert!((output_data[0] - (1.5_f32 * 0.8).tanh()).abs() < 1e-5);

        // 2. With Normalization disabled
        dsp.effect_flags.set(AudioEffect::Normalization, false);
        let fx_mask_bypass = dsp.effect_flags.load();
        dsp.write_cpal_buffer(&mut output_data, block_size, 0.8, fx_mask_bypass, &profiler);
        // Normalization stage bypassed -> no global gain scaling, raw clamping to [-1.0, 1.0]
        assert_eq!(output_data[0], 1.0);
        assert_eq!(output_data[1], -1.0);
    }

    #[test]
    fn test_audio_effect_flags_set_all() {
        use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};

        let flags = AudioEffectFlags::new_all_enabled();
        // Initially all are enabled
        assert!(flags.is_enabled(AudioEffect::Binaural));
        assert!(flags.is_enabled(AudioEffect::Panning));

        // Disable all
        flags.set_all(false);
        assert!(!flags.is_enabled(AudioEffect::Binaural));
        assert!(!flags.is_enabled(AudioEffect::Panning));
        assert_eq!(flags.load(), 0);

        // Enable all
        flags.set_all(true);
        assert!(flags.is_enabled(AudioEffect::Binaural));
        assert!(flags.is_enabled(AudioEffect::Panning));
    }
}

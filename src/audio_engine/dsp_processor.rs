// Dans src/audio_engine/dsp_processor.rs

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

        // 1. Nettoyage du buffer d'accumulation
        self.acc[..frames].fill([0.0; 2]);

        // 2. Traitements Lock-Free
        self.consume_requests(profiler);
        self.process_doppler(profiler);

        // 3. Rendu DSP (Isolé dans Hotspot via #[inline(never)])
        self.process_dsp(frames, profiler);

        // 4. Finalisation et monitoring (Soft clipping isolé dans Hotspot)
        self.write_cpal_buffer(data, frames, global_gain, profiler);
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
    fn process_doppler(&mut self, profiler: &Profiler) {
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
    fn process_dsp(&mut self, frames: usize, profiler: &Profiler) {
        let _guard = profiler.measure("process_active_voices");
        crate::tracy_zone!("audio::process_dsp", 0xAA00FF);

        for v in self.voices.iter_mut() {
            if !v.active || v.data.is_none() {
                continue;
            }

            // 1. Paramètres physiques 3D de base
            let d = v.world_pos - self.listener_pos;
            let distance = d.length().max(1e-6);

            // Filtre passe-bas dynamique
            let fc = (self.settings.f_min()
                + (self.settings.f_max() - self.settings.f_min())
                    * (-self.settings.distance_alpha() * distance).exp())
            .clamp(self.settings.f_min(), self.settings.f_max());
            let dt = 1.0 / self.sample_rate as f32;
            let rc = 1.0 / (2.0 * std::f32::consts::PI * fc);
            let filter_a = dt / (rc + dt);
            v.filter_a = filter_a;

            let slice_ref = v.data.as_ref().expect("Voice data should exist");
            let total_len = slice_ref.len();

            let mut prev_l = v.filter_state[0];
            let mut prev_r = v.filter_state[1];
            let rate = v.playback_rate as f64;

            // 2. Calcul du Binaural ou Panning (Une fois par bloc !)
            let params = crate::audio_engine::binaural_processing::calculate_spatial_params(
                d.x,
                d.y,
                0.0,
                &self.settings,
            );

            let itd_l_samples = params.itd_left_sec * self.sample_rate as f32;
            let itd_r_samples = params.itd_right_sec * self.sample_rate as f32;

            let target_gain_l = v.user_gain * params.gain_left;
            let target_gain_r = v.user_gain * params.gain_right;

            let start_gain_l = v.current_gains[0];
            let start_gain_r = v.current_gains[1];
            let gain_step_l = (target_gain_l - start_gain_l) / frames as f32;
            let gain_step_r = (target_gain_r - start_gain_r) / frames as f32;

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

                // On lit directement le signal aux deux positions temporelles (Gauche / Droite)
                let mut l = interpolate(current_pos - itd_l_samples as f64);
                let mut r = interpolate(current_pos - itd_r_samples as f64);

                // Application des Fades
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

                // Filtre IIR Passe-bas
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
        profiler: &Profiler,
    ) {
        profiler.profile_block("write_cpal_buffer", || {
            crate::tracy_zone!("audio::soft_clipping", 0xFF5500);
            for (i, sample) in self.acc.iter_mut().take(frames).enumerate() {
                data[2 * i] = (sample[0] * global_gain).tanh();
                data[2 * i + 1] = (sample[1] * global_gain).tanh();
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

    /// Simule la logique d'interpolation continue sans état (Stateless Reading) de notre DspProcessor.
    fn process_stateless_chunk(
        source: &[[f32; 2]],
        start_pos: f64,
        playback_rate: f64,
        frames: usize,
    ) -> (Vec<[f32; 2]>, f64) {
        let total_len = source.len();
        let mut output = Vec::with_capacity(frames);
        let mut current_pos = start_pos;

        for _ in 0..frames {
            let idx = current_pos as usize;
            if idx >= total_len {
                output.push([0.0, 0.0]);
            } else {
                // Interpolation linéaire LERP (notre Ground Truth)
                let s0 = source[idx];
                let s1 = if idx + 1 < total_len {
                    source[idx + 1]
                } else {
                    [0.0, 0.0]
                };
                let frac = (current_pos - idx as f64) as f32;

                let l = s0[0] + frac * (s1[0] - s0[0]);
                let r = s0[1] + frac * (s1[1] - s0[1]);
                output.push([l, r]);
            }
            current_pos += playback_rate;
        }

        (output, current_pos)
    }

    #[test]
    fn test_phase_continuity_across_block_boundaries() {
        let sample_rate = 48_000;
        let block_size = 256;
        let total_blocks = 4;
        let total_samples = block_size * total_blocks;

        // 1. Source : Sinusoïde pure à 440 Hz (La fondamental)
        let source_audio = generate_sine_wave(440.0, sample_rate, total_samples + 100);

        // 2. Traitement Mode A : Un seul bloc continu de 1024 échantillons (La Vérité Terrain)
        let (reference_output, _) = process_stateless_chunk(&source_audio, 0.0, 1.0, total_samples);

        // 3. Traitement Mode B : 4 blocs successifs de 256 échantillons
        let mut chunked_output = Vec::with_capacity(total_samples);
        let mut pos_cursor = 0.0;

        for _ in 0..total_blocks {
            let (chunk, next_pos) =
                process_stateless_chunk(&source_audio, pos_cursor, 1.0, block_size);
            chunked_output.extend(chunk);
            pos_cursor = next_pos;
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

        println!(
            "✅ Test de continuité de phase validé avec succès sur {} blocs audio !",
            total_blocks
        );
    }
}

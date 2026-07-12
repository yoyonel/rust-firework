// Dans src/audio_engine/dsp_processor.rs

use crate::audio_engine::binaural_processing::binauralize_mono_fast_into; // 🎯 IMPORT D'ÉTAPE 1
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
    pub listener_pos: (f32, f32),
    pub sample_rate: u32,
    pub export_writer: Option<Arc<Mutex<SafeWavWriter>>>,
    pub block_index: u64,
    pub acc: Vec<[f32; 2]>,
    pub last_log: Instant,
    pub log_interval: Duration,

    /// 🎯 TAMPON DE BROUILLON MONO : Alloué 1 seule fois au démarrage.
    pub scratch_mono: Vec<f32>,
    /// 🎯 TAMPON DE BROUILLON STÉRÉO : Alloué 1 seule fois au démarrage.
    /// Reste chaud dans le cache L1/L2 du CPU à chaque bloc audio !
    pub scratch_stereo: Vec<[f32; 2]>,
}

impl DspProcessor {
    /// Point d'entrée principal du callback CPAL
    #[inline(always)]
    pub fn process_block(&mut self, data: &mut [f32], global_gain: f32, profiler: &Profiler) {
        let _audio_frame_guard = profiler.measure("audio_frame");
        let frames = data.len() / 2;

        if frames > self.acc.len()
            || frames > self.scratch_mono.len()
            || frames > self.scratch_stereo.len()
        {
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

                    let dx = v.world_pos.0 - self.listener_pos.0;
                    let dy = v.world_pos.1 - self.listener_pos.1;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.001);

                    let dir_x = -dx / dist;
                    let dir_y = -dy / dist;
                    let v_radial = v.velocity.0 * dir_x + v.velocity.1 * dir_y;

                    let c = 343.0_f32;
                    v.playback_rate = (c / (c - v_radial)).clamp(0.25, 4.0);

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

            // 1. Calcul des paramètres physiques au Block-Rate (une fois par bloc de ~5ms)
            let dx = v.world_pos.0 - self.listener_pos.0;
            let dy = v.world_pos.1 - self.listener_pos.1;
            let distance = (dx * dx + dy * dy).sqrt();

            let att = (1.0 - distance / self.settings.max_distance()).max(0.0);

            // Filtre passe-bas dynamique en fonction de la distance
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
            let gain = v.user_gain * att; // Gain combiné avec l'atténuation de distance

            // 2. Extraction LERP (Doppler) + Filtre dans le Scratchpad Mono (au Sample-Rate)
            // ZÉRO ALLOCATION : on écrit directement dans self.scratch_mono dans le cache L1 !
            for mono_out in self.scratch_mono[..frames].iter_mut() {
                let current_pos_f = v.pos;
                let index = current_pos_f as usize;

                if index >= total_len {
                    *mono_out = 0.0; // Silence de sécurité en fin de buffer
                    continue;
                }

                let sample0 = slice_ref[index];
                let sample1 = if index + 1 < total_len {
                    slice_ref[index + 1]
                } else {
                    [0.0, 0.0]
                };
                let frac = (current_pos_f - index as f64) as f32;

                // Interpolation linéaire (LERP) sur les canaux d'origine
                let mut l = sample0[0] + frac * (sample1[0] - sample0[0]);
                let mut r = sample0[1] + frac * (sample1[1] - sample0[1]);

                // Fade-in / Fade-out
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

                // Filtre passe-bas IIR
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

                // Somme mono pour alimenter l'étage de spatialisation
                *mono_out = (l + r) * 0.5 * gain;
                v.pos += rate;
            }

            v.filter_state[0] = prev_l;
            v.filter_state[1] = prev_r;

            // 3. Spatialisation 3D (Binaural ou Panning) dans le Scratchpad Stéréo
            if self.settings.use_binaural() {
                // ZÉRO ALLOCATION : binauralise depuis scratch_mono vers scratch_stereo
                binauralize_mono_fast_into(
                    &self.scratch_mono[..frames],
                    &mut self.scratch_stereo[..frames],
                    (v.world_pos.0, v.world_pos.1, 0.0),
                    (self.listener_pos.0, self.listener_pos.1, 0.0),
                    self.sample_rate,
                    &self.settings,
                );

                // Sommation vectorisée par LLVM dans le buffer d'accumulation principal
                for (acc_frame, &spatial_frame) in self.acc[..frames]
                    .iter_mut()
                    .zip(&self.scratch_stereo[..frames])
                {
                    acc_frame[0] += spatial_frame[0];
                    acc_frame[1] += spatial_frame[1];
                }
            } else {
                // Panning 2D basique si le binaural est désactivé
                let pan = (dx / self.settings.max_distance()).clamp(-1.0, 1.0);
                let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
                let gain_l = angle.cos();
                let gain_r = angle.sin();

                for (acc_frame, &mono_sample) in self.acc[..frames]
                    .iter_mut()
                    .zip(&self.scratch_mono[..frames])
                {
                    acc_frame[0] += mono_sample * gain_l;
                    acc_frame[1] += mono_sample * gain_r;
                }
            }

            // Désactivation de la voix si le pointeur dépasse la taille totale
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

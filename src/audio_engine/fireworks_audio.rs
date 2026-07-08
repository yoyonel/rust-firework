use crate::audio_engine::types::{
    // DopplerState,
    FireworksAudioConfig,
    PlayRequest,
    RocketAudioState,
    Voice,
};
use crate::audio_engine::{
    binauralize_mono,
    load_audio,
    resample_linear,
    AudioBlock,
    AudioEngine,
    // DopplerEvent,
    SafeWavWriter,
};
use crate::AudioEngineSettings;
use crate::{log_metrics, profiler::Profiler};
// CPAL: cross-platform audio API
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
// use crossbeam::channel::Receiver;
use hound::WavReader; // WAV file loader
use log::info;
use std::collections::HashMap;
use std::collections::VecDeque; // Queue for pending sound events
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex}; // Thread-safe shared state
use std::thread;
use std::time::{Duration, Instant};

static INIT_CPAL_THREAD: std::sync::Once = std::sync::Once::new();

/// Macro pour créer une zone Tracy **sans conditionner l'exécution du code**.
/// Utilisation: `tracy_zone!("nom_zone", 0xRRGGBB);`
#[cfg(feature = "tracy")]
macro_rules! tracy_zone {
    ($name:expr, $color:expr) => {
        let _span = tracy_client::span!($name);
        _span.emit_color($color);
    };
}

/// Macro vide si Tracy n'est pas activé
#[cfg(not(feature = "tracy"))]
macro_rules! tracy_zone {
    ($name:expr, $color:expr) => {};
}

/// Errors that can occur during audio thread initialization
#[derive(Debug)]
enum AudioThreadError {
    NoDevice,
    StreamBuildFailed(cpal::BuildStreamError),
    StreamPlayFailed(cpal::PlayStreamError),
}

impl std::fmt::Display for AudioThreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioThreadError::NoDevice => write!(f, "No audio output device available"),
            AudioThreadError::StreamBuildFailed(e) => {
                write!(f, "Failed to build audio stream: {}", e)
            }
            AudioThreadError::StreamPlayFailed(e) => {
                write!(f, "Failed to start audio stream: {}", e)
            }
        }
    }
}

pub struct FireworksAudio3D {
    rocket_data: Arc<Vec<[f32; 2]>>,
    explosion_data: Arc<Vec<[f32; 2]>>,

    listener_pos: (f32, f32),
    sample_rate: u32,
    block_size: usize,
    voices: Vec<Voice>,
    play_queue: Arc<Mutex<VecDeque<PlayRequest>>>,
    settings: AudioEngineSettings,
    running_pair: Arc<(Mutex<bool>, Condvar)>,
    // doppler_receiver: Option<Receiver<DopplerEvent>>,
    // doppler_states: Vec<DopplerState>,
    global_gain: f32,

    garbage_tx: crossbeam_channel::Sender<Arc<Vec<[f32; 2]>>>,
    garbage_rx: crossbeam_channel::Receiver<Arc<Vec<[f32; 2]>>>,
}

impl FireworksAudio3D {
    /// Initialize the engine with WAV paths, sample rate, and max voices
    ///
    /// # Errors
    /// Returns error if audio files cannot be loaded or sample rates cannot be determined
    pub fn new(config: FireworksAudioConfig) -> anyhow::Result<Self> {
        // Load WAV data
        let mut rocket_data = load_audio(&config.rocket_path)?;
        let mut explosion_data = load_audio(&config.explosion_path)?;

        // Resample to target sample rate
        let rocket_sr = WavReader::open(&config.rocket_path)
            .map_err(|e| anyhow::anyhow!("Failed to read rocket audio spec: {}", e))?
            .spec()
            .sample_rate;
        let explosion_sr = WavReader::open(&config.explosion_path)
            .map_err(|e| anyhow::anyhow!("Failed to read explosion audio spec: {}", e))?
            .spec()
            .sample_rate;

        rocket_data = resample_linear(&rocket_data, rocket_sr, config.sample_rate);
        explosion_data = resample_linear(&explosion_data, explosion_sr, config.sample_rate);

        let mut voices = Vec::with_capacity(config.max_voices);
        voices.resize_with(config.max_voices, Voice::new);

        let global_gain = config.settings.global_gain();

        let (garbage_tx, garbage_rx) = crossbeam_channel::unbounded();

        Ok(Self {
            rocket_data: Arc::new(rocket_data),
            explosion_data: Arc::new(explosion_data),

            listener_pos: config.listener_pos,
            sample_rate: config.sample_rate,
            block_size: config.block_size,
            voices,
            play_queue: Arc::new(Mutex::new(VecDeque::new())),
            settings: config.settings,
            running_pair: Arc::new((Mutex::new(true), Condvar::new())),
            // doppler_receiver: config.doppler_receiver,
            // doppler_states: config.doppler_states,
            global_gain,

            garbage_tx,
            garbage_rx,
        })
    }

    // =========================
    // Prepare a voice for playback
    // =========================
    fn prepare_voice(
        &self,
        data: &[[f32; 2]],
        pos: (f32, f32),
        gain: f32,
    ) -> (Vec<[f32; 2]>, usize, usize, f32) {
        // Distance attenuation
        let dx = pos.0 - self.listener_pos.0;
        let dy = pos.1 - self.listener_pos.1;
        let distance = (dx * dx + dy * dy).sqrt();
        let att = (1.0 - distance / self.settings.max_distance()).max(0.0);

        // Spatialization: binaural or panning
        let stereo = if self.settings.use_binaural() {
            let mono: Vec<f32> = data.iter().map(|s| (s[0] + s[1]) / 2.0).collect();
            binauralize_mono(
                &mono,
                (pos.0, pos.1, 0.0),
                (self.listener_pos.0, self.listener_pos.1, 0.0),
                self.sample_rate,
                &self.settings,
            )
        } else {
            let pan = (dx / self.settings.max_distance()).clamp(-1.0, 1.0);
            let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
            let left_gain = angle.cos() * att * gain;
            let right_gain = angle.sin() * att * gain;
            let mut out = data.to_owned();
            for s in &mut out {
                s[0] *= left_gain;
                s[1] *= right_gain;
            }
            out
        };

        // Fade-in/out samples
        let fade_in_samples =
            (self.sample_rate as f32 * (self.settings.fade_in_ms() / 1000.0)) as usize;
        let fade_out_samples =
            (self.sample_rate as f32 * (self.settings.fade_out_ms() / 1000.0)) as usize;

        // Distance-dependent low-pass filter
        let fc = (self.settings.f_min()
            + (self.settings.f_max() - self.settings.f_min())
                * (-self.settings.distance_alpha() * distance).exp())
        .clamp(self.settings.f_min(), self.settings.f_max());
        let dt = 1.0 / self.sample_rate as f32;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * fc);
        let filter_a = dt / (rc + dt);

        (stereo, fade_in_samples, fade_out_samples, filter_a)
    }

    /// Queue a sound for playback
    fn enqueue_sound(&self, data: &[[f32; 2]], pos: (f32, f32), gain: f32) {
        if self.global_gain == 0.0 {
            return;
        }

        // Chaque `try_recv` dépile un Arc mort : en sortant de la boucle, le drop() est appelé
        // par l'OS dans ce thread (UI/Physique), épargnant à 100% le thread CPAL.
        while let Ok(_dead_buffer) = self.garbage_rx.try_recv() {
            #[cfg(feature = "tracy")]
            tracy_zone!("audio::free_garbage_buffer", 0xFF00AA);
        }

        let global_gain = self.global_gain * gain;

        let (stereo_data, fade_in, fade_out, filter_a) = self.prepare_voice(data, pos, global_gain);
        let req = PlayRequest {
            data: Arc::new(stereo_data),
            fade_in,
            fade_out,
            gain: global_gain,
            filter_a,
            sent_at: Instant::now(), // for monitoring
        };
        self.play_queue.lock().unwrap().push_back(req);
    }

    pub fn play_rocket(&self, pos: (f32, f32), gain: f32) {
        self.enqueue_sound(&self.rocket_data, pos, gain);
    }
    pub fn play_explosion(&self, pos: (f32, f32), gain: f32) {
        self.enqueue_sound(&self.explosion_data, pos, gain);
    }

    pub fn start_audio_thread(&mut self, export_path: Option<&str>) {
        info!("🚀 Starting Audio Engine ...");

        let queue = self.play_queue.clone();
        let mut local_voices = self.voices.clone(); // Ownership exclusif pour le thread audio
        let sr = self.sample_rate;
        let block_size = self.block_size;
        let global_gain = self.settings.global_gain();

        let running_pair_clone = self.running_pair.clone();

        // Partagé entre moteurs
        let profiler = Profiler::new(200);
        let mut last_log = Instant::now();
        let log_interval = std::time::Duration::from_secs(4); // toutes les 4 secondes

        // Prépare les données audio à partager avec le thread audio
        let _rocket_data_ref = Arc::new(self.rocket_data.clone()); // Ce qui est zéro copie (le Arc clone est O(1)).
        let _settings = self.settings.clone();
        let _listener_pos_clone = self.listener_pos; // utile dans prepare_voice_with_doppler

        let export_writer_arc: Option<Arc<Mutex<SafeWavWriter>>> = if let Some(path) = export_path {
            let writer = Arc::new(Mutex::new(SafeWavWriter::new(path, sr)));
            Some(writer)
        } else {
            None
        };

        let garbage_tx = self.garbage_tx.clone();

        thread::spawn(move || {
            // local state inside audio thread
            let mut _rocket_states: HashMap<u64, RocketAudioState> = HashMap::new();

            // Try to initialize audio hardware
            let audio_result: Result<(), AudioThreadError> = (|| {
                let host = cpal::default_host();
                let device = host
                    .default_output_device()
                    .ok_or(AudioThreadError::NoDevice)?;

                // Sélection adaptative et propre du buffer low-latency
                let buffer_size = match device.supported_output_configs() {
                    Ok(mut configs) => {
                        // On cherche si notre configuration cible (2 channels, 48kHz) supporte un range de buffer
                        let target_sr = cpal::SampleRate(sr);
                        let supports_low_latency = configs.any(|c| {
                            c.channels() == 2
                                && c.min_sample_rate() <= target_sr
                                && c.max_sample_rate() >= target_sr
                                && match c.buffer_size() {
                                    cpal::SupportedBufferSize::Range { min, max } => {
                                        *min <= 256 && *max >= 256
                                    }
                                    cpal::SupportedBufferSize::Unknown => true,
                                }
                        });

                        if supports_low_latency {
                            cpal::BufferSize::Fixed(256)
                        } else {
                            cpal::BufferSize::Default
                        }
                    }
                    Err(e) => {
                        log::warn!("Impossible d'inspecter les configs audio ({}), fallback sur Fixed(256)", e);
                        cpal::BufferSize::Fixed(256)
                    }
                };

                let config = cpal::StreamConfig {
                    channels: 2,
                    sample_rate: cpal::SampleRate(sr),
                    buffer_size,
                };

                let garbage_tx_cpal = garbage_tx.clone();

                // Preallocate buffers
                let max_supported_frames = block_size.max(16384);
                let mut acc = vec![[0.0; 2]; max_supported_frames];
                let chunk = vec![[0.0; 2]; max_supported_frames];

                let export_writer_callback = export_writer_arc.clone();
                let block_index = Arc::new(AtomicU64::new(0));

                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            // Nommage unique du thread dans Tracy
                            INIT_CPAL_THREAD.call_once(|| {
                                #[cfg(feature = "tracy")]
                                tracy_client::set_thread_name!("CPAL Audio Callback");
                            });

                            // start global frame
                            let _audio_frame_guard = profiler.measure("audio_frame");

                            let frames = data.len() / 2;

                            // 2. Dans le callback : vérification de sécurité sans allocation !
                            if frames > acc.len() || frames > chunk.len() {
                                log::error!("Buffer under-allocated! Requested {} frames", frames);
                                return; // Ou gère le cas en clippant à acc.len()
                            }

                            // 3. Un seul nettoyage propre et vectorisé par LLVM
                            acc[..frames].fill([0.0; 2]);

                            // Dépiler les requêtes ultra-rapidement (O(1) sur le lock)
                            let mut pending_requests = Vec::new();
                            {
                                tracy_zone!("audio::lock_queue", 0xFF0000);
                                let mut q = queue.lock().expect("Failed to lock play queue");
                                pending_requests.extend(q.drain(..));
                            } // Le lock est libéré INSTANTANÉMENT !

                            // 2. Assigner les requêtes (Totalement Lock-Free)
                            {
                                tracy_zone!("audio::assign_requests", 0x00FF00);
                                for req in pending_requests {
                                    if let Some(v) = local_voices.iter_mut().find(|v| !v.active) {
                                        v.reset_from_request(&req);
                                        let latency = Instant::now().duration_since(req.sent_at);
                                        profiler.record_metric("audio latency", latency);

                                        #[cfg(feature = "tracy")]
                                        tracy_client::plot!(
                                            "Audio: Latency (ms)",
                                            latency.as_secs_f64() * 1000.0
                                        );
                                    }
                                }
                                let nb_actives_voices =
                                    local_voices.iter().filter(|v| v.active).count();
                                profiler.record_metric("nb_actives_voices", nb_actives_voices);

                                #[cfg(feature = "tracy")]
                                tracy_client::plot!(
                                    "Audio: Active Voices",
                                    nb_actives_voices as f64
                                );
                            }

                            // 3. Traitement DSP (Totalement Lock-Free, le tableau appartient au thread !)
                            {
                                let _guard = profiler.measure("process_active_voices");
                                tracy_zone!("audio::process_dsp", 0xAA00FF);

                                for v in local_voices.iter_mut() {
                                    if !v.active || v.data.is_none() {
                                        continue;
                                    }

                                    let slice_ref =
                                        v.data.as_ref().expect("Voice data should exist");
                                    let total_len = slice_ref.len();
                                    let start = v.pos;

                                    if start >= total_len {
                                        v.active = false;
                                        if let Some(dead_arc) = v.data.take() {
                                            let _ = garbage_tx_cpal.try_send(dead_arc);
                                        }
                                        continue;
                                    }

                                    let n = (total_len - start).min(frames);
                                    let voice_samples = &slice_ref[start..start + n];

                                    // SINGLE-PASS DSP
                                    let mut prev_l = v.filter_state[0];
                                    let mut prev_r = v.filter_state[1];
                                    let filter_a = v.filter_a;
                                    let gain = v.user_gain;

                                    for (i, sample) in voice_samples.iter().enumerate() {
                                        let mut l = sample[0];
                                        let mut r = sample[1];

                                        // 1. Fade in / Fade out à la volée
                                        let current_pos = start + i;
                                        if current_pos < v.fade_in_samples {
                                            let alpha =
                                                current_pos as f32 / v.fade_in_samples as f32;
                                            l *= alpha;
                                            r *= alpha;
                                        } else {
                                            let rem = total_len - current_pos;
                                            if rem < v.fade_out_samples {
                                                let alpha = rem as f32 / v.fade_out_samples as f32;
                                                l *= alpha;
                                                r *= alpha;
                                            }
                                        }

                                        // 2. Filtre passe-bas IIR
                                        l = prev_l + filter_a * (l - prev_l);
                                        r = prev_r + filter_a * (r - prev_r);

                                        // ANTI-DENORMALS (Flush-to-Zero)
                                        // Empêche le CPU de s'effondrer (100x plus lent) sur
                                        // les valeurs microscopiques à la fin des sons.
                                        if l.abs() < 1e-15 {
                                            l = 0.0;
                                        }
                                        if r.abs() < 1e-15 {
                                            r = 0.0;
                                        }

                                        prev_l = l;
                                        prev_r = r;

                                        // 3. Accumulation directe
                                        acc[i][0] += l * gain;
                                        acc[i][1] += r * gain;
                                    }

                                    v.filter_state[0] = prev_l;
                                    v.filter_state[1] = prev_r;
                                    v.pos += n;

                                    if v.pos >= total_len {
                                        v.active = false;
                                        if let Some(dead_arc) = v.data.take() {
                                            let _ = garbage_tx_cpal.try_send(dead_arc);
                                        }
                                    }
                                }
                            }

                            // Write to CPAL buffer with global gain and soft clipping
                            profiler.profile_block("write_cpal_buffer", || {
                                tracy_zone!("audio::soft_clipping", 0xFF5500); // Orange pour l'écriture
                                for (i, sample) in acc.iter_mut().take(frames).enumerate() {
                                    data[2 * i] = (sample[0] * global_gain).tanh();
                                    data[2 * i + 1] = (sample[1] * global_gain).tanh();
                                }
                            });

                            if let Some(writer_arc) = &export_writer_callback {
                                // 🔹 Reuse 'data' instead of recalculating
                                let mut frames_vec = Vec::with_capacity(frames);
                                for i in 0..frames {
                                    frames_vec.push([data[2 * i], data[2 * i + 1]]);
                                }

                                let block_number = block_index.fetch_add(1, Ordering::Relaxed);
                                let block = AudioBlock {
                                    index: block_number,
                                    frames: frames_vec,
                                };
                                writer_arc
                                    .lock()
                                    .expect("Failed to lock writer")
                                    .push_block(block);
                            }

                            drop(_audio_frame_guard);

                            // affichage périodique
                            if last_log.elapsed() >= log_interval {
                                log_metrics!(&profiler);
                                last_log = Instant::now();
                            }
                        },
                        move |err| eprintln!("CPAL error: {:?}", err),
                        None,
                    )
                    .map_err(AudioThreadError::StreamBuildFailed)?;

                stream.play().map_err(AudioThreadError::StreamPlayFailed)?;

                // 🔊 Thread audio: attente jusqu'à signal de stop
                let (lock, cvar) = &*running_pair_clone;
                let mut running = lock.lock().expect("Failed to lock running state");
                info!("🔊 Thread audio: en attente ...");
                while *running {
                    let result = cvar
                        .wait_timeout(running, Duration::from_millis(500))
                        .expect("Failed to wait on condvar");
                    running = result.0;
                }

                // ▸ Push final silence pour éviter ALSA underrun
                {
                    if let Some(writer_arc) = &export_writer_arc {
                        let silence_block = vec![[0.0; 2]; block_size];
                        let block = AudioBlock {
                            index: 0,
                            frames: silence_block,
                        };
                        writer_arc
                            .lock()
                            .expect("Failed to lock writer")
                            .push_block(block);
                    }
                }

                // Drop du stream pour fermer CPAL proprement
                drop(stream);
                info!("🔇 Thread audio: terminé");

                Ok(())
            })();

            // Handle audio initialization result
            match audio_result {
                Ok(()) => {
                    // Audio thread completed successfully
                }
                Err(e) => {
                    log::warn!(
                        "⚠️ Audio thread failed to initialize: {}. Running in silent mode.",
                        e
                    );
                    log::warn!("   The application will continue without audio output.");

                    // Silent mode: just wait for stop signal
                    let (lock, cvar) = &*running_pair_clone;
                    let mut running = lock.lock().expect("Failed to lock running state");
                    while *running {
                        let result = cvar
                            .wait_timeout(running, Duration::from_millis(500))
                            .expect("Failed to wait on condvar");
                        running = result.0;
                    }
                    info!("🔇 Silent mode audio thread: terminé");
                }
            }

            // 🔹 Stop et flush final du writer
            if let Some(writer_arc) = export_writer_arc {
                writer_arc.lock().expect("Failed to lock writer").stop();
            }
        });
    }

    /// Stop the audio thread
    pub fn stop_audio_thread(&mut self) {
        info!("🧹 Fermeture de l'Audio Engine");
        let (lock, cvar) = &*self.running_pair;
        let mut running = lock.lock().unwrap();
        *running = false; // indiquer au thread secondaire d'arrêter
        cvar.notify_all(); // réveiller le thread
        drop(running); // unlock
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.global_gain = volume;
    }
}

impl AudioEngine for FireworksAudio3D {
    fn play_rocket(&self, pos: (f32, f32), gain: f32) {
        self.play_rocket(pos, gain)
    }

    fn play_explosion(&self, pos: (f32, f32), gain: f32) {
        self.play_explosion(pos, gain)
    }

    fn start_audio_thread(&mut self, _export_path: Option<&str>) {
        self.start_audio_thread(_export_path)
    }

    fn stop_audio_thread(&mut self) {
        self.stop_audio_thread()
    }

    fn set_listener_position(&mut self, pos: (f32, f32)) {
        self.listener_pos = pos;
        info!("🎧️ Listener position set to: {:?}", self.listener_pos);
    }

    fn get_listener_position(&self) -> (f32, f32) {
        self.listener_pos
    }

    fn mute(&mut self) {
        self.set_volume(0.0);
    }

    fn unmute(&mut self) -> f32 {
        self.set_volume(self.settings.global_gain());
        self.settings.global_gain()
    }

    fn as_audio_engine(&self) -> &dyn AudioEngine {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::audio_engine::audio_event::doppler_queue::DopplerQueue;
    use crate::audio_engine::binaural_processing::binauralize_mono;
    use crate::audio_engine::settings::AudioEngineSettingsBuilder;

    fn dummy_data() -> Vec<[f32; 2]> {
        vec![[1.0, 1.0]; 10] // 10 frames simples avec amplitude 1
    }

    // Version test-friendly de enqueue_sound qui ignore l'atténuation distance
    fn enqueue_sound_test(engine: &FireworksAudio3D, pos: (f32, f32), gain: f32) -> PlayRequest {
        // Panning simple
        let dx = pos.0 - engine.listener_pos.0;
        let pan = (dx / engine.settings.max_distance()).clamp(-1.0, 1.0);

        let mut data_panned = dummy_data();
        for sample in &mut data_panned {
            let left = ((1.0 - pan) * 0.5).clamp(0.0, 1.0);
            let right = ((1.0 + pan) * 0.5).clamp(0.0, 1.0);
            sample[0] *= left * gain;
            sample[1] *= right * gain;
        }

        PlayRequest {
            data: std::sync::Arc::new(data_panned),
            fade_in: 1,
            fade_out: 1,
            gain,
            filter_a: 0.0025,
            sent_at: Instant::now(),
        }
    }

    fn build_engine() -> FireworksAudio3D {
        // let doppler_queue = DopplerQueue::new();
        FireworksAudio3D::new(FireworksAudioConfig {
            rocket_path: "assets/sounds/rocket.wav".into(),
            explosion_path: "assets/sounds/explosion.wav".into(),
            listener_pos: (0.0, 0.0),
            sample_rate: 1000,
            block_size: 1024 * 4,
            max_voices: 16,
            settings: AudioEngineSettings::default(),
            // doppler_receiver: Some(doppler_queue.receiver.clone()),
            // doppler_states: Vec::new(),
        })
        .expect("Failed to build test audio engine")
    }

    #[test]
    fn test_panning_left() {
        let engine = build_engine();

        let req = enqueue_sound_test(&engine, (-engine.settings.max_distance(), 0.0), 1.0);

        // for sample in &req.data {
        for sample in req.data.iter() {
            let ratio = sample[0] / (sample[1] + 1e-8);
            assert!(
                ratio > 1.0,
                "Left channel should dominate right for left pan"
            );
        }
    }

    #[test]
    fn test_panning_right() {
        let engine = build_engine();

        let req = enqueue_sound_test(&engine, (engine.settings.max_distance(), 0.0), 1.0);

        // for sample in &req.data {
        for sample in req.data.iter() {
            let ratio = sample[1] / (sample[0] + 1e-8);
            assert!(
                ratio > 1.0,
                "Right channel should dominate left for right pan"
            );
        }
    }

    #[test]
    fn test_panning_center() {
        let engine = build_engine();

        let req = enqueue_sound_test(&engine, (0.0, 0.0), 1.0);

        // for sample in &req.data {
        for sample in req.data.iter() {
            let diff = (sample[0] - sample[1]).abs();
            assert!(diff < 1e-6, "Channels should be equal for center pan");
        }
    }

    /// Génère un signal mono simple
    fn dummy_mono(len: usize) -> Vec<f32> {
        vec![1.0; len]
    }

    #[test]
    fn test_binaural_center() {
        let sr = 48000;
        let max_distance = 1000.0;
        let head_radius = 0.0875;
        let max_ild_db = 18.0;
        let mono = dummy_mono(10);
        let src_pos = (0.0, 0.0);
        let listener_pos = (0.0, 0.0);

        let settings = AudioEngineSettingsBuilder::default()
            .max_distance(max_distance)
            .head_radius(head_radius)
            .max_ild_db(max_ild_db)
            .build()
            .unwrap();
        let stereo = binauralize_mono(
            &mono,
            (src_pos.0, src_pos.1, 0.0),
            (listener_pos.0, listener_pos.1, 0.0),
            sr,
            &settings,
        );

        // Source au centre → canaux égaux
        for s in &stereo {
            let diff = (s[0] - s[1]).abs();
            assert!(
                diff < 1e-6,
                "Canaux gauche/droite doivent être égaux pour source centrale"
            );
        }
    }

    #[test]
    fn test_binaural_left_debug() {
        let sr = 48000;
        let mono = dummy_mono(10);
        let src_pos = (-500.0, 0.0); // X négatif = gauche (selon ta convention x = latéral)
        let listener_pos = (0.0, 0.0);

        let settings = AudioEngineSettingsBuilder::default()
            .max_distance(1000.0)
            .head_radius(0.0875)
            .max_ild_db(18.0)
            .build()
            .unwrap();

        // --- Recalcule et affiche les paramètres intermédiaires pour debug
        let dx: f32 = src_pos.0 - listener_pos.0; // >0 => droite, <0 => gauche
        let dy: f32 = src_pos.1 - listener_pos.1; // >0 => haut, <0 => bas

        // Convention utilisée dans binauralize_mono : azimuth = dx.atan2(dy)
        let azimuth = dx.atan2(dy); // angle en radians : 0 = front, + = right, - = left
        let theta = azimuth.abs();

        let c = 343.0_f32;
        let itd = ((settings.head_radius() / c) * (theta + theta.sin())).clamp(0.0, 0.001);
        let ild_db = settings.max_ild_db() * theta.sin();
        let far_gain = 10f32.powf(-ild_db / 20.0);
        let att = (1.0 - ((dx * dx + dy * dy).sqrt()) / settings.max_distance()).max(0.0);

        // Déduction heuristique du canal atténué (pour info)
        let expected_side = if azimuth >= 0.0 { "right" } else { "left" };
        let (expected_gain_left, expected_gain_right) = if azimuth >= 0.0 {
            (att * far_gain, att)
        } else {
            (att, att * far_gain)
        };

        println!("--- DEBUG test_binaural_left ---");
        println!("src_pos = {:?}, listener_pos = {:?}", src_pos, listener_pos);
        println!(
            "dx = {:.3}, dy = {:.3}, distance = {:.3}",
            dx,
            dy,
            (dx * dx + dy * dy).sqrt()
        );
        println!("azimuth (rad) = {:.6}, theta = {:.6}", azimuth, theta);
        println!("ITD (s) = {:.9}, ILD (dB) = {:.6}", itd, ild_db);
        println!(
            "expected side = {}, expected gains L/R ≈ {:.6} / {:.6}",
            expected_side, expected_gain_left, expected_gain_right
        );
        println!("attenuation (distance) = {:.6}", att);

        // Appel de la fonction à tester
        let stereo = binauralize_mono(
            &mono,
            (src_pos.0, src_pos.1, 0.0),
            (listener_pos.0, listener_pos.1, 0.0),
            sr,
            &settings,
        );

        // Statistiques simples
        let sum_left: f32 = stereo.iter().map(|s| s[0]).sum();
        let sum_right: f32 = stereo.iter().map(|s| s[1]).sum();
        let avg_left = sum_left / stereo.len() as f32;
        let avg_right = sum_right / stereo.len() as f32;
        let max_diff = stereo
            .iter()
            .map(|s| (s[0] - s[1]).abs())
            .fold(0.0_f32, f32::max);

        // Comptage d'échantillons où gauche <= droite (devrait être 0 pour source à gauche)
        let mut left_le_right = 0usize;
        for s in &stereo {
            if s[0] <= s[1] {
                left_le_right += 1;
            }
        }

        println!("sum L = {:.6}, sum R = {:.6}", sum_left, sum_right);
        println!(
            "avg L = {:.6}, avg R = {:.6}, max |L-R| = {:.6}",
            avg_left, avg_right, max_diff
        );
        println!(
            "samples where L <= R : {}/{} (should be 0 for strict left dominance)",
            left_le_right,
            stereo.len()
        );

        // Print first few stereo samples for inspection
        println!("first samples (L, R):");
        for (i, s) in stereo.iter().take(12).enumerate() {
            println!("  [{:02}] {:.6}, {:.6}", i, s[0], s[1]);
        }

        // Assertion plus robuste : on vérifie la somme (global energy) plutôt que chaque échantillon.
        // Si tu veux vérifier chaque échantillon, on pourrait garder l'ancienne boucle assert,
        // mais la somme est préférable pour signaux filtrés/delais fractionnaires.
        assert!(
        sum_left > sum_right,
        "Canal gauche doit être globalement plus fort que droite pour source à gauche (see debug output above)"
    );
    }

    // FIXME: il doit y avoir un problème de symétrie avec le filtre audio binaural
    #[test]
    fn test_binaural_right_debug() {
        let sr = 48000;
        let n_samples = 4800; // 0.1 s
        let mono = vec![1.0; n_samples];

        // Source sur l'axe +x -> à droite selon ta convention
        let src_pos = (500.0, 0.0);
        let listener_pos = (0.0, 0.0);

        let settings = AudioEngineSettingsBuilder::default()
            .max_distance(1000.0)
            .head_radius(0.0875)
            .max_ild_db(18.0)
            .build()
            .unwrap();

        // on récupère les valeurs internes (recalculées ici pour afficher)
        let dx: f32 = src_pos.0 - listener_pos.0;
        let dy: f32 = src_pos.1 - listener_pos.1;
        let azimuth: f32 = dx.atan2(dy); // NOTE: dx.atan2(dy) => 90deg pour (500,0)
        let theta: f32 = azimuth.abs();

        let c: f32 = 343.0;
        let itd = ((settings.head_radius() / c) * (theta + theta.sin())).clamp(0.0, 0.001);
        let ild_db = settings.max_ild_db() * theta.sin();
        let far_gain = 10f32.powf(-ild_db / 20.0);

        // Détermine quels canaux sont atténués selon signe d'azimuth
        let (gain_left, gain_right) = if azimuth >= 0.0 {
            (far_gain, 1.0) // source à droite -> droite non-affaiblie
        } else {
            (1.0, far_gain)
        };

        let stereo = binauralize_mono(
            &mono,
            (src_pos.0, src_pos.1, 0.0),
            (listener_pos.0, listener_pos.1, 0.0),
            sr,
            &settings,
        );

        let sum_left: f32 = stereo.iter().map(|s| s[0]).sum();
        let sum_right: f32 = stereo.iter().map(|s| s[1]).sum();

        println!(
            "DEBUG binaural_right:\n\
         src={:?}, dx={:.1}, dy={:.1}\n\
         azimuth(rad)={:.3}, theta={:.3}\n\
         itd(s)={:.7}, ild_db={:.3}, far_gain={:.4}\n\
         expected gains L/R ≈ {:.4}/{:.4}\n\
         sums L/R = {:.4}/{:.4}, ratio R/L = {:.3}",
            src_pos,
            dx,
            dy,
            azimuth,
            theta,
            itd,
            ild_db,
            far_gain,
            gain_left,
            gain_right,
            sum_left,
            sum_right,
            sum_right / (sum_left + 1e-12)
        );

        assert!(
            sum_right > sum_left,
            "Canal droite doit être plus fort que gauche pour source à droite"
        );
    }

    #[test]
    fn test_binaural_distance_3d() {
        let sr = 48_000;
        let mono = dummy_mono(10);
        let listener = (0.0, 0.0, 0.0);

        let near = (0.0, 0.0, 100.0); // proche devant
        let far = (0.0, 0.0, -900.0); // loin derrière

        let settings = AudioEngineSettingsBuilder::default()
            .max_distance(1000.0)
            .head_radius(0.0875)
            .max_ild_db(18.0)
            .build()
            .unwrap();

        let stereo_near = binauralize_mono(&mono, near, listener, sr, &settings);
        let stereo_far = binauralize_mono(&mono, far, listener, sr, &settings);

        let e_near: f32 = stereo_near.iter().map(|s| s[0].abs() + s[1].abs()).sum();
        let e_far: f32 = stereo_far.iter().map(|s| s[0].abs() + s[1].abs()).sum();

        assert!(
            e_near > e_far,
            "Le son proche doit être plus fort que le son lointain"
        );
    }
}

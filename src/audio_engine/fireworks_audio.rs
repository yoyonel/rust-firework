use crate::audio_engine::types::{FireworksAudioConfig, PlayRequest, Voice};
use crate::audio_engine::{
    binauralize_mono_fast, load_audio, resample_linear, AudioBlock, AudioEngine, SafeWavWriter,
};
use crate::profiler::Profiler;
#[cfg(feature = "tracy")]
use crate::tracy_zone;
use crate::AudioEngineSettings;
// CPAL: cross-platform audio API
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
// use crossbeam::channel::Receiver;
use crossbeam_channel::{Receiver, Sender};
use hound::WavReader; // WAV file loader
use log::info;
use std::sync::{Arc, Condvar, Mutex}; // Thread-safe shared state
use std::thread;
use std::time::{Duration, Instant};

static INIT_CPAL_THREAD: std::sync::Once = std::sync::Once::new();

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

    play_tx: Sender<PlayRequest>,
    play_rx: Receiver<PlayRequest>,

    settings: AudioEngineSettings,
    running_pair: Arc<(Mutex<bool>, Condvar)>,
    global_gain: f32,

    garbage_tx: crossbeam_channel::Sender<Arc<Vec<[f32; 2]>>>,
    garbage_rx: crossbeam_channel::Receiver<Arc<Vec<[f32; 2]>>>,

    doppler_receiver: Option<Receiver<crate::audio_engine::DopplerEvent>>,
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

        // --- NOUVEAU : Ring buffer SPSC borné pour les requêtes audio ---
        let (play_tx, play_rx) = crossbeam_channel::bounded(512);

        Ok(Self {
            rocket_data: Arc::new(rocket_data),
            explosion_data: Arc::new(explosion_data),
            listener_pos: config.listener_pos,
            sample_rate: config.sample_rate,
            block_size: config.block_size,
            voices,
            play_tx,
            play_rx,
            settings: config.settings,
            running_pair: Arc::new((Mutex::new(true), Condvar::new())),
            global_gain,
            garbage_tx,
            garbage_rx,
            doppler_receiver: config.doppler_receiver, // MODIFIÉ : Initialisation
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
            binauralize_mono_fast(
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
    fn enqueue_sound(
        &self,
        id: u64,
        data: &[[f32; 2]],
        pos: (f32, f32),
        gain: f32,
        is_dynamic: bool,
    ) {
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

        let (stereo_data, fade_in, fade_out, filter_a) = if is_dynamic {
            // Pour le Doppler, on envoie la donnée BRUTE (non spatialisée)
            // On calcule quand même les fades initiaux en fonction de sample_rate
            let fade_in_samples =
                (self.sample_rate as f32 * (self.settings.fade_in_ms() / 1000.0)) as usize;
            let fade_out_samples =
                (self.sample_rate as f32 * (self.settings.fade_out_ms() / 1000.0)) as usize;

            // Le filtre sera recalculé dynamiquement, on met une valeur par défaut sûre
            (data.to_owned(), fade_in_samples, fade_out_samples, 0.05)
        } else {
            // Pour les explosions statiques, on garde l'optimisation existante (pré-calcul total)
            self.prepare_voice(data, pos, global_gain)
        };
        let req = PlayRequest {
            data: Arc::new(stereo_data),
            fade_in,
            fade_out,
            gain: global_gain,
            filter_a,
            sent_at: Instant::now(), // for monitoring
            id,                      // NOUVEAU
            pos,                     // NOUVEAU
            is_dynamic,              // NOUVEAU
        };

        if let Err(e) = self.play_tx.try_send(req) {
            log::warn!("⚠️ Audio play_queue full! Dropping sound event: {:?}", e);
        }
    }

    pub fn play_rocket(&self, pos: (f32, f32), gain: f32) {
        self.enqueue_sound(0, &self.explosion_data, pos, gain, false);
    }

    pub fn play_rocket_with_id(&self, id: u64, pos: (f32, f32), gain: f32) {
        self.enqueue_sound(id, &self.rocket_data, pos, gain, true);
    }

    pub fn play_explosion(&self, pos: (f32, f32), gain: f32) {
        self.enqueue_sound(0, &self.rocket_data, pos, gain, false);
    }

    pub fn start_audio_thread(&mut self, export_path: Option<&str>) {
        info!("🚀 Starting Audio Engine ...");

        let play_rx = self.play_rx.clone();
        let local_voices = self.voices.clone();
        let sr = self.sample_rate;
        let block_size = self.block_size;
        let global_gain = self.settings.global_gain();
        let running_pair_clone = self.running_pair.clone();

        let profiler = Profiler::new(200);
        let _settings = self.settings.clone();
        let doppler_rx_clone = self.doppler_receiver.clone();
        let listener_pos_clone = self.listener_pos;

        let export_writer_arc: Option<Arc<Mutex<SafeWavWriter>>> =
            export_path.map(|path| Arc::new(Mutex::new(SafeWavWriter::new(path, sr))));

        let garbage_tx = self.garbage_tx.clone();

        thread::spawn(move || {
            let audio_result: Result<(), AudioThreadError> = (|| {
                #[cfg(target_os = "linux")]
                unsafe {
                    libc::pthread_setname_np(libc::pthread_self(), c"cpal_audio_dsp".as_ptr());
                }

                let host = cpal::default_host();
                let device = host
                    .default_output_device()
                    .ok_or(AudioThreadError::NoDevice)?;

                // 1. Configuration Matérielle (déléguée !)
                let config = get_cpal_config(&device, sr);

                // 2. Instanciation du processeur DSP
                let max_supported_frames = block_size.max(16384);
                let mut dsp_processor = crate::audio_engine::dsp_processor::DspProcessor {
                    voices: local_voices,
                    play_rx,
                    doppler_rx: doppler_rx_clone,
                    garbage_tx,
                    settings: _settings,
                    listener_pos: listener_pos_clone,
                    sample_rate: sr,
                    export_writer: export_writer_arc.clone(),
                    block_index: 0,
                    acc: vec![[0.0; 2]; max_supported_frames],
                    last_log: Instant::now(),
                    log_interval: Duration::from_secs(4),
                };

                // 3. Lancement du Flux Audio
                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            INIT_CPAL_THREAD.call_once(|| {
                                #[cfg(feature = "tracy")]
                                tracy_client::set_thread_name!("CPAL Audio Callback");
                            });
                            dsp_processor.process_block(data, global_gain, &profiler);
                        },
                        move |err| eprintln!("CPAL error: {:?}", err),
                        None,
                    )
                    .map_err(AudioThreadError::StreamBuildFailed)?;

                stream.play().map_err(AudioThreadError::StreamPlayFailed)?;

                // 4. Attente sans bloquer le CPU (Déléguée !)
                info!("🔊 Thread audio: en attente ...");
                wait_for_stop_signal(&running_pair_clone);

                // 5. Nettoyage final
                if let Some(writer_arc) = &export_writer_arc {
                    writer_arc.lock().unwrap().push_block(AudioBlock {
                        index: 0,
                        frames: vec![[0.0; 2]; block_size],
                    });
                }

                drop(stream);
                info!("🔇 Thread audio: terminé");
                Ok(())
            })();

            // Gestion de l'erreur / Mode Silencieux
            if let Err(e) = audio_result {
                log::warn!("⚠️ Audio thread failed: {}. Running silent.", e);
                wait_for_stop_signal(&running_pair_clone);
            }

            if let Some(writer_arc) = export_writer_arc {
                writer_arc.lock().unwrap().stop();
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

    fn play_rocket_with_id(&self, id: u64, pos: (f32, f32), gain: f32) {
        self.play_rocket_with_id(id, pos, gain)
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

/// Négocie la meilleure taille de buffer (low-latency) avec le matériel
fn get_cpal_config(device: &cpal::Device, sr: u32) -> cpal::StreamConfig {
    let buffer_size = match device.supported_output_configs() {
        Ok(mut configs) => {
            let target_sr = cpal::SampleRate(sr);
            let supports_low_latency = configs.any(|c| {
                c.channels() == 2
                    && c.min_sample_rate() <= target_sr
                    && c.max_sample_rate() >= target_sr
                    && match c.buffer_size() {
                        cpal::SupportedBufferSize::Range { min, max } => *min <= 256 && *max >= 256,
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
            log::warn!(
                "Impossible d'inspecter les configs audio ({}), fallback sur Fixed(256)",
                e
            );
            cpal::BufferSize::Fixed(256)
        }
    };

    cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(sr),
        buffer_size,
    }
}

/// Met en pause le thread proprement jusqu'au signal d'arrêt de l'application
#[inline(always)]
fn wait_for_stop_signal(running_pair: &Arc<(Mutex<bool>, Condvar)>) {
    let (lock, cvar) = &**running_pair;
    let mut running = lock.lock().unwrap();
    while *running {
        running = cvar
            .wait_timeout(running, Duration::from_millis(500))
            .unwrap()
            .0;
    }
}

#[cfg(test)]
#[path = "fireworks_audio_tests.rs"]
mod tests;

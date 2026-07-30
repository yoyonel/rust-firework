use crate::audio_engine::effect_flags::{AudioEffect, AudioEffectFlags};
use crate::audio_engine::types::{FireworksAudioConfig, PlayRequest, Voice};
use crate::audio_engine::{load_audio, resample_linear, AudioBlock, AudioEngine, SafeWavWriter};
use crate::profiler::Profiler;
#[cfg(feature = "tracy")]
use crate::tracy_zone;
use crate::AudioEngineSettings;
use glam::Vec2;
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

    listener_pos: Arc<crate::audio_engine::types::AtomicVec2>,
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

    /// Masque atomique des effets DSP activés. Partagé avec le `DspProcessor` via `Arc`.
    /// Lock-free : le thread CPAL lit, le main thread écrit.
    effect_flags: std::sync::Arc<AudioEffectFlags>,

    /// Gain du signal réverbéré (Wet gain de 0.00 à 1.00) partagé de manière lock-free.
    reverb_wet: Arc<std::sync::atomic::AtomicU32>,

    /// Volume principal général (0.00 à 2.00) partagé de manière lock-free avec le thread CPAL.
    master_volume: Arc<std::sync::atomic::AtomicU32>,
    saved_master_volume: Arc<std::sync::atomic::AtomicU32>,

    // NOUVEAU : Tracking et debug des événements audio
    debug_rx: crossbeam_channel::Receiver<crate::audio_engine::types::AudioDebugEvent>,
    debug_tx: crossbeam_channel::Sender<crate::audio_engine::types::AudioDebugEvent>,
    next_request_id: std::sync::atomic::AtomicU64,
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

        let rocket_arc = Arc::new(rocket_data);
        let explosion_arc = Arc::new(explosion_data);

        let mut voices = Vec::with_capacity(config.max_voices);
        voices.resize_with(config.max_voices, Voice::new);

        let global_gain = config.settings.global_gain();

        let (garbage_tx, garbage_rx) = crossbeam_channel::unbounded();

        // --- Ring buffer SPSC borné pour les requêtes audio ---
        let (play_tx, play_rx) = crossbeam_channel::bounded(
            crate::audio_engine::constants::PLAY_REQUEST_CHANNEL_CAPACITY,
        );

        // Canaux de debug
        let (debug_tx, debug_rx) = crossbeam_channel::bounded(
            crate::audio_engine::constants::DEBUG_EVENT_CHANNEL_CAPACITY,
        );

        Ok(Self {
            rocket_data: rocket_arc,
            explosion_data: explosion_arc,
            listener_pos: Arc::new(crate::audio_engine::types::AtomicVec2::new(
                config.listener_pos,
            )),
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
            doppler_receiver: config.doppler_receiver,
            effect_flags: AudioEffectFlags::new_all_enabled(),
            reverb_wet: Arc::new(std::sync::atomic::AtomicU32::new(
                crate::audio_engine::constants::REVERB_DEFAULT_WET_GAIN.to_bits(),
            )),
            master_volume: Arc::new(std::sync::atomic::AtomicU32::new(global_gain.to_bits())),
            saved_master_volume: Arc::new(std::sync::atomic::AtomicU32::new(
                crate::audio_engine::constants::DEFAULT_GLOBAL_GAIN.to_bits(),
            )),
            debug_tx,
            debug_rx,
            next_request_id: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// Queue a sound for playback — 100% Zero-Heap Allocation !
    #[allow(clippy::too_many_arguments)]
    fn enqueue_sound(
        &self,
        id: u64,
        data: &Arc<Vec<[f32; 2]>>, // 🎯 MODIFICATION : On reçoit la référence vers l'Arc d'origine !
        pos: Vec2,
        gain: f32,
        is_dynamic: bool,
        sound_type: crate::audio_engine::types::AudioSoundType,
    ) {
        if self.global_gain == 0.0 {
            return;
        }

        // Nettoyage lock-free du garbage collector (libération par l'OS hors thread CPAL)
        while let Ok(_dead_buffer) = self.garbage_rx.try_recv() {
            #[cfg(feature = "tracy")]
            tracy_zone!("audio::free_garbage_buffer", 0xFF00AA);
        }

        let request_id = self
            .next_request_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let sent_at = Instant::now();

        // Envoyer l'event "Sent"
        let _ = self
            .debug_tx
            .try_send(crate::audio_engine::types::AudioDebugEvent::Sent {
                request_id,
                sound_type,
                entity_id: id,
                sent_at,
            });

        let global_gain = self.global_gain * gain;

        // Calcul des fades en nombre d'échantillons
        let fade_in_samples =
            (self.sample_rate as f32 * (self.settings.fade_in_ms() / 1000.0)) as usize;
        let fade_out_samples =
            (self.sample_rate as f32 * (self.settings.fade_out_ms() / 1000.0)) as usize;

        let req = PlayRequest {
            data: Arc::clone(data), // ZÉRO ALLOCATION MÉMOIRE : Pointeur partagé !
            fade_in: fade_in_samples,
            fade_out: fade_out_samples,
            gain: global_gain,
            filter_a: 0.05, // Valeur initiale, recalculée au 1er bloc par DspProcessor
            sent_at,
            request_id,
            id,
            pos,
            is_dynamic,
            sound_type,
        };

        if let Err(e) = self.play_tx.try_send(req) {
            log::warn!("⚠️ Audio play_queue full! Dropping sound event: {:?}", e);
            let _ = self
                .debug_tx
                .try_send(crate::audio_engine::types::AudioDebugEvent::Dropped {
                    request_id,
                    dropped_at: Instant::now(),
                    reason: "Play queue full",
                });
        }
    }

    pub fn play_rocket(&self, pos: Vec2, gain: f32) {
        self.enqueue_sound(
            0,
            &self.rocket_data,
            pos,
            gain,
            false,
            crate::audio_engine::types::AudioSoundType::Rocket,
        );
    }

    pub fn play_rocket_with_id(&self, id: u64, pos: Vec2, gain: f32) {
        self.enqueue_sound(
            id,
            &self.rocket_data,
            pos,
            gain,
            true,
            crate::audio_engine::types::AudioSoundType::Rocket,
        );
    }

    pub fn play_explosion(&self, pos: Vec2, gain: f32) {
        self.enqueue_sound(
            0,
            &self.explosion_data,
            pos,
            gain,
            false,
            crate::audio_engine::types::AudioSoundType::Explosion,
        );
    }

    pub fn play_explosion_with_id(&self, id: u64, pos: Vec2, gain: f32) {
        self.enqueue_sound(
            id,
            &self.explosion_data,
            pos,
            gain,
            true,
            crate::audio_engine::types::AudioSoundType::Explosion,
        );
    }

    pub fn start_audio_thread(&mut self, export_path: Option<&str>) {
        info!("🚀 Starting Audio Engine ...");

        let play_rx = self.play_rx.clone();
        let local_voices = self.voices.clone();
        let sr = self.sample_rate;
        let block_size = self.block_size;
        let master_volume_clone = self.master_volume.clone();
        let running_pair_clone = self.running_pair.clone();

        let profiler = Profiler::new(200);
        let _settings = self.settings.clone();
        let doppler_rx_clone = self.doppler_receiver.clone();
        let listener_pos_clone = self.listener_pos.clone();
        let effect_flags_clone = self.effect_flags.clone();
        let reverb_wet_clone = self.reverb_wet.clone();

        let export_writer_arc: Option<Arc<Mutex<SafeWavWriter>>> =
            export_path.map(|path| Arc::new(Mutex::new(SafeWavWriter::new(path, sr))));

        let garbage_tx = self.garbage_tx.clone();
        let debug_tx_clone = self.debug_tx.clone();

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
                let config = get_cpal_config(&device, sr, block_size);

                // 2. Instanciation du processeur DSP
                let max_supported_frames =
                    block_size.max(crate::audio_engine::constants::HARDWARE_BUFFER_SIZE as usize);
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
                    bus_w: vec![0.0; max_supported_frames],
                    bus_x: vec![0.0; max_supported_frames],
                    export_buffer: vec![[0.0; 2]; max_supported_frames],
                    last_log: Instant::now(),
                    log_interval: Duration::from_secs(4),
                    effect_flags: effect_flags_clone,
                    spatial_reverb: crate::audio_engine::SpatialReverb::new_with_wet(
                        sr,
                        reverb_wet_clone,
                    ),
                    hrtf_convolver: crate::audio_engine::HrtfConvolver::new_default(sr, block_size),
                    debug_tx: Some(debug_tx_clone),
                };

                // 3. Lancement du Flux Audio
                let stream = device
                    .build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            INIT_CPAL_THREAD.call_once(|| {
                                #[cfg(feature = "tracy")]
                                tracy_client::set_thread_name!("CPAL Audio Callback");

                                #[allow(deprecated)]
                                #[cfg(target_arch = "x86_64")]
                                unsafe {
                                    use std::arch::x86_64::{_mm_getcsr, _mm_setcsr};
                                    let mut csr = _mm_getcsr();
                                    csr |= 0x8000; // FTZ (Bit 15)
                                    csr |= 0x0040; // DAZ (Bit 6)
                                    _mm_setcsr(csr);
                                }

                                #[cfg(target_os = "linux")]
                                unsafe {
                                    let mut param: libc::sched_param = std::mem::zeroed();
                                    param.sched_priority = 20;
                                    let res = libc::pthread_setschedparam(
                                        libc::pthread_self(),
                                        libc::SCHED_FIFO,
                                        &param,
                                    );
                                    if res != 0 {
                                        libc::setpriority(libc::PRIO_PROCESS, 0, -20);
                                    }
                                }
                            });
                            let cur_gain = f32::from_bits(
                                master_volume_clone.load(std::sync::atomic::Ordering::Relaxed),
                            );
                            dsp_processor.process_block(data, cur_gain, &profiler);
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
        self.set_master_volume(volume);
    }
}

impl AudioEngine for FireworksAudio3D {
    fn play_rocket(&self, pos: Vec2, gain: f32) {
        self.play_rocket(pos, gain)
    }

    fn play_rocket_with_id(&self, id: u64, pos: Vec2, gain: f32) {
        self.play_rocket_with_id(id, pos, gain)
    }

    fn play_explosion(&self, pos: Vec2, gain: f32) {
        self.play_explosion(pos, gain)
    }

    fn play_explosion_with_id(&self, id: u64, pos: Vec2, gain: f32) {
        self.play_explosion_with_id(id, pos, gain)
    }

    fn start_audio_thread(&mut self, _export_path: Option<&str>) {
        self.start_audio_thread(_export_path)
    }

    fn stop_audio_thread(&mut self) {
        self.stop_audio_thread()
    }

    fn set_listener_position(&mut self, pos: Vec2) {
        self.listener_pos.store(pos);
        info!("🎧️ Listener position set to: {:?}", pos);
    }

    fn get_listener_position(&self) -> Vec2 {
        self.listener_pos.load()
    }

    fn mute(&mut self) {
        let cur = self.get_master_volume();
        if cur > 0.0001 {
            self.saved_master_volume
                .store(cur.to_bits(), std::sync::atomic::Ordering::Relaxed);
        }
        self.set_master_volume(0.0);
    }

    fn unmute(&mut self) -> f32 {
        let saved = f32::from_bits(
            self.saved_master_volume
                .load(std::sync::atomic::Ordering::Relaxed),
        );
        let restore = if saved > 0.0001 { saved } else { 0.8 };
        self.set_master_volume(restore);
        restore
    }

    fn is_muted(&self) -> bool {
        self.get_master_volume() <= 0.0001
    }

    fn set_master_volume(&self, volume: f32) {
        let clamped = volume.clamp(0.0, 2.0);
        self.master_volume
            .store(clamped.to_bits(), std::sync::atomic::Ordering::Relaxed);
        if clamped > 0.0001 {
            self.saved_master_volume
                .store(clamped.to_bits(), std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn get_master_volume(&self) -> f32 {
        f32::from_bits(
            self.master_volume
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    fn set_effect_enabled(&self, effect: AudioEffect, enabled: bool) {
        self.effect_flags.set(effect, enabled);
    }

    fn set_all_effects_enabled(&self, enabled: bool) {
        self.effect_flags.set_all(enabled);
    }

    fn get_effect_enabled(&self, effect: AudioEffect) -> bool {
        self.effect_flags.is_enabled(effect)
    }

    fn get_effects_status(&self) -> String {
        self.effect_flags.status_string()
    }

    fn pop_debug_events(&self, buf: &mut Vec<crate::audio_engine::types::AudioDebugEvent>) {
        while let Ok(evt) = self.debug_rx.try_recv() {
            buf.push(evt);
        }
    }

    fn get_max_distance(&self) -> f32 {
        self.settings.max_distance()
    }

    fn set_reverb_wet(&self, wet: f32) {
        self.reverb_wet.store(
            wet.clamp(0.0, 1.0).to_bits(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    fn get_reverb_wet(&self) -> f32 {
        f32::from_bits(self.reverb_wet.load(std::sync::atomic::Ordering::Relaxed))
    }

    fn as_audio_engine(&self) -> &dyn AudioEngine {
        self
    }
}

/// Configure la configuration CPAL avec un buffer par défaut pour autoriser
/// une gestion de buffer multi-période stable par le système (PipeWire/PulseAudio/ALSA).
fn get_cpal_config(_device: &cpal::Device, sr: u32, _block_size: usize) -> cpal::StreamConfig {
    cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(sr),
        buffer_size: cpal::BufferSize::Default,
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

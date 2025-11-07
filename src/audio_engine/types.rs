// use crate::audio_engine::DopplerEvent;
use crate::AudioEngineSettings;
// use crossbeam::channel::Receiver;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

// Global static compteur unique
static ROCKET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct RocketAudioState {
    _last_pos: (f32, f32), // dernière position connue côté audio
    _last_update: Instant, // dernière time where we processed an event
}

impl RocketAudioState {
    fn _new(pos: (f32, f32), t: Instant) -> Self {
        Self {
            _last_pos: pos,
            _last_update: t,
        }
    }
}

// =========================
// Voice Struct
// =========================

/// Represents a single active sound instance (voice)
#[derive(Clone, Default)]
pub struct Voice {
    _id: u64,
    pub active: bool,                // Is the voice currently playing?
    pub data: Option<Vec<[f32; 2]>>, // Stereo audio samples
    pub pos: usize,                  // Current sample index
    pub fade_in_samples: usize,      // Number of samples for fade-in
    pub fade_out_samples: usize,     // Number of samples for fade-out
    pub filter_state: [f32; 2],      // Low-pass filter state per channel
    pub filter_a: f32,               // Low-pass filter coefficient
    pub user_gain: f32,              // Per-voice gain multiplier
}

impl Voice {
    /// Create a new inactive voice
    pub fn new() -> Self {
        Self {
            _id: ROCKET_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            active: false,
            data: None,
            pos: 0,
            fade_in_samples: 0,
            fade_out_samples: 0,
            filter_state: [0.0, 0.0],
            filter_a: 0.0,
            user_gain: 1.0,
        }
    }

    fn from_request(req: &PlayRequest) -> Self {
        Self {
            data: Some(req.data.clone()),
            pos: 0,
            active: true,
            fade_in_samples: req.fade_in,
            fade_out_samples: req.fade_out,
            filter_a: req.filter_a,
            user_gain: req.gain,
            filter_state: [0.0; 2],
            _id: 0, // ou gérer l’ID
        }
    }

    pub fn reset_from_request(&mut self, req: &PlayRequest) {
        *self = Voice::from_request(req);
    }
}

// =========================
// PlayRequest Struct
// =========================

/// A request to play a sound, queued for playback in the audio thread
pub struct PlayRequest {
    pub data: Vec<[f32; 2]>, // Stereo audio data
    pub fade_in: usize,      // Fade-in samples
    pub fade_out: usize,     // Fade-out samples
    pub gain: f32,           // Per-sound gain
    pub filter_a: f32,       // Low-pass coefficient
    pub sent_at: Instant,    // Timestamp of request
}

#[derive(Clone)]
pub struct DopplerState {
    pub pos: (f32, f32),
    pub vel: (f32, f32),
    pub voice_index: u64,
    pub duration_left: f32,   // en secondes
    pub sample_offset: usize, // position dans l'échantillon audio
    pub sample_rate: u32,
    pub rocket_data: Vec<[f32; 2]>, // le son de la rocket
    pub doppler_factor: f32,
}

impl DopplerState {
    /// Met à jour la position selon la vitesse et le delta temps
    pub fn step(&mut self, dt: f32) {
        self.pos.0 += self.vel.0 * dt;
        self.pos.1 += self.vel.1 * dt;
        self.duration_left -= dt;
    }

    /// Vérifie si le son est terminé
    pub fn finished(&self) -> bool {
        self.duration_left <= 0.0 || self.sample_offset >= self.rocket_data.len()
    }
}

// =========================
// FireworksAudio3D Engine
// =========================

pub struct FireworksAudioConfig {
    pub rocket_path: String,
    pub explosion_path: String,
    pub listener_pos: (f32, f32),
    pub sample_rate: u32,
    pub block_size: usize,
    pub max_voices: usize,
    pub settings: AudioEngineSettings,
    // pub doppler_receiver: Option<Receiver<DopplerEvent>>,
    // pub doppler_states: Vec<DopplerState>,
}

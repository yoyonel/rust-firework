use crate::audio_engine::DopplerEvent;
use crate::AudioEngineSettings;
use crossbeam::channel::Receiver;
use glam::Vec2;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

// Global static compteur unique
static ROCKET_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct RocketAudioState {
    _last_pos: Vec2,       // dernière position connue côté audio
    _last_update: Instant, // dernière time where we processed an event
}

impl RocketAudioState {
    fn _new(pos: Vec2, t: Instant) -> Self {
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
    pub id: u64,
    pub active: bool, // Is the voice currently playing?
    // pub data: Option<Vec<[f32; 2]>>, // Stereo audio samples
    pub data: Option<Arc<Vec<[f32; 2]>>>,

    pub pos: f64,           // MODIFIÉ : usize -> f64 pour la lecture fractionnaire
    pub playback_rate: f32, // NOUVEAU : Facteur de vitesse (alpha)
    pub is_dynamic: bool,   // NOUVEAU : Distingue les objets en mouvement (Doppler)
    pub world_pos: Vec2,    // NOUVEAU : Position absolue de la source
    pub velocity: Vec2,     // NOUVEAU : Vecteur vitesse (vx, vy)

    pub fade_in_samples: usize,  // Number of samples for fade-in
    pub fade_out_samples: usize, // Number of samples for fade-out
    pub filter_state: [f32; 2],  // Low-pass filter state per channel
    pub filter_a: f32,           // Low-pass filter coefficient
    pub user_gain: f32,          // Per-voice gain multiplier
    pub current_gains: [f32; 2], // Gain actuel gauche/droite (pour l'interpolation)
    pub target_gains: [f32; 2],  // Gain cible gauche/droite à atteindre à la fin du bloc
    pub current_itd: [f32; 2],   // ITD actuel gauche/droite (en échantillons)
    pub target_itd: [f32; 2],    // ITD cible gauche/droite à la fin du bloc
}

impl Voice {
    /// Create a new inactive voice
    pub fn new() -> Self {
        Self {
            id: ROCKET_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            active: false,
            data: None,
            pos: 0.0,              // MODIFIÉ : 0 -> 0.0
            playback_rate: 1.0,    // NOUVEAU
            is_dynamic: false,     // NOUVEAU
            world_pos: Vec2::ZERO, // NOUVEAU
            velocity: Vec2::ZERO,  // NOUVEAU
            fade_in_samples: 0,
            fade_out_samples: 0,
            filter_state: [0.0, 0.0],
            filter_a: 0.0,
            user_gain: 1.0,
            current_gains: [1.0, 1.0], // NOUVEAU
            target_gains: [1.0, 1.0],  // NOUVEAU
            current_itd: [0.0, 0.0],
            target_itd: [0.0, 0.0],
        }
    }

    fn from_request(req: &PlayRequest) -> Self {
        Self {
            // data: Some(req.data.clone()),
            data: Some(Arc::clone(&req.data)),
            pos: 0.0,                   // MODIFIÉ : 0 -> 0.0
            playback_rate: 1.0,         // NOUVEAU
            is_dynamic: req.is_dynamic, // MODIFIÉ : Prend la valeur du request
            world_pos: req.pos,         // MODIFIÉ : Initialise avec la position du request
            velocity: Vec2::ZERO,       // NOUVEAU
            active: true,
            fade_in_samples: req.fade_in,
            fade_out_samples: req.fade_out,
            filter_a: req.filter_a,
            user_gain: req.gain,
            filter_state: [0.0; 2],
            id: req.id, // MODIFIÉ : Relie la voix à l'ID physique (renommer _id -> id)
            // NOUVEAU : On initialise les gains à 0 pour forcer une rampe d'apparition
            // ou à des valeurs neutres. Le vrai calcul aura lieu au premier bloc.
            current_gains: [0.0, 0.0],
            target_gains: [0.0, 0.0],
            current_itd: [0.0, 0.0],
            target_itd: [0.0, 0.0],
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
    // pub data: Vec<[f32; 2]>, // Stereo audio data
    pub data: Arc<Vec<[f32; 2]>>,
    pub fade_in: usize,   // Fade-in samples
    pub fade_out: usize,  // Fade-out samples
    pub gain: f32,        // Per-sound gain
    pub filter_a: f32,    // Low-pass coefficient
    pub sent_at: Instant, // Timestamp of request

    pub id: u64,          // ID de la entité physique (0 si statique)
    pub pos: Vec2,        // Position initiale
    pub is_dynamic: bool, // true si sujet au Doppler
}

// =========================
// FireworksAudio3D Engine
// =========================

pub struct FireworksAudioConfig {
    pub rocket_path: String,
    pub explosion_path: String,
    pub listener_pos: Vec2,
    pub sample_rate: u32,
    pub block_size: usize,
    pub max_voices: usize,
    pub settings: AudioEngineSettings,
    pub doppler_receiver: Option<Receiver<DopplerEvent>>,
    // pub export_in_wav: bool,
}

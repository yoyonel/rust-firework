// =============================================================
// AudioEffectFlags — Bitmask atomique pour les effets DSP audio
// =============================================================
//
// Architecture lock-free :
//  - Le thread CPAL lit le masque UNE SEULE FOIS par bloc via `load(Relaxed)`.
//  - Le main thread écrit via `fetch_or`/`fetch_and` (Relaxed, hors chemin critique).
//  - Zéro Mutex, zéro allocation dans le chemin audio real-time.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub use crate::domain_contracts::AudioEffect;

/// Valeur par défaut du masque : tous les effets sont activés.
pub const DEFAULT_FLAGS: u32 = AudioEffect::Binaural as u32
    | AudioEffect::Panning as u32
    | AudioEffect::DistanceAtten as u32
    | AudioEffect::LowPassFilter as u32
    | AudioEffect::Doppler as u32
    | AudioEffect::FadeInOut as u32
    | AudioEffect::GainLerp as u32
    | AudioEffect::Normalization as u32
    | AudioEffect::SpatialReverb as u32
    | AudioEffect::SpatialBus as u32
    | AudioEffect::HrtfBus as u32;

/// Bitmask partagé entre le main thread et le thread CPAL.
///
/// # Utilisation dans le thread audio
/// ```rust,ignore
/// let fx_mask = self.effect_flags.load(); // 1 seul load atomique / bloc
/// if fx_enabled(fx_mask, AudioEffect::LowPassFilter) { /* ... */ }
/// ```
pub struct AudioEffectFlags(AtomicU32);

impl AudioEffectFlags {
    /// Crée un nouveau masque avec tous les effets activés, enveloppé dans un `Arc`.
    pub fn new_all_enabled() -> Arc<Self> {
        Arc::new(Self(AtomicU32::new(DEFAULT_FLAGS)))
    }

    /// Lit le masque en lock-free — à appeler **une seule fois par bloc** dans le thread CPAL.
    ///
    /// `Relaxed` suffit car il n'y a pas de données à synchroniser au-delà du masque lui-même,
    /// et une lecture légèrement obsolète n'a aucun impact sur la sécurité audio.
    #[inline(always)]
    pub fn load(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }

    /// Active ou désactive un effet depuis le main thread.
    ///
    /// Utilise `fetch_or`/`fetch_and` (opération atomique lecture-modification-écriture)
    /// pour ne modifier que le bit concerné sans race condition.
    pub fn set(&self, effect: AudioEffect, enabled: bool) {
        if enabled {
            self.0.fetch_or(effect as u32, Ordering::Relaxed);
        } else {
            self.0.fetch_and(!(effect as u32), Ordering::Relaxed);
        }
    }

    /// Active ou désactive tous les effets DSP en une seule opération atomique.
    pub fn set_all(&self, enabled: bool) {
        if enabled {
            self.0.store(DEFAULT_FLAGS, Ordering::Relaxed);
        } else {
            self.0.store(0, Ordering::Relaxed);
        }
    }

    /// Retourne l'état courant d'un effet (lecture depuis le main thread).
    pub fn is_enabled(&self, effect: AudioEffect) -> bool {
        self.0.load(Ordering::Relaxed) & (effect as u32) != 0
    }

    /// Retourne une chaîne lisible listant tous les effets et leur état courant.
    /// Utilisée par la console pour `audio.fx_status`.
    pub fn status_string(&self) -> String {
        let mask = self.load();
        AudioEffect::all_names()
            .iter()
            .map(|(name, fx)| {
                let state = if mask & (*fx as u32) != 0 {
                    "ON "
                } else {
                    "OFF"
                };
                format!("[{}] {}", state, name)
            })
            .collect::<Vec<_>>()
            .join("  |  ")
    }
}

/// Helper inline pour tester un bit dans un masque pré-chargé.
///
/// Permet d'écrire `if fx_enabled(fx_mask, AudioEffect::FadeInOut)` sans relire l'atomique.
/// Le compilateur élimine le branchement si l'effet est toujours activé (branch prediction parfaite).
#[inline(always)]
pub fn fx_enabled(mask: u32, effect: AudioEffect) -> bool {
    mask & (effect as u32) != 0
}

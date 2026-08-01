//! Domain contracts (enums & read-only state reader traits) decoupling UI/UX from core engines.
//!
//! Designed for Data-Oriented, continuous-flow architecture with strict zero-allocation constraints
//! and optimized memory layout for cache-friendly command queue iteration.

use crate::audio_engine::effect_flags::AudioEffect;
use crate::audio_engine::AudioEngine;
use glam::Vec2;

/// Commands sent from UI to Audio engine without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioCommand {
    SetMasterVolume(f32),
    SetMuted(bool),
    SetSpatialReverb(f32),
    SetHrtfEnabled(bool),
    SetAllEffectsEnabled(bool),
    SetEffectEnabled { effect: AudioEffect, enabled: bool },
    SetListenerPosition(Vec2),
    StartStressTest,
}

/// Commands sent from UI to Physic engine without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhysicCommand {
    SetGravity(f32),
    SetDrag(f32),
    SetMaxParticles(u32),
    SetExplosionForce(f32),
}

/// Commands sent from UI to Renderer engine without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RendererCommand {
    SetBloomIntensity(f32),
    SetExposure(f32),
    SetWireframe(bool),
    SetVsync(bool),
}

/// Commands sent from UI to Smoke simulation engine without dynamic allocations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SmokeCommand {
    SetDensity(f32),
    SetDissipation(f32),
    SetWind([f32; 2]),
}

/// Unified domain command enum decoupling UI from core engines.
///
/// Designed with minimal memory layout to fit tightly into CPU cache lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EngineCommand {
    Audio(AudioCommand),
    Physic(PhysicCommand),
    Renderer(RendererCommand),
    Smoke(SmokeCommand),
}

/// Read-only interface exposing Audio engine state to UI.
pub trait AudioStateReader {
    fn master_volume(&self) -> f32;
    fn is_muted(&self) -> bool;
    fn spatial_reverb(&self) -> f32;
    fn hrtf_enabled(&self) -> bool;
    fn effect_enabled(&self, effect: AudioEffect) -> bool;
}

impl<T: AudioEngine> AudioStateReader for T {
    #[inline(always)]
    fn master_volume(&self) -> f32 {
        self.get_master_volume()
    }
    #[inline(always)]
    fn is_muted(&self) -> bool {
        self.is_muted()
    }
    #[inline(always)]
    fn spatial_reverb(&self) -> f32 {
        self.get_reverb_wet()
    }
    #[inline(always)]
    fn hrtf_enabled(&self) -> bool {
        self.get_effect_enabled(AudioEffect::HrtfBus)
    }
    #[inline(always)]
    fn effect_enabled(&self, effect: AudioEffect) -> bool {
        self.get_effect_enabled(effect)
    }
}

/// Read-only interface exposing Physic engine state to UI.
pub trait PhysicStateReader {
    fn gravity(&self) -> f32;
    fn drag(&self) -> f32;
    fn max_particles(&self) -> u32;
    fn explosion_force(&self) -> f32;
}

/// Read-only interface exposing Renderer engine state to UI.
pub trait RendererStateReader {
    fn bloom_intensity(&self) -> f32;
    fn exposure(&self) -> f32;
    fn is_wireframe(&self) -> bool;
    fn vsync_enabled(&self) -> bool;
}

/// Read-only interface exposing Smoke engine state to UI.
pub trait SmokeStateReader {
    fn density(&self) -> f32;
    fn dissipation(&self) -> f32;
    fn wind(&self) -> [f32; 2];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_command_memory_layout() {
        let size = std::mem::size_of::<EngineCommand>();
        // EngineCommand must fit tightly into memory for cache efficiency (<= 16 bytes on 64-bit target)
        assert!(
            size <= 16,
            "EngineCommand footprint ({} bytes) exceeds cache optimization limit of 16 bytes",
            size
        );
    }

    #[test]
    fn test_zero_cost_command_creation() {
        let cmd_audio = EngineCommand::Audio(AudioCommand::SetMasterVolume(0.85));
        let cmd_physic = EngineCommand::Physic(PhysicCommand::SetGravity(9.81));
        let cmd_renderer = EngineCommand::Renderer(RendererCommand::SetBloomIntensity(1.5));
        let cmd_smoke = EngineCommand::Smoke(SmokeCommand::SetWind([1.0, -0.5]));

        assert_eq!(
            cmd_audio,
            EngineCommand::Audio(AudioCommand::SetMasterVolume(0.85))
        );
        assert_eq!(
            cmd_physic,
            EngineCommand::Physic(PhysicCommand::SetGravity(9.81))
        );
        assert_eq!(
            cmd_renderer,
            EngineCommand::Renderer(RendererCommand::SetBloomIntensity(1.5))
        );
        assert_eq!(
            cmd_smoke,
            EngineCommand::Smoke(SmokeCommand::SetWind([1.0, -0.5]))
        );
    }

    struct MockState {
        volume: f32,
        muted: bool,
        reverb: f32,
        hrtf: bool,
        gravity: f32,
        drag: f32,
        particles: u32,
        explosion: f32,
        bloom: f32,
        exposure: f32,
        wireframe: bool,
        vsync: bool,
        smoke_density: f32,
        smoke_dissipation: f32,
        smoke_wind: [f32; 2],
    }

    impl AudioStateReader for MockState {
        #[inline(always)]
        fn master_volume(&self) -> f32 {
            self.volume
        }
        #[inline(always)]
        fn is_muted(&self) -> bool {
            self.muted
        }
        #[inline(always)]
        fn spatial_reverb(&self) -> f32 {
            self.reverb
        }
        #[inline(always)]
        fn hrtf_enabled(&self) -> bool {
            self.hrtf
        }
        #[inline(always)]
        fn effect_enabled(&self, _effect: AudioEffect) -> bool {
            true
        }
    }

    impl PhysicStateReader for MockState {
        #[inline(always)]
        fn gravity(&self) -> f32 {
            self.gravity
        }
        #[inline(always)]
        fn drag(&self) -> f32 {
            self.drag
        }
        #[inline(always)]
        fn max_particles(&self) -> u32 {
            self.particles
        }
        #[inline(always)]
        fn explosion_force(&self) -> f32 {
            self.explosion
        }
    }

    impl RendererStateReader for MockState {
        #[inline(always)]
        fn bloom_intensity(&self) -> f32 {
            self.bloom
        }
        #[inline(always)]
        fn exposure(&self) -> f32 {
            self.exposure
        }
        #[inline(always)]
        fn is_wireframe(&self) -> bool {
            self.wireframe
        }
        #[inline(always)]
        fn vsync_enabled(&self) -> bool {
            self.vsync
        }
    }

    impl SmokeStateReader for MockState {
        #[inline(always)]
        fn density(&self) -> f32 {
            self.smoke_density
        }
        #[inline(always)]
        fn dissipation(&self) -> f32 {
            self.smoke_dissipation
        }
        #[inline(always)]
        fn wind(&self) -> [f32; 2] {
            self.smoke_wind
        }
    }

    #[test]
    fn test_state_readers() {
        let state = MockState {
            volume: 0.75,
            muted: false,
            reverb: 0.3,
            hrtf: true,
            gravity: -9.81,
            drag: 0.01,
            particles: 50000,
            explosion: 100.0,
            bloom: 0.8,
            exposure: 1.0,
            wireframe: false,
            vsync: true,
            smoke_density: 0.5,
            smoke_dissipation: 0.05,
            smoke_wind: [0.2, -0.1],
        };

        assert_eq!(state.master_volume(), 0.75);
        assert!(!state.is_muted());
        assert_eq!(state.spatial_reverb(), 0.3);
        assert!(state.hrtf_enabled());

        assert_eq!(state.gravity(), -9.81);
        assert_eq!(state.drag(), 0.01);
        assert_eq!(state.max_particles(), 50000);
        assert_eq!(state.explosion_force(), 100.0);

        assert_eq!(state.bloom_intensity(), 0.8);
        assert_eq!(state.exposure(), 1.0);
        assert!(!state.is_wireframe());
        assert!(state.vsync_enabled());

        assert_eq!(state.density(), 0.5);
        assert_eq!(state.dissipation(), 0.05);
        assert_eq!(state.wind(), [0.2, -0.1]);
    }
}

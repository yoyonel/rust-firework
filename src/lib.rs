pub mod simulator;
pub use simulator::Simulator;
// Renderer engine
pub mod renderer_engine;
pub use renderer_engine::RendererEngine;
// Audio engine
pub mod audio_engine;
pub use audio_engine::AudioEngine;
pub use audio_engine::AudioEngineSettings;
pub use audio_engine::FireworksAudio3D;
// Physic engine
pub mod physic_engine;
pub use physic_engine::PhysicEngine;

// Profiler
pub mod profiler;
// Utilities
pub mod utils;

// #[cfg(all(feature = "simd", feature = "no_simd"))]
// compile_error!("Features `simd` et `no_simd` ne peuvent pas être activées en même temps");

// #[cfg(feature = "simd")]
// pub mod audio_filters_simd;
// #[cfg(feature = "simd")]
// pub use audio_filters_simd as audio_filters;

// #[cfg(feature = "no_simd")]
// pub mod audio_filters_scalar;
// #[cfg(feature = "no_simd")]
// pub use audio_filters_scalar as audio_filters;

// #[cfg(feature = "fft")]
// pub mod audio_filters_fft;
// #[cfg(feature = "fft")]
// pub use audio_filters_fft as af_fft;
// #[cfg(feature = "fft")]
// pub mod audio_data;

// pub mod audio_wav;
// pub mod audio_rodio;
// pub mod physic_engine;
// pub mod physic_engine_static;

//! CLI Arguments Parsing for Fireworks Simulator.
//!
//! Provides strict separation between CLI ingestion and simulator execution.

use std::path::PathBuf;

/// Parsed CLI configuration options.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CliArgs {
    /// Optional output file for WAV audio recording
    pub export_path: Option<PathBuf>,
    /// Deterministic RNG seed for reproducible simulations
    pub deterministic_seed: Option<u64>,
    /// Maximum number of simulation frames before exit
    pub max_frames: Option<u64>,
    /// Maximum wall-clock timeout in seconds before exit
    pub timeout_secs: Option<u64>,
    /// Fixed delta time for discrete physics stepping
    pub fixed_dt: Option<f32>,
    /// Flag to disable CPAL audio output
    pub disable_audio: bool,
    /// Duration in seconds for headless audio stress testing mode
    pub headless_audio_stress: Option<u64>,
    /// Number of virtual sound sources for interactive stress scene
    pub audio_stress_sources: Option<usize>,
    /// Whether to randomize virtual sound source positions
    pub randomize_stress_positions: bool,
}

impl CliArgs {
    /// Parses CLI arguments from `std::env::args()` and environment variables.
    pub fn parse() -> Self {
        Self::parse_from(std::env::args())
    }

    /// Parses CLI arguments from an arbitrary iterator of strings.
    pub fn parse_from<I, T>(args_iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let args: Vec<String> = args_iter.into_iter().map(Into::into).collect();

        // 1. Gestion du chemin d'export audio (priorité argument CLI 1 non-flag, sinon env var)
        let export_path = args
            .get(1)
            .filter(|arg| !arg.starts_with('-'))
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("FIREWORKS_AUDIO_EXPORT")
                    .ok()
                    .map(PathBuf::from)
            });

        let mut deterministic_seed = None;
        let mut max_frames = None;
        let mut timeout_secs = None;
        let mut fixed_dt = None;
        let mut disable_audio = false;

        // TODO: Technical debt - invalid argument values are silently ignored (if let Ok without else)
        // for backward compatibility with existing test scripts and headless invocations.
        // Migration to clap or strict error reporting should be done in a dedicated UX phase.
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--deterministic-seed" if i + 1 < args.len() => {
                    if let Ok(val) = args[i + 1].parse() {
                        deterministic_seed = Some(val);
                    }
                    i += 1;
                }
                "--max-frames" if i + 1 < args.len() => {
                    if let Ok(val) = args[i + 1].parse() {
                        max_frames = Some(val);
                    }
                    i += 1;
                }
                "--timeout-secs" if i + 1 < args.len() => {
                    if let Ok(val) = args[i + 1].parse() {
                        timeout_secs = Some(val);
                    }
                    i += 1;
                }
                "--fixed-dt" if i + 1 < args.len() => {
                    if let Ok(val) = args[i + 1].parse() {
                        fixed_dt = Some(val);
                    }
                    i += 1;
                }
                "--disable-audio" => {
                    disable_audio = true;
                }
                _ => {}
            }
            i += 1;
        }

        // Mode Headless Audio Stress
        let headless_audio_stress = args
            .iter()
            .position(|a| a == "--headless-audio-stress")
            .map(|pos| {
                args.get(pos + 1).and_then(|s| s.parse().ok()).unwrap_or(10) // 10 secondes par défaut
            });

        // Mode Audio Stress Scene
        let mut audio_stress_sources = None;
        let mut randomize_stress_positions = false;
        if let Some(pos) = args.iter().position(|a| a == "--audio-stress-scene") {
            let num_sources: usize = args.get(pos + 1).and_then(|s| s.parse().ok()).unwrap_or(32);
            audio_stress_sources = Some(num_sources);
            randomize_stress_positions = args.iter().any(|a| a == "--randomize-stress-positions");
        }

        Self {
            export_path,
            deterministic_seed,
            max_frames,
            timeout_secs,
            fixed_dt,
            disable_audio,
            headless_audio_stress,
            audio_stress_sources,
            randomize_stress_positions,
        }
    }

    /// Logs active overrides for diagnostic visibility.
    pub fn log_overrides(&self) {
        if let Some(path) = &self.export_path {
            log::info!("Audio export path set to: {}", path.display());
        }
        if let Some(seed) = self.deterministic_seed {
            log::info!("🔧 Deterministic mode activated. Seed: {}", seed);
        }
        if let Some(max_f) = self.max_frames {
            log::info!("🔧 Max frames override: {}", max_f);
        }
        if let Some(to) = self.timeout_secs {
            log::info!("🔧 Timeout override: {}s", to);
        }
        if let Some(dt) = self.fixed_dt {
            log::info!("🔧 Fixed DT override: {}", dt);
        }
        if self.disable_audio {
            log::info!("🔧 Audio disabled via CLI");
        }
        if let Some(n) = self.audio_stress_sources {
            log::info!(
                "🎧 [STRESS TEST SCENE] Starting interactive audio stress-test with {} virtual sources (randomize positions: {})...",
                n,
                self.randomize_stress_positions
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_empty() {
        let args = CliArgs::parse_from(["fireworks_sim"]);
        assert_eq!(args.export_path, None);
        assert_eq!(args.deterministic_seed, None);
        assert_eq!(args.max_frames, None);
        assert_eq!(args.timeout_secs, None);
        assert_eq!(args.fixed_dt, None);
        assert!(!args.disable_audio);
        assert_eq!(args.headless_audio_stress, None);
        assert_eq!(args.audio_stress_sources, None);
        assert!(!args.randomize_stress_positions);
    }

    #[test]
    fn test_cli_parsing_export_path_positional() {
        let args = CliArgs::parse_from(["fireworks_sim", "output/test.wav"]);
        assert_eq!(args.export_path, Some(PathBuf::from("output/test.wav")));
    }

    #[test]
    fn test_cli_parsing_flags_not_confused_with_export_path() {
        let args = CliArgs::parse_from(["fireworks_sim", "--disable-audio"]);
        assert_eq!(args.export_path, None);
        assert!(args.disable_audio);
    }

    #[test]
    fn test_cli_parsing_valid_overrides() {
        let args = CliArgs::parse_from([
            "fireworks_sim",
            "--deterministic-seed",
            "42",
            "--max-frames",
            "120",
            "--timeout-secs",
            "15",
            "--fixed-dt",
            "0.01667",
            "--disable-audio",
        ]);
        assert_eq!(args.deterministic_seed, Some(42));
        assert_eq!(args.max_frames, Some(120));
        assert_eq!(args.timeout_secs, Some(15));
        assert_eq!(args.fixed_dt, Some(0.01667));
        assert!(args.disable_audio);
    }

    #[test]
    fn test_cli_parsing_invalid_overrides_silent_ignore() {
        // Validation stricte du silence historique (if let Ok sans else)
        let args = CliArgs::parse_from([
            "fireworks_sim",
            "--deterministic-seed",
            "not_a_number",
            "--max-frames",
            "invalid_frame",
            "--timeout-secs",
            "nan",
            "--fixed-dt",
            "xyz",
        ]);
        assert_eq!(args.deterministic_seed, None);
        assert_eq!(args.max_frames, None);
        assert_eq!(args.timeout_secs, None);
        assert_eq!(args.fixed_dt, None);
    }

    #[test]
    fn test_cli_parsing_headless_audio_stress_defaults_and_explicit() {
        let args_default = CliArgs::parse_from(["fireworks_sim", "--headless-audio-stress"]);
        assert_eq!(args_default.headless_audio_stress, Some(10));

        let args_explicit = CliArgs::parse_from(["fireworks_sim", "--headless-audio-stress", "25"]);
        assert_eq!(args_explicit.headless_audio_stress, Some(25));

        let args_invalid =
            CliArgs::parse_from(["fireworks_sim", "--headless-audio-stress", "invalid"]);
        assert_eq!(args_invalid.headless_audio_stress, Some(10));
    }

    #[test]
    fn test_cli_parsing_audio_stress_scene() {
        let args = CliArgs::parse_from([
            "fireworks_sim",
            "--audio-stress-scene",
            "64",
            "--randomize-stress-positions",
        ]);
        assert_eq!(args.audio_stress_sources, Some(64));
        assert!(args.randomize_stress_positions);
    }
}

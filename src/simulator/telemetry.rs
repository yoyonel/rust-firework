//! Telemetry and diagnostic tracking modules extracted from Simulator.
//!
//! Decouples synchronization telemetry, audio diagnostics, visual debugging,
//! and FPS metrics from core simulation state.

use crate::audio_engine::types::{AudioDebugEvent, AudioDebugRecord};
use crate::renderer_engine::utils::adaptative_sampler::AdaptiveSampler;
use crate::renderer_engine::{AudioEvent, AudioEventRenderer};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Tracks physical-to-audio synchronization metrics and timestamp matching.
#[derive(Debug, Default, Clone)]
pub struct SyncTelemetryTracker {
    pub sync_launch_sum: f64,
    pub sync_launch_count: u64,
    pub sync_explosion_sum: f64,
    pub sync_explosion_count: u64,
    pub phys_launch_times: HashMap<u64, Instant>,
    pub phys_explosion_times: HashMap<u64, Instant>,
    pub audio_start_launch_times: HashMap<u64, Instant>,
    pub audio_start_explosion_times: HashMap<u64, Instant>,
    pub launch_trend_dir: i32, // 1: augmentation, -1: diminution, 0: stable
    pub explosion_trend_dir: i32,
}

#[inline]
fn signed_diff_ms(audio_time: Instant, phys_time: Instant) -> f32 {
    if audio_time >= phys_time {
        audio_time.duration_since(phys_time).as_secs_f32() * 1000.0
    } else {
        phys_time.duration_since(audio_time).as_secs_f32() * -1000.0
    }
}

impl SyncTelemetryTracker {
    pub fn new() -> Self {
        Self {
            sync_launch_sum: 0.0,
            sync_launch_count: 0,
            sync_explosion_sum: 0.0,
            sync_explosion_count: 0,
            phys_launch_times: HashMap::new(),
            phys_explosion_times: HashMap::new(),
            audio_start_launch_times: HashMap::new(),
            audio_start_explosion_times: HashMap::new(),
            launch_trend_dir: 0,
            explosion_trend_dir: 0,
        }
    }

    /// Records physical launch event. If audio was already started, computes diff and returns Some(diff_ms).
    pub fn record_launch_physic(&mut self, id: u64, now: Instant) -> Option<f32> {
        if let Some(audio_start) = self.audio_start_launch_times.remove(&id) {
            let diff_ms = signed_diff_ms(audio_start, now);
            self.sync_launch_sum += diff_ms as f64;
            self.sync_launch_count += 1;
            Some(diff_ms)
        } else {
            self.phys_launch_times.insert(id, now);
            None
        }
    }

    /// Records physical explosion event. If audio was already started, computes diff and returns Some(diff_ms).
    pub fn record_explosion_physic(&mut self, id: u64, now: Instant) -> Option<f32> {
        if let Some(audio_start) = self.audio_start_explosion_times.remove(&id) {
            let diff_ms = signed_diff_ms(audio_start, now);
            self.sync_explosion_sum += diff_ms as f64;
            self.sync_explosion_count += 1;
            Some(diff_ms)
        } else {
            self.phys_explosion_times.insert(id, now);
            None
        }
    }

    /// Records audio started launch event. If physic already happened, computes diff and returns Some(diff_ms).
    pub fn record_audio_started_launch(&mut self, id: u64, started_at: Instant) -> Option<f32> {
        if let Some(phys_time) = self.phys_launch_times.remove(&id) {
            let diff_ms = signed_diff_ms(started_at, phys_time);
            self.sync_launch_sum += diff_ms as f64;
            self.sync_launch_count += 1;
            Some(diff_ms)
        } else {
            self.audio_start_launch_times.insert(id, started_at);
            None
        }
    }

    /// Records audio started explosion event. If physic already happened, computes diff and returns Some(diff_ms).
    pub fn record_audio_started_explosion(&mut self, id: u64, started_at: Instant) -> Option<f32> {
        if let Some(phys_time) = self.phys_explosion_times.remove(&id) {
            let diff_ms = signed_diff_ms(started_at, phys_time);
            self.sync_explosion_sum += diff_ms as f64;
            self.sync_explosion_count += 1;
            Some(diff_ms)
        } else {
            self.audio_start_explosion_times.insert(id, started_at);
            None
        }
    }

    /// Purges residual timestamp tracking for dropped sound requests.
    pub fn purge_dropped(&mut self, id: u64) {
        self.phys_launch_times.remove(&id);
        self.phys_explosion_times.remove(&id);
        self.audio_start_launch_times.remove(&id);
        self.audio_start_explosion_times.remove(&id);
    }

    /// Updates direction of trend indicators for launch and explosion anticipation.
    pub fn update_trends(&mut self, launch_diff: f32, explosion_diff: f32) {
        self.launch_trend_dir = if launch_diff > 0.0 {
            1
        } else if launch_diff < 0.0 {
            -1
        } else {
            0
        };
        self.explosion_trend_dir = if explosion_diff > 0.0 {
            1
        } else if explosion_diff < 0.0 {
            -1
        } else {
            0
        };
    }

    /// Returns average physical-to-audio sync offsets in milliseconds for (launch, explosion).
    pub fn average_syncs(&self) -> (f64, f64) {
        let avg_launch = if self.sync_launch_count > 0 {
            self.sync_launch_sum / self.sync_launch_count as f64
        } else {
            0.0
        };
        let avg_explosion = if self.sync_explosion_count > 0 {
            self.sync_explosion_sum / self.sync_explosion_count as f64
        } else {
            0.0
        };
        (avg_launch, avg_explosion)
    }
}

/// Manages audio diagnostic tracking, ring buffer logs, and counters.
#[derive(Debug)]
pub struct AudioDiagnosticOverlay {
    pub show_audio_diagnostic: bool,
    pub last_audio_debug_update: Instant,
    pub audio_debug_records: VecDeque<AudioDebugRecord>,
    pub audio_events_buf: Vec<AudioDebugEvent>,
    pub audio_sent_rocket: u64,
    pub audio_received_rocket: u64,
    pub audio_played_rocket: u64,
    pub audio_dropped_rocket: u64,
    pub audio_completed_rocket: u64,
    pub audio_sent_explosion: u64,
    pub audio_received_explosion: u64,
    pub audio_played_explosion: u64,
    pub audio_dropped_explosion: u64,
    pub audio_completed_explosion: u64,
    pub latency_dispatch_sum: Duration,
    pub latency_dispatch_count: u64,
    pub latency_play_sum: Duration,
    pub latency_play_count: u64,
}

impl Default for AudioDiagnosticOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioDiagnosticOverlay {
    pub fn new() -> Self {
        Self {
            show_audio_diagnostic: false,
            last_audio_debug_update: Instant::now(),
            audio_debug_records: VecDeque::with_capacity(128),
            audio_events_buf: Vec::with_capacity(2048),
            audio_sent_rocket: 0,
            audio_received_rocket: 0,
            audio_played_rocket: 0,
            audio_dropped_rocket: 0,
            audio_completed_rocket: 0,
            audio_sent_explosion: 0,
            audio_received_explosion: 0,
            audio_played_explosion: 0,
            audio_dropped_explosion: 0,
            audio_completed_explosion: 0,
            latency_dispatch_sum: Duration::ZERO,
            latency_dispatch_count: 0,
            latency_play_sum: Duration::ZERO,
            latency_play_count: 0,
        }
    }

    /// Pushes a debug record into the fixed-size ring buffer (cap 128).
    pub fn push_record(&mut self, record: AudioDebugRecord) {
        if self.audio_debug_records.len() >= 128 {
            self.audio_debug_records.pop_front();
        }
        self.audio_debug_records.push_back(record);
    }

    /// Records dispatch latency sample.
    pub fn record_dispatch_latency(&mut self, duration: Duration) {
        self.latency_dispatch_sum += duration;
        self.latency_dispatch_count += 1;
    }

    /// Records play latency sample.
    pub fn record_play_latency(&mut self, duration: Duration) {
        self.latency_play_sum += duration;
        self.latency_play_count += 1;
    }

    /// Returns average dispatch latency in milliseconds.
    pub fn average_dispatch_latency_ms(&self) -> f64 {
        if self.latency_dispatch_count > 0 {
            self.latency_dispatch_sum.as_secs_f64() * 1000.0 / self.latency_dispatch_count as f64
        } else {
            0.0
        }
    }

    /// Returns average play latency in milliseconds.
    pub fn average_play_latency_ms(&self) -> f64 {
        if self.latency_play_count > 0 {
            self.latency_play_sum.as_secs_f64() * 1000.0 / self.latency_play_count as f64
        } else {
            0.0
        }
    }
}

/// Visual debug indicators for audio events in 3D space.
pub struct AudioVisualDebugOverlay {
    pub audio_event_renderer: Option<AudioEventRenderer>,
    pub audio_event_pool: Vec<AudioEvent>,
    pub show_audio_visual_overlay: bool,
}

impl Default for AudioVisualDebugOverlay {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioVisualDebugOverlay {
    pub fn new() -> Self {
        Self {
            audio_event_renderer: None,
            audio_event_pool: Vec::with_capacity(2048),
            show_audio_visual_overlay: false,
        }
    }

    pub fn push_event(&mut self, event: AudioEvent) {
        self.audio_event_pool.push(event);
    }
}

/// Adaptive framerate sampler and rolling metrics.
pub struct FpsMetrics {
    pub sampler: AdaptiveSampler,
    pub sampled_fps: Vec<f32>,
    pub fps_avg: f32,
    pub fps_avg_iter: f32,
}

impl Default for FpsMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl FpsMetrics {
    pub fn new() -> Self {
        Self {
            sampler: AdaptiveSampler::new(Duration::from_secs(5), 200, 60.0),
            sampled_fps: Vec::with_capacity(200),
            fps_avg: 0.0,
            fps_avg_iter: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio_engine::types::{AudioDebugRecord, AudioPlayStatus, AudioSoundType};

    #[test]
    fn test_t1_sync_telemetry_audio_first_inversion() {
        let mut tracker = SyncTelemetryTracker::new();
        let t0 = Instant::now();

        // 1. Audio event arrives first at t0 + 10ms
        let t_audio = t0 + Duration::from_millis(10);
        let res_audio = tracker.record_audio_started_launch(1001, t_audio);
        assert_eq!(res_audio, None);
        assert!(tracker.audio_start_launch_times.contains_key(&1001));

        // 2. Physical launch occurs later at t0 + 25ms (inversion)
        let t_phys = t0 + Duration::from_millis(25);
        let res_phys = tracker.record_launch_physic(1001, t_phys);
        assert!(res_phys.is_some());
        let diff = res_phys.unwrap();
        assert!(
            (diff - (-15.0)).abs() < 0.1,
            "Expected -15ms diff, got {}",
            diff
        );

        assert_eq!(tracker.sync_launch_count, 1);
        assert!((tracker.sync_launch_sum - (-15.0)).abs() < 0.1);
        assert!(!tracker.audio_start_launch_times.contains_key(&1001));

        // Explosion audio-first counterpart
        let exp_audio = t0 + Duration::from_millis(50);
        assert_eq!(
            tracker.record_audio_started_explosion(2002, exp_audio),
            None
        );
        let exp_phys = t0 + Duration::from_millis(70);
        let exp_diff = tracker.record_explosion_physic(2002, exp_phys).unwrap();
        assert!((exp_diff - (-20.0)).abs() < 0.1);
        assert_eq!(tracker.sync_explosion_count, 1);
    }

    #[test]
    fn test_t2_sync_telemetry_trend_transitions() {
        let mut tracker = SyncTelemetryTracker::new();
        assert_eq!(tracker.launch_trend_dir, 0);
        assert_eq!(tracker.explosion_trend_dir, 0);

        // Augmentation launch, diminution explosion
        tracker.update_trends(1.5, -2.0);
        assert_eq!(tracker.launch_trend_dir, 1);
        assert_eq!(tracker.explosion_trend_dir, -1);

        // Inversion
        tracker.update_trends(-0.5, 3.2);
        assert_eq!(tracker.launch_trend_dir, -1);
        assert_eq!(tracker.explosion_trend_dir, 1);

        // Stabilisation
        tracker.update_trends(0.0, 0.0);
        assert_eq!(tracker.launch_trend_dir, 0);
        assert_eq!(tracker.explosion_trend_dir, 0);
    }

    #[test]
    fn test_t3_sync_telemetry_purge_dropped_sanitizes_maps() {
        let mut tracker = SyncTelemetryTracker::new();
        let t0 = Instant::now();

        tracker.record_launch_physic(42, t0);
        tracker.record_explosion_physic(42, t0);
        tracker.record_audio_started_launch(99, t0);
        tracker.record_audio_started_explosion(99, t0);

        assert!(tracker.phys_launch_times.contains_key(&42));
        assert!(tracker.phys_explosion_times.contains_key(&42));
        assert!(tracker.audio_start_launch_times.contains_key(&99));
        assert!(tracker.audio_start_explosion_times.contains_key(&99));

        // Purge on Drop
        tracker.purge_dropped(42);
        tracker.purge_dropped(99);

        assert!(!tracker.phys_launch_times.contains_key(&42));
        assert!(!tracker.phys_explosion_times.contains_key(&42));
        assert!(!tracker.audio_start_launch_times.contains_key(&99));
        assert!(!tracker.audio_start_explosion_times.contains_key(&99));

        // Reusing same id works cleanly without stale interference
        let t1 = t0 + Duration::from_millis(100);
        assert_eq!(tracker.record_launch_physic(42, t1), None);
        assert!(tracker.phys_launch_times.contains_key(&42));
    }

    #[test]
    fn test_t4_audio_diagnostic_overlay_ring_buffer_cap_and_latency() {
        let mut overlay = AudioDiagnosticOverlay::new();

        // 1. Cap 128 of ring buffer
        for i in 0..135 {
            let record = AudioDebugRecord {
                request_id: i,
                sound_type: AudioSoundType::Rocket,
                entity_id: i,
                sent_at: Instant::now(),
                received_at: None,
                started_at: None,
                dropped_at: None,
                completed_at: None,
                status: AudioPlayStatus::Sent,
                voice_index: None,
                drop_reason: None,
            };
            overlay.push_record(record);
        }

        assert_eq!(overlay.audio_debug_records.len(), 128);
        assert_eq!(overlay.audio_debug_records.front().unwrap().request_id, 7);
        assert_eq!(overlay.audio_debug_records.back().unwrap().request_id, 134);

        // 2. Latency accumulators
        overlay.record_dispatch_latency(Duration::from_millis(5));
        overlay.record_dispatch_latency(Duration::from_millis(15));
        assert_eq!(overlay.latency_dispatch_count, 2);
        assert_eq!(overlay.latency_dispatch_sum, Duration::from_millis(20));
        assert!((overlay.average_dispatch_latency_ms() - 10.0).abs() < 0.001);

        overlay.record_play_latency(Duration::from_millis(20));
        overlay.record_play_latency(Duration::from_millis(40));
        assert_eq!(overlay.latency_play_count, 2);
        assert!((overlay.average_play_latency_ms() - 30.0).abs() < 0.001);
    }
}

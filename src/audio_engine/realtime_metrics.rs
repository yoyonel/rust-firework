use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// Statistiques et compteurs atomiques lock-free pour le thread audio temps réel CPAL.
/// Zéro allocation, zéro verrou, zéro I/O bloquante.
#[derive(Debug, Default)]
pub struct AudioRealtimeStats {
    pub underrun_count: AtomicU64,
    pub active_voices: AtomicUsize,
    pub last_block_duration_us: AtomicU64,
    pub last_budget_us: AtomicU64,
    pub buffer_under_allocated_count: AtomicU64,
    pub bus_under_allocated_count: AtomicU64,
    pub underrun_send_error_count: AtomicU64,
    pub dropped_blocks: AtomicU64,
    pub cpal_error_count: AtomicU64,
}

impl AudioRealtimeStats {
    pub const fn new() -> Self {
        Self {
            underrun_count: AtomicU64::new(0),
            active_voices: AtomicUsize::new(0),
            last_block_duration_us: AtomicU64::new(0),
            last_budget_us: AtomicU64::new(0),
            buffer_under_allocated_count: AtomicU64::new(0),
            bus_under_allocated_count: AtomicU64::new(0),
            underrun_send_error_count: AtomicU64::new(0),
            dropped_blocks: AtomicU64::new(0),
            cpal_error_count: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub fn record_block(&self, elapsed_us: u64, budget_us: u64, active_voices: usize) {
        self.last_block_duration_us
            .store(elapsed_us, Ordering::Relaxed);
        self.last_budget_us.store(budget_us, Ordering::Relaxed);
        self.active_voices.store(active_voices, Ordering::Relaxed);
        if elapsed_us > budget_us {
            self.underrun_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

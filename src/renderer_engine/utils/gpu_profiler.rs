use log::{debug, info};
use std::sync::{Arc, Mutex};

/// Nombre maximum de requêtes de timestamp par frame (début + fin = 32 zones max, comme en C11).
pub const MAX_GPU_QUERIES: usize = 64;

/// Représente un buffer de requêtes OpenGL pour une frame donnée.
/// Gère automatiquement l'allocation et la destruction des ID OpenGL via RAII.
pub struct GpuQueryBuffer {
    queries: [u32; MAX_GPU_QUERIES],
    pub query_count: usize,

    pub records: Vec<GpuStageRecord>,
}

impl GpuQueryBuffer {
    /// Alloue un nouveau pool de requêtes sur le GPU.
    ///
    /// # Safety
    /// Le contexte OpenGL doit être initialisé et courant sur le thread appelant.
    pub unsafe fn new() -> Self {
        let mut queries = [0u32; MAX_GPU_QUERIES];
        gl::GenQueries(MAX_GPU_QUERIES as i32, queries.as_mut_ptr());

        info!(
            "🎮 GpuQueryBuffer: {} requêtes OpenGL allouées avec succès.",
            MAX_GPU_QUERIES
        );

        Self {
            queries,
            query_count: 0,
            records: Vec::with_capacity(32),
        }
    }

    /// Récupère l'identifiant OpenGL d'une requête par son index.
    #[inline]
    pub fn get_query_id(&self, index: usize) -> u32 {
        self.queries[index]
    }

    /// Réinitialise le compteur de requêtes utilisées pour la nouvelle frame.
    #[inline]
    pub fn reset(&mut self) {
        self.query_count = 0;
        self.records.clear();
    }
}

impl Drop for GpuQueryBuffer {
    fn drop(&mut self) {
        unsafe {
            if self.queries[0] != 0 && gl::DeleteQueries::is_loaded() {
                gl::DeleteQueries(MAX_GPU_QUERIES as i32, self.queries.as_ptr());
                debug!("🧹 GpuQueryBuffer: requêtes OpenGL libérées.");
            }
        }
    }
}

/// Nombre de buffers pour le Ring-Buffer (2 = Double Buffering, parfait pour éviter les stalls).
pub const GPU_QUERY_BUFFER_COUNT: usize = 2;

/// Métadonnées d'une zone enregistrée pendant la frame courante.
pub struct GpuStageRecord {
    pub name: &'static str,
    pub color: u32,
    pub start_query_index: usize,
    pub end_query_index: usize,
    #[cfg(feature = "tracy")]
    pub tracy_span: Option<tracy_client::GpuSpan>,
}

#[derive(Clone, Debug)]
pub struct GpuStageResult {
    pub name: &'static str,
    pub color: u32,
    pub start_timestamp_ns: u64,
    pub end_timestamp_ns: u64,
    pub duration_ms: f64,
}

/// Moteur de profilage GPU double-bufferisé.
/// Alterne entre un buffer d'écriture (frame courante) et un buffer de lecture (frame N-1).
pub struct GpuProfiler {
    buffers: [GpuQueryBuffer; GPU_QUERY_BUFFER_COUNT],
    pub write_index: usize,
    pub read_index: usize,
    pub latest_results: Vec<GpuStageResult>,
    pub enabled: bool,
    #[cfg(feature = "tracy")]
    tracy_ctx: Option<tracy_client::GpuContext>,
}

impl GpuProfiler {
    /// Initialise le profiler GPU avec ses 2 buffers de requêtes OpenGL.
    ///
    /// # Safety
    /// Le contexte OpenGL doit être actif sur le thread appelant.
    pub unsafe fn new() -> Self {
        let buffers = [GpuQueryBuffer::new(), GpuQueryBuffer::new()];

        #[cfg(feature = "tracy")]
        let tracy_ctx = {
            let mut gl_timestamp: i64 = 0;
            if gl::GetInteger64v::is_loaded() {
                gl::GetInteger64v(gl::TIMESTAMP, &mut gl_timestamp);
            }
            tracy_client::Client::running().and_then(|client| {
                client
                    .new_gpu_context(
                        Some("OpenGL Main Context"),
                        tracy_client::GpuContextType::OpenGL,
                        gl_timestamp,
                        1.0,
                    )
                    .ok()
            })
        };
        Self {
            buffers,
            write_index: 0,
            read_index: 1,
            latest_results: Vec::with_capacity(32),
            enabled: false,
            #[cfg(feature = "tracy")]
            tracy_ctx,
        }
    }

    /// Prépare le profiler pour une nouvelle frame : bascule les index d'écriture et de lecture.
    pub fn begin_frame(&mut self) {
        if !self.enabled {
            return;
        }

        unsafe {
            self.collect_results();
        }

        // Bascule (Swap) du double-buffering
        self.read_index = self.write_index;
        self.write_index = (self.write_index + 1) % GPU_QUERY_BUFFER_COUNT;

        // Réinitialisation du buffer d'écriture pour la nouvelle frame
        self.buffers[self.write_index].reset();
    }

    unsafe fn collect_results(&mut self) {
        self.latest_results.clear();
        if !gl::GetQueryObjectui64v::is_loaded() {
            return;
        }

        let read_buffer = &mut self.buffers[self.read_index];
        if read_buffer.records.is_empty() {
            return;
        }

        // 1. Test anti-blocage : on vérifie si la dernière requête enregistrée est terminée par le GPU
        if let Some(last_record) = read_buffer.records.last() {
            let last_query_id = read_buffer.get_query_id(last_record.end_query_index);
            let mut available: i32 = 0;
            if gl::GetQueryObjectiv::is_loaded() {
                gl::GetQueryObjectiv(last_query_id, gl::QUERY_RESULT_AVAILABLE, &mut available);
                if available == gl::FALSE as i32 {
                    // ⚠️ Le GPU est encore en train de travailler sur cette frame !
                    // On abandonne la lecture pour cette frame afin de ne surtout pas bloquer le CPU.
                    return;
                }
            }
        }

        // 2. Lecture garantie sans stall : toutes les requêtes sont prêtes !
        let records_count = read_buffer.records.len();
        for i in 0..records_count {
            let start_id = read_buffer.get_query_id(read_buffer.records[i].start_query_index);
            let end_id = read_buffer.get_query_id(read_buffer.records[i].end_query_index);

            let mut start_ts: u64 = 0;
            let mut end_ts: u64 = 0;

            gl::GetQueryObjectui64v(start_id, gl::QUERY_RESULT, &mut start_ts);
            gl::GetQueryObjectui64v(end_id, gl::QUERY_RESULT, &mut end_ts);

            let duration_ns = end_ts.saturating_sub(start_ts);
            let duration_ms = (duration_ns as f64) / 1_000_000.0;

            self.latest_results.push(GpuStageResult {
                name: read_buffer.records[i].name,
                color: read_buffer.records[i].color,
                start_timestamp_ns: start_ts,
                end_timestamp_ns: end_ts,
                duration_ms,
            });

            // 🟢 NOUVEAU : On transfère les timestamps du passé à la zone Tracy sauvée à la frame N-1
            #[cfg(feature = "tracy")]
            if let Some(span) = read_buffer.records[i].tracy_span.take() {
                span.upload_timestamp_start(start_ts as i64);
                span.upload_timestamp_end(end_ts as i64);
            }
        }
    }

    /// Enregistre le timestamp de DÉBUT d'une zone sur le GPU.
    /// Retourne l'index du record créé si l'enregistrement a réussi.
    ///
    /// # Safety
    /// Le contexte OpenGL doit être actif.
    /// Enregistre le timestamp de DÉBUT d'une zone sur le GPU.
    pub unsafe fn start_stage(
        &mut self,
        name: &'static str,
        color: u32,
        _file: &'static str,
        _line: u32,
    ) -> Option<usize> {
        if !self.enabled || !gl::QueryCounter::is_loaded() {
            return None;
        }

        let buffer = &mut self.buffers[self.write_index];
        if buffer.query_count + 2 > MAX_GPU_QUERIES {
            return None;
        }

        let start_query_index = buffer.query_count;
        let query_id = buffer.get_query_id(start_query_index);
        buffer.query_count += 1;

        gl::QueryCounter(query_id, gl::TIMESTAMP);

        #[cfg(feature = "tracy")]
        let tracy_span = self.tracy_ctx.as_ref().and_then(|ctx| {
            #[cfg(feature = "tracy_custom_color")]
            {
                ctx.span_alloc_color(name, "GPU", _file, _line, color).ok()
            }
            #[cfg(not(feature = "tracy_custom_color"))]
            {
                ctx.span_alloc(name, "GPU", _file, _line).ok()
            }
        });

        let record_index = buffer.records.len();
        buffer.records.push(GpuStageRecord {
            name,
            color,
            start_query_index,
            end_query_index: 0,
            #[cfg(feature = "tracy")]
            tracy_span,
        });

        Some(record_index)
    }

    /// Enregistre le timestamp de FIN d'une zone sur le GPU.
    ///
    /// # Safety
    /// Le contexte OpenGL doit être actif.
    pub unsafe fn end_stage(&mut self, record_index: usize) {
        if !self.enabled || !gl::QueryCounter::is_loaded() {
            return;
        }

        let buffer = &mut self.buffers[self.write_index];
        if buffer.query_count >= MAX_GPU_QUERIES || record_index >= buffer.records.len() {
            return;
        }

        let end_query_index = buffer.query_count;
        let query_id = buffer.get_query_id(end_query_index);
        buffer.query_count += 1;

        gl::QueryCounter(query_id, gl::TIMESTAMP);

        buffer.records[record_index].end_query_index = end_query_index;

        // On informe Tracy que la commande GPU est poussée
        #[cfg(feature = "tracy")]
        if let Some(span) = &mut buffer.records[record_index].tracy_span {
            span.end_zone();
        }
    }
}

/// Garde RAII unifié gérant simultanément un groupe RenderDoc (Push/PopDebugGroup)
/// et une zone de chronométrage OpenGL dans le ring-buffer (start_stage/end_stage).
pub struct GpuProfileGuard {
    profiler: Arc<Mutex<GpuProfiler>>,
    record_index: Option<usize>,
}

impl GpuProfileGuard {
    pub fn new(
        profiler: Arc<Mutex<GpuProfiler>>,
        id: u32,
        name: &'static str,
        color: u32,
        _file: &'static str, // 🟢 NOUVEAU
        _line: u32,          // 🟢 NOUVEAU
    ) -> Self {
        if gl::PushDebugGroup::is_loaded() {
            if let Ok(c_str) = std::ffi::CString::new(name) {
                #[allow(unused_unsafe)]
                unsafe {
                    gl::PushDebugGroup(gl::DEBUG_SOURCE_APPLICATION, id, -1, c_str.as_ptr());
                }
            }
        }

        let record_index = if let Ok(mut lock) = profiler.lock() {
            // 🟢 NOUVEAU : Transfert au profiler
            unsafe { lock.start_stage(name, color, _file, _line) }
        } else {
            None
        };

        Self {
            profiler,
            record_index,
        }
    }
}

impl Drop for GpuProfileGuard {
    fn drop(&mut self) {
        // 1. Clôture de la zone matérielle dans notre GpuProfiler
        if let Some(index) = self.record_index {
            if let Ok(mut lock) = self.profiler.lock() {
                unsafe {
                    lock.end_stage(index);
                }
            }
        }

        // 2. RenderDoc Pop Debug Group (garantit l'ordre LIFO parfait)
        if gl::PopDebugGroup::is_loaded() {
            #[allow(unused_unsafe)]
            unsafe {
                gl::PopDebugGroup();
            }
        }
    }
}

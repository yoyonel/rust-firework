use crossbeam_channel::{unbounded, Receiver, Sender};
use hound::{WavSpec, WavWriter};
use log::info;
use std::{
    fs::File,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

/// Bloc audio identifié
#[derive(Debug)]
pub struct AudioBlock {
    pub index: u64,
    pub frames: Vec<[f32; 2]>,
}

/// Writer audio sûr et asynchrone
pub struct SafeWavWriter {
    pub tx: Sender<AudioBlock>,
    handle: Option<thread::JoinHandle<()>>,
    stop_pair: Arc<(Mutex<bool>, Condvar)>, // signal de fin
}

const BLOCK_DURATION_SECS: u64 = 2; // flush toutes les 2 secondes

impl SafeWavWriter {
    /// Crée un nouveau writer avec un fichier WAV existant ou nouveau
    pub fn new(path: &str, sample_rate: u32) -> Self {
        type AudioSender = Sender<AudioBlock>;
        type AudioReceiver = Receiver<AudioBlock>;

        let (tx, rx): (AudioSender, AudioReceiver) = unbounded();

        // Condvar pour arrêter le thread proprement
        let stop_pair = Arc::new((Mutex::new(true), Condvar::new()));
        let stop_pair_clone = stop_pair.clone();

        let path_string = path.to_string();
        info!(
            "Starting SafeWavWriter thread for exporting audio to WAV file at path: {}",
            path_string
        );
        let handle = thread::spawn(move || {
            let spec = WavSpec {
                channels: 2,
                sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let file = File::create(&path_string).unwrap_or_else(|e| {
                panic!("Failed to open WAV file at '{}': {}", path_string, e);
            });
            let mut writer = WavWriter::new(file, spec).expect("Failed to create WAV writer");

            let mut total_samples: u64 = 0;
            let mut last_flush = Instant::now();

            loop {
                // Lecture bloc audio avec timeout pour gérer le flush périodique
                let block_opt = rx.recv_timeout(Duration::from_millis(50));
                match block_opt {
                    Ok(block) => {
                        // 🔹 Écriture du bloc
                        for frame in block.frames {
                            let left = (frame[0].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            let right = (frame[1].clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                            writer.write_sample(left).ok();
                            writer.write_sample(right).ok();
                            total_samples += 2;
                        }

                        // 🔹 Flush périodique
                        if last_flush.elapsed() >= Duration::from_secs(BLOCK_DURATION_SECS) {
                            writer.flush().ok();
                            info!(
                                "💾 [SafeWavWriter] Periodic flush after block #{:04} ({} samples)",
                                block.index, total_samples
                            );
                            last_flush = Instant::now();
                        }
                    }
                    Err(_) => {
                        // Vérifie signal de stop
                        let (lock, _cvar) = &*stop_pair_clone;
                        let running = lock.lock().unwrap();
                        if *running {
                            continue;
                        } else {
                            break;
                        }
                    }
                }
            }

            // 🔸 Flush final et finalize
            writer.flush().ok();
            writer.finalize().ok();
            info!(
                "🛑 [SafeWavWriter] Thread stopped, WAV file finalized ({} samples)",
                total_samples
            );
        });

        Self {
            tx,
            handle: Some(handle),
            stop_pair,
        }
    }

    /// Pousse un bloc audio dans le writer
    pub fn push_block(&self, block: AudioBlock) {
        let _ = self.tx.send(block);
    }

    /// Stoppe le thread et finalise le fichier
    pub fn stop(&mut self) {
        let (lock, cvar) = &*self.stop_pair;
        {
            let mut running = lock.lock().unwrap();
            *running = false; // signal de stop
            cvar.notify_all();
        }

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

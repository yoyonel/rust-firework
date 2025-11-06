use std::time::Instant;

/// Event envoyé par le renderer/physic au moteur audio
#[derive(Clone, Debug)]
pub struct DopplerEvent {
    pub id: u64,            // id unique de la rocket
    pub pos: (f32, f32),    // (x, y)
    pub vel: (f32, f32),    // velocity (vx, vy) si dispo (optionnel)
    pub gain: f32,          // per-source gain
    pub timestamp: Instant, // moment de mesure côté renderer
}

impl Default for DopplerEvent {
    fn default() -> Self {
        Self {
            id: 0,
            pos: (0.0, 0.0),
            vel: (0.0, 0.0),
            gain: 1.0,
            timestamp: Instant::now(),
        }
    }
}

/// Thread-safe queue : crossbeam channel (sender côté renderer, receiver côté audio)
pub mod doppler_queue {
    use super::DopplerEvent;
    use crossbeam::channel::{unbounded, Receiver, Sender};

    #[derive(Clone)]
    pub struct DopplerQueue {
        pub sender: Sender<DopplerEvent>,
        pub receiver: Receiver<DopplerEvent>,
    }

    impl Default for DopplerQueue {
        fn default() -> Self {
            Self::new()
        }
    }

    impl DopplerQueue {
        pub fn new() -> Self {
            let (s, r) = unbounded();
            Self {
                sender: s,
                receiver: r,
            }
        }
    }
}

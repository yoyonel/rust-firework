pub use crate::domain_contracts::DopplerEvent;

/// Thread-safe queue : crossbeam channel (sender côté renderer, receiver côté audio)
pub mod doppler_queue {
    use super::DopplerEvent;
    use crossbeam::channel::{bounded, Receiver, Sender};

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
            let (s, r) = bounded(8192);
            Self {
                sender: s,
                receiver: r,
            }
        }
    }
}

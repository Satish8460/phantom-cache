use rand::prelude::*;
use std::sync::Mutex;

pub trait RngSource: Send + Sync {
    fn next_u64(&self) -> u64;
    fn next_f64(&self) -> f64;
}

pub struct ThreadRngSource;

impl RngSource for ThreadRngSource {
    fn next_u64(&self) -> u64 {
        rand::thread_rng().gen()
    }

    fn next_f64(&self) -> f64 {
        rand::thread_rng().gen()
    }
}

pub struct SeededRngSource {
    rng: Mutex<StdRng>,
}

impl SeededRngSource {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: Mutex::new(StdRng::seed_from_u64(seed)),
        }
    }
}

impl RngSource for SeededRngSource {
    fn next_u64(&self) -> u64 {
        let mut guard = self.rng.lock().unwrap();
        guard.gen()
    }

    fn next_f64(&self) -> f64 {
        let mut guard = self.rng.lock().unwrap();
        guard.gen()
    }
}

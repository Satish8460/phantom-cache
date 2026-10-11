pub trait Clock: Send + Sync {
    fn now_ms(&self) -> u64;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

pub struct VirtualClock {
    current_time_ms: std::sync::atomic::AtomicU64,
}

impl VirtualClock {
    pub fn new(start_time_ms: u64) -> Self {
        Self {
            current_time_ms: std::sync::atomic::AtomicU64::new(start_time_ms),
        }
    }

    pub fn advance_ms(&self, delta_ms: u64) {
        self.current_time_ms
            .fetch_add(delta_ms, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn set_ms(&self, time_ms: u64) {
        self.current_time_ms
            .store(time_ms, std::sync::atomic::Ordering::SeqCst);
    }
}

impl Clock for VirtualClock {
    fn now_ms(&self) -> u64 {
        self.current_time_ms
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

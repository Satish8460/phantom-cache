//! Virtual integer-microsecond clock for Tier-1 simulation (M4-01).
//!
//! Enforces non-decreasing virtual time. Never accesses the wall clock.
//! All time is integer microseconds (`u64`), converting cleanly to integer milliseconds (`now_ms = time_us / 1000`)
//! for compatibility with `PartitionedCache` and RFC 9111 freshness evaluation.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct VirtualClock {
    time_us: u64,
}

impl VirtualClock {
    pub fn new() -> Self {
        Self { time_us: 0 }
    }

    pub fn with_start_time(start_us: u64) -> Self {
        Self { time_us: start_us }
    }

    pub fn now_us(&self) -> u64 {
        self.time_us
    }

    pub fn now_ms(&self) -> u64 {
        self.time_us / 1000
    }

    pub fn advance_to(&mut self, target_us: u64) {
        assert!(
            target_us >= self.time_us,
            "Virtual clock violation: attempted to regress from {} us to {} us",
            self.time_us,
            target_us
        );
        self.time_us = target_us;
    }

    pub fn advance_by(&mut self, delta_us: u64) {
        self.time_us = self.time_us.saturating_add(delta_us);
    }
}


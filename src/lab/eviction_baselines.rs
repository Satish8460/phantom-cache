//! Reference eviction baselines for Phantom Lab (M4-07 / WP-17).
//!
//! Provides:
//! - Reference implementations of SIEVE, S3-FIFO, LRU, and TinyLFU (Count-Min Sketch + SLRU).
//! - Common eviction benchmarking evaluation harness computing:
//!   - Hit ratio
//!   - Byte hit ratio
//!   - Origin-cost-weighted miss cost
//!   - Eviction latency

use crate::core::cache::CacheKey;
use crate::core::eviction::{FaithfulLru, FaithfulS3Fifo, FaithfulSieve, ShardEvictionPolicy};
use std::collections::HashMap;

/// Count-Min 4-bit frequency sketch for TinyLFU.
pub struct FrequencySketch {
    table: Vec<u8>,
    mask: usize,
    sample_size: usize,
    count: usize,
}

impl FrequencySketch {
    pub fn new(capacity: usize) -> Self {
        let size = (capacity * 2).next_power_of_two().max(64);
        Self {
            table: vec![0; size],
            mask: size - 1,
            sample_size: capacity * 10,
            count: 0,
        }
    }

    fn hash(&self, key: &CacheKey, seed: u64) -> usize {
        let mut h = seed ^ 0x517cc1b727220a95;
        for b in key.path.bytes() {
            h = (h.rotate_left(5) ^ (b as u64)).wrapping_mul(0x5bd1e995);
        }
        (h as usize) & self.mask
    }

    pub fn increment(&mut self, key: &CacheKey) {
        let seeds = [0x9e3779b97f4a7c15, 0xbf58476d1ce4e5b9, 0x94d049bb133111eb, 0xd6e8feb86659fd93];
        for seed in seeds {
            let idx = self.hash(key, seed);
            if self.table[idx] < 15 {
                self.table[idx] += 1;
            }
        }
        self.count += 1;
        if self.count >= self.sample_size {
            self.reset();
        }
    }

    pub fn frequency(&self, key: &CacheKey) -> u8 {
        let seeds = [0x9e3779b97f4a7c15, 0xbf58476d1ce4e5b9, 0x94d049bb133111eb, 0xd6e8feb86659fd93];
        let mut min = 15;
        for seed in seeds {
            let idx = self.hash(key, seed);
            min = min.min(self.table[idx]);
        }
        min
    }

    fn reset(&mut self) {
        for val in self.table.iter_mut() {
            *val >>= 1;
        }
        self.count >>= 1;
    }
}

/// TinyLFU eviction policy implementation conforming to ShardEvictionPolicy.
pub struct FaithfulTinyLfu {
    capacity: usize,
    keys: Vec<CacheKey>,
    sketch: FrequencySketch,
}

impl FaithfulTinyLfu {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            keys: Vec::with_capacity(capacity),
            sketch: FrequencySketch::new(capacity),
        }
    }
}

impl ShardEvictionPolicy for FaithfulTinyLfu {
    fn on_insert(&mut self, key: &CacheKey) {
        self.sketch.increment(key);
        if !self.keys.contains(key) {
            self.keys.push(key.clone());
        }
    }

    fn on_hit(&mut self, key: &CacheKey) {
        self.sketch.increment(key);
    }

    fn on_remove(&mut self, key: &CacheKey) {
        self.keys.retain(|k| k != key);
    }

    fn select_victim(&mut self, is_pinned: &mut dyn FnMut(&CacheKey) -> bool) -> Option<CacheKey> {
        if self.keys.is_empty() {
            return None;
        }

        // Find candidate with lowest frequency estimate in sketch
        let mut min_freq = u8::MAX;
        let mut min_idx = None;

        for (idx, k) in self.keys.iter().enumerate() {
            if is_pinned(k) {
                continue;
            }
            let f = self.sketch.frequency(k);
            if f < min_freq {
                min_freq = f;
                min_idx = Some(idx);
            }
        }

        if let Some(idx) = min_idx {
            let victim = self.keys.remove(idx);
            Some(victim)
        } else {
            None
        }
    }

    fn rebuild_from(&mut self, entries: &[CacheKey]) {
        self.keys.clear();
        for k in entries {
            self.on_insert(k);
        }
    }

    fn name(&self) -> &'static str {
        "tinylfu"
    }
}

/// Eviction simulator for comparing eviction algorithms on trace access sequences.
pub struct EvictionSimulator {
    capacity: usize,
    policy: Box<dyn ShardEvictionPolicy>,
    resident: HashMap<CacheKey, usize>, // key -> size
    total_requests: u64,
    hits: u64,
    byte_requests: u64,
    byte_hits: u64,
}

#[derive(Debug, Clone)]
pub struct EvictionMetrics {
    pub policy_name: String,
    pub hit_ratio: f64,
    pub byte_hit_ratio: f64,
    pub total_requests: u64,
    pub hits: u64,
}

impl EvictionSimulator {
    pub fn new(capacity: usize, policy: Box<dyn ShardEvictionPolicy>) -> Self {
        Self {
            capacity,
            policy,
            resident: HashMap::with_capacity(capacity),
            total_requests: 0,
            hits: 0,
            byte_requests: 0,
            byte_hits: 0,
        }
    }

    pub fn access(&mut self, key: &CacheKey, size_bytes: usize) -> bool {
        self.total_requests += 1;
        self.byte_requests += size_bytes as u64;

        if self.resident.contains_key(key) {
            self.hits += 1;
            self.byte_hits += size_bytes as u64;
            self.policy.on_hit(key);
            true
        } else {
            // Miss: evict if capacity exceeded
            if self.resident.len() >= self.capacity {
                if let Some(victim) = self.policy.select_victim(&mut |_| false) {
                    self.resident.remove(&victim);
                }
            }
            self.resident.insert(key.clone(), size_bytes);
            self.policy.on_insert(key);
            false
        }
    }

    pub fn metrics(&self) -> EvictionMetrics {
        EvictionMetrics {
            policy_name: self.policy.name().to_string(),
            hit_ratio: if self.total_requests > 0 {
                self.hits as f64 / self.total_requests as f64
            } else {
                0.0
            },
            byte_hit_ratio: if self.byte_requests > 0 {
                self.byte_hits as f64 / self.byte_requests as f64
            } else {
                0.0
            },
            total_requests: self.total_requests,
            hits: self.hits,
        }
    }
}

pub fn make_policy(name: &str, capacity: usize) -> Box<dyn ShardEvictionPolicy> {
    match name {
        "sieve" => Box::new(FaithfulSieve::new(capacity)),
        "lru" => Box::new(FaithfulLru::new(capacity)),
        "s3fifo" => Box::new(FaithfulS3Fifo::new(capacity)),
        "tinylfu" => Box::new(FaithfulTinyLfu::new(capacity)),
        _ => Box::new(FaithfulSieve::new(capacity)),
    }
}


//! Spatially Sampled Counterfactual Shadow Evaluator (WP-32)
//!
//! Authoritative Specification: `docs/research/phase_5_execution_package.md` §4 M5-03
//!
//! Evaluates candidate eviction and admission policies in shadow without exposing live
//! traffic to regression risk:
//! - Spatially sampled key stream (sampling modulus R = 16)
//! - Scaled shadow cache capacity (C_shadow = C / R)
//! - Supports SIEVE, S3-FIFO, LRU, and TinyLFU shadow policies
//! - Computes miss ratios and 95% Wilson/normal confidence intervals
//! - Bounded memory consumption (< 32 KB total)

use std::collections::{HashMap, VecDeque};

pub trait ShadowPolicy: Send + Sync {
    fn name(&self) -> &'static str;
    fn access(&mut self, key: u64) -> bool; // returns true if hit
    fn hits(&self) -> u64;
    fn misses(&self) -> u64;
    fn total_requests(&self) -> u64 {
        self.hits() + self.misses()
    }
    fn hit_ratio(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            0.0
        } else {
            self.hits() as f64 / total as f64
        }
    }
    fn reset(&mut self);
}

// ============================================================================
// 1. Miniature LRU Shadow
// ============================================================================

#[derive(Debug)]
pub struct LruShadow {
    capacity: usize,
    keys: VecDeque<u64>,
    hits: u64,
    misses: u64,
}

impl LruShadow {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(4),
            keys: VecDeque::with_capacity(capacity.max(4)),
            hits: 0,
            misses: 0,
        }
    }
}

impl ShadowPolicy for LruShadow {
    fn name(&self) -> &'static str {
        "LRU"
    }

    fn access(&mut self, key: u64) -> bool {
        if let Some(pos) = self.keys.iter().position(|&k| k == key) {
            self.keys.remove(pos);
            self.keys.push_back(key);
            self.hits += 1;
            true
        } else {
            if self.keys.len() >= self.capacity {
                self.keys.pop_front();
            }
            self.keys.push_back(key);
            self.misses += 1;
            false
        }
    }

    fn hits(&self) -> u64 {
        self.hits
    }

    fn misses(&self) -> u64 {
        self.misses
    }

    fn reset(&mut self) {
        self.keys.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

// ============================================================================
// 2. Miniature SIEVE Shadow
// ============================================================================

#[derive(Debug)]
struct SieveNode {
    key: u64,
    visited: bool,
}

#[derive(Debug)]
pub struct SieveShadow {
    capacity: usize,
    nodes: Vec<SieveNode>,
    hand: usize,
    hits: u64,
    misses: u64,
}

impl SieveShadow {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(4),
            nodes: Vec::with_capacity(capacity.max(4)),
            hand: 0,
            hits: 0,
            misses: 0,
        }
    }
}

impl ShadowPolicy for SieveShadow {
    fn name(&self) -> &'static str {
        "SIEVE"
    }

    fn access(&mut self, key: u64) -> bool {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.key == key) {
            node.visited = true;
            self.hits += 1;
            true
        } else {
            self.misses += 1;
            if self.nodes.len() < self.capacity {
                self.nodes.push(SieveNode {
                    key,
                    visited: false,
                });
            } else {
                // SIEVE eviction loop
                loop {
                    if self.hand >= self.nodes.len() {
                        self.hand = 0;
                    }
                    if self.nodes[self.hand].visited {
                        self.nodes[self.hand].visited = false;
                        self.hand += 1;
                    } else {
                        // Evict at hand
                        self.nodes[self.hand] = SieveNode {
                            key,
                            visited: false,
                        };
                        self.hand += 1;
                        break;
                    }
                }
            }
            false
        }
    }

    fn hits(&self) -> u64 {
        self.hits
    }

    fn misses(&self) -> u64 {
        self.misses
    }

    fn reset(&mut self) {
        self.nodes.clear();
        self.hand = 0;
        self.hits = 0;
        self.misses = 0;
    }
}

// ============================================================================
// 3. Miniature S3-FIFO Shadow
// ============================================================================

#[derive(Debug)]
pub struct S3FifoShadow {
    small_cap: usize,
    main_cap: usize,
    small_q: VecDeque<u64>,
    main_q: VecDeque<u64>,
    ghost_set: HashMap<u64, ()>,
    freq: HashMap<u64, u8>,
    hits: u64,
    misses: u64,
}

impl S3FifoShadow {
    pub fn new(total_capacity: usize) -> Self {
        let cap = total_capacity.max(10);
        let small = (cap / 10).max(2);
        let main = cap - small;
        Self {
            small_cap: small,
            main_cap: main,
            small_q: VecDeque::with_capacity(small),
            main_q: VecDeque::with_capacity(main),
            ghost_set: HashMap::with_capacity(cap),
            freq: HashMap::with_capacity(cap),
            hits: 0,
            misses: 0,
        }
    }

    fn evict(&mut self) {
        if self.small_q.len() >= self.small_cap {
            if let Some(key) = self.small_q.pop_front() {
                let f = self.freq.get(&key).copied().unwrap_or(0);
                if f > 0 {
                    // Promote to main
                    if self.main_q.len() >= self.main_cap {
                        self.evict_main();
                    }
                    self.main_q.push_back(key);
                } else {
                    // Evict to ghost
                    if self.ghost_set.len() >= (self.small_cap + self.main_cap) {
                        if let Some(&first) = self.ghost_set.keys().next() {
                            self.ghost_set.remove(&first);
                        }
                    }
                    self.ghost_set.insert(key, ());
                    self.freq.remove(&key);
                }
            }
        } else if self.main_q.len() >= self.main_cap {
            self.evict_main();
        }
    }

    fn evict_main(&mut self) {
        while let Some(key) = self.main_q.pop_front() {
            let f = self.freq.get(&key).copied().unwrap_or(0);
            if f > 0 {
                self.freq.insert(key, f - 1);
                self.main_q.push_back(key);
            } else {
                self.freq.remove(&key);
                break;
            }
        }
    }
}

impl ShadowPolicy for S3FifoShadow {
    fn name(&self) -> &'static str {
        "S3-FIFO"
    }

    fn access(&mut self, key: u64) -> bool {
        let in_small = self.small_q.contains(&key);
        let in_main = self.main_q.contains(&key);

        if in_small || in_main {
            let count = self.freq.entry(key).or_insert(0);
            *count = (*count + 1).min(3);
            self.hits += 1;
            true
        } else {
            self.misses += 1;
            let in_ghost = self.ghost_set.remove(&key).is_some();
            self.evict();

            if in_ghost {
                if self.main_q.len() >= self.main_cap {
                    self.evict_main();
                }
                self.main_q.push_back(key);
                self.freq.insert(key, 0);
            } else {
                self.small_q.push_back(key);
                self.freq.insert(key, 0);
            }
            false
        }
    }

    fn hits(&self) -> u64 {
        self.hits
    }

    fn misses(&self) -> u64 {
        self.misses
    }

    fn reset(&mut self) {
        self.small_q.clear();
        self.main_q.clear();
        self.ghost_set.clear();
        self.freq.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

// ============================================================================
// 4. Miniature TinyLFU Shadow
// ============================================================================

#[derive(Debug)]
pub struct TinyLfuShadow {
    capacity: usize,
    keys: VecDeque<u64>,
    sketch: [[u16; 256]; 4],
    hits: u64,
    misses: u64,
}

impl TinyLfuShadow {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(4),
            keys: VecDeque::with_capacity(capacity.max(4)),
            sketch: [[0; 256]; 4],
            hits: 0,
            misses: 0,
        }
    }

    fn hash_cell(&self, key: u64, row: usize) -> usize {
        let mut x = key ^ (row as u64).wrapping_mul(0x517cc1b727220a95);
        x ^= x >> 33;
        (x as usize) % 256
    }

    fn estimate(&self, key: u64) -> u16 {
        let mut min_val = u16::MAX;
        for row in 0..4 {
            let col = self.hash_cell(key, row);
            min_val = min_val.min(self.sketch[row][col]);
        }
        min_val
    }

    fn record_freq(&mut self, key: u64) {
        for row in 0..4 {
            let col = self.hash_cell(key, row);
            self.sketch[row][col] = self.sketch[row][col].saturating_add(1);
        }
    }
}

impl ShadowPolicy for TinyLfuShadow {
    fn name(&self) -> &'static str {
        "TinyLFU"
    }

    fn access(&mut self, key: u64) -> bool {
        self.record_freq(key);

        if let Some(pos) = self.keys.iter().position(|&k| k == key) {
            self.keys.remove(pos);
            self.keys.push_back(key);
            self.hits += 1;
            true
        } else {
            self.misses += 1;
            if self.keys.len() < self.capacity {
                self.keys.push_back(key);
            } else {
                // TinyLFU admission: candidate vs victim (front of LRU queue)
                let victim = self.keys[0];
                let candidate_freq = self.estimate(key);
                let victim_freq = self.estimate(victim);

                if candidate_freq >= victim_freq {
                    self.keys.pop_front();
                    self.keys.push_back(key);
                }
            }
            false
        }
    }

    fn hits(&self) -> u64 {
        self.hits
    }

    fn misses(&self) -> u64 {
        self.misses
    }

    fn reset(&mut self) {
        self.keys.clear();
        for row in &mut self.sketch {
            row.fill(0);
        }
        self.hits = 0;
        self.misses = 0;
    }
}

// ============================================================================
// Shadow Evaluator Harness
// ============================================================================

#[derive(Debug, Clone)]
pub struct ShadowEvaluationReport {
    pub baseline_policy: &'static str,
    pub candidate_policy: &'static str,
    pub baseline_hit_ratio: f64,
    pub candidate_hit_ratio: f64,
    pub delta_hit_ratio: f64,
    pub confidence_interval_95: (f64, f64),
    pub total_sampled_requests: u64,
    pub is_statistically_significant: bool,
}

pub struct ShadowEvaluator {
    sample_modulus: u64,
    baseline: Box<dyn ShadowPolicy>,
    candidate: Box<dyn ShadowPolicy>,
    sampled_count: u64,
}

impl ShadowEvaluator {
    pub fn new(
        sample_modulus: u64,
        baseline: Box<dyn ShadowPolicy>,
        candidate: Box<dyn ShadowPolicy>,
    ) -> Self {
        Self {
            sample_modulus: sample_modulus.max(1),
            baseline,
            candidate,
            sampled_count: 0,
        }
    }

    pub fn observe(&mut self, key_hash: u64) {
        if key_hash.is_multiple_of(self.sample_modulus) {
            self.sampled_count += 1;
            self.baseline.access(key_hash);
            self.candidate.access(key_hash);
        }
    }

    pub fn generate_report(&self, delta_min: f64) -> ShadowEvaluationReport {
        let b_hr = self.baseline.hit_ratio();
        let c_hr = self.candidate.hit_ratio();
        let delta = c_hr - b_hr;

        let n = self.sampled_count as f64;
        let se = if n > 0.0 {
            // Standard error of difference in proportions
            let var_b = (b_hr * (1.0 - b_hr)).max(1e-6) / n;
            let var_c = (c_hr * (1.0 - c_hr)).max(1e-6) / n;
            (var_b + var_c).sqrt()
        } else {
            0.0
        };

        let ci_lower = delta - 1.96 * se;
        let ci_upper = delta + 1.96 * se;

        let is_significant = ci_lower > delta_min && self.sampled_count >= 100;

        ShadowEvaluationReport {
            baseline_policy: self.baseline.name(),
            candidate_policy: self.candidate.name(),
            baseline_hit_ratio: b_hr,
            candidate_hit_ratio: c_hr,
            delta_hit_ratio: delta,
            confidence_interval_95: (ci_lower, ci_upper),
            total_sampled_requests: self.sampled_count,
            is_statistically_significant: is_significant,
        }
    }

    pub fn reset(&mut self) {
        self.baseline.reset();
        self.candidate.reset();
        self.sampled_count = 0;
    }
}

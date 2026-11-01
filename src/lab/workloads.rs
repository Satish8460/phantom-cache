//! Workload generator library for Phantom Lab (M4-02 / WP-15).
//!
//! Implements stable workload families:
//! - W-U: Uniform key distribution
//! - W-Z(α): Zipfian skew with α in {0.6, 0.8, 1.0, 1.2} via O(1) AliasSampler
//! - W-SC: Scans (one-hit wonders at fraction f)
//! - W-SH: Hot-set shift at virtual time t_s
//! - W-FC: Flash crowd (rate x k on m keys for duration d)
//! - W-HK: Hot-key emergence
//! - W-DI: Diurnal pattern (sinusoidal rate variation)
//! - W-MX: Composite mixtures
//! - W-SZ: Object size distributions (fixed, lognormal, bimodal)
//! - W-TTL: Heterogeneous freshness lifetimes
//! - W-INV: Invalidation streams

use super::rng::{AliasSampler, StreamRng};

/// A generated request specification.
#[derive(Debug, Clone)]
pub struct RequestSpec {
    pub arrival_delta_us: u64,
    pub key: String,
    pub body_size_bytes: usize,
    pub ttl_ms: u64,
    pub is_invalidation: bool,
}

pub trait WorkloadGenerator {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec;
}

// ---------------------------------------------------------------------------
// W-U: Uniform Workload
// ---------------------------------------------------------------------------

pub struct UniformWorkload {
    catalog_size: usize,
    base_rate_req_sec: f64,
    default_ttl_ms: u64,
    default_body_size: usize,
}

impl UniformWorkload {
    pub fn new(catalog_size: usize, base_rate_req_sec: f64) -> Self {
        Self {
            catalog_size,
            base_rate_req_sec,
            default_ttl_ms: 60_000,
            default_body_size: 1024,
        }
    }
}

impl WorkloadGenerator for UniformWorkload {
    fn next_request(&mut self, _now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        // Poisson inter-arrival time in microseconds
        let lambda = self.base_rate_req_sec / 1_000_000.0;
        let delta_us = (rng.next_exponential(lambda) as u64).max(1);
        let key_idx = rng.next_range_u64(0, (self.catalog_size - 1) as u64);

        RequestSpec {
            arrival_delta_us: delta_us,
            key: format!("/key_{}", key_idx),
            body_size_bytes: self.default_body_size,
            ttl_ms: self.default_ttl_ms,
            is_invalidation: false,
        }
    }
}

// ---------------------------------------------------------------------------
// W-Z(α): Zipfian Skew Workload
// ---------------------------------------------------------------------------

pub struct ZipfWorkload {
    catalog_size: usize,
    base_rate_req_sec: f64,
    sampler: AliasSampler,
    default_ttl_ms: u64,
    default_body_size: usize,
}

impl ZipfWorkload {
    pub fn new(catalog_size: usize, alpha: f64, base_rate_req_sec: f64) -> Self {
        let sampler = AliasSampler::new_zipf(catalog_size, alpha);
        Self {
            catalog_size,
            base_rate_req_sec,
            sampler,
            default_ttl_ms: 60_000,
            default_body_size: 1024,
        }
    }
}

impl WorkloadGenerator for ZipfWorkload {
    fn next_request(&mut self, _now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        let lambda = self.base_rate_req_sec / 1_000_000.0;
        let delta_us = (rng.next_exponential(lambda) as u64).max(1);
        let rank = self.sampler.sample(rng);

        RequestSpec {
            arrival_delta_us: delta_us,
            key: format!("/key_{}", rank),
            body_size_bytes: self.default_body_size,
            ttl_ms: self.default_ttl_ms,
            is_invalidation: false,
        }
    }
}

// ---------------------------------------------------------------------------
// W-SC: Scan / One-Hit-Wonder Workload
// ---------------------------------------------------------------------------

pub struct ScanWorkload {
    zipf: ZipfWorkload,
    scan_fraction: f64,
    next_scan_id: u64,
}

impl ScanWorkload {
    pub fn new(catalog_size: usize, alpha: f64, scan_fraction: f64, rate: f64) -> Self {
        Self {
            zipf: ZipfWorkload::new(catalog_size, alpha, rate),
            scan_fraction,
            next_scan_id: 1_000_000,
        }
    }
}

impl WorkloadGenerator for ScanWorkload {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        let mut req = self.zipf.next_request(now_us, rng);
        if rng.next_f64() < self.scan_fraction {
            // One-hit wonder scan: completely unique, never-repeated key
            req.key = format!("/scan_{}", self.next_scan_id);
            self.next_scan_id += 1;
        }
        req
    }
}

// ---------------------------------------------------------------------------
// W-SH: Hot-Set Shift Workload
// ---------------------------------------------------------------------------

pub struct HotSetShiftWorkload {
    catalog_size: usize,
    base_rate: f64,
    sampler: AliasSampler,
    shift_time_us: u64,
    shift_offset: usize,
}

impl HotSetShiftWorkload {
    pub fn new(catalog_size: usize, alpha: f64, shift_time_us: u64, shift_offset: usize, rate: f64) -> Self {
        let sampler = AliasSampler::new_zipf(catalog_size, alpha);
        Self {
            catalog_size,
            base_rate: rate,
            sampler,
            shift_time_us,
            shift_offset,
        }
    }
}

impl WorkloadGenerator for HotSetShiftWorkload {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        let lambda = self.base_rate / 1_000_000.0;
        let delta_us = (rng.next_exponential(lambda) as u64).max(1);
        let rank = self.sampler.sample(rng);

        let key_idx = if now_us >= self.shift_time_us {
            (rank + self.shift_offset) % self.catalog_size
        } else {
            rank
        };

        RequestSpec {
            arrival_delta_us: delta_us,
            key: format!("/key_{}", key_idx),
            body_size_bytes: 1024,
            ttl_ms: 60_000,
            is_invalidation: false,
        }
    }
}

// ---------------------------------------------------------------------------
// W-FC: Flash Crowd Workload
// ---------------------------------------------------------------------------

pub struct FlashCrowdWorkload {
    zipf: ZipfWorkload,
    crowd_start_us: u64,
    crowd_end_us: u64,
    rate_multiplier: f64,
    crowd_keys: Vec<String>,
}

impl FlashCrowdWorkload {
    pub fn new(
        catalog_size: usize,
        alpha: f64,
        rate: f64,
        start_us: u64,
        end_us: u64,
        multiplier: f64,
        num_keys: usize,
    ) -> Self {
        let crowd_keys: Vec<String> = (0..num_keys).map(|i| format!("/flash_crowd_{}", i)).collect();
        Self {
            zipf: ZipfWorkload::new(catalog_size, alpha, rate),
            crowd_start_us: start_us,
            crowd_end_us: end_us,
            rate_multiplier: multiplier,
            crowd_keys,
        }
    }
}

impl WorkloadGenerator for FlashCrowdWorkload {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        let is_crowd = now_us >= self.crowd_start_us && now_us <= self.crowd_end_us;
        let effective_rate = if is_crowd {
            self.zipf.base_rate_req_sec * self.rate_multiplier
        } else {
            self.zipf.base_rate_req_sec
        };

        let lambda = effective_rate / 1_000_000.0;
        let delta_us = (rng.next_exponential(lambda) as u64).max(1);

        let key = if is_crowd && rng.next_f64() < 0.75 {
            // 75% of flash crowd traffic hits the concentrated target keys
            let idx = rng.next_range_u64(0, (self.crowd_keys.len() - 1) as u64) as usize;
            self.crowd_keys[idx].clone()
        } else {
            let rank = self.zipf.sampler.sample(rng);
            format!("/key_{}", rank)
        };

        RequestSpec {
            arrival_delta_us: delta_us,
            key,
            body_size_bytes: 1024,
            ttl_ms: 60_000,
            is_invalidation: false,
        }
    }
}

// ---------------------------------------------------------------------------
// W-DI: Diurnal Workload
// ---------------------------------------------------------------------------

pub struct DiurnalWorkload {
    catalog_size: usize,
    sampler: AliasSampler,
    mean_rate: f64,
    amplitude: f64,
    period_us: u64,
}

impl DiurnalWorkload {
    pub fn new(catalog_size: usize, alpha: f64, mean_rate: f64, amplitude: f64, period_us: u64) -> Self {
        let sampler = AliasSampler::new_zipf(catalog_size, alpha);
        Self {
            catalog_size,
            sampler,
            mean_rate,
            amplitude,
            period_us,
        }
    }
}

impl WorkloadGenerator for DiurnalWorkload {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        // Sinusoidal rate: lambda(t) = mean * (1.0 + amplitude * sin(2*pi*t / period))
        let phase = (now_us % self.period_us) as f64 / self.period_us as f64;
        let factor = 1.0 + self.amplitude * (2.0 * std::f64::consts::PI * phase).sin();
        let current_rate = (self.mean_rate * factor).max(1.0);

        let lambda = current_rate / 1_000_000.0;
        let delta_us = (rng.next_exponential(lambda) as u64).max(1);
        let rank = self.sampler.sample(rng);

        RequestSpec {
            arrival_delta_us: delta_us,
            key: format!("/key_{}", rank),
            body_size_bytes: 1024,
            ttl_ms: 60_000,
            is_invalidation: false,
        }
    }
}

// ---------------------------------------------------------------------------
// W-INV: Invalidation Stream Workload
// ---------------------------------------------------------------------------

pub struct InvalidationWorkload {
    inner: ZipfWorkload,
    invalidation_probability: f64,
}

impl InvalidationWorkload {
    pub fn new(catalog_size: usize, alpha: f64, rate: f64, inval_prob: f64) -> Self {
        Self {
            inner: ZipfWorkload::new(catalog_size, alpha, rate),
            invalidation_probability: inval_prob,
        }
    }
}

impl WorkloadGenerator for InvalidationWorkload {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        let mut req = self.inner.next_request(now_us, rng);
        if rng.next_f64() < self.invalidation_probability {
            req.is_invalidation = true;
        }
        req
    }
}

// ---------------------------------------------------------------------------
// W-HK: Hot-Key Emergence Workload
// ---------------------------------------------------------------------------

pub struct HotKeyWorkload {
    inner: ZipfWorkload,
    emerging_key: String,
    start_time_us: u64,
    growth_rate_per_sec: f64,
    max_probability: f64,
}

impl HotKeyWorkload {
    pub fn new(
        catalog_size: usize,
        alpha: f64,
        rate: f64,
        emerging_key: String,
        start_time_us: u64,
        growth_rate_per_sec: f64,
        max_probability: f64,
    ) -> Self {
        Self {
            inner: ZipfWorkload::new(catalog_size, alpha, rate),
            emerging_key,
            start_time_us,
            growth_rate_per_sec,
            max_probability,
        }
    }
}

impl WorkloadGenerator for HotKeyWorkload {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        let mut req = self.inner.next_request(now_us, rng);
        if now_us >= self.start_time_us {
            let elapsed_sec = (now_us - self.start_time_us) as f64 / 1_000_000.0;
            // Probability increases exponentially towards max_probability: P(t) = P_max * (1 - e^(-lambda * t))
            let p = self.max_probability * (1.0 - (-self.growth_rate_per_sec * elapsed_sec).exp());
            if rng.next_f64() < p {
                req.key = self.emerging_key.clone();
            }
        }
        req
    }
}

// ---------------------------------------------------------------------------
// W-SZ: Object Size Distributions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ObjectSizeDistribution {
    Fixed(usize),
    Lognormal {
        mu: f64,
        sigma: f64,
        min_bytes: usize,
        max_bytes: usize,
    },
    Bimodal {
        small_bytes: usize,
        large_bytes: usize,
        p_small: f64,
    },
}

impl ObjectSizeDistribution {
    pub fn sample(&self, rng: &mut StreamRng) -> usize {
        match self {
            Self::Fixed(size) => *size,
            Self::Lognormal {
                mu,
                sigma,
                min_bytes,
                max_bytes,
            } => {
                // Box-Muller transform for standard normal sample
                let u1 = rng.next_f64().max(1e-15);
                let u2 = rng.next_f64();
                let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let log_val = mu + sigma * z;
                let bytes = log_val.exp() as usize;
                bytes.clamp(*min_bytes, *max_bytes)
            }
            Self::Bimodal {
                small_bytes,
                large_bytes,
                p_small,
            } => {
                if rng.next_f64() < *p_small {
                    *small_bytes
                } else {
                    *large_bytes
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W-TTL: Heterogeneous Freshness Lifetimes
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum TtlDistribution {
    Fixed(u64),
    Tiered(Vec<(u64, f64)>), // (ttl_ms, cumulative_weight)
}

impl TtlDistribution {
    pub fn new_tiered(tiers: &[(u64, f64)]) -> Self {
        let total: f64 = tiers.iter().map(|(_, w)| *w).sum();
        let mut cum = 0.0;
        let mut normalized = Vec::with_capacity(tiers.len());
        for (ttl, w) in tiers {
            cum += w / total;
            normalized.push((*ttl, cum));
        }
        Self::Tiered(normalized)
    }

    pub fn sample(&self, rng: &mut StreamRng) -> u64 {
        match self {
            Self::Fixed(ttl) => *ttl,
            Self::Tiered(tiers) => {
                let p = rng.next_f64();
                for (ttl, cum_w) in tiers {
                    if p <= *cum_w {
                        return *ttl;
                    }
                }
                tiers.last().map(|(ttl, _)| *ttl).unwrap_or(60_000)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W-MX: Composite Workload Mixtures
// ---------------------------------------------------------------------------

pub struct MixtureWorkload {
    components: Vec<(Box<dyn WorkloadGenerator + Send>, f64)>, // (generator, cumulative_weight)
}

impl MixtureWorkload {
    pub fn new(mut items: Vec<(Box<dyn WorkloadGenerator + Send>, f64)>) -> Self {
        let total: f64 = items.iter().map(|(_, w)| *w).sum();
        let mut cum = 0.0;
        for (_, w) in items.iter_mut() {
            cum += *w / total;
            *w = cum;
        }
        Self { components: items }
    }
}

impl WorkloadGenerator for MixtureWorkload {
    fn next_request(&mut self, now_us: u64, rng: &mut StreamRng) -> RequestSpec {
        let p = rng.next_f64();
        for (gen, cum_w) in self.components.iter_mut() {
            if p <= *cum_w {
                return gen.next_request(now_us, rng);
            }
        }
        self.components
            .last_mut()
            .map(|(gen, _)| gen.next_request(now_us, rng))
            .unwrap_or_else(|| RequestSpec {
                arrival_delta_us: 1000,
                key: "/fallback".to_string(),
                body_size_bytes: 1024,
                ttl_ms: 60_000,
                is_invalidation: false,
            })
    }
}

use crate::sim::rng::RngSource;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestType {
    CacheableGet { key: String },
    UncacheableGet { key: String, reason: String },
    DeleteInvalidation { key: String },
    Bypass { key: String },
    UpstreamError { key: String, status_code: u16 },
}

#[derive(Debug, Clone)]
pub struct WorkloadWeights {
    pub cacheable_get: f64,
    pub uncacheable_get: f64,
    pub delete_invalidation: f64,
    pub bypass: f64,
    pub upstream_error: f64,
}

impl Default for WorkloadWeights {
    fn default() -> Self {
        Self {
            cacheable_get: 0.70,
            uncacheable_get: 0.10,
            delete_invalidation: 0.10,
            bypass: 0.05,
            upstream_error: 0.05,
        }
    }
}

#[derive(Debug, Clone)]
pub enum KeyDistribution {
    Uniform {
        catalog_size: usize,
    },
    Zipfian {
        catalog_size: usize,
        skew: f64,
    },
    FlashCrowd {
        catalog_size: usize,
        hotspot_keys: Vec<String>,
        hotspot_probability: f64,
    },
}

pub struct TrafficGenerator {
    rng: Arc<dyn RngSource>,
}

impl TrafficGenerator {
    pub fn new(rng: Arc<dyn RngSource>) -> Self {
        Self { rng }
    }

    /// Selects a key based on the specified KeyDistribution.
    pub fn sample_key(&self, distribution: &KeyDistribution) -> String {
        match distribution {
            KeyDistribution::Uniform { catalog_size } => {
                let size = (*catalog_size).max(1);
                let idx = (self.rng.next_f64() * size as f64) as usize;
                format!("/resource/{}", idx)
            }
            KeyDistribution::Zipfian {
                catalog_size,
                skew,
            } => {
                let key_id = self.zipf_key(*catalog_size, *skew);
                format!("/resource/{}", key_id)
            }
            KeyDistribution::FlashCrowd {
                catalog_size,
                hotspot_keys,
                hotspot_probability,
            } => {
                if !hotspot_keys.is_empty() && self.rng.next_f64() < *hotspot_probability {
                    let idx = (self.rng.next_f64() * hotspot_keys.len() as f64) as usize;
                    hotspot_keys[idx.min(hotspot_keys.len() - 1)].clone()
                } else {
                    let size = (*catalog_size).max(1);
                    let idx = (self.rng.next_f64() * size as f64) as usize;
                    format!("/resource/{}", idx)
                }
            }
        }
    }

    /// Generates a single typed request using the given distribution and weights.
    pub fn generate_request(
        &self,
        distribution: &KeyDistribution,
        weights: &WorkloadWeights,
    ) -> RequestType {
        let key = self.sample_key(distribution);
        let total_weight = weights.cacheable_get
            + weights.uncacheable_get
            + weights.delete_invalidation
            + weights.bypass
            + weights.upstream_error;

        let roll = self.rng.next_f64() * total_weight;
        let mut cumulative = weights.cacheable_get;

        if roll < cumulative {
            return RequestType::CacheableGet { key };
        }
        cumulative += weights.uncacheable_get;
        if roll < cumulative {
            return RequestType::UncacheableGet {
                key,
                reason: "no-cache".to_string(),
            };
        }
        cumulative += weights.delete_invalidation;
        if roll < cumulative {
            return RequestType::DeleteInvalidation { key };
        }
        cumulative += weights.bypass;
        if roll < cumulative {
            return RequestType::Bypass { key };
        }

        let err_status = if self.rng.next_f64() < 0.5 { 500 } else { 504 };
        RequestType::UpstreamError {
            key,
            status_code: err_status,
        }
    }

    /// Generates a batch of requests deterministically.
    pub fn generate_batch(
        &self,
        count: usize,
        distribution: &KeyDistribution,
        weights: &WorkloadWeights,
    ) -> Vec<RequestType> {
        let mut batch = Vec::with_capacity(count);
        for _ in 0..count {
            batch.push(self.generate_request(distribution, weights));
        }
        batch
    }

    /// Generates the number of requests arriving in a given delta_ms window using Poisson distribution.
    pub fn poisson_requests(&self, rate_per_sec: f64, delta_ms: u64) -> u64 {
        let lambda = rate_per_sec * (delta_ms as f64 / 1000.0);
        if lambda <= 0.0 {
            return 0;
        }

        // Knuth's algorithm for Poisson random variables
        let l = (-lambda).exp();
        let mut k = 0;
        let mut p = 1.0;

        while p > l {
            k += 1;
            p *= self.rng.next_f64();
        }

        (k - 1) as u64
    }

    /// Returns a key hash according to a Zipfian distribution of a defined catalog size.
    pub fn zipf_key(&self, catalog_size: usize, skew: f64) -> u64 {
        if catalog_size <= 1 {
            return 0;
        }

        // Approximate Zipfian sampler
        let rand_val = self.rng.next_f64();
        let mut sum = 0.0;
        let mut c = 0.0;

        // Precompute normalizing constant denominator approximation
        for i in 1..=catalog_size {
            c += 1.0 / (i as f64).powf(skew);
        }

        for i in 1..=catalog_size {
            sum += (1.0 / (i as f64).powf(skew)) / c;
            if rand_val <= sum {
                return i as u64;
            }
        }

        catalog_size as u64
    }
}

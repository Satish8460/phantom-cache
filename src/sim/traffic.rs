use crate::sim::rng::RngSource;
use std::sync::Arc;

pub struct TrafficGenerator {
    rng: Arc<dyn RngSource>,
}

impl TrafficGenerator {
    pub fn new(rng: Arc<dyn RngSource>) -> Self {
        Self { rng }
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

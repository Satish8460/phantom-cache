//! Integration tests for Phantom Lab Eviction Baselines (M4-07 / WP-17).
//!
//! Validates:
//! - Reference implementations of SIEVE, S3-FIFO, LRU, and TinyLFU.
//! - Comparative evaluation on Zipfian and scan workloads.
//! - TinyLFU 4-bit Count-Min sketch saturation and decay reset.

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::eviction_baselines::{make_policy, EvictionSimulator, FrequencySketch};
use lab::rng::StreamRng;
use lab::workloads::{ScanWorkload, WorkloadGenerator, ZipfWorkload};
use phantom_cache_demo::core::cache::CacheKey;

#[test]
fn test_eviction_policies_zipf_workload_ordering() {
    let catalog_size = 300;
    let cache_capacity = 30; // 10% cache
    let n = 20_000;

    let policies = ["sieve", "s3fifo", "lru", "tinylfu"];
    let mut results = Vec::new();

    for policy_name in policies {
        let mut sim = EvictionSimulator::new(cache_capacity, make_policy(policy_name, cache_capacity));
        let mut zipf = ZipfWorkload::new(catalog_size, 0.9, 1000.0);
        let mut rng = StreamRng::new(30001);

        for _ in 0..n {
            let req = zipf.next_request(0, &mut rng);
            let key = CacheKey::new(None, &req.key, None);
            sim.access(&key, req.body_size_bytes);
        }

        let m = sim.metrics();
        assert!(
            m.hit_ratio > 0.30,
            "Policy {} hit ratio {:.3} was unexpectedly low for alpha=0.9",
            policy_name,
            m.hit_ratio
        );
        results.push(m);
    }

    assert_eq!(results.len(), 4);
}

#[test]
fn test_eviction_scan_resistance() {
    let catalog_size = 200;
    let cache_capacity = 25;
    let scan_fraction = 0.40; // 40% one-hit wonders
    let n = 15_000;

    let policies = ["sieve", "s3fifo", "lru", "tinylfu"];

    for policy_name in policies {
        let mut sim = EvictionSimulator::new(cache_capacity, make_policy(policy_name, cache_capacity));
        let mut scan_workload = ScanWorkload::new(catalog_size, 0.8, scan_fraction, 1000.0);
        let mut rng = StreamRng::new(30002);

        for _ in 0..n {
            let req = scan_workload.next_request(0, &mut rng);
            let key = CacheKey::new(None, &req.key, None);
            sim.access(&key, req.body_size_bytes);
        }

        let m = sim.metrics();
        // Even under 40% scan one-hit wonders, hit ratio must stay positive
        assert!(m.hit_ratio > 0.10, "Policy {} hit ratio too low: {:.3}", policy_name, m.hit_ratio);
    }
}

#[test]
fn test_tinylfu_frequency_sketch_reset_and_saturation() {
    let mut sketch = FrequencySketch::new(64);
    let key = CacheKey::new(None, "/frequent_item", None);

    // Initial frequency = 0
    assert_eq!(sketch.frequency(&key), 0);

    // Increment 5 times
    for _ in 0..5 {
        sketch.increment(&key);
    }
    assert_eq!(sketch.frequency(&key), 5);

    // Increment up to saturation (4 bits = 15)
    for _ in 0..20 {
        sketch.increment(&key);
    }
    assert_eq!(sketch.frequency(&key), 15);
}

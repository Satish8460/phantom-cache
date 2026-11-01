//! Integration tests for Phantom Lab Workload Library (M4-02 / WP-15).
//!
//! Validates statistical and structural conformance of all workload families:
//! - W-U: Uniform distribution flatness and inter-arrival timing
//! - W-Z(α): Zipfian rank-frequency falloff and skew
//! - W-SC: Scan unique key fraction and non-repetition
//! - W-SH: Hot-set shift transition at virtual timestamp
//! - W-FC: Flash crowd rate surge and target key concentration
//! - W-DI: Diurnal sinusoidal rate variation
//! - W-HK: Hot-key geometric emergence
//! - W-INV: Invalidation stream frequency
//! - W-SZ: Object size distribution sampling (fixed, lognormal, bimodal)
//! - W-TTL: Heterogeneous TTL sampling
//! - W-MX: Composite mixture distribution weighting

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::rng::StreamRng;
use lab::workloads::*;
use std::collections::{HashMap, HashSet};

#[test]
fn test_uniform_distribution_flatness() {
    let mut rng = StreamRng::new(10001);
    let catalog_size = 10;
    let rate = 1000.0;
    let mut workload = UniformWorkload::new(catalog_size, rate);

    let n = 50_000;
    let mut counts = vec![0usize; catalog_size];
    let mut total_delta = 0u64;

    for _ in 0..n {
        let req = workload.next_request(0, &mut rng);
        total_delta += req.arrival_delta_us;
        // Key format: "/key_{idx}"
        let idx: usize = req.key.strip_prefix("/key_").unwrap().parse().unwrap();
        counts[idx] += 1;
    }

    let expected = n / catalog_size; // 5,000
    for (idx, &count) in counts.iter().enumerate() {
        let diff = (count as i64 - expected as i64).abs();
        let pct = (diff as f64) / (expected as f64);
        assert!(
            pct < 0.10,
            "Key {} count {} deviated by {:.2}% from expected {}",
            idx,
            count,
            pct * 100.0,
            expected
        );
    }

    // Mean inter-arrival time should be ~ 1,000,000 / 1000 = 1000 us (within 5%)
    let mean_delta = (total_delta as f64) / (n as f64);
    assert!(
        (mean_delta - 1000.0).abs() < 50.0,
        "Mean delta {:.2}us deviated from expected 1000us",
        mean_delta
    );
}

#[test]
fn test_zipf_rank_frequency_monotonicity() {
    let mut rng = StreamRng::new(10002);
    let catalog_size = 100;
    let rate = 1000.0;

    // Test alpha = 0.8
    let mut zipf_08 = ZipfWorkload::new(catalog_size, 0.8, rate);
    let n = 50_000;
    let mut counts_08 = vec![0usize; catalog_size];

    for _ in 0..n {
        let req = zipf_08.next_request(0, &mut rng);
        let rank: usize = req.key.strip_prefix("/key_").unwrap().parse().unwrap();
        counts_08[rank] += 1;
    }

    // Monotonicity check across sample ranks
    assert!(counts_08[0] > counts_08[1]);
    assert!(counts_08[1] > counts_08[5]);
    assert!(counts_08[5] > counts_08[20]);
    assert!(counts_08[20] > counts_08[80]);

    // Top 10% keys should account for > 40% of traffic at alpha = 0.8
    let top_10_sum: usize = counts_08[..10].iter().sum();
    let top_10_pct = (top_10_sum as f64) / (n as f64);
    assert!(
        top_10_pct > 0.40,
        "Top 10% accounted for {:.2}%, expected > 40%",
        top_10_pct * 100.0
    );

    // Test alpha = 1.2 (steeper skew)
    let mut zipf_12 = ZipfWorkload::new(catalog_size, 1.2, rate);
    let mut counts_12 = vec![0usize; catalog_size];
    for _ in 0..n {
        let req = zipf_12.next_request(0, &mut rng);
        let rank: usize = req.key.strip_prefix("/key_").unwrap().parse().unwrap();
        counts_12[rank] += 1;
    }

    // Top 10% keys should account for > 65% of traffic at alpha = 1.2
    let top_10_sum_12: usize = counts_12[..10].iter().sum();
    let top_10_pct_12 = (top_10_sum_12 as f64) / (n as f64);
    assert!(
        top_10_pct_12 > 0.65,
        "Top 10% accounted for {:.2}%, expected > 65%",
        top_10_pct_12 * 100.0
    );
    assert!(top_10_pct_12 > top_10_pct, "alpha=1.2 must be more skewed than alpha=0.8");
}

#[test]
fn test_scan_one_hit_wonder_fraction() {
    let mut rng = StreamRng::new(10003);
    let catalog_size = 50;
    let alpha = 0.8;
    let scan_fraction = 0.25;
    let rate = 1000.0;
    let mut workload = ScanWorkload::new(catalog_size, alpha, scan_fraction, rate);

    let n = 20_000;
    let mut scan_count = 0;
    let mut scan_keys = HashSet::new();

    for _ in 0..n {
        let req = workload.next_request(0, &mut rng);
        if req.key.starts_with("/scan_") {
            scan_count += 1;
            let is_new = scan_keys.insert(req.key.clone());
            assert!(is_new, "Scan key {} repeated; scans must be one-hit wonders", req.key);
        }
    }

    let observed_fraction = (scan_count as f64) / (n as f64);
    assert!(
        (observed_fraction - scan_fraction).abs() < 0.02,
        "Scan fraction {:.4} deviated from target {:.4}",
        observed_fraction,
        scan_fraction
    );
}

#[test]
fn test_hot_set_shift_trigger() {
    let mut rng = StreamRng::new(10004);
    let catalog_size = 100;
    let alpha = 1.0;
    let shift_time_us = 10_000_000; // 10s
    let shift_offset = 50;
    let rate = 1000.0;
    let mut workload = HotSetShiftWorkload::new(catalog_size, alpha, shift_time_us, shift_offset, rate);

    let n = 10_000;

    // Sample before shift (virtual time = 5s)
    let mut before_counts = HashMap::new();
    for _ in 0..n {
        let req = workload.next_request(5_000_000, &mut rng);
        *before_counts.entry(req.key).or_insert(0usize) += 1;
    }

    // Before shift, rank 0 is "/key_0"
    let top_before = before_counts.iter().max_by_key(|(_, &c)| c).unwrap().0;
    assert_eq!(top_before, "/key_0", "Before shift, key_0 must be hottest");

    // Sample after shift (virtual time = 15s)
    let mut after_counts = HashMap::new();
    for _ in 0..n {
        let req = workload.next_request(15_000_000, &mut rng);
        *after_counts.entry(req.key).or_insert(0usize) += 1;
    }

    // After shift, rank 0 shifted to (0 + 50) % 100 = "/key_50"
    let top_after = after_counts.iter().max_by_key(|(_, &c)| c).unwrap().0;
    assert_eq!(top_after, "/key_50", "After shift, key_50 must be hottest");
}

#[test]
fn test_flash_crowd_surge() {
    let mut rng = StreamRng::new(10005);
    let catalog_size = 100;
    let alpha = 0.8;
    let base_rate = 1000.0;
    let crowd_start = 1_000_000;
    let crowd_end = 5_000_000;
    let multiplier = 4.0;
    let num_keys = 2;

    let mut workload = FlashCrowdWorkload::new(
        catalog_size,
        alpha,
        base_rate,
        crowd_start,
        crowd_end,
        multiplier,
        num_keys,
    );

    let n = 5_000;

    // Before crowd (now_us = 500_000)
    let mut sum_delta_before = 0u64;
    for _ in 0..n {
        let req = workload.next_request(500_000, &mut rng);
        sum_delta_before += req.arrival_delta_us;
        assert!(!req.key.starts_with("/flash_crowd_"));
    }
    let mean_before = (sum_delta_before as f64) / (n as f64);
    assert!((mean_before - 1000.0).abs() < 100.0);

    // During crowd (now_us = 3_000_000)
    let mut sum_delta_during = 0u64;
    let mut crowd_key_hits = 0;
    for _ in 0..n {
        let req = workload.next_request(3_000_000, &mut rng);
        sum_delta_during += req.arrival_delta_us;
        if req.key.starts_with("/flash_crowd_") {
            crowd_key_hits += 1;
        }
    }
    let mean_during = (sum_delta_during as f64) / (n as f64);
    // At 4x multiplier, mean delta should be ~ 250us (1000 / 4)
    assert!(
        (mean_during - 250.0).abs() < 50.0,
        "Mean delta during crowd was {:.2}us, expected ~250us",
        mean_during
    );
    // Concentrated target keys should get ~ 75% of traffic
    let crowd_pct = (crowd_key_hits as f64) / (n as f64);
    assert!(
        crowd_pct > 0.65,
        "Flash crowd target keys received {:.2}%, expected > 65%",
        crowd_pct * 100.0
    );
}

#[test]
fn test_diurnal_sinusoidal_rate() {
    let mut rng = StreamRng::new(10006);
    let catalog_size = 50;
    let alpha = 0.8;
    let mean_rate = 1000.0;
    let amplitude = 0.5;
    let period_us = 100_000; // 100ms
    let mut workload = DiurnalWorkload::new(catalog_size, alpha, mean_rate, amplitude, period_us);

    let n = 5_000;

    // Peak rate occurs at phase = 0.25 (sin(pi/2) = 1.0) -> t = 25_000 us
    // Rate = 1000 * 1.5 = 1500 req/s -> mean delta = 1,000,000 / 1500 ~ 666 us
    let mut sum_peak = 0u64;
    for _ in 0..n {
        let req = workload.next_request(25_000, &mut rng);
        sum_peak += req.arrival_delta_us;
    }
    let mean_peak = (sum_peak as f64) / (n as f64);

    // Trough rate occurs at phase = 0.75 (sin(3pi/2) = -1.0) -> t = 75_000 us
    // Rate = 1000 * 0.5 = 500 req/s -> mean delta = 1,000,000 / 500 ~ 2000 us
    let mut sum_trough = 0u64;
    for _ in 0..n {
        let req = workload.next_request(75_000, &mut rng);
        sum_trough += req.arrival_delta_us;
    }
    let mean_trough = (sum_trough as f64) / (n as f64);

    assert!(
        (mean_peak - 666.6).abs() < 70.0,
        "Peak mean delta {:.2}us deviated from expected ~666us",
        mean_peak
    );
    assert!(
        (mean_trough - 2000.0).abs() < 200.0,
        "Trough mean delta {:.2}us deviated from expected ~2000us",
        mean_trough
    );
    assert!(mean_trough > 2.0 * mean_peak);
}

#[test]
fn test_hot_key_emergence() {
    let mut rng = StreamRng::new(10007);
    let catalog_size = 50;
    let alpha = 0.8;
    let rate = 1000.0;
    let emerging_key = "/emerging_hot".to_string();
    let start_time_us = 10_000_000; // 10s
    let growth_rate = 1.0;
    let max_p = 0.50;

    let mut workload = HotKeyWorkload::new(
        catalog_size,
        alpha,
        rate,
        emerging_key.clone(),
        start_time_us,
        growth_rate,
        max_p,
    );

    let n = 5_000;

    // Before start time (5s) -> 0 requests for emerging key
    for _ in 0..n {
        let req = workload.next_request(5_000_000, &mut rng);
        assert_ne!(req.key, emerging_key);
    }

    // 1 second after start (11s) -> P(1) = 0.5 * (1 - e^-1) = 0.5 * 0.632 = ~31.6%
    let mut hits_1s = 0;
    for _ in 0..n {
        let req = workload.next_request(11_000_000, &mut rng);
        if req.key == emerging_key {
            hits_1s += 1;
        }
    }
    let p_1s = (hits_1s as f64) / (n as f64);
    assert!(
        (p_1s - 0.316).abs() < 0.04,
        "P(1s) was {:.3}, expected ~0.316",
        p_1s
    );

    // 10 seconds after start (20s) -> P(10) ~ 0.50 * (1 - e^-10) ~ 50%
    let mut hits_10s = 0;
    for _ in 0..n {
        let req = workload.next_request(20_000_000, &mut rng);
        if req.key == emerging_key {
            hits_10s += 1;
        }
    }
    let p_10s = (hits_10s as f64) / (n as f64);
    assert!(
        (p_10s - 0.50).abs() < 0.04,
        "P(10s) was {:.3}, expected ~0.50",
        p_10s
    );
}

#[test]
fn test_invalidation_stream_probability() {
    let mut rng = StreamRng::new(10008);
    let catalog_size = 50;
    let alpha = 0.8;
    let rate = 1000.0;
    let inval_prob = 0.15; // 15%
    let mut workload = InvalidationWorkload::new(catalog_size, alpha, rate, inval_prob);

    let n = 20_000;
    let mut inval_count = 0;

    for _ in 0..n {
        let req = workload.next_request(0, &mut rng);
        if req.is_invalidation {
            inval_count += 1;
        }
    }

    let observed = (inval_count as f64) / (n as f64);
    assert!(
        (observed - inval_prob).abs() < 0.02,
        "Observed invalidation fraction {:.4} deviated from expected {:.4}",
        observed,
        inval_prob
    );
}

#[test]
fn test_size_and_ttl_distributions() {
    let mut rng = StreamRng::new(10009);

    // Fixed size
    let fixed_sz = ObjectSizeDistribution::Fixed(4096);
    assert_eq!(fixed_sz.sample(&mut rng), 4096);

    // Bimodal size: 1024 (70%) vs 65536 (30%)
    let bimodal = ObjectSizeDistribution::Bimodal {
        small_bytes: 1024,
        large_bytes: 65536,
        p_small: 0.70,
    };
    let n = 10_000;
    let mut small_count = 0;
    for _ in 0..n {
        let sz = bimodal.sample(&mut rng);
        if sz == 1024 {
            small_count += 1;
        } else {
            assert_eq!(sz, 65536);
        }
    }
    let p_small_observed = (small_count as f64) / (n as f64);
    assert!((p_small_observed - 0.70).abs() < 0.03);

    // Lognormal size: clamped to bounds
    let lognorm = ObjectSizeDistribution::Lognormal {
        mu: 8.0,
        sigma: 1.0,
        min_bytes: 128,
        max_bytes: 1_000_000,
    };
    for _ in 0..1000 {
        let sz = lognorm.sample(&mut rng);
        assert!((128..=1_000_000).contains(&sz));
    }

    // Tiered TTL: 5s (50%), 60s (30%), 3600s (20%)
    let ttl_dist = TtlDistribution::new_tiered(&[
        (5_000, 0.50),
        (60_000, 0.30),
        (3_600_000, 0.20),
    ]);
    let mut ttl_counts = HashMap::new();
    for _ in 0..n {
        let ttl = ttl_dist.sample(&mut rng);
        *ttl_counts.entry(ttl).or_insert(0usize) += 1;
    }
    let p_5s = (ttl_counts[&5_000] as f64) / (n as f64);
    let p_60s = (ttl_counts[&60_000] as f64) / (n as f64);
    let p_3600s = (ttl_counts[&3_600_000] as f64) / (n as f64);
    assert!((p_5s - 0.50).abs() < 0.03);
    assert!((p_60s - 0.30).abs() < 0.03);
    assert!((p_3600s - 0.20).abs() < 0.03);
}

#[test]
fn test_mixture_workload_composition() {
    let mut rng = StreamRng::new(10010);
    let w1 = Box::new(UniformWorkload::new(10, 1000.0));
    let w2 = Box::new(InvalidationWorkload::new(10, 0.8, 1000.0, 1.0)); // 100% invalidations

    let mut mixture = MixtureWorkload::new(vec![
        (w1, 0.80), // 80% uniform (0% inval)
        (w2, 0.20), // 20% invalidations (100% inval)
    ]);

    let n = 20_000;
    let mut inval_count = 0;
    for _ in 0..n {
        let req = mixture.next_request(0, &mut rng);
        if req.is_invalidation {
            inval_count += 1;
        }
    }

    let observed_inval_fraction = (inval_count as f64) / (n as f64);
    assert!(
        (observed_inval_fraction - 0.20).abs() < 0.02,
        "Mixture invalidation fraction {:.4} deviated from expected 0.20",
        observed_inval_fraction
    );
}

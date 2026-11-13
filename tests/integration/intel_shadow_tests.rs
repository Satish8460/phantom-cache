use phantom_cache_demo::intelligence::shadow::*;

#[test]
fn test_shadow_policies_basic_conformance() {
    let mut lru = LruShadow::new(10);
    let mut sieve = SieveShadow::new(10);
    let mut s3fifo = S3FifoShadow::new(30); // small_cap = 3
    let mut tinylfu = TinyLfuShadow::new(10);

    // Warm up with keys 1, 2, 3
    for k in 1..=3 {
        assert!(!lru.access(k));
        assert!(!sieve.access(k));
        assert!(!s3fifo.access(k));
        assert!(!tinylfu.access(k));
    }

    // Hits on keys 1, 2
    for k in 1..=2 {
        assert!(lru.access(k));
        assert!(sieve.access(k));
        assert!(s3fifo.access(k));
        assert!(tinylfu.access(k));
    }

    assert_eq!(lru.hits(), 2);
    assert_eq!(lru.misses(), 3);
    assert_eq!(sieve.hits(), 2);
    assert_eq!(s3fifo.hits(), 2);
    assert_eq!(tinylfu.hits(), 2);
}

#[test]
fn test_spatial_sampling_filtering() {
    let baseline = Box::new(LruShadow::new(16));
    let candidate = Box::new(SieveShadow::new(16));
    let mut evaluator = ShadowEvaluator::new(16, baseline, candidate);

    // Feed 1600 requests with sequential keys
    for k in 0..1600u64 {
        evaluator.observe(k);
    }

    // Exactly 1600 / 16 = 100 requests should be sampled
    let report = evaluator.generate_report(0.05);
    assert_eq!(report.total_sampled_requests, 100);
}

#[test]
fn test_shadow_scan_resistance_advantage() {
    let baseline = Box::new(LruShadow::new(32));
    let candidate = Box::new(S3FifoShadow::new(32));
    let mut evaluator = ShadowEvaluator::new(1, baseline, candidate);

    // 1. Repeat hot keys 1..10 to build cache residency
    for _ in 0..20 {
        for k in 1..=10u64 {
            evaluator.observe(k);
        }
    }

    // 2. Inject one-hit wonder scan: 1000..1200
    for k in 1000..1200u64 {
        evaluator.observe(k);
    }

    // 3. Re-access hot keys 1..10
    for _ in 0..10 {
        for k in 1..=10u64 {
            evaluator.observe(k);
        }
    }

    let report = evaluator.generate_report(0.0);
    // S3-FIFO filters one-hit wonders and should preserve hot keys better than LRU
    assert!(
        report.candidate_hit_ratio >= report.baseline_hit_ratio,
        "Candidate hit ratio {} was lower than baseline LRU {}",
        report.candidate_hit_ratio,
        report.baseline_hit_ratio
    );
}

#[test]
fn test_shadow_evaluator_report_and_ci() {
    let baseline = Box::new(LruShadow::new(16));
    let candidate = Box::new(SieveShadow::new(16));
    let mut evaluator = ShadowEvaluator::new(1, baseline, candidate);

    for k in 0..500u64 {
        evaluator.observe(k % 10);
    }

    let report = evaluator.generate_report(0.05);
    assert_eq!(report.total_sampled_requests, 500);
    assert!(report.baseline_hit_ratio > 0.0);
    assert!(report.candidate_hit_ratio > 0.0);
    assert!(report.confidence_interval_95.0 <= report.delta_hit_ratio);
    assert!(report.confidence_interval_95.1 >= report.delta_hit_ratio);
}

#[test]
fn test_shadow_evaluator_determinism() {
    let mut ev1 = ShadowEvaluator::new(
        1,
        Box::new(LruShadow::new(16)),
        Box::new(SieveShadow::new(16)),
    );
    let mut ev2 = ShadowEvaluator::new(
        1,
        Box::new(LruShadow::new(16)),
        Box::new(SieveShadow::new(16)),
    );

    let keys: Vec<u64> = (0..1000).map(|i| (i * 37 + 13) % 100).collect();

    for &k in &keys {
        ev1.observe(k);
        ev2.observe(k);
    }

    let rep1 = ev1.generate_report(0.05);
    let rep2 = ev2.generate_report(0.05);

    assert_eq!(rep1.baseline_hit_ratio, rep2.baseline_hit_ratio);
    assert_eq!(rep1.candidate_hit_ratio, rep2.candidate_hit_ratio);
    assert_eq!(rep1.delta_hit_ratio, rep2.delta_hit_ratio);
}

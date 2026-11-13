use phantom_cache_demo::intelligence::signals::*;
use phantom_cache_demo::intelligence::*;
use std::collections::HashMap;

#[test]
fn test_rate_detector_accuracy() {
    let mut rate_det = RateDetector::new(0.5);

    // Initial tick to set start time
    let _ = rate_det.tick(1000);

    // 500 requests over 1000 ms = 500 rps
    for _ in 0..500 {
        rate_det.record_request();
    }
    let val = rate_det.tick(2000);
    assert!(val.is_valid());
    assert!(
        (val.value - 500.0).abs() <= 25.0,
        "Rate {} deviated from expected 500.0",
        val.value
    );
}

#[test]
fn test_heavy_hitters_precision_recall_zipf() {
    let mut hh = HeavyHitterDetector::new();
    let mut exact_counts: HashMap<u64, u32> = HashMap::new();

    // Generate synthetic Zipf-like distribution: key i chosen with weight ~ 1/i
    // Keep total samples under 10,000 decay interval to test exact Count-Min frequency bounds
    for i in 1..=30u64 {
        let count = (1000.0 / (i as f64)).round() as u32;
        for _ in 0..count {
            hh.observe(i);
            *exact_counts.entry(i).or_insert(0) += 1;
        }
    }

    let top_k = hh.get_top_k();
    assert!(top_k.len() >= 10);

    // Verify top-5 keys are present in top_k
    let top_k_keys: Vec<u64> = top_k.iter().take(10).map(|(k, _)| *k).collect();
    for expected_top_key in 1..=5u64 {
        assert!(
            top_k_keys.contains(&expected_top_key),
            "Top key {} missing from Space-Saving top-k",
            expected_top_key
        );
    }

    // Verify Count-Min frequency estimates are within bounds (CM property: est >= exact)
    for (key, _count) in top_k.iter().take(5) {
        let exact = exact_counts.get(key).copied().unwrap_or(0);
        let est = hh.estimate_frequency(*key);
        assert!(
            est >= exact,
            "Count-Min sketch underestimated: est {}, exact {}",
            est,
            exact
        );
    }
}

#[test]
fn test_zipf_skew_mle_regression() {
    // Construct synthetic top-k keys matching Zipf alpha = 0.8: f_r = C / r^0.8
    let mut top_k = Vec::new();
    let c = 10_000.0;
    for r in 1..=40 {
        let count = (c / (r as f64).powf(0.8)).round() as u32;
        top_k.push((r as u64, count));
    }

    let skew_feat = ZipfSkewDetector::estimate(&top_k, 5000);
    assert!(skew_feat.is_valid());
    let alpha = skew_feat.value.expect("Alpha estimate must be Some");

    assert!(
        (alpha - 0.8).abs() <= 0.15,
        "Estimated alpha {} deviated from 0.8 by more than 0.15",
        alpha
    );
}

#[test]
fn test_shards_working_set_accuracy() {
    let mut shards = WorkingSetDetector::new(16);

    // Insert 3200 distinct keys
    for k in 0..3200u64 {
        shards.observe(k, 1000);
    }

    let ws = shards.estimate_working_set(5000, 1000);
    assert!(ws.is_valid());

    // Estimated keys = active_sampled * 16. Target = 3200.
    // 3200 keys * 1024 bytes = 3,276,800 bytes (~3.2 MB)
    let estimated_keys = ws.value / 1024;
    let error = ((estimated_keys as f64 - 3200.0) / 3200.0).abs();
    assert!(
        error <= 0.20,
        "SHARDS working set estimate {} deviated from 3200 by {:.2}%",
        estimated_keys,
        error * 100.0
    );
}

#[test]
fn test_cusum_burst_detector() {
    let mut burst = BurstDetector::new(200.0, 20.0);

    // Steady state at 100 rps
    for _ in 0..10 {
        let feat = burst.update(100.0);
        assert!(!feat.value);
    }

    // Sudden rate spike to 1000 rps
    let feat = burst.update(1000.0);
    assert!(feat.value, "CUSUM failed to detect sudden rate spike");
}

#[test]
fn test_hot_key_emergence_detection() {
    let mut hot_det = HotKeyEmergenceDetector::default();

    // Baseline window
    let window_1 = vec![(101, 10), (102, 10), (103, 10)];
    let res1 = hot_det.evaluate(&window_1);
    assert!(!res1.value);

    // Next window: key 101 surges from 10 to 60 (6x increase)
    let window_2 = vec![(101, 60), (102, 12), (103, 11)];
    let res2 = hot_det.evaluate(&window_2);
    assert!(
        res2.value,
        "Failed to detect emergent hot key with 6x velocity"
    );
}

#[test]
fn test_workload_pipeline_provenance_contract() {
    let mut pipeline = WorkloadPipeline::new();

    // Record sample requests
    for i in 0..100 {
        pipeline.observe_request(i % 10, i % 2 == 0, 500, 1000 + i);
    }
    pipeline.observe_origin_fetch(false);
    pipeline.observe_origin_fetch(true);
    pipeline.observe_invalidation();
    pipeline.observe_stale_serve();

    let window = pipeline.produce_window(2000, 1000);

    assert_eq!(window.request_count, 100);
    assert_eq!(window.f2_hit_ratio.provenance, MetricProvenance::Derived);
    assert_eq!(window.f2_hit_ratio.value, 0.5);
    assert_eq!(
        window.f10_origin_error_rate.provenance,
        MetricProvenance::Measured
    );
    assert_eq!(window.f10_origin_error_rate.value, 0.5);
    assert_eq!(window.f12_stale_served_ratio.value, 0.01);
}

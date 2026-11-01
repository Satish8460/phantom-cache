use phantom_cache_demo::intel::telemetry::{
    AtomicLogLinearHistogram, RawObservationUs, TelemetryCollector, HISTOGRAM_TOTAL_BUCKETS,
};
use std::sync::Arc;
use std::thread;

#[test]
fn test_histogram_relative_error_bounded() {
    // 1. Verify exact buckets [0, 127] have exactly 0 error
    for v in 0..128 {
        let b = AtomicLogLinearHistogram::value_to_bucket(v);
        assert_eq!(b, v as usize);
        let reconstructed = AtomicLogLinearHistogram::bucket_to_value(b);
        assert_eq!(reconstructed, v);
    }

    // 2. Verify relative error across [128 us, 10,000,000 us (10s)]
    let mut val = 128u64;
    let mut max_rel_error = 0.0f64;

    while val <= 10_000_000 {
        let b = AtomicLogLinearHistogram::value_to_bucket(val);
        assert!(b < HISTOGRAM_TOTAL_BUCKETS, "Bucket {} out of bounds for val {}", b, val);
        let reconstructed = AtomicLogLinearHistogram::bucket_to_value(b);
        let diff = (reconstructed as f64 - val as f64).abs();
        let rel_error = diff / (val as f64);
        if rel_error > max_rel_error {
            max_rel_error = rel_error;
        }

        // Must strictly satisfy <= 1.0% (0.010); design targets <= 0.5% (0.005)
        assert!(
            rel_error <= 0.005,
            "Relative error {:.4}% exceeded 0.5% for val={}, reconstructed={}, bucket={}",
            rel_error * 100.0,
            val,
            reconstructed,
            b
        );

        // Step linearly when small, geometrically when large
        if val < 2000 {
            val += 1;
        } else if val < 100_000 {
            val += 19;
        } else {
            val += 531;
        }
    }

    println!("Max measured relative error in log-linear histogram: {:.4}%", max_rel_error * 100.0);
    assert!(max_rel_error <= 0.004); // Empirically bounded <= 0.3906%
}

#[test]
fn test_histogram_concurrency_count_conservation() {
    let hist = Arc::new(AtomicLogLinearHistogram::new());
    let num_threads = 8;
    let records_per_thread = 100_000;
    let mut handles = Vec::new();

    for t in 0..num_threads {
        let h = Arc::clone(&hist);
        handles.push(thread::spawn(move || {
            for i in 0..records_per_thread {
                // Record values spanning different octaves
                let latency_us = ((t as u64 + 1) * (i as u64 + 1) * 31) % 5_000_000 + 1;
                h.record(latency_us);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let expected_total = (num_threads * records_per_thread) as u64;
    assert_eq!(hist.count(), expected_total);

    let (p50, p90, p95, p99) = hist.calculate_percentiles_us();
    assert!(p50 > 0);
    assert!(p90 >= p50);
    assert!(p95 >= p90);
    assert!(p99 >= p95);
}

#[test]
fn test_histogram_percentile_accuracy_vs_exact() {
    let hist = AtomicLogLinearHistogram::new();
    let mut exact_latencies = Vec::with_capacity(20_000);

    // Generate smooth continuous distribution spanning 20 us to 200,000 us (200 ms)
    for i in 1..=20_000 {
        let x = (i as f64) / 20_000.0;
        // Non-linear power distribution modeling realistic cache latency curve
        let latency_us = (20.0 + 50_000.0 * x.powf(2.5)) as u64;

        exact_latencies.push(latency_us);
        hist.record(latency_us);
    }

    exact_latencies.sort_unstable();
    let n = exact_latencies.len();
    let exact_p50 = exact_latencies[(n as f64 * 0.50) as usize];
    let exact_p90 = exact_latencies[(n as f64 * 0.90) as usize];
    let exact_p95 = exact_latencies[(n as f64 * 0.95) as usize];
    let exact_p99 = exact_latencies[(n as f64 * 0.99) as usize];

    let (hist_p50, hist_p90, hist_p95, hist_p99) = hist.calculate_percentiles_us();

    // Verify each percentile is within 1.0% error of exact value
    let check_pct = |name: &str, exact: u64, approx: u64| {
        let diff = (approx as f64 - exact as f64).abs();
        let rel_err = diff / (exact as f64);
        println!(
            "{}: exact={} us, approx={} us, rel_err={:.4}%",
            name, exact, approx, rel_err * 100.0
        );
        assert!(
            rel_err <= 0.010,
            "{} error {:.4}% exceeded 1.0%: exact={}, approx={}",
            name,
            rel_err * 100.0,
            exact,
            approx
        );
    };

    check_pct("p50", exact_p50, hist_p50);
    check_pct("p90", exact_p90, hist_p90);
    check_pct("p95", exact_p95, hist_p95);
    check_pct("p99", exact_p99, hist_p99);
}

#[test]
fn test_histogram_clear_reset() {
    let hist = AtomicLogLinearHistogram::new();
    hist.record(1234);
    hist.record(5678);
    assert_eq!(hist.count(), 2);

    hist.clear();
    assert_eq!(hist.count(), 0);
    assert_eq!(hist.calculate_percentiles_us(), (0, 0, 0, 0));
}

#[test]
fn test_telemetry_collector_histogram_integration() {
    let collector = TelemetryCollector::new(100);

    // Record GET observations
    collector.record_us(RawObservationUs::new_get(1000, 123, 250, true, 0));
    collector.record_us(RawObservationUs::new_get(1001, 124, 750, false, 0));
    collector.record_us(RawObservationUs::new_get(1002, 125, 1500, true, 0));

    // Record DELETE observation (must NOT be counted in GET latency histogram)
    collector.record_us(RawObservationUs::new_delete(1003, 126, 999_999));

    assert_eq!(collector.histogram.count(), 3);

    let (p50, _p90, _p95, p99) = collector.calculate_percentiles_histogram_us();
    assert!(p50 > 0);
    assert!(p99 >= p50);

    let (p50_ms, _p90_ms, _p95_ms, _p99_ms) = collector.calculate_percentiles_histogram_f64_ms();
    assert!(p50_ms > 0.0);

    collector.clear();
    assert_eq!(collector.histogram.count(), 0);
    assert_eq!(collector.len(), 0);
}

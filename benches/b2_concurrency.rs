//! Benchmark B2: Scaling with Threads and Tenant Count (Core C3)
//!
//! Specifications:
//! - C3: Threads {1, 8, 16} x tenants {1, 1000}, Zipf α=0.8 over 100,000 prewarmed keys (≈100% hits).
//! - Non-core: Threads {2, 4} and 8-thread 90/10 hit/miss mix.
//! - Metrics: Throughput (ops/s), HDR histogram latency percentiles (p50, p90, p99, p99.9, max).

use hdrhistogram::Histogram;
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use std::hint::black_box;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// Simple deterministic Zipf approximation (α=0.8) for bounded prewarmed keys
fn generate_zipf_indices(n_samples: usize, universe_size: usize) -> Vec<usize> {
    let mut indices = Vec::with_capacity(n_samples);
    let mut rng_state: u64 = 0x853c49e6748fea9b;
    for _ in 0..n_samples {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        let u = (rng_state as f64) / (u64::MAX as f64);
        // Inverse CDF approximation for Zipf α = 0.8: F(x) ≈ (x / N)^(1 - α)
        // x ≈ N * u^(1 / (1 - α)) = N * u^(1 / 0.2) = N * u^5.0
        let idx = ((universe_size as f64) * u.powf(5.0)) as usize;
        indices.push(idx.min(universe_size.saturating_sub(1)));
    }
    indices
}

fn run_concurrency_test(
    num_threads: usize,
    tenant_count: usize,
    duration_secs: u64,
    miss_percentage: u32, // 0 for 100% hits, 10 for 90/10 mix
) -> serde_json::Value {
    let cache = Arc::new(PartitionedCache::new(16));
    let universe_size = 100_000;
    let body = Arc::new(vec![0x55u8; 1024]);
    let now_ms = 1_000_000;

    // Pre-populate tenants if > 1
    if tenant_count > 1 {
        for t in 0..tenant_count {
            let _ = cache.try_invalidate_tenant(&format!("tenant_{}", t));
        }
    }

    // Pre-warm 100,000 keys
    for i in 0..universe_size {
        let tenant_idx = i % tenant_count.max(1);
        let key_str = format!("key_{}", i);
        let tenant_str = format!("tenant_{}", tenant_idx);
        let k = CacheKey::new(Some(&tenant_str), &key_str, None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k, now_ms) {
            let guard = reg.into_guard();
            guard.complete(FinalizeOutcome::Success(Arc::new(CachedResource {
                status: 200,
                content_type: "text/plain".to_string(),
                body: body.clone(),
                etag: None,
                created_at_ms: now_ms,
                ttl_ms: 3_600_000,
            })));
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    let measuring = Arc::new(AtomicBool::new(false));
    let total_ops = Arc::new(AtomicU64::new(0));
    let mut handles = Vec::new();

    let pregenerated_keys = Arc::new(generate_zipf_indices(200_000, universe_size));

    for thread_id in 0..num_threads {
        let c = cache.clone();
        let r = running.clone();
        let m = measuring.clone();
        let ops_counter = total_ops.clone();
        let keys = pregenerated_keys.clone();
        let b = body.clone();

        handles.push(thread::spawn(move || {
            let mut hist = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap();
            let mut local_ops: u64 = 0;
            let mut key_idx = (thread_id * 10_000) % keys.len();

            while r.load(Ordering::Relaxed) {
                let is_m = m.load(Ordering::Relaxed);
                let k_num = keys[key_idx];
                key_idx = (key_idx + 1) % keys.len();

                let (tenant_str, key_str) = if miss_percentage > 0 && (k_num % 100) < (miss_percentage as usize) {
                    // Guaranteed miss key
                    (None, format!("miss_{}_{}", thread_id, local_ops))
                } else {
                    let tenant_idx = k_num % tenant_count.max(1);
                    (Some(format!("tenant_{}", tenant_idx)), format!("key_{}", k_num))
                };

                let k = CacheKey::new(tenant_str.as_deref(), &key_str, None);

                let start = if is_m { Some(Instant::now()) } else { None };
                match c.begin_fetch_atomic(&k, now_ms) {
                    FetchRegistrationResult::Hit(res) => {
                        black_box(res);
                    }
                    FetchRegistrationResult::Initiate(reg) => {
                        let guard = reg.into_guard();
                        guard.complete(FinalizeOutcome::Success(Arc::new(CachedResource {
                            status: 200,
                            content_type: "text/plain".to_string(),
                            body: b.clone(),
                            etag: None,
                            created_at_ms: now_ms,
                            ttl_ms: 3_600_000,
                        })));
                    }
                    _ => {}
                }
                if let Some(s) = start {
                    let elapsed_ns = s.elapsed().as_nanos() as u64;
                    let _ = hist.record(elapsed_ns.clamp(1, 60_000_000_000));
                    local_ops += 1;
                }
            }

            ops_counter.fetch_add(local_ops, Ordering::SeqCst);
            hist
        }));
    }

    // Warm-up 2 seconds
    thread::sleep(Duration::from_secs(2));

    // Measurement window
    let bench_start = Instant::now();
    measuring.store(true, Ordering::SeqCst);
    thread::sleep(Duration::from_secs(duration_secs));
    measuring.store(false, Ordering::SeqCst);
    running.store(false, Ordering::SeqCst);

    let mut merged_hist = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3).unwrap();
    for h in handles {
        let th_hist = h.join().unwrap();
        merged_hist.add(th_hist).unwrap();
    }

    let actual_duration = bench_start.elapsed().as_secs_f64();
    let ops = total_ops.load(Ordering::SeqCst);
    let throughput_ops_sec = (ops as f64) / actual_duration;

    serde_json::json!({
        "threads": num_threads,
        "tenants": tenant_count,
        "miss_pct": miss_percentage,
        "duration_sec": actual_duration,
        "total_ops": ops,
        "throughput_ops_sec": throughput_ops_sec,
        "latency_ns": {
            "p50": merged_hist.value_at_quantile(0.50),
            "p90": merged_hist.value_at_quantile(0.90),
            "p95": merged_hist.value_at_quantile(0.95),
            "p99": merged_hist.value_at_quantile(0.99),
            "p999": merged_hist.value_at_quantile(0.999),
            "max": merged_hist.max()
        }
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let duration = if args.len() > 1 && args[1] == "--quick" {
        2
    } else {
        5
    };

    let mut results = Vec::new();

    // Core C3: threads {1, 8, 16} x tenants {1, 1000}
    for &threads in &[1, 8, 16] {
        for &tenants in &[1, 1000] {
            results.push(run_concurrency_test(threads, tenants, duration, 0));
        }
    }

    // Non-core: threads {2, 4} with 1 tenant, and 8 threads with 90/10 mix
    results.push(run_concurrency_test(2, 1, duration, 0));
    results.push(run_concurrency_test(4, 1, duration, 0));
    results.push(run_concurrency_test(8, 1, duration, 10));

    let output = serde_json::json!({
        "benchmark": "B2",
        "runs": results
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

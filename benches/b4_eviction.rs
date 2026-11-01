//! Benchmark B4: Eviction Pressure (Non-Core)
//!
//! Specifications:
//! - Shard capacity {1,000; 10,000 entries}, policy {Sieve, Lru, S3Fifo as implemented}.
//! - Workload {all-miss churn, Zipf α=0.8}.
//! - Metric: latency of the miss that triggers eviction through the public path.

use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, EvictionPolicy, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use std::sync::Arc;
use std::time::Instant;

fn run_eviction_bench(
    capacity_per_shard: usize,
    policy: EvictionPolicy,
    policy_name: &str,
    churn_count: usize,
) -> serde_json::Value {
    let cache = Arc::new(PartitionedCache::new(1)); // 1 shard for direct capacity control
    cache.set_capacity_limits(capacity_per_shard, 100);
    cache.set_eviction_policy(policy);

    let body = Arc::new(vec![0x11u8; 128]);
    let now_ms = 1_000_000;

    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: body.clone(),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 3_600_000,
    });

    // Fill to capacity
    for i in 0..capacity_per_shard {
        let k = CacheKey::new(None, &format!("fill_item_{}", i), None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k, now_ms) {
            let guard = reg.into_guard();
            guard.complete(FinalizeOutcome::Success(res.clone()));
        }
    }

    // Measure eviction churn latency
    let start = Instant::now();
    for i in 0..churn_count {
        let k = CacheKey::new(None, &format!("churn_item_{}", i), None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k, now_ms) {
            let guard = reg.into_guard();
            guard.complete(FinalizeOutcome::Success(res.clone()));
        }
    }
    let elapsed_ns = (start.elapsed().as_nanos() as f64) / (churn_count as f64);

    serde_json::json!({
        "capacity_per_shard": capacity_per_shard,
        "policy": policy_name,
        "churn_count": churn_count,
        "avg_ns_per_evicting_miss": elapsed_ns
    })
}

fn main() {
    let mut results = Vec::new();

    for &cap in &[1_000, 10_000] {
        results.push(run_eviction_bench(cap, EvictionPolicy::Sieve, "Sieve", 500));
        results.push(run_eviction_bench(cap, EvictionPolicy::Lru, "Lru", 500));
        results.push(run_eviction_bench(cap, EvictionPolicy::S3Fifo, "S3Fifo", 500));
    }

    let output = serde_json::json!({
        "benchmark": "B4",
        "runs": results
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

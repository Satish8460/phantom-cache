//! Benchmark B7: Memory & Allocation Accounting (Non-Core)
//!
//! Specifications:
//! - Per-entry growth: 10,000 entries with body {1 KiB, 64 KiB}.
//! - Counting-allocator allocations per hit and per fill.
//! - Deterministic allocation counts (R2 >= 3 must agree exactly).

#[path = "support/alloc_counter.rs"]
mod alloc_counter;

use alloc_counter::CountingAlloc;
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use std::hint::black_box;
use std::sync::Arc;

#[global_allocator]
static ALLOCATOR: CountingAlloc = CountingAlloc::new();

fn measure_fill_and_hit_allocations(body_size: usize, count: usize) -> serde_json::Value {
    let cache = Arc::new(PartitionedCache::new(8));
    let body_bytes = Arc::new(vec![0xAAu8; body_size]);
    let now_ms = 1_000_000;

    let res = Arc::new(CachedResource {
        status: 200,
        content_type: "application/octet-stream".to_string(),
        body: body_bytes.clone(),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 3_600_000,
    });

    // Reset allocator stats before fill
    ALLOCATOR.reset();
    let stats_before_fill = ALLOCATOR.stats();

    for i in 0..count {
        let k = CacheKey::new(None, &format!("b7_key_{}", i), None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&k, now_ms) {
            let guard = reg.into_guard();
            guard.complete(FinalizeOutcome::Success(res.clone()));
        }
    }

    let stats_after_fill = ALLOCATOR.stats();
    let fill_allocs = stats_after_fill.alloc_count - stats_before_fill.alloc_count;
    let fill_allocated_bytes = stats_after_fill.allocated_bytes - stats_before_fill.allocated_bytes;

    // Measure hit allocations
    ALLOCATOR.reset();
    let stats_before_hits = ALLOCATOR.stats();

    for i in 0..count {
        let k = CacheKey::new(None, &format!("b7_key_{}", i), None);
        let r = cache.begin_fetch_atomic(&k, now_ms);
        black_box(r);
    }

    let stats_after_hits = ALLOCATOR.stats();
    let hit_allocs = stats_after_hits.alloc_count - stats_before_hits.alloc_count;
    let hit_allocated_bytes = stats_after_hits.allocated_bytes - stats_before_hits.allocated_bytes;

    serde_json::json!({
        "body_size_bytes": body_size,
        "entry_count": count,
        "fill_alloc_count": fill_allocs,
        "fill_allocated_bytes": fill_allocated_bytes,
        "allocs_per_fill": (fill_allocs as f64) / (count as f64),
        "hit_alloc_count": hit_allocs,
        "hit_allocated_bytes": hit_allocated_bytes,
        "allocs_per_hit": (hit_allocs as f64) / (count as f64)
    })
}

fn main() {
    let mut results = Vec::new();
    for &size in &[1024, 65536] {
        results.push(measure_fill_and_hit_allocations(size, 10_000));
    }

    let output = serde_json::json!({
        "benchmark": "B7",
        "runs": results
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

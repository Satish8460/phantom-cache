//! Benchmark B1: Hit Path and Component Cost Isolation (Core C1, C2)
//!
//! Specifications:
//! - C1: Full hit (`begin_fetch_atomic` Hit) with key 64 B, body 4 KiB, tenants {1, 1000}.
//! - C2: Component cost isolation:
//!   - `get_effective_epochs` (tenant-map sizes {1, 1000})
//!   - `get_shard_index`
//!   - `CachedResource::clone`
//!   - fill (`Initiate` -> `complete`)

use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

fn run_b1_measurements(iterations: usize) -> serde_json::Value {
    let now_ms = 1_000_000;
    let body_4k = Arc::new(vec![0xAAu8; 4096]);
    let key_64 = "k".repeat(64);

    let test_resource = Arc::new(CachedResource {
        status: 200,
        content_type: "application/octet-stream".to_string(),
        body: body_4k.clone(),
        etag: None,
        created_at_ms: now_ms,
        ttl_ms: 3_600_000,
    });

    // -----------------------------------------------------------------------
    // C1: Full Hit Path with 1 tenant vs 1000 tenants
    // -----------------------------------------------------------------------
    let cache_1t = Arc::new(PartitionedCache::new(8));
    let k_1t = CacheKey::new(Some("tenant_0"), &key_64, None);
    if let FetchRegistrationResult::Initiate(reg) = cache_1t.begin_fetch_atomic(&k_1t, now_ms) {
        let guard = reg.into_guard();
        guard.complete(FinalizeOutcome::Success(test_resource.clone()));
    }

    let cache_1000t = Arc::new(PartitionedCache::new(8));
    for i in 0..1000 {
        let _ = cache_1000t.try_invalidate_tenant(&format!("tenant_{}", i));
    }
    let k_1000t = CacheKey::new(Some("tenant_500"), &key_64, None);
    if let FetchRegistrationResult::Initiate(reg) = cache_1000t.begin_fetch_atomic(&k_1000t, now_ms) {
        let guard = reg.into_guard();
        guard.complete(FinalizeOutcome::Success(test_resource.clone()));
    }

    // Warmup
    for _ in 0..1000 {
        black_box(cache_1t.begin_fetch_atomic(&k_1t, now_ms));
        black_box(cache_1000t.begin_fetch_atomic(&k_1000t, now_ms));
    }

    // Measure C1: 1 tenant
    let start = Instant::now();
    for _ in 0..iterations {
        let res = cache_1t.begin_fetch_atomic(&k_1t, now_ms);
        black_box(res);
    }
    let c1_1t_ns = (start.elapsed().as_nanos() as f64) / (iterations as f64);

    // Measure C1: 1000 tenants
    let start = Instant::now();
    for _ in 0..iterations {
        let res = cache_1000t.begin_fetch_atomic(&k_1000t, now_ms);
        black_box(res);
    }
    let c1_1000t_ns = (start.elapsed().as_nanos() as f64) / (iterations as f64);

    // -----------------------------------------------------------------------
    // C2: Component Cost Isolation
    // -----------------------------------------------------------------------
    // 1. get_effective_epochs (1 vs 1000 tenants)
    let start = Instant::now();
    for _ in 0..iterations {
        let epochs = cache_1t.get_effective_epochs("tenant_0");
        black_box(epochs);
    }
    let c2_epochs_1t_ns = (start.elapsed().as_nanos() as f64) / (iterations as f64);

    let start = Instant::now();
    for _ in 0..iterations {
        let epochs = cache_1000t.get_effective_epochs("tenant_500");
        black_box(epochs);
    }
    let c2_epochs_1000t_ns = (start.elapsed().as_nanos() as f64) / (iterations as f64);

    // 2. get_shard_index
    let start = Instant::now();
    for _ in 0..iterations {
        let idx = cache_1t.get_shard_index(&k_1t);
        black_box(idx);
    }
    let c2_shard_index_ns = (start.elapsed().as_nanos() as f64) / (iterations as f64);

    // 3. CachedResource::clone
    let start = Instant::now();
    for _ in 0..iterations {
        let cloned = test_resource.clone();
        black_box(cloned);
    }
    let c2_resource_clone_ns = (start.elapsed().as_nanos() as f64) / (iterations as f64);

    // 4. Fill (Initiate -> complete)
    let fill_iterations = (iterations / 10).max(1000);
    let start = Instant::now();
    for i in 0..fill_iterations {
        let key_str = format!("fill_key_{}", i);
        let k = CacheKey::new(None, &key_str, None);
        if let FetchRegistrationResult::Initiate(reg) = cache_1t.begin_fetch_atomic(&k, now_ms) {
            let guard = reg.into_guard();
            guard.complete(FinalizeOutcome::Success(test_resource.clone()));
        }
    }
    let c2_fill_ns = (start.elapsed().as_nanos() as f64) / (fill_iterations as f64);

    serde_json::json!({
        "benchmark": "B1",
        "iterations": iterations,
        "c1_hit_1_tenant_ns": c1_1t_ns,
        "c1_hit_1000_tenants_ns": c1_1000t_ns,
        "c2_epochs_1_tenant_ns": c2_epochs_1t_ns,
        "c2_epochs_1000_tenants_ns": c2_epochs_1000t_ns,
        "c2_shard_index_ns": c2_shard_index_ns,
        "c2_resource_clone_ns": c2_resource_clone_ns,
        "c2_fill_ns": c2_fill_ns
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let iters = if args.len() > 1 && args[1] == "--quick" {
        10_000
    } else {
        100_000
    };

    let result = run_b1_measurements(iters);
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}

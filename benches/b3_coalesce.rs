//! Benchmark B3: Request Coalescing (Non-Core)
//!
//! Specifications:
//! - N {10, 100, 1000} waiters on one key, simulated origin latency 10 ms (tokio timer).
//! - Assert origin calls == 1 inside the benchmark (sanity assertion).

use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

async fn run_coalesce_bench(waiters: usize, simulated_latency_ms: u64) -> serde_json::Value {
    let cache = Arc::new(PartitionedCache::new(8));
    let origin_calls = Arc::new(AtomicUsize::new(0));
    let body = Arc::new(vec![0x42u8; 2048]);
    let now_ms = 1_000_000;
    let key = CacheKey::new(None, "coalesced_stampede_key", None);

    let start = Instant::now();
    let mut handles = Vec::new();

    for _ in 0..waiters {
        let c = cache.clone();
        let k = key.clone();
        let oc = origin_calls.clone();
        let b = body.clone();

        handles.push(tokio::spawn(async move {
            match c.begin_fetch_atomic(&k, now_ms) {
                FetchRegistrationResult::Hit(res) => res.body.len(),
                FetchRegistrationResult::Wait(rx) => {
                    if let Ok(Ok(res)) = rx.await {
                        res.body.len()
                    } else {
                        0
                    }
                }
                FetchRegistrationResult::Initiate(reg) => {
                    oc.fetch_add(1, Ordering::SeqCst);
                    let guard = reg.into_guard();
                    tokio::time::sleep(Duration::from_millis(simulated_latency_ms)).await;
                    let res = Arc::new(CachedResource {
                        status: 200,
                        content_type: "application/octet-stream".to_string(),
                        body: b,
                        etag: None,
                        created_at_ms: now_ms,
                        ttl_ms: 3_600_000,
                    });
                    guard.complete(FinalizeOutcome::Success(res));
                    2048
                }
                FetchRegistrationResult::Rejected(_) => 0,
            }
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let actual_origin_calls = origin_calls.load(Ordering::SeqCst);
    assert_eq!(
        actual_origin_calls, 1,
        "Coalescing sanity violated: expected exactly 1 origin call, got {}",
        actual_origin_calls
    );

    serde_json::json!({
        "waiters": waiters,
        "simulated_latency_ms": simulated_latency_ms,
        "origin_calls": actual_origin_calls,
        "total_elapsed_ms": elapsed_ms
    })
}

#[tokio::main]
async fn main() {
    let mut results = Vec::new();
    for &n in &[10, 100, 1000] {
        results.push(run_coalesce_bench(n, 10).await);
    }

    let output = serde_json::json!({
        "benchmark": "B3",
        "runs": results
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

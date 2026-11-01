//! M7 finding D-2: the idle gateway burned more than one core. Root cause (reproduced): the sweeper
//! task selected on `shutdown.changed()`, which resolves immediately with `Err` once the watch
//! sender is dropped (start_server dropped it), so the loop spun without ever waiting.
//!
//! Behavioural test on wake-up counts, not a CPU benchmark.

use phantom_cache_demo::core::cache::PartitionedCache;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

fn cache_with_interval(ms: u64) -> Arc<PartitionedCache> {
    let cache = Arc::new(PartitionedCache::new(2));
    cache.set_sweeper_interval_ms(ms);
    cache
}

// A dropped shutdown sender must not turn the sweeper into a busy loop.
#[tokio::test]
async fn sweeper_with_dropped_shutdown_sender_wakes_only_on_interval() {
    let cache = cache_with_interval(50);
    let (tx, rx) = tokio::sync::watch::channel(false);
    let h = PartitionedCache::spawn_sweeper_task(cache.clone(), rx);
    drop(tx);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let wakeups = cache.sweeper_wakeups.load(Ordering::Relaxed);
    // 500 ms / 50 ms = ~10 ticks (+1 immediate tick). Generous bound; the bug yields thousands.
    assert!(wakeups <= 30, "sweeper woke {} times in 500 ms at a 50 ms interval (busy loop)", wakeups);
    assert!(wakeups >= 3, "sweeper must still tick after the sender is dropped, got {}", wakeups);
    h.abort();
}

// The shutdown signal still stops the sweeper.
#[tokio::test]
async fn sweeper_stops_on_shutdown_signal() {
    let cache = cache_with_interval(50);
    let (tx, rx) = tokio::sync::watch::channel(false);
    let h = PartitionedCache::spawn_sweeper_task(cache.clone(), rx);
    tokio::time::sleep(Duration::from_millis(120)).await;
    tx.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(2), h).await.expect("sweeper must stop").unwrap();
}

// start_server-level regression: the server must not turn its own sweeper into a busy loop.
// (start_server used to drop the shutdown sender immediately.) The wake-up bound is the observable
// symptom; it is not a direct assertion that the sender is held. With only the loop-side guard in
// place this still passes (that guard alone prevents the spin), so it guards the symptom end to end.
#[tokio::test]
async fn start_server_keeps_sweeper_idle() {
    use phantom_cache_demo::api::server::{start_server, ApiState, SimulationSnapshot};
    use phantom_cache_demo::intel::diagnostics::{CacheCounters, DiagnosticBuffer};
    use std::sync::Mutex;

    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = probe.local_addr().unwrap();
    drop(probe);
    std::env::set_var("PHANTOM_BIND_ADDR", addr.to_string());
    std::env::remove_var("PHANTOM_SIMULATION");
    std::env::remove_var("PHANTOM_TLS_CERT");
    std::env::remove_var("PHANTOM_TLS_KEY");

    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(64, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(2, counters, diagnostics));
    cache.set_sweeper_interval_ms(50);
    let state = Arc::new(ApiState {
        snapshot: Mutex::new(SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: 0,
            throughput_req_sec: 0.0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            db_load_ratio: 0.0,
            avoided_compute_dollars: 0.0,
            nodes: vec![],
        }),
        nodes: vec![],
        workload_type: Mutex::new("static".to_string()),
        cache: Some(cache.clone()),
        origin_url: None,
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    });
    let h = tokio::spawn(start_server(state));
    tokio::time::sleep(Duration::from_millis(600)).await;
    let wakeups = cache.sweeper_wakeups.load(Ordering::Relaxed);
    assert!(wakeups >= 3, "sweeper must be running under start_server, got {}", wakeups);
    assert!(wakeups <= 40, "sweeper busy-looping under start_server: {} wake-ups in ~600 ms", wakeups);
    h.abort();
}

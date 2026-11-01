//! Integration tests for Phantom Lab Tier-2 Controls: Real Origin & Open-Loop Loadgen (M4-04 / WP-16).
//!
//! Validates:
//! - Controllable HTTP origin lifecycle, ephemeral port binding, and clean shutdown.
//! - Dynamic admin API (/admin/config, /admin/metrics, /admin/reset).
//! - Concurrency limiting via Semaphore and natural queueing delay emergence.
//! - Degradation modes (outage 503, brownout 500).
//! - Open-loop generator execution and EV-4 generator health (achieved rate >= 99%, scheduling lag bounds).

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::loadgen::OpenLoopLoadgen;
use lab::origin_proc::{ControllableOrigin, OriginConfig, OriginMetricsSnapshot};
use lab::rng::StreamRng;
use lab::workloads::UniformWorkload;
use reqwest::Client;

#[tokio::test]
async fn test_origin_proc_lifecycle_and_admin_api() {
    let origin = ControllableOrigin::start(OriginConfig::default())
        .await
        .expect("Failed to start origin");

    let client = Client::new();

    // 1. Fetch data
    let resp = client
        .get(origin.url("/data/my_key"))
        .send()
        .await
        .expect("Fetch failed");
    assert_eq!(resp.status(), 200);
    let cc = resp
        .headers()
        .get("cache-control")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(cc.contains("max-age=60"));
    let body = resp.text().await.unwrap();
    assert_eq!(body, "DATA:my_key");

    // 2. Query metrics
    let snap: OriginMetricsSnapshot = client
        .get(origin.url("/admin/metrics"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap.total_received, 1);
    assert_eq!(snap.total_completed, 1);
    assert_eq!(snap.total_errors, 0);

    // 3. Update config via admin API
    let new_cfg = OriginConfig {
        capacity: 4,
        latency_ms: 5,
        error_rate: 0.0,
        is_outage: false,
        default_ttl_sec: 120,
    };
    let update_res = client
        .post(origin.url("/admin/config"))
        .json(&new_cfg)
        .send()
        .await
        .unwrap();
    assert_eq!(update_res.status(), 200);

    // 4. Verify new config reflected
    let read_cfg: OriginConfig = client
        .get(origin.url("/admin/config"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(read_cfg.capacity, 4);
    assert_eq!(read_cfg.default_ttl_sec, 120);

    // 5. Reset metrics
    let reset_res = client
        .post(origin.url("/admin/reset"))
        .send()
        .await
        .unwrap();
    assert_eq!(reset_res.status(), 200);

    let snap_after: OriginMetricsSnapshot = client
        .get(origin.url("/admin/metrics"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap_after.total_received, 0);
    assert_eq!(snap_after.total_completed, 0);

    origin.shutdown();
}

#[tokio::test]
async fn test_origin_proc_degradation_modes() {
    let origin = ControllableOrigin::start(OriginConfig::default())
        .await
        .expect("Failed to start origin");
    let client = Client::new();

    // Outage injection
    origin
        .update_config(OriginConfig {
            capacity: 8,
            latency_ms: 0,
            error_rate: 0.0,
            is_outage: true,
            default_ttl_sec: 60,
        })
        .await;

    let r_out = client
        .get(origin.url("/data/item"))
        .send()
        .await
        .unwrap();
    assert_eq!(r_out.status(), 503);

    // Brownout 100% error injection
    origin
        .update_config(OriginConfig {
            capacity: 8,
            latency_ms: 0,
            error_rate: 1.0,
            is_outage: false,
            default_ttl_sec: 60,
        })
        .await;

    let r_br = client
        .get(origin.url("/data/item"))
        .send()
        .await
        .unwrap();
    assert_eq!(r_br.status(), 500);

    // Recovery to normal
    origin
        .update_config(OriginConfig {
            capacity: 8,
            latency_ms: 0,
            error_rate: 0.0,
            is_outage: false,
            default_ttl_sec: 60,
        })
        .await;

    let r_ok = client
        .get(origin.url("/data/item"))
        .send()
        .await
        .unwrap();
    assert_eq!(r_ok.status(), 200);

    origin.shutdown();
}

#[tokio::test]
async fn test_origin_proc_concurrency_queueing() {
    // Capacity 2, simulated latency 40ms
    let origin = ControllableOrigin::start(OriginConfig {
        capacity: 2,
        latency_ms: 40,
        error_rate: 0.0,
        is_outage: false,
        default_ttl_sec: 60,
    })
    .await
    .unwrap();

    let client = Client::new();
    let mut tasks = Vec::new();

    // Spawn 6 concurrent requests
    for i in 0..6 {
        let c = client.clone();
        let u = origin.url(&format!("/data/item_{}", i));
        tasks.push(tokio::spawn(async move {
            c.get(&u).send().await
        }));
    }

    for t in tasks {
        let res = t.await.unwrap().unwrap();
        assert_eq!(res.status(), 200);
    }

    let snap = origin.snapshot_metrics();
    assert_eq!(snap.total_completed, 6);
    assert!(
        snap.max_in_flight <= 2,
        "Max in-flight was {}, expected <= 2",
        snap.max_in_flight
    );
    assert!(
        snap.total_queue_wait_us > 0,
        "Queue wait time should be non-zero when concurrency cap is exceeded"
    );

    origin.shutdown();
}

#[tokio::test]
async fn test_open_loop_loadgen_health_and_ev4() {
    // Healthy origin with ample capacity
    let origin = ControllableOrigin::start(OriginConfig {
        capacity: 32,
        latency_ms: 1,
        error_rate: 0.0,
        is_outage: false,
        default_ttl_sec: 60,
    })
    .await
    .unwrap();

    let loadgen = OpenLoopLoadgen::new(origin.url("/data"));
    let workload = UniformWorkload::new(10, 500.0); // 500 req/sec
    let rng = StreamRng::new(20009);

    let report = loadgen.run(workload, 100, rng).await.unwrap();

    // EV-4 Generator health checks
    assert!(
        report.achieved_ratio >= 0.99,
        "Achieved ratio {:.4} violated EV-4 (must be >= 0.99)",
        report.achieved_ratio
    );
    assert_eq!(report.completed_requests, 100);
    assert_eq!(report.error_requests, 0);
    assert!(
        report.max_scheduling_lag_ms < 50.0,
        "Scheduling lag {:.2}ms was excessive (generator bottle-necked)",
        report.max_scheduling_lag_ms
    );
    assert!(report.latency_p50_us > 0);
    assert!(report.latency_max_us >= report.latency_p50_us);

    origin.shutdown();
}

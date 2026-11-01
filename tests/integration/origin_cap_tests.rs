//! R-01: origin-concurrency cap on the HTTP serving path.
//!
//! The intelligence executor is process-global (`ApiState::set_intelligence` uses a OnceLock),
//! so every test takes `SERIAL` and reconfigures the one shared executor.
//! Origins are raw-TCP servers (pattern of `spawn_raw_tcp_origin` in m1_http_tests.rs) that
//! record the maximum number of simultaneously open origin connections.

use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::intelligence::audit::AuditStore;
use phantom_cache_demo::intelligence::executor::DefaultPolicyExecutor;
use phantom_cache_demo::intelligence::{IntelligenceMode, PolicyExecutor};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static EXEC: OnceLock<Arc<DefaultPolicyExecutor>> = OnceLock::new();

fn shared_executor() -> Arc<DefaultPolicyExecutor> {
    EXEC.get_or_init(|| {
        let exec = Arc::new(DefaultPolicyExecutor::new(None));
        let audit = Arc::new(Mutex::new(AuditStore::new(10)));
        ApiState::set_intelligence(exec.clone(), audit);
        exec
    })
    .clone()
}

struct RawOrigin {
    url: String,
    max: Arc<AtomicUsize>,
    total: Arc<AtomicU64>,
}

/// Raw-TCP origin. Path routing: `/hang*` never answers (waits for the client to close),
/// `/err*` closes without a response, everything else answers after `delay_ms`.
async fn spawn_origin(delay_ms: u64, cache_control: &'static str) -> RawOrigin {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let cur = Arc::new(AtomicUsize::new(0));
    let max = Arc::new(AtomicUsize::new(0));
    let total = Arc::new(AtomicU64::new(0));
    let (c, m, t) = (cur.clone(), max.clone(), total.clone());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let (c, m, t) = (c.clone(), m.clone(), t.clone());
            let now = c.fetch_add(1, Ordering::SeqCst) + 1;
            m.fetch_max(now, Ordering::SeqCst);
            t.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                if path.starts_with("/hang") {
                    let mut sink = [0u8; 64];
                    while matches!(socket.read(&mut sink).await, Ok(k) if k > 0) {}
                } else if path.starts_with("/err") {
                    // drop without answering
                } else {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    let body = format!("{{\"path\":\"{}\"}}", path);
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        cache_control,
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.flush().await;
                }
                c.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });
    RawOrigin { url: format!("http://127.0.0.1:{}", port), max, total }
}

struct Gateway {
    base: String,
    client: reqwest::Client,
}

async fn spawn_gateway(origin_url: &str) -> Gateway {
    let cache = Arc::new(PartitionedCache::new(8));
    let nodes = vec![Arc::new(phantom_cache_demo::dist::node::VirtualNode::new(1))];
    let state = Arc::new(ApiState {
        snapshot: Mutex::new(SimulationSnapshot {
            version: "1.0".into(),
            timestamp_ms: 0,
            throughput_req_sec: 0.0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            db_load_ratio: 0.0,
            avoided_compute_dollars: 0.0,
            nodes: vec![NodeSnapshot {
                node_id: 1,
                state: "Active".into(),
                cpu_utilization: 0.0,
                memory_used_bytes: 0,
            }],
        }),
        nodes,
        workload_type: Mutex::new("Normal".into()),
        cache: Some(cache),
        origin_url: Some(origin_url.to_string()),
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    });
    let app = create_app(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    Gateway {
        base: format!("http://127.0.0.1:{}", port),
        client: reqwest::Client::builder().pool_max_idle_per_host(0).build().unwrap(),
    }
}

impl Gateway {
    fn url(&self, key: &str) -> String {
        format!("{}/api/v1/cache/{}", self.base, key)
    }
    async fn metrics(&self) -> Value {
        self.client
            .get(format!("{}/api/v1/metrics", self.base))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    }
}

fn configure(exec: &DefaultPolicyExecutor, mode: IntelligenceMode, cap: usize, ceiling_ms: u32, window: usize) {
    exec.set_mode(mode).unwrap();
    exec.configure_origin_cap(1, cap, ceiling_ms, window);
}

async fn wait_in_flight_zero(exec: &DefaultPolicyExecutor, max_wait_ms: u64) -> bool {
    let deadline = std::time::Instant::now() + Duration::from_millis(max_wait_ms);
    while std::time::Instant::now() < deadline {
        if exec.origin_gate_snapshot().in_flight == 0 {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    exec.origin_gate_snapshot().in_flight == 0
}

// ---------------------------------------------------------------------------

#[tokio::test]
async fn cap_never_exceeded_by_concurrent_distinct_misses() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    const CAP: usize = 4;
    const N: usize = 24;
    configure(&exec, IntelligenceMode::Active, CAP, 60_000, 1_000_000);
    let origin = spawn_origin(250, "max-age=60").await;
    let gw = spawn_gateway(&origin.url).await;
    let before = exec.origin_gate_snapshot();

    let mut tasks = Vec::new();
    for i in 0..N {
        let c = gw.client.clone();
        let u = gw.url(&format!("slow-{}", i));
        tasks.push(tokio::spawn(async move {
            let r = c.get(u).send().await.unwrap();
            (r.status().as_u16(), r.headers().get("retry-after").is_some())
        }));
    }
    let mut ok = 0;
    let mut rej = 0;
    for t in tasks {
        let (status, retry_after) = t.await.unwrap();
        match status {
            200 => ok += 1,
            503 => {
                assert!(retry_after, "503 must carry Retry-After");
                rej += 1;
            }
            s => panic!("unexpected status {}", s),
        }
    }
    let max_seen = origin.max.load(Ordering::SeqCst);
    assert!(max_seen <= CAP, "origin saw {} concurrent connections, cap is {}", max_seen, CAP);
    assert!(max_seen >= 2, "test did not generate concurrency (max {})", max_seen);
    assert_eq!(ok + rej, N);
    assert!(rej >= 1, "expected saturation with {} concurrent misses vs cap {}", N, CAP);
    assert_eq!(origin.total.load(Ordering::SeqCst) as usize, ok, "only admitted requests reach the origin");
    let after = exec.origin_gate_snapshot();
    assert_eq!((after.rejections - before.rejections) as usize, rej);
    assert!(wait_in_flight_zero(&exec, 2000).await);
}

#[tokio::test]
async fn off_mode_is_uncapped() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    // A cap is configured but mode is Off: behavior must be unchanged (uncapped).
    configure(&exec, IntelligenceMode::Off, 2, 60_000, 1_000_000);
    let origin = spawn_origin(300, "max-age=60").await;
    let gw = spawn_gateway(&origin.url).await;
    let before = exec.origin_gate_snapshot();

    let mut tasks = Vec::new();
    for i in 0..12 {
        let c = gw.client.clone();
        let u = gw.url(&format!("slow-off-{}", i));
        tasks.push(tokio::spawn(async move { c.get(u).send().await.unwrap().status().as_u16() }));
    }
    for t in tasks {
        assert_eq!(t.await.unwrap(), 200);
    }
    assert!(origin.max.load(Ordering::SeqCst) > 2, "Off mode must not enforce the cap");
    let after = exec.origin_gate_snapshot();
    assert!(!after.enforcing);
    assert_eq!(after.rejections, before.rejections);
}

#[tokio::test]
async fn coalesced_waiters_do_not_consume_permits() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    configure(&exec, IntelligenceMode::Active, 1, 60_000, 1_000_000);
    let origin = spawn_origin(400, "max-age=60").await;
    let gw = spawn_gateway(&origin.url).await;
    let before = exec.origin_gate_snapshot();

    let mut tasks = Vec::new();
    for _ in 0..10 {
        let c = gw.client.clone();
        let u = gw.url("slow-same");
        tasks.push(tokio::spawn(async move { c.get(u).send().await.unwrap().status().as_u16() }));
    }
    for t in tasks {
        assert_eq!(t.await.unwrap(), 200, "waiters must be served, not rejected by a cap of 1");
    }
    let after = exec.origin_gate_snapshot();
    assert_eq!(origin.total.load(Ordering::SeqCst), 1, "single-flight: one origin dispatch");
    assert_eq!(after.dispatched - before.dispatched, 1, "only the leader took a permit");
    assert_eq!(after.rejections, before.rejections);
    assert_eq!(origin.max.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn permit_released_after_origin_error() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    configure(&exec, IntelligenceMode::Active, 1, 60_000, 1_000_000);
    let origin = spawn_origin(10, "max-age=60").await;
    let gw = spawn_gateway(&origin.url).await;

    for i in 0..3 {
        let r = gw.client.get(gw.url(&format!("err-{}", i))).send().await.unwrap();
        assert!(r.status().is_server_error(), "origin error surfaces as 5xx, got {}", r.status());
        assert_ne!(r.headers().get("x-cache-error").map(|v| v.to_str().unwrap()), Some("CapacityExceeded"),
            "with cap=1 a leaked permit would make request {} a cap rejection", i);
    }
    assert!(wait_in_flight_zero(&exec, 2000).await);
    let r = gw.client.get(gw.url("ok-after-err")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
}

#[tokio::test]
async fn permit_released_after_origin_timeout() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    configure(&exec, IntelligenceMode::Active, 1, 60_000, 1_000_000);
    let origin = spawn_origin(10, "max-age=60").await;
    let gw = spawn_gateway(&origin.url).await;

    // Held while the origin hangs: a second distinct miss must be rejected.
    let c = gw.client.clone();
    let u = gw.url("hang-1");
    let hung = tokio::spawn(async move { c.get(u).send().await.unwrap().status().as_u16() });
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(exec.origin_gate_snapshot().in_flight, 1);
    let r = gw.client.get(gw.url("other")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 503, "cap=1 is held by the hanging dispatch");

    // fetch_origin_raw read timeout is 5 s -> 504, permit must be freed.
    let status = tokio::time::timeout(Duration::from_secs(12), hung).await.unwrap().unwrap();
    assert_eq!(status, 504);
    assert!(wait_in_flight_zero(&exec, 2000).await, "permit leaked after timeout");
    let r = gw.client.get(gw.url("after-timeout")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
}

#[tokio::test]
async fn permit_released_after_client_disconnect() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    configure(&exec, IntelligenceMode::Active, 1, 60_000, 1_000_000);
    let origin = spawn_origin(500, "max-age=60").await;
    let gw = spawn_gateway(&origin.url).await;

    // Client gives up after 100 ms.
    let impatient = reqwest::Client::builder().timeout(Duration::from_millis(100)).build().unwrap();
    assert!(impatient.get(gw.url("slow-dc")).send().await.is_err());
    // The leader keeps running (it populates the cache), so it still owns the permit.
    assert_eq!(exec.origin_gate_snapshot().in_flight, 1);
    let r = gw.client.get(gw.url("slow-other")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 503);

    assert!(wait_in_flight_zero(&exec, 3000).await, "permit leaked after client disconnect");
    let r = gw.client.get(gw.url("slow-dc")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(r.headers().get("x-cache-status").unwrap(), "HIT", "abandoned fetch still filled the cache");
    let r = gw.client.get(gw.url("slow-after-dc")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
}

#[tokio::test]
async fn permit_is_released_on_future_cancellation() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    configure(&exec, IntelligenceMode::Active, 1, 60_000, 1_000_000);
    let e2 = exec.clone();
    let task = tokio::spawn(async move {
        let _permit = e2.try_acquire_origin_permit().expect("first permit");
        tokio::time::sleep(Duration::from_secs(30)).await;
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(exec.origin_gate_snapshot().in_flight, 1);
    assert!(exec.try_acquire_origin_permit().is_err(), "cap=1 saturated");
    task.abort();
    let _ = task.await;
    assert!(wait_in_flight_zero(&exec, 1000).await, "abort (future cancellation) must drop the permit");
    assert!(exec.try_acquire_origin_permit().is_ok());
}

#[tokio::test]
async fn saturation_serves_stale_if_eligible_else_503() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    configure(&exec, IntelligenceMode::Active, 1, 60_000, 1_000_000);
    // Fresh for 1 s, stale-if-error window 60 s (bounded by the hard stale cap).
    let origin = spawn_origin(800, "max-age=1, stale-if-error=60").await;
    let gw = spawn_gateway(&origin.url).await;

    let r = gw.client.get(gw.url("slow-seed")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let seed_body = r.text().await.unwrap();
    assert!(wait_in_flight_zero(&exec, 2000).await);
    tokio::time::sleep(Duration::from_millis(1200)).await; // expire

    // Occupy the single permit with a different key.
    let c = gw.client.clone();
    let u = gw.url("slow-busy");
    let busy = tokio::spawn(async move { c.get(u).send().await.unwrap().status().as_u16() });
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(exec.origin_gate_snapshot().in_flight, 1);
    let total_before = origin.total.load(Ordering::SeqCst);
    let rej_before = exec.origin_gate_snapshot().rejections;

    // Expired entry with stale-if-error budget: served stale, no origin dispatch.
    let r = gw.client.get(gw.url("slow-seed")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(r.headers().get("x-cache-status").unwrap(), "STALE");
    assert_eq!(r.text().await.unwrap(), seed_body);

    // Never-cached key: nothing eligible -> 503 + Retry-After.
    let r = gw.client.get(gw.url("slow-nothing-cached")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 503);
    assert!(r.headers().get("retry-after").is_some());

    assert_eq!(origin.total.load(Ordering::SeqCst), total_before, "rejected requests never reached the origin");
    assert_eq!(exec.origin_gate_snapshot().rejections - rej_before, 2);

    // Telemetry exposes cap / in-flight / rejections as measured.
    let m = gw.metrics().await;
    let oc = &m["metrics"]["origin_cap"];
    assert_eq!(oc["cap"]["src"], "measured");
    assert_eq!(oc["cap"]["v"], 1);
    assert_eq!(oc["in_flight"]["src"], "measured");
    assert_eq!(oc["in_flight"]["v"], 1);
    assert_eq!(oc["origin_cap_rejections"]["src"], "measured");
    assert!(oc["origin_cap_rejections"]["v"].as_u64().unwrap() >= 2);
    assert_eq!(oc["enforcing"]["v"], true);

    assert_eq!(busy.await.unwrap(), 200);
}

#[tokio::test]
async fn controller_is_fed_real_origin_latency() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    // max 8, p99 ceiling 50 ms, step every 4 real observations; origin takes ~200 ms.
    configure(&exec, IntelligenceMode::Active, 8, 50, 4);
    assert_eq!(exec.origin_gate_snapshot().cap, Some(8));
    let origin = spawn_origin(200, "max-age=60").await;
    let gw = spawn_gateway(&origin.url).await;

    let mut tasks = Vec::new();
    for i in 0..4 {
        let c = gw.client.clone();
        let u = gw.url(&format!("slow-aimd-{}", i));
        tasks.push(tokio::spawn(async move { c.get(u).send().await.unwrap().status().as_u16() }));
    }
    for t in tasks {
        assert_eq!(t.await.unwrap(), 200);
    }
    let snap = exec.origin_gate_snapshot();
    assert!(snap.controller_steps >= 1, "controller never stepped");
    assert!(snap.cap.unwrap() < 8, "AIMD must back off after measured p99 > ceiling, cap={:?}", snap.cap);
}

/// G2 / D2: an origin-cap rejection must not claim that the cache shard is saturated. It keeps the
/// compatibility header `X-Cache-Error: CapacityExceeded` and `Retry-After`, but carries its own
/// reason header and body text.
#[tokio::test]
async fn origin_cap_rejection_has_its_own_body_and_reason_code() {
    let _s = SERIAL.lock().await;
    let exec = shared_executor();
    configure(&exec, IntelligenceMode::Active, 1, 60_000, 1_000_000);
    let origin = spawn_origin(800, "max-age=60").await;
    let gw = spawn_gateway(&origin.url).await;

    let c = gw.client.clone();
    let u = gw.url("d2-busy");
    let busy = tokio::spawn(async move { c.get(u).send().await.unwrap().status().as_u16() });
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(exec.origin_gate_snapshot().in_flight, 1);

    let r = gw.client.get(gw.url("d2-shed")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 503);
    assert_eq!(r.headers().get("x-cache-error").unwrap(), "CapacityExceeded", "compatibility header kept");
    assert_eq!(r.headers().get("retry-after").unwrap(), "1");
    assert_eq!(r.headers().get("x-cache-error-reason").map(|v| v.to_str().unwrap()), Some("OriginCapSaturated"));
    let body = r.text().await.unwrap();
    assert!(body.contains("Origin concurrency cap saturated"), "body: {body}");
    assert!(!body.contains("shard"), "must not claim shard saturation: {body}");

    assert_eq!(busy.await.unwrap(), 200);
    assert!(wait_in_flight_zero(&exec, 2000).await);
}

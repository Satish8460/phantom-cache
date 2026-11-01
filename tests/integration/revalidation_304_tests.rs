//! D-1 / S-1 and S-2: 304 revalidation must leave the refreshed lifetime in the cache,
//! and late freshness-metadata writes from a pre-DELETE leader must not clobber newer state.
//!
//! Origin is a raw-TCP mock (pattern from m1_http_tests). Timing is real wall-clock,
//! loopback, debug profile: these are behavioural tests, not performance measurements.

use phantom_cache_demo::api::server::{create_app, ApiState, SimulationSnapshot};
use phantom_cache_demo::core::cache::{CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use phantom_cache_demo::core::freshness::FreshnessMetadata;
use phantom_cache_demo::intel::diagnostics::{CacheCounters, DiagnosticBuffer};
use phantom_cache_demo::intel::telemetry::TelemetryCollector;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

struct Origin {
    addr: std::net::SocketAddr,
    full: Arc<AtomicU64>,
    conditional: Arc<AtomicU64>, // number of 304 responses served
    inm_seen: Arc<AtomicU64>,    // number of requests that carried If-None-Match
}

/// `first_cc` is the Cache-Control of the 200; `reval_cc` of the 304.
async fn spawn_origin(first_cc: &'static str, reval_cc: &'static str) -> Origin {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let full = Arc::new(AtomicU64::new(0));
    let conditional = Arc::new(AtomicU64::new(0));
    let inm_seen = Arc::new(AtomicU64::new(0));
    let (f, c, i) = (full.clone(), conditional.clone(), inm_seen.clone());
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => break,
            };
            let (f, c, i) = (f.clone(), c.clone(), i.clone());
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let n = sock.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
                if req.contains("if-none-match") {
                    i.fetch_add(1, Ordering::SeqCst);
                }
                // ROBUSTNESS fixture: answers 304 to every request after the first, whether or not the
                // request is conditional. The gateway now sends real conditional requests (see the
                // "validator forwarding" tests at the end of this file); a 304 that ignores the
                // validator is what a sloppy origin does, and the 304 handlers must stay safe.
                let resp = if f.load(Ordering::SeqCst) >= 1 {
                    c.fetch_add(1, Ordering::SeqCst);
                    format!(
                        "HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nCache-Control: {}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
                        reval_cc
                    )
                } else {
                    f.fetch_add(1, Ordering::SeqCst);
                    let body = "{\"v\":1}";
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nETag: \"v1\"\r\nCache-Control: {}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                        first_cc,
                        body.len(),
                        body
                    )
                };
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    Origin { addr, full, conditional, inm_seen }
}

async fn spawn_gateway(origin: std::net::SocketAddr) -> (String, Arc<PartitionedCache>) {
    spawn_gateway_opts(origin, None).await
}

/// `sweeper_ms = Some(n)` also runs the lazy-reclamation sweeper at an n ms interval, as the
/// shipped binary (start_server) does. Plain `spawn_gateway` has NO sweeper.
async fn spawn_gateway_opts(origin: std::net::SocketAddr, sweeper_ms: Option<u64>) -> (String, Arc<PartitionedCache>) {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters, diagnostics));
    if let Some(ms) = sweeper_ms {
        cache.set_sweeper_interval_ms(ms);
        let (tx, rx) = tokio::sync::watch::channel(false);
        PartitionedCache::spawn_sweeper_task(cache.clone(), rx);
        std::mem::forget(tx); // keep the sender alive for the test process lifetime
    }
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
        origin_url: Some(format!("http://{}", origin)),
        telemetry: Some(Arc::new(TelemetryCollector::new(512))),
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    });
    let app = create_app(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{}", addr), cache)
}

fn xcache(r: &reqwest::Response) -> String {
    r.headers()
        .get("X-Cache-Status")
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default()
}

fn age(r: &reqwest::Response) -> u64 {
    r.headers()
        .get("Age")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(u64::MAX)
}

async fn wait_for(counter: &Arc<AtomicU64>, at_least: u64) {
    for _ in 0..100 {
        if counter.load(Ordering::SeqCst) >= at_least {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("origin never saw the expected request");
}

// ROBUSTNESS (S-1 background path). The mock origin answers 304 regardless of any validator. The gateway
// does send conditional requests now (real exchanges are tested at the end of this file); this keeps
// the 304 handler covered against a sloppy origin that sends a 304 anyway.
#[tokio::test]
async fn robust_unsolicited_304_background_revalidation_leaves_entry_fresh() {
    let origin = spawn_origin("max-age=1, stale-while-revalidate=60", "max-age=300, stale-while-revalidate=60").await;
    let (gw, cache) = spawn_gateway(origin.addr).await;
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/s1-bg", gw);

    let r1 = client.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r1), "MISS");
    assert_eq!(origin.full.load(Ordering::SeqCst), 1);

    tokio::time::sleep(Duration::from_millis(1400)).await;

    let r2 = client.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r2), "STALE", "expired entry within SWR window must be served stale");

    tokio::time::sleep(Duration::from_millis(500)).await;
    wait_for(&origin.conditional, 1).await;
    assert_eq!(origin.inm_seen.load(Ordering::SeqCst), 1, "the gateway sent the stored ETag upstream");
    assert_eq!(cache.counters.revalidation_not_modified.load(Ordering::Relaxed), 1);

    let r3 = client.get(&url).send().await.unwrap();
    let key = CacheKey::new(Some("__default__"), "/s1-bg", None);
    let entry_dbg = cache
        .get_resource(&key, DiagnosticBuffer::now_ms())
        .map(|e| (e.created_at_ms, e.ttl_ms));
    assert_eq!(
        xcache(&r3),
        "HIT",
        "after a 304 refresh the next GET must be a HIT (http status {}, entry now = {:?}, freshness = {:?})",
        r3.status(),
        entry_dbg,
        cache.get_freshness(&key)
    );
    assert!(age(&r3) <= 1, "Age must be reset by the 304, got {}", age(&r3));
    assert_eq!(origin.full.load(Ordering::SeqCst), 1, "no full refetch expected");
}

// ROBUSTNESS (S-1 foreground path; 304 from a mock origin that ignores validators): the entry is only
// stale-if-error eligible, so the foreground leader receives the 304 from origin.
#[tokio::test]
async fn robust_unsolicited_304_foreground_revalidation_leaves_entry_fresh() {
    let origin = spawn_origin("max-age=1, stale-if-error=60", "max-age=300, stale-if-error=60").await;
    let (gw, cache) = spawn_gateway(origin.addr).await;
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/s1-fg", gw);

    let r1 = client.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r1), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;

    let r2 = client.get(&url).send().await.unwrap();
    assert_eq!(
        origin.conditional.load(Ordering::SeqCst),
        1,
        "origin must have seen the conditional request (status {})",
        r2.status()
    );
    assert_eq!(origin.inm_seen.load(Ordering::SeqCst), 1, "the gateway sent the stored ETag upstream");
    assert_eq!(cache.counters.revalidation_not_modified.load(Ordering::Relaxed), 1);
    tokio::time::sleep(Duration::from_millis(200)).await;

    let r3 = client.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r3), "HIT", "after a foreground 304 the next GET must be a HIT (status {})", r3.status());
    assert!(age(&r3) <= 1, "Age must be reset, got {}", age(&r3));
    assert_eq!(origin.full.load(Ordering::SeqCst), 1, "no full refetch expected");
}

// S-2: freshness metadata writes are not fenced by generation.
fn meta(swr: u64, must_revalidate: bool) -> FreshnessMetadata {
    FreshnessMetadata {
        must_revalidate,
        stale_while_revalidate_ms: Some(swr),
        stale_if_error_ms: None,
        last_modified: None,
        date_ms: Some(1),
        initial_age_ms: 0,
    }
}

fn res(body: &[u8], now: u64) -> CachedResource {
    CachedResource {
        status: 200,
        content_type: "text/plain".to_string(),
        body: Arc::new(body.to_vec()),
        etag: None,
        created_at_ms: now,
        ttl_ms: 60_000,
    }
}

fn new_cache() -> Arc<PartitionedCache> {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    Arc::new(PartitionedCache::new_with_diagnostics(2, counters, diagnostics))
}

#[tokio::test]
async fn s2_late_metadata_from_pre_delete_leader_does_not_clobber_newer_fill() {
    let cache = new_cache();
    let key = CacheKey::new(Some("t"), "/s2", None);
    let now = DiagnosticBuffer::now_ms();

    let g1 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!("expected Initiate"),
    };
    cache.delete(&key);
    let g2 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!("expected second Initiate after delete"),
    };
    assert!(g2.cache.update_freshness_for_guard(&g2, meta(0, true)));
    assert_eq!(cache.counters.metadata_setter_rejections.load(Ordering::Relaxed), 0, "live leader is not refused");
    if let Some(mut b) = g2.complete(FinalizeOutcome::Success(Arc::new(res(b"v2", now)))) {
        b.deliver();
    }
    assert!(cache.get_freshness(&key).unwrap().must_revalidate, "precondition: v2 metadata present");

    // Pre-DELETE leader 1 applies its late metadata (as server.rs does just before complete()).
    // Note: with metadata parked on the op and installed by finalize_fetch (M1), finalize's own
    // generation fence also protects v2, so the live-op fence is defence in depth; what only the
    // fence produces is the refusal itself, asserted via the return value AND the dedicated counter.
    assert!(!g1.cache.update_freshness_for_guard(&g1, meta(60_000, false)), "late write from overtaken leader must be refused");
    assert_eq!(
        cache.counters.metadata_setter_rejections.load(Ordering::Relaxed),
        1,
        "exactly the overtaken leader's write is counted as fence-refused"
    );
    if let Some(mut b) = g1.complete(FinalizeOutcome::Success(Arc::new(res(b"v1", now)))) {
        b.deliver();
    }

    let entry = cache.get_resource(&key, now).expect("v2 entry resident");
    assert_eq!(&*entry.body, b"v2", "generation fencing keeps v2 body");
    let f = cache.get_freshness(&key).expect("freshness present");
    assert!(
        f.must_revalidate && f.stale_while_revalidate_ms == Some(0),
        "v1's late metadata (swr=60000, must_revalidate=false) must not overwrite v2's: {:?}",
        f
    );
}

#[tokio::test]
async fn s2_late_refresh_entry_lifetime_after_newer_fill_does_not_alter_v2() {
    let cache = new_cache();
    let key = CacheKey::new(Some("t"), "/s2b", None);
    let now = DiagnosticBuffer::now_ms();

    let g1 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    cache.delete(&key);
    let g2 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    assert!(g2.cache.update_freshness_for_guard(&g2, meta(0, true)));
    if let Some(mut b) = g2.complete(FinalizeOutcome::Success(Arc::new(res(b"v2", now)))) {
        b.deliver();
    }

    assert!(!g1.cache.refresh_entry_lifetime_for_guard(
        &g1,
        now + 5_000,
        999_000,
        Some("\"old\"".to_string()),
        meta(60_000, false),
    ));
    let entry = cache.get_resource(&key, now + 6_000).expect("v2 resident");
    assert_eq!(entry.ttl_ms, 60_000, "v2 ttl must not be changed by a pre-DELETE leader's 304 refresh");
    assert!(cache.get_freshness(&key).unwrap().must_revalidate);
    if let Some(mut b) = g1.complete(FinalizeOutcome::Success(Arc::new(res(b"v1", now)))) {
        b.deliver();
    }
}

#[tokio::test]
async fn s2_late_metadata_after_delete_without_refill_leaves_no_orphan() {
    let cache = new_cache();
    let key = CacheKey::new(Some("t"), "/s2c", None);
    let now = DiagnosticBuffer::now_ms();
    let g1 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    cache.delete(&key);
    assert!(!g1.cache.update_freshness_for_guard(&g1, meta(60_000, false)), "late write from overtaken leader must be refused");
    if let Some(mut b) = g1.complete(FinalizeOutcome::Success(Arc::new(res(b"v1", now)))) {
        b.deliver();
    }
    assert!(cache.get_resource(&key, now).is_none(), "fenced finalize must not store v1 after DELETE");
    assert!(
        cache.get_freshness(&key).is_none(),
        "orphan freshness metadata for a deleted key must not linger"
    );
}

// DOCUMENTATION TEST (not regression coverage). S-1 mechanism ONLY (characterization, passes before and after the fix; it does not prove the HTTP
// behaviour and must be read together with the two robust_unsolicited_304_* HTTP tests above): finalize_fetch stores
// exactly the resource it is handed. Handing it a clone taken BEFORE refresh_entry_lifetime
// therefore reverts created_at_ms/ttl_ms; the server fix passes the refreshed resource instead.
#[tokio::test]
async fn doc_characterization_s1_finalize_stores_supplied_resource_so_prerefresh_clone_reverts_refresh() {
    let cache = new_cache();
    let key = CacheKey::new(Some("t"), "/s1-mech", None);
    let t0 = 1_000_000u64;
    let mut v1 = res(b"body", t0);
    v1.ttl_ms = 1_000;
    let g0 = match cache.begin_fetch_atomic(&key, t0) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    if let Some(mut b) = g0.complete(FinalizeOutcome::Success(Arc::new(v1.clone()))) {
        b.deliver();
    }

    // Revalidation leader at t0+5000: clone taken first, then 304 refresh, then finalize.
    let g1 = match cache.begin_fetch_atomic(&key, t0 + 5_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    let pre_refresh_clone = v1.clone();
    assert!(g1.cache.refresh_entry_lifetime_for_guard(&g1, t0 + 5_000, 300_000, None, meta(0, false)));
    assert_eq!(cache.get_resource(&key, t0 + 6_000).expect("refreshed").ttl_ms, 300_000);

    let mut refreshed = pre_refresh_clone.clone();
    refreshed.created_at_ms = t0 + 5_000;
    refreshed.ttl_ms = 300_000;
    if let Some(mut b) = g1.complete(FinalizeOutcome::Success(Arc::new(refreshed))) {
        b.deliver();
    }
    assert_eq!(cache.get_resource(&key, t0 + 6_000).expect("still fresh").ttl_ms, 300_000);

    // Same sequence with the pre-refresh clone (what server.rs did before the fix).
    let g2 = match cache.begin_fetch_atomic(&key, t0 + 400_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    assert!(g2.cache.refresh_entry_lifetime_for_guard(&g2, t0 + 400_000, 300_000, None, meta(0, false)));
    if let Some(mut b) = g2.complete(FinalizeOutcome::Success(Arc::new(pre_refresh_clone))) {
        b.deliver();
    }
    assert!(
        cache.get_resource(&key, t0 + 401_000).is_none(),
        "finalize overwrote the refreshed entry with the pre-refresh clone"
    );
}

// Byte accounting stays exact when a 304 changes the ETag length.
#[tokio::test]
async fn s1_refresh_with_longer_etag_keeps_byte_audit_exact() {
    let cache = new_cache();
    let key = CacheKey::new(Some("t"), "/s1-bytes", None);
    let t0 = 1_000_000u64;
    let g0 = match cache.begin_fetch_atomic(&key, t0) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    if let Some(mut b) = g0.complete(FinalizeOutcome::Success(Arc::new(res(b"body", t0)))) {
        b.deliver();
    }
    let g1 = match cache.begin_fetch_atomic(&key, t0 + 100_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    let long_etag = "\"a-considerably-longer-etag-value-0123456789\"".to_string();
    assert!(g1.cache.refresh_entry_lifetime_for_guard(&g1, t0 + 100_000, 60_000, Some(long_etag.clone()), meta(0, false)));
    let mut refreshed = res(b"body", t0 + 100_000);
    refreshed.etag = Some(long_etag);
    if let Some(mut b) = g1.complete(FinalizeOutcome::Success(Arc::new(refreshed))) {
        b.deliver();
    }
    let (resident, recorded) = cache.audit_bytes();
    assert_eq!(resident, recorded, "bytes_resident must equal recorded sum (I1-I8)");
}

// Found while reproducing S-1 (call it S-9): check_stale_serving did not require the entry to be
// expired, so a FRESH entry carrying a stale-while-revalidate directive was served as STALE
// (never HIT) and triggered a background revalidation on every request.
#[tokio::test]
async fn s9_fresh_entry_with_swr_is_hit_and_does_not_revalidate() {
    let origin = spawn_origin("max-age=300, stale-while-revalidate=60", "max-age=300").await;
    let (gw, cache) = spawn_gateway(origin.addr).await;
    let client = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/s9", gw);
    let r1 = client.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r1), "MISS");
    let r2 = client.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r2), "HIT", "fresh entry must be a HIT even if it has an SWR window");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(cache.counters.revalidation_started.load(Ordering::Relaxed), 0, "no revalidation for a fresh entry");
    assert_eq!(cache.counters.stale_served_total.load(Ordering::Relaxed), 0);
    assert_eq!(origin.conditional.load(Ordering::SeqCst), 0, "origin must not be contacted again");
}

// ---------------------------------------------------------------------------------------------
// Independent-review findings M1 / M2 / L1 / L3 (2026-10-29).
// ---------------------------------------------------------------------------------------------

fn one_shard_cache() -> Arc<PartitionedCache> {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    Arc::new(PartitionedCache::new_with_diagnostics(1, counters, diagnostics))
}

fn fill(cache: &Arc<PartitionedCache>, key: &CacheKey, r: CachedResource, m: FreshnessMetadata, now: u64) {
    let g = match cache.begin_fetch_atomic(key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!("expected Initiate"),
    };
    assert!(g.cache.update_freshness_for_guard(&g, m));
    if let Some(mut b) = g.complete(FinalizeOutcome::Success(Arc::new(r))) {
        b.deliver();
    }
}

// M1: metadata of a REJECTED finalize (oversize / budget) must not survive and relabel V1's body.
#[tokio::test]
async fn m1_rejected_oversize_finalize_does_not_replace_resident_freshness() {
    let cache = one_shard_cache();
    let key = CacheKey::new(Some("t"), "/m1a", None);
    let now = 1_000_000u64;
    let mut v1 = res(b"v1", now);
    v1.ttl_ms = 1_000;
    fill(&cache, &key, v1, meta(600_000, true), now);
    let (_, recorded) = cache.audit_bytes();
    cache.set_byte_budget(recorded + 100);

    let later = now + 5_000; // V1 is expired
    let g2 = match cache.begin_fetch_atomic(&key, later) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!("expected Initiate"),
    };
    g2.cache.update_freshness_for_guard(&g2, meta(600_000, false));
    let big = res(&vec![7u8; 10_000], later);
    if let Some(mut b) = g2.complete(FinalizeOutcome::Success(Arc::new(big))) {
        b.deliver();
    }
    assert_eq!(cache.counters.oversize_rejections.load(Ordering::Relaxed), 1, "precondition: V2 rejected");
    assert_eq!(
        cache.get_freshness(&key),
        Some(meta(600_000, true)),
        "V1 must keep its must_revalidate directive after V2's finalize was rejected"
    );
    assert!(
        matches!(
            cache.check_stale_serving(&key, later, 86_400_000, false),
            phantom_cache_demo::core::cache::StaleServingDecision::Ineligible
        ),
        "V1 (must-revalidate) must not be served STALE under V2's directives"
    );
}

// M1: same, rejection caused by a tenant invalidation between begin and complete.
#[tokio::test]
async fn m1_epoch_rejected_finalize_does_not_replace_resident_freshness() {
    let cache = one_shard_cache();
    let key = CacheKey::new(Some("t"), "/m1b", None);
    let now = 1_000_000u64;
    let mut v1 = res(b"v1", now);
    v1.ttl_ms = 1_000;
    fill(&cache, &key, v1, meta(600_000, true), now);

    let later = now + 5_000;
    let g2 = match cache.begin_fetch_atomic(&key, later) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!("expected Initiate"),
    };
    g2.cache.update_freshness_for_guard(&g2, meta(600_000, false));
    cache.try_invalidate_tenant("t").unwrap();
    if let Some(mut b) = g2.complete(FinalizeOutcome::Success(Arc::new(res(b"v2", later)))) {
        b.deliver();
    }
    // The invalidated V1 is still physically resident (nothing has read/swept it), so its own
    // metadata must be exactly what it was: an exact match discriminates from V2's directives.
    assert_eq!(
        cache.get_freshness(&key),
        Some(meta(600_000, true)),
        "rejected (epoch) finalize must leave V1's metadata untouched, not V2's"
    );
    assert_eq!(cache.counters.stale_writes_prevented.load(Ordering::Relaxed), 1, "precondition: V2 was epoch-rejected");
}

// M1 caveat A: a 304 that grows the ETag must not push the shard over its byte budget.
#[tokio::test]
async fn m1_refresh_with_longer_etag_never_exceeds_byte_budget() {
    let cache = one_shard_cache();
    let key = CacheKey::new(Some("t"), "/m1c", None);
    let t0 = 1_000_000u64;
    fill(&cache, &key, res(b"body", t0), meta(0, false), t0);
    let (_, recorded) = cache.audit_bytes();
    cache.set_byte_budget(recorded); // zero headroom
    let g1 = match cache.begin_fetch_atomic(&key, t0 + 100_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    let long_etag = "\"a-considerably-longer-etag-value-0123456789\"".to_string();
    g1.cache.refresh_entry_lifetime_for_guard(&g1, t0 + 100_000, 60_000, Some(long_etag.clone()), meta(0, false));
    let mut refreshed = res(b"body", t0 + 100_000);
    refreshed.etag = Some(long_etag);
    if let Some(mut b) = g1.complete(FinalizeOutcome::Success(Arc::new(refreshed))) {
        b.deliver();
    }
    let (resident, rec) = cache.audit_bytes();
    assert_eq!(resident, rec, "audit must stay exact");
    assert!(rec <= recorded, "shard bytes {} exceed byte budget {}", rec, recorded);
    assert!(
        cache.get_resource(&key, t0 + 100_001).is_some(),
        "the refreshed entry must stay resident (ETag growth that does not fit must be skipped, not evict the entry)"
    );
}

// L3: setters refused by the fence are counted (metadata_setter_rejections).
#[tokio::test]
async fn l3_fenced_setter_refusals_are_counted() {
    let cache = new_cache();
    let key = CacheKey::new(Some("t"), "/l3", None);
    let now = DiagnosticBuffer::now_ms();
    let g1 = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    cache.delete(&key);
    assert!(!g1.cache.update_freshness_for_guard(&g1, meta(1, false)));
    assert!(!g1.cache.refresh_entry_lifetime_for_guard(&g1, now, 1, None, meta(1, false)));
    assert_eq!(cache.counters.metadata_setter_rejections.load(Ordering::Relaxed), 2);
}

/// Raw origin: first request gets a 200 (`first_extra` header lines), every later request gets a
/// 304 after `delay_ms` with `reval_extra` header lines. ROBUSTNESS fixture: the later answers are 304s
/// whether or not the request carried a validator (the gateway does send them now); the tests that
/// use it exercise the gateway's 304 handlers against an origin that does not check validators.
struct RawOrigin {
    addr: std::net::SocketAddr,
    later: Arc<AtomicU64>,
}

async fn spawn_raw_origin(first_extra: &'static str, reval_extra: &'static str, delay_ms: u64) -> RawOrigin {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let first_done = Arc::new(AtomicU64::new(0));
    let later = Arc::new(AtomicU64::new(0));
    let (fd, l) = (first_done.clone(), later.clone());
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => break,
            };
            let (fd, l) = (fd.clone(), l.clone());
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let _ = sock.read(&mut buf).await;
                let resp = if fd.fetch_add(1, Ordering::SeqCst) == 0 {
                    let body = "{\"v\":1}";
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{}Connection: close\r\nContent-Length: {}\r\n\r\n{}",
                        first_extra,
                        body.len(),
                        body
                    )
                } else {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    l.fetch_add(1, Ordering::SeqCst);
                    format!("HTTP/1.1 304 Not Modified\r\n{}Connection: close\r\nContent-Length: 0\r\n\r\n", reval_extra)
                };
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    RawOrigin { addr, later }
}

fn dkey(p: &str) -> CacheKey {
    CacheKey::new(Some("__default__"), p, None)
}

// M2 (background): a 304 without Cache-Control keeps stored SWR / Last-Modified / ttl.
#[tokio::test]
async fn robust_unsolicited_304_m2_bg_without_cache_control_keeps_stored_metadata() {
    let o = spawn_raw_origin(
        "ETag: \"v1\"\r\nLast-Modified: Sun, 06 Nov 1994 08:49:37 GMT\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        "ETag: \"v1\"\r\n",
        0,
    )
    .await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/m2-bg", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_for(&o.later, 1).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let f = cache.get_freshness(&dkey("/m2-bg")).expect("freshness kept");
    assert_eq!(f.stale_while_revalidate_ms, Some(600_000), "SWR wiped by 304 without Cache-Control: {:?}", f);
    assert!(f.last_modified.is_some(), "Last-Modified wiped: {:?}", f);
    let e = cache.get_resource(&dkey("/m2-bg"), DiagnosticBuffer::now_ms()).expect("refreshed, fresh again");
    assert_eq!(e.ttl_ms, 1_000, "stored lifetime must be retained when the 304 carries no freshness headers");
    tokio::time::sleep(Duration::from_millis(1300)).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE", "SWR window must still apply");
}

// M2 (foreground): same via the stale-if-error path.
#[tokio::test]
async fn robust_unsolicited_304_m2_fg_without_cache_control_keeps_stored_metadata() {
    let o = spawn_raw_origin("ETag: \"v1\"\r\nCache-Control: max-age=1, stale-if-error=600\r\n", "ETag: \"v1\"\r\n", 0).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/m2-fg", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;
    let _ = c.get(&url).send().await.unwrap();
    wait_for(&o.later, 1).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let f = cache.get_freshness(&dkey("/m2-fg")).expect("freshness kept");
    assert_eq!(f.stale_if_error_ms, Some(600_000), "SIE wiped by 304 without Cache-Control: {:?}", f);
}

// M2: a 304 carrying a different validator must not relabel the stored body (RFC 9111 4.3.4).
#[tokio::test]
async fn robust_unsolicited_304_m2_bg_with_different_etag_does_not_relabel_body() {
    let o = spawn_raw_origin(
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        "ETag: \"v2\"\r\nCache-Control: max-age=300\r\n",
        0,
    )
    .await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/m2-etag", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_for(&o.later, 1).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let e = cache.get_resource(&dkey("/m2-etag"), DiagnosticBuffer::now_ms());
    assert!(
        e.as_ref().is_none_or(|r| r.etag.as_deref() != Some("\"v2\"")),
        "v1 body relabelled with v2 ETag: {:?}",
        e.map(|r| r.etag)
    );
    assert_eq!(cache.counters.revalidation_not_modified.load(Ordering::Relaxed), 0);
    // Pin the chosen behaviour: a validator mismatch DELETES the entry (it is neither refreshed nor
    // kept), and counts as a revalidation error.
    assert!(e_is_none_after_mismatch(&cache), "mismatching 304 must delete the stored entry");
    assert_eq!(cache.counters.revalidation_error.load(Ordering::Relaxed), 1);
}

fn e_is_none_after_mismatch(cache: &Arc<PartitionedCache>) -> bool {
    cache.get_resource(&dkey("/m2-etag"), 0).is_none() && cache.get_freshness(&dkey("/m2-etag")).is_none()
}

// L1: an uncacheable 304 must back off, not spawn a new revalidation on every stale request.
#[tokio::test]
async fn l1_uncacheable_304_backs_off_revalidation() {
    let o = spawn_raw_origin(
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        "ETag: \"v1\"\r\nCache-Control: no-store\r\n",
        0,
    )
    .await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/l1", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;
    let _ = c.get(&url).send().await.unwrap();
    wait_for(&o.later, 1).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    for _ in 0..3 {
        let _ = c.get(&url).send().await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(o.later.load(Ordering::SeqCst), 1, "backoff must suppress repeat revalidations");
    assert_eq!(cache.counters.revalidation_not_modified.load(Ordering::Relaxed), 0);
}

// L3 (HTTP): a DELETE while the 304 is in flight fences the refresh; it must not count as a 304.
#[tokio::test]
async fn l3_304_refused_by_fence_after_delete_is_not_counted() {
    let o = spawn_raw_origin(
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        "ETag: \"v1\"\r\nCache-Control: max-age=300\r\n",
        700,
    )
    .await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/l3h", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;
    let r_stale = c.get(&url).send().await.unwrap();
    assert_eq!(r_stale.status().as_u16(), 200);
    assert_eq!(xcache(&r_stale), "STALE");
    tokio::time::sleep(Duration::from_millis(100)).await;
    cache.delete(&dkey("/l3h"));
    wait_for(&o.later, 1).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(cache.counters.revalidation_not_modified.load(Ordering::Relaxed), 0, "fenced 304 counted as not-modified");
    assert_eq!(cache.counters.revalidation_not_applied.load(Ordering::Relaxed), 1, "the refused 304 is counted as not-applied");
    assert!(cache.get_resource(&dkey("/l3h"), DiagnosticBuffer::now_ms()).is_none(), "DELETE must win");
}

// ---- H2: the sweeper (running by default in start_server) must not reclaim entries that are still
// inside their usable stale-while-revalidate / stale-if-error window.

// HTTP, with the sweeper running alongside the gateway as in the shipped binary.
#[tokio::test]
async fn h2_http_with_sweeper_running_expired_entry_in_swr_window_is_served_stale() {
    let o = spawn_raw_origin(
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        0,
    )
    .await;
    let (gw, _cache) = spawn_gateway_opts(o.addr, Some(20)).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/h2-http", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1500)).await; // expired; sweeper has had ~75 ticks
    assert_eq!(
        xcache(&c.get(&url).send().await.unwrap()),
        "STALE",
        "sweeper must not reclaim an entry inside its SWR window"
    );
    wait_for(&o.later, 1).await; // and the background revalidation happened
}

fn sweep_fixture(m: FreshnessMetadata, ttl_ms: u64) -> (Arc<PartitionedCache>, CacheKey, u64) {
    let cache = one_shard_cache();
    let key = CacheKey::new(Some("t"), "/sweep", None);
    let t0 = 1_000_000u64;
    let mut r = res(b"body", t0);
    r.ttl_ms = ttl_ms;
    fill(&cache, &key, r, m, t0);
    (cache, key, t0)
}

fn resident(cache: &Arc<PartitionedCache>) -> bool {
    let (accounted, _) = cache.audit_bytes();
    accounted > 0
}

#[test]
fn h2_sweeper_keeps_entry_inside_swr_window() {
    let (cache, _k, t0) = sweep_fixture(meta(5_000, false), 1_000);
    // expired at t0+1000, SWR window until t0+6000
    let (n, _) = cache.sweep_lazy_reclamation(32, t0 + 3_000);
    assert_eq!(n, 0, "entry inside its SWR window must be retained");
    assert!(resident(&cache));
}

#[test]
fn h2_sweeper_reclaims_entry_beyond_swr_window() {
    let (cache, _k, t0) = sweep_fixture(meta(5_000, false), 1_000);
    let (n, _) = cache.sweep_lazy_reclamation(32, t0 + 7_000);
    assert_eq!(n, 1, "entry beyond ttl+window must be reclaimed");
    assert!(!resident(&cache));
}

#[test]
fn h2_sweeper_retains_for_stale_if_error_window_too() {
    let m = FreshnessMetadata {
        must_revalidate: false,
        stale_while_revalidate_ms: None,
        stale_if_error_ms: Some(5_000),
        last_modified: None,
        date_ms: Some(1),
        initial_age_ms: 0,
    };
    let (cache, _k, t0) = sweep_fixture(m, 1_000);
    assert_eq!(cache.sweep_lazy_reclamation(32, t0 + 3_000).0, 0);
    assert_eq!(cache.sweep_lazy_reclamation(32, t0 + 7_000).0, 1);
}

#[test]
fn h2_sweeper_reclaims_must_revalidate_entry_at_expiry() {
    // must-revalidate forbids any stale serving, so there is no window to protect.
    let (cache, _k, t0) = sweep_fixture(meta(600_000, true), 1_000);
    assert_eq!(cache.sweep_lazy_reclamation(32, t0 + 3_000).0, 1);
}

#[test]
fn h2_sweeper_window_is_bounded_by_hard_stale_cap() {
    let (cache, _k, t0) = sweep_fixture(meta(10_000_000_000, false), 1_000);
    cache.set_max_stale_ms(10_000); // hard cap 10 s
    assert_eq!(cache.sweep_lazy_reclamation(32, t0 + 1_000 + 9_000).0, 0, "inside cap");
    assert_eq!(cache.sweep_lazy_reclamation(32, t0 + 1_000 + 11_000).0, 1, "beyond hard cap despite huge SWR");
}

#[test]
fn h2_sweeper_still_reclaims_epoch_stale_entries_inside_window() {
    let (cache, _k, t0) = sweep_fixture(meta(600_000, false), 1_000);
    cache.try_invalidate_tenant("t").unwrap();
    assert_eq!(cache.sweep_lazy_reclamation(32, t0 + 500).0, 1, "invalidated entries are reclaimed regardless of window");
}

// ---- M-a: a validator-mismatch 304 must not be relayed to coalesced waiters as a 304.
#[tokio::test]
async fn ma_mismatch_304_gives_waiters_502_not_304() {
    let o = spawn_raw_origin(
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-if-error=600\r\n",
        "ETag: \"v2\"\r\nCache-Control: max-age=300\r\n",
        600,
    )
    .await;
    let (gw, _cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/ma", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;
    let (r1, r2) = tokio::join!(c.get(&url).send(), c.get(&url).send());
    let (s1, s2) = (r1.unwrap().status().as_u16(), r2.unwrap().status().as_u16());
    assert!(s1 != 304 && s2 != 304, "waiters got a 304 they never asked for: {} {}", s1, s2);
    assert_eq!((s1, s2), (502, 502), "leader and waiter both get 502");
    assert_eq!(o.later.load(Ordering::SeqCst), 1, "the two requests were coalesced into one origin fetch");
}

// ---- L-1: a zero sweeper interval must not kill the sweeper task (tokio::time::interval panics).
#[tokio::test]
async fn l1_zero_sweeper_interval_is_clamped_and_task_survives() {
    let cache = new_cache();
    cache.set_sweeper_interval_ms(0);
    assert!(cache.get_sweeper_interval_ms() >= 1);
    let (tx, rx) = tokio::sync::watch::channel(false);
    let h = PartitionedCache::spawn_sweeper_task(cache.clone(), rx);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(!h.is_finished(), "sweeper task died (interval(0) panic)");
    drop(tx);
    h.abort();
}

// ---- H1: a background 304 must act on the state resident AFTER registration, never resurrect a
// purged entry from a copy captured before it.

fn h304(cc: Option<&str>, etag: Option<&str>) -> Vec<(Option<axum::http::header::HeaderName>, axum::http::HeaderValue)> {
    let mut v = Vec::new();
    if let Some(c) = cc {
        v.push((Some(axum::http::header::CACHE_CONTROL), axum::http::HeaderValue::from_str(c).unwrap()));
    }
    if let Some(e) = etag {
        v.push((Some(axum::http::header::ETAG), axum::http::HeaderValue::from_str(e).unwrap()));
    }
    v
}

fn purge_fixture() -> (Arc<PartitionedCache>, CacheKey, CachedResource, u64) {
    let cache = one_shard_cache();
    let key = CacheKey::new(Some("t"), "/h1", None);
    let t0 = 1_000_000u64;
    let mut a = res(b"purged-body", t0);
    a.ttl_ms = 1_000;
    a.etag = Some("\"v1\"".to_string());
    fill(&cache, &key, a, meta(600_000, false), t0);
    let copy = cache.get_resource(&key, t0).expect("resident");
    (cache, key, copy, t0)
}

// DOCUMENTATION TEST (not regression coverage; it passes before and after the fix). Characterization of the OLD handler sequence (captured copy + unconditional Success): it
// resurrects purged content. Passes before and after; documents the hazard that H1 fixed.
#[tokio::test]
async fn doc_characterization_h1_legacy_sequence_resurrects_purged_entry() {
    let (cache, key, copy, t0) = purge_fixture();
    cache.delete(&key); // purge between spawn (copy captured) and registration
    let later = t0 + 5_000;
    let g = match cache.begin_fetch_atomic(&key, later) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    let applied = g.cache.refresh_entry_lifetime_for_guard(&g, later, 60_000, None, meta(0, false));
    assert!(!applied, "refresh is refused: nothing resident");
    let mut refreshed = copy;
    refreshed.created_at_ms = later;
    refreshed.ttl_ms = 60_000;
    if let Some(mut b) = g.complete(FinalizeOutcome::Success(Arc::new(refreshed))) {
        b.deliver();
    }
    assert!(cache.get_resource(&key, later + 1).is_some(), "old behaviour: purged body resurrected");
}

#[tokio::test]
async fn h1_complete_not_modified_after_delete_stores_nothing() {
    use phantom_cache_demo::core::cache::NotModifiedOutcome;
    let (cache, key, _copy, t0) = purge_fixture();
    cache.delete(&key);
    let later = t0 + 5_000;
    let g = match cache.begin_fetch_atomic(&key, later) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    let out = cache.complete_not_modified(g, &h304(Some("max-age=60"), Some("\"v1\"")), later, false);
    assert_eq!(out, NotModifiedOutcome::NoResident);
    assert!(cache.get_resource(&key, later + 1).is_none(), "purged entry must stay purged");
    assert_eq!(cache.counters.revalidation_not_applied.load(Ordering::Relaxed), 1);
    assert_eq!(cache.counters.revalidation_not_modified.load(Ordering::Relaxed), 0);
    let (a, r) = cache.audit_bytes();
    assert_eq!(a, r);
}

#[tokio::test]
async fn h1_complete_not_modified_after_tenant_invalidation_stores_nothing() {
    use phantom_cache_demo::core::cache::NotModifiedOutcome;
    let (cache, key, _copy, t0) = purge_fixture();
    let later = t0 + 5_000;
    let g = match cache.begin_fetch_atomic(&key, later) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    cache.try_invalidate_tenant("t").unwrap();
    let out = cache.complete_not_modified(g, &h304(Some("max-age=60"), Some("\"v1\"")), later, false);
    assert_eq!(out, NotModifiedOutcome::NoResident);
    assert!(cache.get_resource(&key, later + 1).is_none(), "invalidated entry must not be revived");
}

#[tokio::test]
async fn h1_complete_not_modified_refreshes_live_entry() {
    use phantom_cache_demo::core::cache::NotModifiedOutcome;
    let (cache, key, _copy, t0) = purge_fixture();
    let later = t0 + 5_000;
    let g = match cache.begin_fetch_atomic(&key, later) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    let out = cache.complete_not_modified(g, &h304(Some("max-age=60"), Some("\"v1\"")), later, false);
    assert_eq!(out, NotModifiedOutcome::Refreshed);
    let e = cache.get_resource(&key, later + 1).expect("refreshed");
    assert_eq!(e.ttl_ms, 60_000);
    assert_eq!(cache.counters.revalidation_not_modified.load(Ordering::Relaxed), 1);
}

// ---- L-4: the ETag the refresh adopted is what finalize stores; a skipped ETag is not re-added
// (which would make finalize evict neighbours to fit it).
fn l4_fixture() -> (Arc<PartitionedCache>, CacheKey, CacheKey, CacheKey, usize, u64) {
    let cache = one_shard_cache();
    let t0 = 1_000_000u64;
    let (a, b, c) = (
        CacheKey::new(Some("t"), "/l4a", None),
        CacheKey::new(Some("t"), "/l4b", None),
        CacheKey::new(Some("t"), "/l4c", None),
    );
    for k in [&b, &c, &a] {
        let mut r = res(b"body", t0);
        r.ttl_ms = if *k == a { 1_000 } else { 100_000_000 };
        fill(&cache, k, r, meta(0, false), t0);
    }
    // A is the only one we revalidate; make it expired-eligible by registering at a later time.
    let (_, recorded) = cache.audit_bytes();
    cache.set_byte_budget(recorded + 5); // headroom (5 B) < the long ETag (45 B)
    (cache, a, b, c, recorded + 5, t0)
}

#[tokio::test]
async fn l4_budget_skipped_etag_is_not_readded_by_finalize_and_neighbours_survive() {
    use phantom_cache_demo::core::cache::NotModifiedOutcome;
    let (cache, a, b, c, budget, t0) = l4_fixture();
    let g = match cache.begin_fetch_atomic(&a, t0 + 10_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!("A expired, expected Initiate"),
    };
    let evictions_before = cache.counters.evictions_total.load(Ordering::Relaxed);
    let long = "\"a-considerably-longer-etag-value-0123456789\"";
    let out = cache.complete_not_modified(g, &h304(Some("max-age=1000"), Some(long)), t0 + 10_000, false);
    assert_eq!(out, NotModifiedOutcome::Refreshed);
    assert_eq!(cache.counters.evictions_total.load(Ordering::Relaxed), evictions_before, "no neighbour may be evicted");
    let now = t0 + 10_001;
    assert!(cache.get_resource(&b, now).is_some() && cache.get_resource(&c, now).is_some(), "neighbours survive");
    assert_eq!(cache.get_resource(&a, now).expect("A resident").etag, None, "skipped ETag stays skipped");
    let (acc, rec) = cache.audit_bytes();
    assert_eq!(acc, rec);
    assert!(rec <= budget, "{} > budget {}", rec, budget);
}

// DOCUMENTATION TEST (not regression coverage; passes before and after the fix). Characterization of the OLD sequence (finalize with the merged long ETag): neighbours evicted.
#[tokio::test]
async fn doc_characterization_l4_legacy_sequence_evicts_neighbours() {
    let (cache, a, _b, _c, _budget, t0) = l4_fixture();
    let g = match cache.begin_fetch_atomic(&a, t0 + 10_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    let long = "\"a-considerably-longer-etag-value-0123456789\"".to_string();
    let now = t0 + 10_000;
    assert!(g.cache.refresh_entry_lifetime_for_guard(&g, now, 1_000_000, Some(long.clone()), meta(0, false)));
    let mut merged = res(b"body", now);
    merged.ttl_ms = 1_000_000;
    merged.etag = Some(long);
    if let Some(mut bt) = g.complete(FinalizeOutcome::Success(Arc::new(merged))) {
        bt.deliver();
    }
    assert!(cache.counters.evictions_total.load(Ordering::Relaxed) > 0 || cache.get_resource(&a, now + 1).is_none(),
        "old behaviour: finalize had to make room for the merged ETag");
}

// ---- M-b: counter semantics.
#[tokio::test]
async fn mb_pending_metadata_dropped_on_rejected_store_and_refresh_not_applied_on_epoch() {
    let cache = one_shard_cache();
    let key = CacheKey::new(Some("t"), "/mb", None);
    let now = 1_000_000u64;
    let mut v1 = res(b"v1", now);
    v1.ttl_ms = 1_000;
    fill(&cache, &key, v1, meta(600_000, true), now);
    let (_, recorded) = cache.audit_bytes();
    cache.set_byte_budget(recorded + 100);
    let g = match cache.begin_fetch_atomic(&key, now + 5_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    assert!(g.cache.update_freshness_for_guard(&g, meta(1, false)));
    if let Some(mut b) = g.complete(FinalizeOutcome::Success(Arc::new(res(&vec![1u8; 10_000], now + 5_000)))) {
        b.deliver();
    }
    assert_eq!(cache.counters.pending_metadata_dropped.load(Ordering::Relaxed), 1, "rejected store drops parked metadata");
    assert_eq!(cache.counters.metadata_setter_rejections.load(Ordering::Relaxed), 0, "the fence did not refuse anything");

    // Accepted store consumes the parked metadata: not counted as dropped.
    let k2 = CacheKey::new(Some("t"), "/mb2", None);
    fill(&cache, &k2, res(b"x", now), meta(0, false), now);
    assert_eq!(cache.counters.pending_metadata_dropped.load(Ordering::Relaxed), 1);

    // Fence passes but the entry is epoch-stale: counted separately from fence refusals.
    let g2 = match cache.begin_fetch_atomic(&key, now + 6_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    cache.try_invalidate_tenant("t").unwrap();
    assert!(!g2.cache.refresh_entry_lifetime_for_guard(&g2, now, 1, None, meta(0, false)));
    assert_eq!(cache.counters.metadata_refresh_not_applied.load(Ordering::Relaxed), 1);
    assert_eq!(cache.counters.metadata_setter_rejections.load(Ordering::Relaxed), 0);
    drop(g2);
}

// revalidation_started counts real leaders only; others are revalidation_not_initiated, and the
// background slot is reserved before spawning so the bound holds (non-deterministic under load).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn mb_concurrent_stale_requests_one_leader_bounded_background_slots() {
    let o = spawn_raw_origin(
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        600,
    )
    .await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    cache.set_max_background_refresh(3);
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/mb-slots", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;

    let peak = Arc::new(AtomicU64::new(0));
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (cc, pk, st) = (cache.clone(), peak.clone(), stop.clone());
    let sampler = tokio::spawn(async move {
        while !st.load(Ordering::Relaxed) {
            pk.fetch_max(cc.counters.background_refresh_in_flight.load(Ordering::SeqCst), Ordering::SeqCst);
            tokio::task::yield_now().await;
        }
    });
    let hs: Vec<_> = (0..20)
        .map(|_| {
            let (c, u) = (c.clone(), url.clone());
            tokio::spawn(async move { xcache(&c.get(&u).send().await.unwrap()) })
        })
        .collect();
    for h in hs {
        assert_eq!(h.await.unwrap(), "STALE");
    }
    wait_for(&o.later, 1).await;
    tokio::time::sleep(Duration::from_millis(900)).await;
    stop.store(true, Ordering::Relaxed);
    let _ = sampler.await;
    assert!(peak.load(Ordering::SeqCst) <= 3, "background slots exceeded the bound: {}", peak.load(Ordering::SeqCst));
    assert_eq!(cache.counters.background_refresh_in_flight.load(Ordering::SeqCst), 0, "all slots released");
    assert_eq!(cache.counters.revalidation_started.load(Ordering::Relaxed), 1, "one single-flight leader");
    assert!(cache.counters.revalidation_not_initiated.load(Ordering::Relaxed) >= 1, "joiners are counted");
}

// ---- Third review round -------------------------------------------------------------------

type Waiter = tokio::sync::oneshot::Receiver<phantom_cache_demo::core::coalesce::CoalesceResult>;

/// Becomes the revalidation leader at `now` and registers one coalesced waiter, as server.rs does.
fn leader_with_waiter(
    cache: &Arc<PartitionedCache>,
    key: &CacheKey,
    now: u64,
) -> (phantom_cache_demo::core::coalesce::OriginFetchGuard, Waiter) {
    let g = match cache.begin_fetch_atomic(key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!("expected Initiate"),
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    g.shared_op.waiters.lock().unwrap().push(tx);
    (g, rx)
}

const H25: u64 = 25 * 3_600_000;

// M1: an uncacheable 304 must not hand out a body beyond the hard stale cap.
#[tokio::test]
async fn m1r_uncacheable_304_beyond_hard_cap_hands_out_no_body() {
    let (cache, key, t0) = {
        let (c, k, t0) = sweep_fixture(meta(10_000_000_000, false), 1_000);
        (c, k, t0)
    };
    let now = t0 + H25;
    let (g, rx) = leader_with_waiter(&cache, &key, now);
    let out = cache.complete_not_modified(g, &h304(Some("no-store"), Some("\"v1\"")), now, false);
    assert_eq!(out, phantom_cache_demo::core::cache::NotModifiedOutcome::Uncacheable);
    assert!(rx.await.unwrap().is_err(), "25 h old entry (cap 24 h) must not be served to waiters");
    assert_eq!(cache.counters.stale_served_total.load(Ordering::Relaxed), 0);
    assert_eq!(cache.counters.stale_cap_violations.load(Ordering::Relaxed), 0);
}

// M1: must-revalidate forbids stale serving even through the uncacheable-304 path.
#[tokio::test]
async fn m1r_uncacheable_304_for_must_revalidate_entry_hands_out_no_body() {
    let (cache, key, t0) = sweep_fixture(meta(600_000, true), 1_000);
    let now = t0 + 5_000;
    let (g, rx) = leader_with_waiter(&cache, &key, now);
    cache.complete_not_modified(g, &h304(Some("no-store"), Some("\"v1\"")), now, false);
    assert!(rx.await.unwrap().is_err(), "must-revalidate entry must not be served stale");
}

// M1: inside the SIE/SWR window the body is still handed out, and counted as stale served.
#[tokio::test]
async fn m1r_uncacheable_304_inside_window_serves_and_counts_stale() {
    let (cache, key, t0) = sweep_fixture(meta(600_000, false), 1_000);
    let now = t0 + 5_000;
    let (g, rx) = leader_with_waiter(&cache, &key, now);
    cache.complete_not_modified(g, &h304(Some("no-store"), Some("\"v1\"")), now, false);
    let r = rx.await.unwrap().expect("inside window: still served");
    assert_eq!(&*r.body, b"body");
    assert_eq!(cache.counters.stale_served_total.load(Ordering::Relaxed), 1);
    assert_eq!(cache.counters.stale_cap_violations.load(Ordering::Relaxed), 0);
}

// M2: a stale leader's validator-mismatch 304 must not delete a NEWER fill.
#[tokio::test]
async fn m2r_mismatch_304_from_overtaken_leader_does_not_delete_newer_fill() {
    let cache = one_shard_cache();
    let key = CacheKey::new(Some("t"), "/m2r", None);
    let t0 = 1_000_000u64;
    let mut v1 = res(b"v1", t0);
    v1.ttl_ms = 1_000;
    v1.etag = Some("\"v1\"".to_string());
    fill(&cache, &key, v1, meta(600_000, false), t0);

    let a = match cache.begin_fetch_atomic(&key, t0 + 5_000) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!(),
    };
    cache.delete(&key); // DELETE overtakes leader A
    let mut v2 = res(b"v2", t0 + 6_000);
    v2.etag = Some("\"v2\"".to_string());
    fill(&cache, &key, v2, meta(600_000, false), t0 + 6_000); // GET B refills E2

    cache.complete_not_modified(a, &h304(Some("max-age=60"), Some("\"v1\"")), t0 + 7_000, false);
    let e = cache.get_resource(&key, t0 + 7_000).expect("newer fill must survive the overtaken leader's mismatch");
    assert_eq!(&*e.body, b"v2");
}

// M3 / L1: background-slot reservation. Hammered from OS threads against bound 3.
#[test]
fn m3_background_slot_reservation_never_exceeds_bound_under_thread_hammer() {
    let cache = one_shard_cache();
    let peak = Arc::new(AtomicU64::new(0));
    let hs: Vec<_> = (0..8)
        .map(|_| {
            let (c, p) = (cache.clone(), peak.clone());
            std::thread::spawn(move || {
                for _ in 0..20_000 {
                    if let Some(slot) = c.try_reserve_background_slot(3) {
                        p.fetch_max(c.counters.background_refresh_in_flight.load(Ordering::SeqCst), Ordering::SeqCst);
                        std::hint::spin_loop();
                        drop(slot);
                    }
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    assert!(peak.load(Ordering::SeqCst) <= 3, "peak in-flight {}", peak.load(Ordering::SeqCst));
    assert_eq!(cache.counters.background_refresh_in_flight.load(Ordering::SeqCst), 0);
}

// Demonstration (ignored by default; non-deterministic) that the hammer discriminates: the OLD
// check-then-increment logic exceeds the bound. Run with --ignored to see it.
#[test]
#[ignore = "demonstration of the old check-then-increment race; non-deterministic"]
fn m3_demo_old_check_then_increment_exceeds_bound() {
    let ctr = Arc::new(AtomicU64::new(0));
    let peak = Arc::new(AtomicU64::new(0));
    let hs: Vec<_> = (0..8)
        .map(|_| {
            let (c, p) = (ctr.clone(), peak.clone());
            std::thread::spawn(move || {
                for _ in 0..50_000 {
                    if c.load(Ordering::Relaxed) < 3 {
                        let now = c.fetch_add(1, Ordering::Relaxed) + 1;
                        p.fetch_max(now, Ordering::SeqCst);
                        c.fetch_sub(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    assert!(peak.load(Ordering::SeqCst) > 3, "old logic did not exceed the bound in this run");
}

// L1: the slot is released even if the holder panics.
#[test]
fn l1r_background_slot_released_on_panic() {
    let cache = one_shard_cache();
    let c = cache.clone();
    let r = std::thread::spawn(move || {
        let _slot = c.try_reserve_background_slot(3).expect("slot");
        assert_eq!(c.counters.background_refresh_in_flight.load(Ordering::SeqCst), 1);
        panic!("injected panic while holding the background slot");
    })
    .join();
    assert!(r.is_err());
    assert_eq!(cache.counters.background_refresh_in_flight.load(Ordering::SeqCst), 0, "slot leaked by the panic");
}

// L2: u64::MAX ttl must neither panic (debug overflow) nor wrap into "expired".
#[tokio::test]
async fn l2_max_ttl_entry_does_not_overflow_or_wrap() {
    let cache = one_shard_cache();
    let key = CacheKey::new(Some("t"), "/l2", None);
    let t0 = 1_000_000u64;
    let mut r = res(b"body", t0);
    r.ttl_ms = u64::MAX;
    fill(&cache, &key, r, meta(0, false), t0);
    assert!(cache.get_resource(&key, t0 + 10).is_some(), "not treated as expired by wrap");
    assert!(cache.get_resource_with_metadata(&key, t0 + 10).is_some());
    assert_eq!(cache.sweep_lazy_reclamation(32, t0 + 10).0, 0);
}

#[tokio::test]
async fn l2_http_304_with_max_age_u64_max_then_get_is_a_hit() {
    let o = spawn_raw_origin(
        "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n",
        "ETag: \"v1\"\r\nCache-Control: max-age=18446744073709551615\r\n",
        0,
    )
    .await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/l2h", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_for(&o.later, 1).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let r = c.get(&url).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(xcache(&r), "HIT", "huge max-age must not wrap into expired");
    assert_eq!(cache.counters.revalidation_not_modified.load(Ordering::Relaxed), 1);
}

// L4: a zero interval written straight into the pub field must not kill the sweeper either.
#[tokio::test]
async fn l4r_sweeper_clamps_interval_read_from_pub_field() {
    let cache = new_cache();
    cache.sweeper_interval_ms.store(0, Ordering::Relaxed);
    let (tx, rx) = tokio::sync::watch::channel(false);
    let h = PartitionedCache::spawn_sweeper_task(cache.clone(), rx);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(!h.is_finished(), "sweeper died on interval(0)");
    drop(tx);
    h.abort();
}

// L5: record_revalidation_failure / is_revalidation_due must recover a poisoned shard.
#[test]
fn l5_revalidation_backoff_recovers_poisoned_shard() {
    let (cache, key, t0) = sweep_fixture(meta(600_000, false), 1_000);
    let c = cache.clone();
    let _ = std::thread::spawn(move || {
        let _g = c.shards[0].lock().unwrap();
        panic!("poison the shard");
    })
    .join();
    assert!(cache.shards[0].is_poisoned());
    cache.record_revalidation_failure(&key, t0 + 10_000);
    assert!(!cache.is_revalidation_due(&key, t0 + 10_001), "backoff must have been recorded despite the poison");
}

// Soak-style: many entries retained inside stale windows by the sweeper stay within byte_budget,
// then are fully reclaimed once the windows (and the hard cap) have passed.
#[test]
fn soak_entries_retained_in_stale_windows_stay_within_byte_budget() {
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(
        4,
        Arc::new(CacheCounters::new()),
        Arc::new(DiagnosticBuffer::new(64, Arc::new(CacheCounters::new()))),
    ));
    let budget = 400_000usize;
    cache.set_byte_budget(budget);
    let t0 = 1_000_000u64;
    for i in 0..5_000u32 {
        let key = CacheKey::new(Some("t"), &format!("/soak/{}", i), None);
        let mut r = res(&vec![b'x'; 256], t0);
        r.ttl_ms = 1_000;
        let g = match cache.begin_fetch_atomic(&key, t0) {
            FetchRegistrationResult::Initiate(r) => r.into_guard(),
            _ => panic!(),
        };
        g.cache.update_freshness_for_guard(&g, meta(600_000, false));
        if let Some(mut b) = g.complete(FinalizeOutcome::Success(Arc::new(r))) {
            b.deliver();
        }
        let (acc, rec) = cache.audit_bytes();
        assert_eq!(acc, rec);
        assert!(rec <= budget, "resident {} > budget {} after insert {}", rec, budget, i);
    }
    let before = cache.audit_bytes().1;
    assert!(before > 0);
    // Inside the SWR window: sweep until stable; everything retained stays accounted and bounded.
    for _ in 0..200 {
        cache.sweep_lazy_reclamation(32, t0 + 10_000);
    }
    let (acc, rec) = cache.audit_bytes();
    assert_eq!(acc, rec);
    assert_eq!(rec, before, "nothing inside its window may be reclaimed");
    assert!(rec <= budget);
    // Beyond ttl + window: all reclaimed.
    for _ in 0..400 {
        cache.sweep_lazy_reclamation(32, t0 + 1_000 + 600_000 + 1);
    }
    let (acc, rec) = cache.audit_bytes();
    assert_eq!((acc, rec), (0, 0), "everything beyond the window is reclaimed");
}


// =============================================================================================
// Validator forwarding (S-1 defect 3, owner-approved implementation): REAL conditional exchanges.
//
// The scripted origin records every request (headers lower-cased) and answers by a script that
// behaves like a correct origin: a request whose If-None-Match / If-Modified-Since matches gets a
// 304, anything else gets the full 200. A gateway that does not forward the STORED validators
// therefore re-downloads the body instead of revalidating, and these tests fail.
// Timing is wall-clock, loopback, debug profile: behavioural tests, not measurements.
// =============================================================================================

const LM: &str = "Sun, 06 Nov 1994 08:49:37 GMT";

#[derive(Clone, Debug)]
struct Req {
    target: String,
    headers: Vec<(String, String)>,
}

impl Req {
    fn h(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }
    fn has(&self, name: &str) -> bool {
        self.h(name).is_some()
    }
    /// Names of all request headers that make a request conditional or ranged.
    fn conditional_names(&self) -> Vec<String> {
        self.headers
            .iter()
            .filter(|(k, _)| {
                matches!(
                    k.as_str(),
                    "if-none-match" | "if-modified-since" | "if-match" | "if-unmodified-since" | "if-range" | "range"
                )
            })
            .map(|(k, _)| k.clone())
            .collect()
    }
}

struct Reply {
    raw: String,
    delay_ms: u64,
}

fn ok200(body: &str, extra: &str) -> Reply {
    Reply {
        raw: format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n{}Connection: close\r\nContent-Length: {}\r\n\r\n{}",
            extra,
            body.len(),
            body
        ),
        delay_ms: 0,
    }
}

fn r304(extra: &str) -> Reply {
    Reply { raw: format!("HTTP/1.1 304 Not Modified\r\n{}Connection: close\r\nContent-Length: 0\r\n\r\n", extra), delay_ms: 0 }
}

fn r500() -> Reply {
    Reply { raw: "HTTP/1.1 500 Internal Server Error\r\nConnection: close\r\nContent-Length: 0\r\n\r\n".to_string(), delay_ms: 0 }
}

fn delayed(mut r: Reply, ms: u64) -> Reply {
    r.delay_ms = ms;
    r
}

type Script = Arc<dyn Fn(usize, &Req) -> Reply + Send + Sync>;

struct Scripted {
    addr: std::net::SocketAddr,
    reqs: Arc<Mutex<Vec<Req>>>,
}

impl Scripted {
    fn count(&self) -> usize {
        self.reqs.lock().unwrap().len()
    }
    fn req(&self, i: usize) -> Req {
        self.reqs.lock().unwrap()[i].clone()
    }
    fn conditional_count(&self) -> usize {
        self.reqs.lock().unwrap().iter().filter(|r| r.has("if-none-match") || r.has("if-modified-since")).count()
    }
}

async fn spawn_scripted(script: Script) -> Scripted {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let reqs: Arc<Mutex<Vec<Req>>> = Arc::new(Mutex::new(Vec::new()));
    let r2 = reqs.clone();
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = match listener.accept().await {
                Ok(x) => x,
                Err(_) => break,
            };
            let (reqs, script) = (r2.clone(), script.clone());
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 4096];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    match sock.read(&mut tmp).await {
                        Ok(0) | Err(_) => break,
                        Ok(n) => buf.extend_from_slice(&tmp[..n]),
                    }
                }
                let text = String::from_utf8_lossy(&buf).to_string();
                let mut lines = text.split("\r\n");
                let target = lines.next().unwrap_or("").to_string();
                let headers = lines
                    .take_while(|l| !l.is_empty())
                    .filter_map(|l| l.split_once(':').map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string())))
                    .collect();
                let req = Req { target, headers };
                let idx = {
                    let mut g = reqs.lock().unwrap();
                    g.push(req.clone());
                    g.len() - 1
                };
                let reply = script(idx, &req);
                if reply.delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(reply.delay_ms)).await;
                }
                let _ = sock.write_all(reply.raw.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    Scripted { addr, reqs }
}

async fn wait_count(o: &Scripted, at_least: usize) {
    for _ in 0..200 {
        if o.count() >= at_least {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("origin saw {} requests, expected at least {}", o.count(), at_least);
}

async fn body_of(r: reqwest::Response) -> String {
    r.text().await.unwrap()
}

fn cnt(c: &AtomicU64) -> u64 {
    c.load(Ordering::SeqCst)
}

const BG_CC: &str = "max-age=1, stale-while-revalidate=60";
const FG_CC: &str = "max-age=1, stale-if-error=60";

/// Correct-origin script: request 0 = 200 v1 with `first_headers`; later requests get a 304 with
/// `reval_headers` iff `matches(req)`, else a 200 v2 (new ETag "v2", max-age=300).
fn validating_origin(
    first_headers: String,
    reval_headers: String,
    matches: fn(&Req) -> bool,
    delay_ms: u64,
) -> Script {
    Arc::new(move |i, req| {
        if i == 0 {
            ok200("{\"v\":1}", &first_headers)
        } else if matches(req) {
            delayed(r304(&reval_headers), delay_ms)
        } else {
            delayed(ok200("{\"v\":2}", "ETag: \"v2\"\r\nCache-Control: max-age=300\r\n"), delay_ms)
        }
    })
}

fn etag_v1(req: &Req) -> bool {
    req.h("if-none-match") == Some("\"v1\"")
}

fn prime_headers(cc: &str) -> String {
    format!("ETag: \"v1\"\r\nLast-Modified: {}\r\nCache-Control: {}\r\n", LM, cc)
}

const REVAL_BG: &str = "ETag: \"v1\"\r\nCache-Control: max-age=300, stale-while-revalidate=60\r\n";
const REVAL_FG: &str = "ETag: \"v1\"\r\nCache-Control: max-age=300, stale-if-error=60\r\n";

async fn miss_then_expire(c: &reqwest::Client, url: &str) {
    let r = c.get(url).send().await.unwrap();
    assert_eq!(xcache(&r), "MISS");
    tokio::time::sleep(Duration::from_millis(1400)).await;
}

// ---- ETag match: 304 refreshes freshness, entry stays, body not re-downloaded -----------------
#[tokio::test]
async fn s1_background_conditional_revalidation_etag_match_refreshes_entry() {
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), REVAL_BG.into(), etag_v1, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-bg-etag", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(400)).await;

    let rv = o.req(1);
    assert_eq!(rv.target, "GET /vf-bg-etag HTTP/1.1", "same resource, same path");
    assert_eq!(rv.h("if-none-match"), Some("\"v1\""), "STORED ETag must be sent upstream");
    assert_eq!(rv.h("if-modified-since"), Some(LM), "STORED Last-Modified must be sent upstream");
    let k = &cache.counters;
    assert_eq!(cnt(&k.revalidation_conditional_sent), 1, "measured: one conditional dispatch");
    assert_eq!(cnt(&k.revalidation_304_received), 1, "measured: one 304 to it");
    assert_eq!(cnt(&k.revalidation_not_modified), 1);
    assert_eq!(cnt(&k.revalidation_ok), 0, "a 304 is not a full-body refresh");

    let r3 = c.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r3), "HIT", "entry stays and is fresh again");
    assert!(age(&r3) <= 1, "Age reset by the 304, got {}", age(&r3));
    assert_eq!(body_of(r3).await, "{\"v\":1}", "the stored body is served, never re-downloaded");
    assert_eq!(o.count(), 2, "exactly the initial fetch + one conditional fetch");
}

#[tokio::test]
async fn s1_foreground_conditional_revalidation_etag_match_refreshes_entry() {
    let o = spawn_scripted(validating_origin(prime_headers(FG_CC), REVAL_FG.into(), etag_v1, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-fg-etag", gw);
    miss_then_expire(&c, &url).await;
    let r2 = c.get(&url).send().await.unwrap();
    assert_eq!(r2.status().as_u16(), 200);
    assert_eq!(body_of(r2).await, "{\"v\":1}");
    let rv = o.req(1);
    assert_eq!(rv.h("if-none-match"), Some("\"v1\""));
    assert_eq!(rv.h("if-modified-since"), Some(LM));
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 1);
    assert_eq!(cnt(&cache.counters.revalidation_304_received), 1);
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let r3 = c.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r3), "HIT");
    assert!(age(&r3) <= 1);
    assert_eq!(o.count(), 2);
}

// ---- ETag changed: the origin answers 200 with the new body; normal fenced replace -------------
#[tokio::test]
async fn s1_background_conditional_revalidation_etag_changed_origin_200_replaces_entry() {
    // The origin has moved on: v1 no longer matches.
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), String::new(), |_| false, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-bg-changed", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(o.req(1).h("if-none-match"), Some("\"v1\""), "the conditional request was sent");
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 1);
    assert_eq!(cnt(&cache.counters.revalidation_304_received), 0);
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 0);
    assert_eq!(cnt(&cache.counters.revalidation_ok), 1, "a 200 to a conditional fetch is a normal refresh");
    let r3 = c.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r3), "HIT");
    assert_eq!(r3.headers().get("etag").and_then(|v| v.to_str().ok()), Some("\"v2\""));
    assert_eq!(body_of(r3).await, "{\"v\":2}", "entry replaced by the new representation");
}

// ---- Last-Modified only -------------------------------------------------------------------------
#[tokio::test]
async fn s1_background_conditional_revalidation_last_modified_only() {
    let first = format!("Last-Modified: {}\r\nCache-Control: {}\r\n", LM, BG_CC);
    let o = spawn_scripted(validating_origin(first, "Cache-Control: max-age=300, stale-while-revalidate=60\r\n".into(), |r| r.h("if-modified-since") == Some(LM), 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-lm-only", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    let rv = o.req(1);
    assert_eq!(rv.h("if-modified-since"), Some(LM));
    assert!(!rv.has("if-none-match"), "no ETag stored: no If-None-Match");
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1);
    let r3 = c.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r3), "HIT");
    assert_eq!(body_of(r3).await, "{\"v\":1}");
}

// ---- No validators stored: unconditional fetch --------------------------------------------------
#[tokio::test]
async fn s1_background_revalidation_without_stored_validators_is_unconditional() {
    let first = format!("Cache-Control: {}\r\n", BG_CC);
    let o = spawn_scripted(validating_origin(first, String::new(), |_| false, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-none", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(o.req(1).conditional_names().is_empty(), "nothing to validate with: {:?}", o.req(1).headers);
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 0);
    let r3 = c.get(&url).send().await.unwrap();
    assert_eq!(body_of(r3).await, "{\"v\":2}");
}

// ---- Strong and weak ETags ----------------------------------------------------------------------
async fn etag_roundtrip(etag: &'static str, path: &str) {
    let first = format!("ETag: {}\r\nCache-Control: {}\r\n", etag, BG_CC);
    let reval = format!("ETag: {}\r\nCache-Control: max-age=300, stale-while-revalidate=60\r\n", etag);
    let script: Script = Arc::new(move |i, req| {
        if i == 0 {
            ok200("{\"v\":1}", &first)
        } else if req.h("if-none-match") == Some(etag) {
            r304(&reval)
        } else {
            ok200("{\"v\":2}", "ETag: \"other\"\r\nCache-Control: max-age=300\r\n")
        }
    });
    let o = spawn_scripted(script).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/{}", gw, path);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(o.req(1).h("if-none-match"), Some(etag), "stored ETag forwarded verbatim (weak prefix and quotes kept)");
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1);
    let r3 = c.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r3), "HIT");
    assert_eq!(body_of(r3).await, "{\"v\":1}");
}

#[tokio::test]
async fn s1_conditional_revalidation_strong_etag_forwarded_verbatim() {
    etag_roundtrip("\"strong-1\"", "vf-strong").await;
}

#[tokio::test]
async fn s1_conditional_revalidation_weak_etag_forwarded_verbatim() {
    etag_roundtrip("W/\"weak-1\"", "vf-weak").await;
}

// ---- Real-exchange versions of the former m2 "unsolicited 304" tests ----------------------------
// A 304 without Cache-Control keeps the stored SWR / Last-Modified / lifetime.
#[tokio::test]
async fn m2_bg_conditional_304_without_cache_control_keeps_stored_metadata() {
    let first = format!("ETag: \"v1\"\r\nLast-Modified: {}\r\nCache-Control: max-age=1, stale-while-revalidate=600\r\n", LM);
    let o = spawn_scripted(validating_origin(first, "ETag: \"v1\"\r\n".into(), etag_v1, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-m2-bg", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(o.req(1).h("if-none-match"), Some("\"v1\""));
    let f = cache.get_freshness(&dkey("/vf-m2-bg")).expect("freshness kept");
    assert_eq!(f.stale_while_revalidate_ms, Some(600_000), "SWR wiped by 304: {:?}", f);
    assert!(f.last_modified.is_some(), "Last-Modified wiped: {:?}", f);
    let e = cache.get_resource(&dkey("/vf-m2-bg"), DiagnosticBuffer::now_ms()).expect("refreshed");
    assert_eq!(e.ttl_ms, 1_000, "stored lifetime retained when the 304 carries no freshness headers");
}

#[tokio::test]
async fn m2_fg_conditional_304_without_cache_control_keeps_stored_metadata() {
    let first = "ETag: \"v1\"\r\nCache-Control: max-age=1, stale-if-error=600\r\n".to_string();
    let o = spawn_scripted(validating_origin(first, "ETag: \"v1\"\r\n".into(), etag_v1, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-m2-fg", gw);
    miss_then_expire(&c, &url).await;
    let _ = c.get(&url).send().await.unwrap();
    assert_eq!(o.req(1).h("if-none-match"), Some("\"v1\""));
    tokio::time::sleep(Duration::from_millis(300)).await;
    let f = cache.get_freshness(&dkey("/vf-m2-fg")).expect("freshness kept");
    assert_eq!(f.stale_if_error_ms, Some(600_000), "SIE wiped by 304: {:?}", f);
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1);
}

// ---- Parser: a 304 may carry the representation's Content-Length but never a body ----------------
#[tokio::test]
async fn s1_conditional_304_with_representation_content_length_is_accepted() {
    let script: Script = Arc::new(|i, req| {
        if i == 0 {
            ok200("{\"v\":1}", &prime_headers(BG_CC))
        } else if etag_v1(req) {
            // RFC 9110 15.4.5: a 304 may repeat Content-Length of the 200 it stands for; no body follows.
            Reply { raw: "HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nCache-Control: max-age=300, stale-while-revalidate=60\r\nContent-Length: 7\r\nConnection: close\r\n\r\n".to_string(), delay_ms: 0 }
        } else {
            ok200("{\"v\":2}", "ETag: \"v2\"\r\n")
        }
    });
    let o = spawn_scripted(script).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-304-cl", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1, "304 with Content-Length must be parsed as bodyless");
    assert_eq!(cnt(&cache.counters.revalidation_error), 0);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "HIT");
}

// ---- Client conditional/range headers never reach the origin; STORED validators do ---------------
const CLIENT_CONDITIONALS: [(&str, &str); 6] = [
    ("If-None-Match", "\"client-etag\""),
    ("If-Modified-Since", "Mon, 01 Jan 2001 00:00:00 GMT"),
    ("If-Match", "\"client-match\""),
    ("If-Unmodified-Since", "Mon, 01 Jan 2001 00:00:00 GMT"),
    ("If-Range", "\"client-range\""),
    ("Range", "bytes=0-3"),
];

fn with_client_conditionals(mut rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    for (k, v) in CLIENT_CONDITIONALS {
        rb = rb.header(k, v);
    }
    rb
}

fn assert_only_stored_validators(r: &Req, ctx: &str) {
    let mut names = r.conditional_names();
    names.sort();
    assert_eq!(
        names,
        vec!["if-modified-since".to_string(), "if-none-match".to_string()],
        "{ctx}: only the STORED validators may be sent, got {:?}",
        r.headers
    );
    assert_eq!(r.h("if-none-match"), Some("\"v1\""), "{ctx}: If-None-Match must be the STORED ETag");
    assert_eq!(r.h("if-modified-since"), Some(LM), "{ctx}: If-Modified-Since must be the STORED Last-Modified");
}

#[tokio::test]
async fn s1_client_conditional_headers_never_reach_origin_background() {
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), REVAL_BG.into(), etag_v1, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-strip-bg", gw);
    // (a) initial miss carrying all client conditionals: the origin must see none.
    let r1 = with_client_conditionals(c.get(&url)).send().await.unwrap();
    assert_eq!(xcache(&r1), "MISS");
    assert!(o.req(0).conditional_names().is_empty(), "miss: client conditionals leaked: {:?}", o.req(0).headers);
    tokio::time::sleep(Duration::from_millis(1400)).await;
    // (b) stale hit that triggers background revalidation, again with client conditionals.
    let r2 = with_client_conditionals(c.get(&url)).send().await.unwrap();
    assert_eq!(xcache(&r2), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_only_stored_validators(&o.req(1), "background revalidation");
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1, "the STORED validator got the 304, not the client's");
}

#[tokio::test]
async fn s1_client_conditional_headers_never_reach_origin_foreground() {
    let o = spawn_scripted(validating_origin(prime_headers(FG_CC), REVAL_FG.into(), etag_v1, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-strip-fg", gw);
    let _ = with_client_conditionals(c.get(&url)).send().await.unwrap();
    assert!(o.req(0).conditional_names().is_empty());
    tokio::time::sleep(Duration::from_millis(1400)).await;
    let r2 = with_client_conditionals(c.get(&url)).send().await.unwrap();
    assert_eq!(r2.status().as_u16(), 200, "foreground revalidation result is served as 200, not as the client's conditional answer");
    assert_only_stored_validators(&o.req(1), "foreground revalidation");
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1);
}

#[tokio::test]
async fn s1_bypass_requests_carry_no_validators_and_no_client_conditionals() {
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), String::new(), |_| false, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-bypass", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    let r = with_client_conditionals(c.get(&url).header("x-cache-bypass", "true")).send().await.unwrap();
    assert_eq!(xcache(&r), "BYPASS");
    assert!(o.req(1).conditional_names().is_empty(), "bypass must be a plain unconditional fetch: {:?}", o.req(1).headers);
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 0);
}

// ---- Client conditional GET to the gateway: answered from the cache ------------------------------
#[tokio::test]
async fn s1_client_if_none_match_matching_cached_etag_gets_304_from_cache_without_origin() {
    let o = spawn_scripted(validating_origin("ETag: \"v1\"\r\nCache-Control: max-age=60\r\n".to_string(), String::new(), |_| false, 0)).await;
    let (gw, _cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-client-304", gw);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "MISS");
    let m = c.get(&url).header("If-None-Match", "\"v1\"").send().await.unwrap();
    assert_eq!(m.status().as_u16(), 304, "client validator matches the cached entry's: 304 from cache");
    assert_eq!(xcache(&m), "HIT");
    assert_eq!(m.headers().get("etag").and_then(|v| v.to_str().ok()), Some("\"v1\""));
    let w = c.get(&url).header("If-None-Match", "W/\"v1\"").send().await.unwrap();
    assert_eq!(w.status().as_u16(), 304, "weak comparison for If-None-Match");
    let n = c.get(&url).header("If-None-Match", "\"other\"").send().await.unwrap();
    assert_eq!(n.status().as_u16(), 200, "non-matching client validator gets the full cached body");
    assert_eq!(body_of(n).await, "{\"v\":1}");
    assert_eq!(o.count(), 1, "all of that was served from the cache");
}

#[tokio::test]
async fn s1_client_if_none_match_on_stale_entry_gets_304_while_upstream_sees_only_stored_validator() {
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), REVAL_BG.into(), etag_v1, 0)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-client-stale", gw);
    miss_then_expire(&c, &url).await;
    let r = c.get(&url).header("If-None-Match", "\"v1\"").send().await.unwrap();
    assert_eq!(r.status().as_u16(), 304);
    assert_eq!(xcache(&r), "STALE");
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_only_stored_validators(&o.req(1), "revalidation triggered by a conditional client");
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1);
}

// ---- Single flight: concurrent stale requests -> exactly one conditional fetch ------------------
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s1_concurrent_stale_requests_produce_exactly_one_conditional_fetch_background() {
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), REVAL_BG.into(), etag_v1, 600)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-sf-bg", gw);
    miss_then_expire(&c, &url).await;
    let hs: Vec<_> = (0..20)
        .map(|_| {
            let (c, u) = (c.clone(), url.clone());
            tokio::spawn(async move { xcache(&c.get(&u).send().await.unwrap()) })
        })
        .collect();
    for h in hs {
        assert_eq!(h.await.unwrap(), "STALE");
    }
    wait_count(&o, 2).await;
    tokio::time::sleep(Duration::from_millis(900)).await;
    assert_eq!(o.count(), 2, "initial fetch + exactly ONE conditional fetch");
    assert_eq!(o.conditional_count(), 1);
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 1);
    assert_eq!(cnt(&cache.counters.revalidation_started), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn s1_concurrent_stale_requests_produce_exactly_one_conditional_fetch_foreground() {
    let o = spawn_scripted(validating_origin(prime_headers(FG_CC), REVAL_FG.into(), etag_v1, 600)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-sf-fg", gw);
    miss_then_expire(&c, &url).await;
    let hs: Vec<_> = (0..20)
        .map(|_| {
            let (c, u) = (c.clone(), url.clone());
            tokio::spawn(async move {
                let r = c.get(&u).send().await.unwrap();
                (r.status().as_u16(), r.text().await.unwrap())
            })
        })
        .collect();
    for h in hs {
        let (s, b) = h.await.unwrap();
        assert_eq!((s, b.as_str()), (200, "{\"v\":1}"), "leader and waiters all get the stored body");
    }
    assert_eq!(o.count(), 2, "initial fetch + exactly ONE conditional fetch");
    assert_eq!(o.conditional_count(), 1);
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 1);
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1);
}

// ---- DELETE during the conditional fetch: a late 304 / 200 cannot resurrect ----------------------
#[tokio::test]
async fn s1_delete_during_conditional_fetch_late_304_cannot_resurrect_entry() {
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), "ETag: \"v1\"\r\nCache-Control: max-age=300\r\n".into(), etag_v1, 700)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-del304", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(o.req(1).h("if-none-match"), Some("\"v1\""), "conditional fetch is in flight");
    cache.delete(&dkey("/vf-del304"));
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert_eq!(cnt(&cache.counters.revalidation_304_received), 1, "the 304 did arrive");
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 0, "but must not be applied");
    assert_eq!(cnt(&cache.counters.revalidation_not_applied), 1);
    assert!(cache.get_resource(&dkey("/vf-del304"), DiagnosticBuffer::now_ms()).is_none(), "DELETE wins");
    assert!(cache.get_freshness(&dkey("/vf-del304")).is_none(), "no orphan metadata");
    let (acc, rec) = cache.audit_bytes();
    assert_eq!(acc, rec);
}

#[tokio::test]
async fn s1_delete_during_conditional_fetch_late_200_cannot_resurrect_entry() {
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), String::new(), |_| false, 700)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-del200", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(o.req(1).h("if-none-match"), Some("\"v1\""), "conditional fetch is in flight");
    cache.delete(&dkey("/vf-del200"));
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert!(
        cache.get_resource(&dkey("/vf-del200"), DiagnosticBuffer::now_ms()).is_none(),
        "a 200 to a pre-DELETE conditional fetch must not be stored"
    );
    assert!(cache.get_freshness(&dkey("/vf-del200")).is_none());
    let (acc, rec) = cache.audit_bytes();
    assert_eq!(acc, rec);
}

#[tokio::test]
async fn s1_delete_during_foreground_conditional_fetch_late_304_cannot_resurrect_entry() {
    let o = spawn_scripted(validating_origin(prime_headers(FG_CC), REVAL_FG.into(), etag_v1, 700)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-fgdel", gw);
    miss_then_expire(&c, &url).await;
    let c2 = c.clone();
    let u2 = url.clone();
    let inflight = tokio::spawn(async move { c2.get(&u2).send().await.unwrap().status().as_u16() });
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(o.count(), 2);
    cache.delete(&dkey("/vf-fgdel"));
    let status = inflight.await.unwrap();
    assert_eq!(status, 502, "the late 304 has nothing to refresh: no body is invented for the in-flight request");
    assert!(cache.get_resource(&dkey("/vf-fgdel"), DiagnosticBuffer::now_ms()).is_none(), "DELETE wins");
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 0);
}

// ---- Failures during revalidation: stale-if-error behaviour unchanged, counters -------------------
#[tokio::test]
async fn s1_conditional_fetch_origin_error_serves_stale_if_error_and_counts() {
    let script: Script = Arc::new(|i, _| if i == 0 { ok200("{\"v\":1}", &prime_headers(FG_CC)) } else { r500() });
    let o = spawn_scripted(script).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-err", gw);
    miss_then_expire(&c, &url).await;
    let r = c.get(&url).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200, "stale-if-error: stale body instead of the 500");
    assert_eq!(body_of(r).await, "{\"v\":1}");
    assert_eq!(o.req(1).h("if-none-match"), Some("\"v1\""), "the failed fetch was conditional");
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 1);
    assert_eq!(cnt(&cache.counters.revalidation_304_received), 0);
    assert_eq!(cnt(&cache.counters.revalidation_error), 1);
    assert_eq!(cnt(&cache.counters.stale_served_total), 1);
    assert_eq!(cnt(&cache.counters.stale_cap_violations), 0);
}

#[tokio::test]
async fn s1_conditional_fetch_origin_timeout_serves_stale_if_error_and_counts() {
    // The origin never answers the conditional request in time (gateway read timeout is 5 s).
    let script: Script = Arc::new(|i, _| if i == 0 { ok200("{\"v\":1}", &prime_headers(FG_CC)) } else { delayed(r304(""), 8_000) });
    let o = spawn_scripted(script).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-timeout", gw);
    miss_then_expire(&c, &url).await;
    let t0 = std::time::Instant::now();
    let r = c.get(&url).send().await.unwrap();
    assert!(t0.elapsed() >= Duration::from_millis(4500), "waited for the origin timeout");
    assert_eq!(r.status().as_u16(), 200, "stale-if-error on timeout");
    assert_eq!(body_of(r).await, "{\"v\":1}");
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 1);
    assert_eq!(cnt(&cache.counters.revalidation_error), 1);
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 0);
}

#[tokio::test]
async fn s1_background_conditional_fetch_timeout_counts_error_and_backs_off() {
    let script: Script = Arc::new(|i, _| if i == 0 { ok200("{\"v\":1}", &prime_headers(BG_CC)) } else { delayed(r304(""), 8_000) });
    let o = spawn_scripted(script).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-bgtimeout", gw);
    miss_then_expire(&c, &url).await;
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE", "client is not delayed by the slow revalidation");
    wait_count(&o, 2).await;
    for _ in 0..80 {
        if cnt(&cache.counters.revalidation_error) >= 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert_eq!(cnt(&cache.counters.revalidation_error), 1);
    assert_eq!(cnt(&cache.counters.revalidation_conditional_sent), 1);
    assert_eq!(xcache(&c.get(&url).send().await.unwrap()), "STALE");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_eq!(o.count(), 2, "backoff after the failure suppresses an immediate retry");
    assert_eq!(cnt(&cache.counters.background_refresh_in_flight), 0);
}

// ---- Client cancels while the gateway revalidates -------------------------------------------------
#[tokio::test]
async fn s1_client_cancellation_while_revalidating_does_not_abort_or_corrupt_the_refresh() {
    let o = spawn_scripted(validating_origin(prime_headers(FG_CC), REVAL_FG.into(), etag_v1, 800)).await;
    let (gw, cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/vf-cancel", gw);
    miss_then_expire(&c, &url).await;
    let cancelled = c.get(&url).timeout(Duration::from_millis(200)).send().await;
    assert!(cancelled.is_err(), "client gave up while the gateway was revalidating");
    tokio::time::sleep(Duration::from_millis(1200)).await;
    assert_eq!(o.req(1).h("if-none-match"), Some("\"v1\""));
    assert_eq!(cnt(&cache.counters.revalidation_not_modified), 1, "single-flight leader finished despite the cancellation");
    let r = c.get(&url).send().await.unwrap();
    assert_eq!(xcache(&r), "HIT");
    assert_eq!(body_of(r).await, "{\"v\":1}");
    assert_eq!(o.count(), 2);
    let (acc, rec) = cache.audit_bytes();
    assert_eq!(acc, rec);
    assert_eq!(cnt(&cache.counters.background_refresh_in_flight), 0);
}

// ---- Guard: client validators are stripped, STORED validators are forwarded ------------------------
// (replaces the pre-implementation guard that asserted nothing conditional was ever forwarded)
#[tokio::test]
async fn guard_client_conditional_headers_are_stripped_and_stored_validators_are_forwarded_upstream() {
    let o = spawn_scripted(validating_origin(prime_headers(BG_CC), "ETag: \"v1\"\r\nCache-Control: max-age=300\r\n".into(), etag_v1, 0)).await;
    let (gw, _cache) = spawn_gateway(o.addr).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/guard-cond", gw);
    let _ = with_client_conditionals(c.get(&url)).send().await.unwrap();
    assert!(o.req(0).conditional_names().is_empty(), "client conditional header reached the origin: {:?}", o.req(0).headers);
    tokio::time::sleep(Duration::from_millis(1400)).await;
    let _ = with_client_conditionals(c.get(&url)).send().await.unwrap();
    wait_count(&o, 2).await;
    assert_only_stored_validators(&o.req(1), "guard");
    for (k, v) in CLIENT_CONDITIONALS {
        assert!(!o.req(1).headers.iter().any(|(hk, hv)| hk == &k.to_ascii_lowercase() && hv == v), "client {k} leaked");
    }
}

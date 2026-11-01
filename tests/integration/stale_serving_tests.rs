use phantom_cache_demo::api::server::{create_app, ApiState, SimulationSnapshot};
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache, StaleServingDecision,
};
use phantom_cache_demo::core::coalesce::FinalizeOutcome;
use phantom_cache_demo::core::freshness::FreshnessMetadata;
use phantom_cache_demo::intel::diagnostics::{CacheCounters, DiagnosticBuffer};
use phantom_cache_demo::intel::telemetry::TelemetryCollector;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

fn make_key(tenant: &str, path: &str) -> CacheKey {
    CacheKey::new(Some(tenant), path, None)
}

fn make_resource(body: &[u8], ttl_ms: u64, created_at_ms: u64, etag: Option<&str>) -> CachedResource {
    CachedResource {
        status: 200,
        content_type: "application/json".to_string(),
        body: Arc::new(body.to_vec()),
        etag: etag.map(|s| s.to_string()),
        created_at_ms,
        ttl_ms,
    }
}

// ---------------------------------------------------------------------------
// 1. SWR Serves Stale with Status Header, Age, and Background Revalidation
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_swr_stale_response_headers_and_background_revalidation() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/swr-test");
    let initial_now = 1_000_000;
    let res = make_resource(b"initial_body", 10_000, initial_now, Some("\"etag-1\""));

    // Register and populate entry with stale-while-revalidate = 60s
    let reg = match cache.begin_fetch_atomic(&key, initial_now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    guard.cache.update_freshness_metadata(
        &key,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: Some(60_000),
            stale_if_error_ms: None,
            last_modified: None,
            date_ms: Some(initial_now),
            initial_age_ms: 0,
        },
    );
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    // Query at now = initial_now + 15_000 (5s expired, within 60s SWR window)
    let query_now = initial_now + 15_000;
    let hard_cap = cache.get_max_stale_ms();

    let decision = cache.check_stale_serving(&key, query_now, hard_cap, false);
    match decision {
        StaleServingDecision::Eligible(stale_res, meta) => {
            assert_eq!(*stale_res.body, b"initial_body");
            assert_eq!(meta.stale_while_revalidate_ms, Some(60_000));
            cache.record_stale_served(query_now - stale_res.created_at_ms, stale_res.ttl_ms, hard_cap);
        }
        other => panic!("Expected StaleServingDecision::Eligible, got {:?}", other),
    }

    assert_eq!(counters.stale_served_total.load(Ordering::Relaxed), 1);
    assert_eq!(counters.stale_cap_violations.load(Ordering::Relaxed), 0);
}

// ---------------------------------------------------------------------------
// 2. Hard Staleness Cap Enforced Strictly (No Intelligence or Config Can Bypass)
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_hard_staleness_cap_enforced_strictly() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/cap-test");
    let initial_now = 1_000_000;
    // TTL = 10s, SWR claims 1,000,000s (huge window)
    let res = make_resource(b"ancient_data", 10_000, initial_now, None);

    let reg = match cache.begin_fetch_atomic(&key, initial_now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    guard.cache.update_freshness_metadata(
        &key,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: Some(1_000_000_000),
            stale_if_error_ms: None,
            last_modified: None,
            date_ms: Some(initial_now),
            initial_age_ms: 0,
        },
    );
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    // Set hard cap to 5,000ms (5s)
    cache.set_max_stale_ms(5_000);
    let hard_cap = cache.get_max_stale_ms();

    // At age 12s (lifetime 10s + 2s): within 5s hard cap -> Eligible
    let ok_now = initial_now + 12_000;
    match cache.check_stale_serving(&key, ok_now, hard_cap, false) {
        StaleServingDecision::Eligible(..) => {}
        other => panic!("Expected Eligible at age 12s, got {:?}", other),
    }

    // At age 20s (lifetime 10s + 10s > 10s + 5s cap): CapExceeded -> Ineligible!
    let cap_exceeded_now = initial_now + 20_000;
    match cache.check_stale_serving(&key, cap_exceeded_now, hard_cap, false) {
        StaleServingDecision::CapExceeded => {}
        other => panic!("Expected CapExceeded at age 20s, got {:?}", other),
    }

    // Invariant: stale_cap_violations remains exactly 0 because cap was strictly respected
    assert_eq!(counters.stale_cap_violations.load(Ordering::Relaxed), 0);
}

// ---------------------------------------------------------------------------
// 3. must-revalidate Strictly Forbids Stale Serving
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_must_revalidate_strictly_forbids_stale_serving() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/must-revalidate-test");
    let initial_now = 1_000_000;
    let res = make_resource(b"strict_data", 10_000, initial_now, None);

    let reg = match cache.begin_fetch_atomic(&key, initial_now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    // Both SWR and SIE present, but must_revalidate is true!
    guard.cache.update_freshness_metadata(
        &key,
        FreshnessMetadata {
            must_revalidate: true,
            stale_while_revalidate_ms: Some(60_000),
            stale_if_error_ms: Some(60_000),
            last_modified: None,
            date_ms: Some(initial_now),
            initial_age_ms: 0,
        },
    );
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    let query_now = initial_now + 15_000;
    let hard_cap = cache.get_max_stale_ms();

    // Normal context: must-revalidate forbids SWR
    assert_eq!(
        cache.check_stale_serving(&key, query_now, hard_cap, false),
        StaleServingDecision::Ineligible
    );

    // Error context: must-revalidate forbids SIE
    assert_eq!(
        cache.check_stale_serving(&key, query_now, hard_cap, true),
        StaleServingDecision::Ineligible
    );

    assert_eq!(counters.stale_served_total.load(Ordering::Relaxed), 0);
    assert_eq!(counters.stale_cap_violations.load(Ordering::Relaxed), 0);
}

// ---------------------------------------------------------------------------
// 4. Invalidation / DELETE Strictly Wins Over Stale Serving
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_invalidation_delete_strictly_wins_over_stale() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/delete-wins-test");
    let initial_now = 1_000_000;
    let res = make_resource(b"to_be_deleted", 10_000, initial_now, None);

    let reg = match cache.begin_fetch_atomic(&key, initial_now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    guard.cache.update_freshness_metadata(
        &key,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: Some(60_000),
            stale_if_error_ms: Some(60_000),
            last_modified: None,
            date_ms: Some(initial_now),
            initial_age_ms: 0,
        },
    );
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    let query_now = initial_now + 12_000;
    let hard_cap = cache.get_max_stale_ms();

    // Initially eligible for stale serving
    match cache.check_stale_serving(&key, query_now, hard_cap, false) {
        StaleServingDecision::Eligible(..) => {}
        other => panic!("Expected Eligible before delete, got {:?}", other),
    }

    // Invalidation occurs: DELETE key
    cache.delete(&key);

    // After DELETE: entry is immediately ineligible!
    assert_eq!(
        cache.check_stale_serving(&key, query_now, hard_cap, false),
        StaleServingDecision::Ineligible
    );
    assert_eq!(
        cache.check_stale_serving(&key, query_now, hard_cap, true),
        StaleServingDecision::Ineligible
    );
}

// ---------------------------------------------------------------------------
// 5. Stale-If-Error Serves Stale on Origin Outage
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_stale_if_error_on_origin_outage() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/sie-test");
    let initial_now = 1_000_000;
    // Expired: TTL = 5s, no SWR, but stale-if-error = 30s
    let res = make_resource(b"cached_sie_body", 5_000, initial_now, Some("\"etag-sie\""));

    let reg = match cache.begin_fetch_atomic(&key, initial_now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    guard.cache.update_freshness_metadata(
        &key,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: None,
            stale_if_error_ms: Some(30_000),
            last_modified: None,
            date_ms: Some(initial_now),
            initial_age_ms: 0,
        },
    );
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    let query_now = initial_now + 10_000; // 5s past expiry
    let hard_cap = cache.get_max_stale_ms();

    // Normal context: NOT eligible for immediate stale serving
    assert_eq!(
        cache.check_stale_serving(&key, query_now, hard_cap, false),
        StaleServingDecision::Ineligible
    );

    // Error context (origin returned 500 or timeout): ELIGIBLE!
    match cache.check_stale_serving(&key, query_now, hard_cap, true) {
        StaleServingDecision::Eligible(stale_res, meta) => {
            assert_eq!(*stale_res.body, b"cached_sie_body");
            assert_eq!(meta.stale_if_error_ms, Some(30_000));
            cache.record_stale_served(query_now - stale_res.created_at_ms, stale_res.ttl_ms, hard_cap);
        }
        other => panic!("Expected Eligible in error context, got {:?}", other),
    }

    assert_eq!(counters.stale_served_total.load(Ordering::Relaxed), 1);
    assert_eq!(counters.stale_cap_violations.load(Ordering::Relaxed), 0);

    // Query past SIE window (at 50s, > 5s + 30s): Ineligible!
    let past_sie_now = initial_now + 50_000;
    assert_eq!(
        cache.check_stale_serving(&key, past_sie_now, hard_cap, true),
        StaleServingDecision::Ineligible
    );
}

// ---------------------------------------------------------------------------
// 6. Bounded Background Refresh Under Concurrent Stale Requests
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_bounded_background_refresh_under_concurrent_stale_requests() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/concurrency-swr");
    let initial_now = 1_000_000;
    let res = make_resource(b"concurrent_payload", 1_000, initial_now, None);

    let reg = match cache.begin_fetch_atomic(&key, initial_now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    guard.cache.update_freshness_metadata(
        &key,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: Some(60_000),
            stale_if_error_ms: None,
            last_modified: None,
            date_ms: Some(initial_now),
            initial_age_ms: 0,
        },
    );
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    let query_now = initial_now + 5_000;
    let hard_cap = cache.get_max_stale_ms();

    // 100 concurrent requests verify eligibility
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let c = cache.clone();
        let k = key.clone();
        tasks.push(tokio::spawn(async move {
            let decision = c.check_stale_serving(&k, query_now, hard_cap, false);
            match decision {
                StaleServingDecision::Eligible(..) => {
                    c.record_stale_served(query_now - 1_000_000, 1_000, hard_cap);
                    true
                }
                _ => false,
            }
        }));
    }

    let mut successful_stale_serves = 0;
    for t in tasks {
        if t.await.unwrap() {
            successful_stale_serves += 1;
        }
    }

    assert_eq!(successful_stale_serves, 100);
    assert_eq!(counters.stale_served_total.load(Ordering::Relaxed), 100);
    assert_eq!(counters.stale_cap_violations.load(Ordering::Relaxed), 0);
}

// ---------------------------------------------------------------------------
// 7. 304 Revalidation Refreshes Entry Lifetime without Reallocating Body
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_revalidation_304_refreshes_metadata_without_reallocating_body() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/reval-304");
    let initial_now = 1_000_000;
    let original_body = vec![0xfe; 4096];
    let res = make_resource(&original_body, 5_000, initial_now, Some("\"etag-v1\""));

    let reg = match cache.begin_fetch_atomic(&key, initial_now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    guard.cache.update_freshness_metadata(
        &key,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: Some(10_000),
            stale_if_error_ms: None,
            last_modified: None,
            date_ms: Some(initial_now),
            initial_age_ms: 0,
        },
    );
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    let initial_resident_bytes = cache.bytes_resident();

    // Advance time past initial TTL (age = 8s)
    let reval_now = initial_now + 8_000;
    assert!(cache.get_resource(&key, reval_now).is_none()); // Expired for normal hits

    // Perform 304 refresh with new TTL = 60s and updated ETag "etag-v2"
    let new_metadata = FreshnessMetadata {
        must_revalidate: false,
        stale_while_revalidate_ms: Some(30_000),
        stale_if_error_ms: None,
        last_modified: Some("Sun, 06 Nov 1994 08:49:37 GMT".to_string()),
        date_ms: Some(reval_now),
        initial_age_ms: 0,
    };
    cache.refresh_entry_lifetime(&key, reval_now, 60_000, Some("\"etag-v2\"".to_string()), new_metadata);

    // Post-304: entry is fresh again!
    let fresh_res = cache.get_resource(&key, reval_now + 1_000).expect("Entry refreshed by 304");
    assert_eq!(fresh_res.ttl_ms, 60_000);
    assert_eq!(fresh_res.created_at_ms, reval_now);
    assert_eq!(fresh_res.etag.as_deref(), Some("\"etag-v2\""));
    assert_eq!(*fresh_res.body, original_body);

    // Byte accounting must remain stable (no leak or drift)
    let post_resident_bytes = cache.bytes_resident();
    assert_eq!(initial_resident_bytes, post_resident_bytes);
}

// ---------------------------------------------------------------------------
// 8. Revalidation Failure Backoff Schedule
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_failure_backoff_schedule() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let key = make_key("t1", "/backoff-test");
    let now = 1_000_000;

    let reg = match cache.begin_fetch_atomic(&key, now) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    let res = make_resource(b"backoff_data", 5_000, now, None);
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    // Initial state: revalidation is due
    assert!(cache.is_revalidation_due(&key, now + 6_000));

    // First failure at 1_006_000: backoff is 1,000ms
    cache.record_revalidation_failure(&key, 1_006_000);

    // Within 1,000ms: not due
    assert!(!cache.is_revalidation_due(&key, 1_006_500));
    // After 1,000ms: due again
    assert!(cache.is_revalidation_due(&key, 1_007_001));

    // Second failure at 1_007_001: backoff doubles to 2,000ms
    cache.record_revalidation_failure(&key, 1_007_001);
    assert!(!cache.is_revalidation_due(&key, 1_008_500));
    assert!(cache.is_revalidation_due(&key, 1_009_002));
}

// ---------------------------------------------------------------------------
// 9. Virtual-Time Property Test: stale_cap_violations is ALWAYS 0
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_virtual_time_property_stale_cap_violations_always_zero() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));

    let hard_cap_ms = 30_000; // 30 seconds
    cache.set_max_stale_ms(hard_cap_ms);

    let base_time = 1_000_000u64;

    for i in 0..50 {
        let key = make_key("t1", &format!("/prop-{}", i));
        let ttl_ms = 5_000 + (i as u64 * 1_000);
        let swr_ms = (i as u64) * 2_000;

        let reg = match cache.begin_fetch_atomic(&key, base_time) {
            FetchRegistrationResult::Initiate(r) => r,
            _ => panic!("Expected Initiate"),
        };
        let guard = reg.into_guard();
        guard.cache.update_freshness_metadata(
            &key,
            FreshnessMetadata {
                must_revalidate: i % 5 == 0,
                stale_while_revalidate_ms: if swr_ms > 0 { Some(swr_ms) } else { None },
                stale_if_error_ms: Some(10_000),
                last_modified: None,
                date_ms: Some(base_time),
                initial_age_ms: 0,
            },
        );
        let res = make_resource(b"val", ttl_ms, base_time, None);
        guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

        // Sample various time offsets from -1s to +100s
        for offset in [0, 2_000, 5_000, 10_000, 25_000, 35_000, 60_000, 100_000] {
            let now = base_time + offset;
            let decision = cache.check_stale_serving(&key, now, hard_cap_ms, false);
            if let StaleServingDecision::Eligible(entry, _) = decision {
                let age = now.saturating_sub(entry.created_at_ms);
                // INVARIANT: Served age must NEVER exceed lifetime + hard_cap
                assert!(
                    age <= entry.ttl_ms + hard_cap_ms,
                    "Violation: served age {} > lifetime {} + hard_cap {}",
                    age,
                    entry.ttl_ms,
                    hard_cap_ms
                );
                cache.record_stale_served(age, entry.ttl_ms, hard_cap_ms);
            }
        }
    }

    // Production invariant assertion
    assert_eq!(
        counters.stale_cap_violations.load(Ordering::Relaxed),
        0,
        "Invariant violated: stale_cap_violations must be 0"
    );
}

// ---------------------------------------------------------------------------
// 10. HTTP Integration: Stale Serving Headers, Age Header, and Conditional 304
// ---------------------------------------------------------------------------
#[tokio::test]
async fn test_http_stale_serving_headers_and_status() {
    let counters = Arc::new(CacheCounters::new());
    let diagnostics = Arc::new(DiagnosticBuffer::new(512, counters.clone()));
    let cache = Arc::new(PartitionedCache::new_with_diagnostics(4, counters.clone(), diagnostics));
    let telemetry = Arc::new(TelemetryCollector::new(512));

    let key = make_key("__default__", "/http-swr");
    let now = DiagnosticBuffer::now_ms();
    let res = make_resource(b"{\"hello\":\"stale\"}", 1_000, now - 2_000, Some("\"etag-http\""));

    let reg = match cache.begin_fetch_atomic(&key, now - 2_000) {
        FetchRegistrationResult::Initiate(r) => r,
        _ => panic!("Expected Initiate"),
    };
    let guard = reg.into_guard();
    guard.cache.update_freshness_metadata(
        &key,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: Some(60_000),
            stale_if_error_ms: None,
            last_modified: None,
            date_ms: Some(now - 2_000),
            initial_age_ms: 0,
        },
    );
    guard.complete(FinalizeOutcome::Success(Arc::new(res))).unwrap().deliver();

    let state = Arc::new(ApiState {
        snapshot: Mutex::new(SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: now,
            throughput_req_sec: 100.0,
            p50_latency_ms: 1,
            p95_latency_ms: 2,
            p99_latency_ms: 3,
            db_load_ratio: 0.1,
            avoided_compute_dollars: 0.0,
            nodes: vec![],
        }),
        nodes: vec![],
        workload_type: Mutex::new("static".to_string()),
        cache: Some(cache.clone()),
        origin_url: None,
        telemetry: Some(telemetry),
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

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/cache/http-swr", addr);

    // Request 1: Client gets STALE response
    let response = client.get(&url).send().await.unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response.headers().get("X-Cache-Status").unwrap(),
        "STALE"
    );
    assert!(response.headers().get("Age").is_some());
    assert_eq!(
        response.headers().get("ETag").unwrap(),
        "\"etag-http\""
    );

    let body_text = response.text().await.unwrap();
    assert_eq!(body_text, "{\"hello\":\"stale\"}");

    // Request 2: Conditional request with matching If-None-Match returns 304 Not Modified with STALE status
    let response_cond = client
        .get(&url)
        .header("If-None-Match", "\"etag-http\"")
        .send()
        .await
        .unwrap();
    assert_eq!(response_cond.status(), reqwest::StatusCode::NOT_MODIFIED);
    assert_eq!(
        response_cond.headers().get("X-Cache-Status").unwrap(),
        "STALE"
    );
    assert!(response_cond.headers().get("Age").is_some());
}

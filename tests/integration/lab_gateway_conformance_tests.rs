//! Conformance tests for Phantom Lab Gateway (M4-05 / WP-17).
//!
//! Validates that the Lab Gateway configured with BL-1 (SingleFlight) satisfies
//! identical conformance criteria to the product gateway:
//! - Cold miss triggers single-flight fetch and sets x-cache: MISS.
//! - Subsequent request serves from cache with x-cache: HIT.
//! - Concurrent requests for the same key coalesce onto exactly 1 upstream origin fetch.
//! - HTTP DELETE immediately invalidates entry (linearizable fencing; subsequent GET misses).
//! - RFC 9111 stale-if-error fallback and hard staleness cap enforcement (EV-12: zero stale-cap violations).

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::gateway::{LabGateway, SingleFlightPolicy};
use lab::origin_proc::{ControllableOrigin, OriginConfig};
use phantom_cache_demo::core::cache::{CacheKey, PartitionedCache};
use phantom_cache_demo::core::freshness::FreshnessMetadata;
use phantom_cache_demo::intel::diagnostics::DiagnosticBuffer;
use reqwest::Client;
use std::sync::Arc;

#[tokio::test]
async fn test_cold_miss_and_caching_conformance() {
    let origin = ControllableOrigin::start(OriginConfig {
        capacity: 16,
        latency_ms: 5,
        error_rate: 0.0,
        is_outage: false,
        default_ttl_sec: 60,
    })
    .await
    .unwrap();

    let cache = Arc::new(PartitionedCache::new(4));
    let gateway = LabGateway::start(cache, origin.url(""), Arc::new(SingleFlightPolicy))
        .await
        .unwrap();

    let client = Client::new();

    // 1. Cold Miss
    let r1 = client.get(gateway.url("/item_1")).send().await.unwrap();
    assert_eq!(r1.status(), 200);
    assert_eq!(
        r1.headers().get("x-cache").unwrap().to_str().unwrap(),
        "MISS"
    );
    let b1 = r1.text().await.unwrap();
    assert_eq!(b1, "DATA:item_1");

    let origin_snap = origin.snapshot_metrics();
    assert_eq!(origin_snap.total_completed, 1);

    // 2. Warm Hit
    let r2 = client.get(gateway.url("/item_1")).send().await.unwrap();
    assert_eq!(r2.status(), 200);
    assert_eq!(
        r2.headers().get("x-cache").unwrap().to_str().unwrap(),
        "HIT"
    );
    let b2 = r2.text().await.unwrap();
    assert_eq!(b2, "DATA:item_1");

    // Origin count must NOT increment
    let origin_snap2 = origin.snapshot_metrics();
    assert_eq!(origin_snap2.total_completed, 1);

    // Gateway telemetry check
    assert_eq!(gateway.telemetry().hits.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(gateway.telemetry().misses.load(std::sync::atomic::Ordering::Relaxed), 1);

    gateway.shutdown();
    origin.shutdown();
}

#[tokio::test]
async fn test_single_flight_coalescing_conformance() {
    // 50ms origin latency to allow multiple concurrent waiters to coalesce
    let origin = ControllableOrigin::start(OriginConfig {
        capacity: 16,
        latency_ms: 50,
        error_rate: 0.0,
        is_outage: false,
        default_ttl_sec: 60,
    })
    .await
    .unwrap();

    let cache = Arc::new(PartitionedCache::new(4));
    let gateway = LabGateway::start(cache, origin.url(""), Arc::new(SingleFlightPolicy))
        .await
        .unwrap();

    let client = Client::new();
    let mut tasks = Vec::new();

    // Spawn 10 simultaneous requests for the exact same cold key
    for _ in 0..10 {
        let c = client.clone();
        let u = gateway.url("/coalesced_key");
        tasks.push(tokio::spawn(async move {
            c.get(&u).send().await
        }));
    }

    for t in tasks {
        let resp = t.await.unwrap().unwrap();
        assert_eq!(resp.status(), 200);
        let body = resp.text().await.unwrap();
        assert_eq!(body, "DATA:coalesced_key");
    }

    // Coalescing Invariant: exactly 1 request reached and completed at the origin!
    let origin_snap = origin.snapshot_metrics();
    assert_eq!(
        origin_snap.total_completed, 1,
        "Coalescing failed: origin received {} requests, expected exactly 1",
        origin_snap.total_completed
    );

    gateway.shutdown();
    origin.shutdown();
}

#[tokio::test]
async fn test_linearizable_delete_fencing() {
    let origin = ControllableOrigin::start(OriginConfig::default())
        .await
        .unwrap();

    let cache = Arc::new(PartitionedCache::new(4));
    let gateway = LabGateway::start(cache, origin.url(""), Arc::new(SingleFlightPolicy))
        .await
        .unwrap();

    let client = Client::new();

    // Populate key
    let r1 = client.get(gateway.url("/fenced_key")).send().await.unwrap();
    assert_eq!(r1.headers().get("x-cache").unwrap().to_str().unwrap(), "MISS");

    // Warm hit
    let r2 = client.get(gateway.url("/fenced_key")).send().await.unwrap();
    assert_eq!(r2.headers().get("x-cache").unwrap().to_str().unwrap(), "HIT");

    // DELETE key
    let del_resp = client.delete(gateway.url("/fenced_key")).send().await.unwrap();
    assert_eq!(del_resp.status(), 204);

    // Immediate subsequent GET must MISS (linearizable fencing)
    let r3 = client.get(gateway.url("/fenced_key")).send().await.unwrap();
    assert_eq!(r3.headers().get("x-cache").unwrap().to_str().unwrap(), "MISS");

    // Origin completed exactly 2 fetches (one before delete, one after delete)
    assert_eq!(origin.snapshot_metrics().total_completed, 2);

    gateway.shutdown();
    origin.shutdown();
}

#[tokio::test]
async fn test_stale_if_error_fallback_conformance() {
    let origin = ControllableOrigin::start(OriginConfig::default())
        .await
        .unwrap();

    let cache = Arc::new(PartitionedCache::new(4));
    let gateway = LabGateway::start(cache.clone(), origin.url(""), Arc::new(SingleFlightPolicy))
        .await
        .unwrap();

    let client = Client::new();

    // Populate key
    let r1 = client.get(gateway.url("/stale_item")).send().await.unwrap();
    assert_eq!(r1.headers().get("x-cache").unwrap().to_str().unwrap(), "MISS");

    // Configure freshness metadata on the cache entry allowing stale-if-error
    let cache_key = CacheKey::new(None, "/stale_item", None);
    let now_ms = DiagnosticBuffer::now_ms();
    cache.update_freshness_metadata(
        &cache_key,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: None,
            stale_if_error_ms: Some(60_000), // 60s stale-if-error window
            last_modified: None,
            date_ms: Some(now_ms),
            initial_age_ms: 0,
        },
    );

    // Expire the base TTL by setting created_at to 20s ago (TTL 10s -> 10s expired, within 60s stale window)
    cache.refresh_entry_lifetime(
        &cache_key,
        now_ms.saturating_sub(20_000), // created 20s ago
        10_000,                         // TTL 10s -> expired!
        None,
        FreshnessMetadata {
            must_revalidate: false,
            stale_while_revalidate_ms: None,
            stale_if_error_ms: Some(60_000),
            last_modified: None,
            date_ms: Some(now_ms),
            initial_age_ms: 0,
        },
    );

    // Put origin into outage (returns 503)
    origin
        .update_config(OriginConfig {
            capacity: 8,
            latency_ms: 0,
            error_rate: 0.0,
            is_outage: true,
            default_ttl_sec: 60,
        })
        .await;

    // Lab Gateway should serve stale data with x-cache: STALE!
    let r2 = client.get(gateway.url("/stale_item")).send().await.unwrap();
    assert_eq!(r2.status(), 200);
    assert_eq!(
        r2.headers().get("x-cache").unwrap().to_str().unwrap(),
        "STALE"
    );

    // Hard staleness cap violation test (EV-12):
    // Set max_stale_ms to 1ms -> entry is now past hard cap!
    cache.set_max_stale_ms(1);

    // Gateway must REFUSE stale serving and return origin error 503
    let r3 = client.get(gateway.url("/stale_item")).send().await.unwrap();
    assert_eq!(r3.status(), 503);

    gateway.shutdown();
    origin.shutdown();
}

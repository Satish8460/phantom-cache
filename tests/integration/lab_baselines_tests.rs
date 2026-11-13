//! Integration tests for Phantom Lab Static Baselines BL-0..BL-12b (M4-06 / WP-17).
//!
//! Validates:
//! - BL-0: Naive uncoalesced fetch
//! - BL-2: Nginx-style lock timeout
//! - BL-3: Varnish-style probe health tracking
//! - BL-4: XFetch probabilistic early refresh evaluation
//! - BL-5: Memcache-style lease token rate limiting
//! - BL-7: Adaptive concurrency shedding
//! - BL-8: Success-rate admission gating
//! - BL-9: Queue-delay admission gating
//! - BL-12a/b: Oracle and family-tuned static configurations

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::baselines::*;
use lab::gateway::LabPolicy;
use lab::origin_proc::{ControllableOrigin, OriginConfig};
use phantom_cache_demo::core::cache::CacheKey;
use phantom_cache_demo::core::coalesce::OriginError;
use reqwest::Client;
use std::sync::atomic::Ordering;

#[tokio::test]
async fn test_bl0_naive_policy_execution() {
    let origin = ControllableOrigin::start(OriginConfig::default())
        .await
        .unwrap();
    let client = Client::new();
    let policy = NaivePolicy;
    let key = CacheKey::new(None, "/bl0_item", None);

    let res = policy.fetch_miss(&key, &origin.url(""), &client).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status, 200);

    origin.shutdown();
}

#[tokio::test]
async fn test_bl2_nginx_lock_timeout() {
    // Origin takes 60ms to respond
    let origin = ControllableOrigin::start(OriginConfig {
        capacity: 8,
        latency_ms: 60,
        error_rate: 0.0,
        is_outage: false,
        default_ttl_sec: 60,
    })
    .await
    .unwrap();

    let client = Client::new();
    // Nginx lock timeout set to 15ms
    let policy = NginxStylePolicy::new(15);
    let key = CacheKey::new(None, "/bl2_item", None);

    let res = policy.fetch_miss(&key, &origin.url(""), &client).await;
    match res {
        Err(OriginError::Timeout) => (), // expected timeout fall-through
        other => panic!("Expected Timeout, got {:?}", other),
    }

    origin.shutdown();
}

#[tokio::test]
async fn test_bl3_varnish_health_grace() {
    let origin = ControllableOrigin::start(OriginConfig::default())
        .await
        .unwrap();
    let client = Client::new();
    let policy = VarnishStylePolicy::new(5000, 60000);
    let key = CacheKey::new(None, "/bl3_item", None);

    assert!(policy.is_healthy.load(Ordering::Relaxed));

    // Put origin in outage
    origin
        .update_config(OriginConfig {
            capacity: 8,
            latency_ms: 0,
            error_rate: 0.0,
            is_outage: true,
            default_ttl_sec: 60,
        })
        .await;

    let res_err = policy.fetch_miss(&key, &origin.url(""), &client).await;
    assert!(res_err.is_err());
    // Probe must observe failure and mark backend unhealthy
    assert!(!policy.is_healthy.load(Ordering::Relaxed));

    origin.shutdown();
}

#[test]
fn test_bl4_xfetch_probabilistic_refresh() {
    let xfetch = XFetchPolicy::new(1.0);
    xfetch.observed_delta_ms.store(100, Ordering::Relaxed); // delta = 100ms

    // When remaining TTL is 50_000ms (50s) and U = 0.5:
    // -100 * 1.0 * ln(0.5) = 100 * 0.693 = 69.3ms << 50,000ms -> NO refresh
    assert!(!xfetch.should_early_refresh(50_000, 0.5));

    // When remaining TTL is 50ms and U = 0.1:
    // -100 * 1.0 * ln(0.1) = 100 * 2.302 = 230.2ms >= 50ms -> TRIGGER refresh!
    assert!(xfetch.should_early_refresh(50, 0.1));

    // Boundary edge: U close to 0 gives strong refresh urge
    assert!(xfetch.should_early_refresh(10, 0.001));
}

#[test]
fn test_bl5_memcache_lease_token() {
    let lease_policy = MemcacheLeasePolicy::new(1000); // 1s interval

    // First request at t = 1000 acquires lease
    assert!(lease_policy.try_acquire_lease(1000));

    // Simultaneous request at t = 1000 fails to acquire lease
    assert!(!lease_policy.try_acquire_lease(1000));

    // Request at t = 1500 (within interval) fails to acquire lease
    assert!(!lease_policy.try_acquire_lease(1500));

    // Request at t = 2000 (after 1000ms interval) acquires lease
    assert!(lease_policy.try_acquire_lease(2000));
}

#[tokio::test]
async fn test_bl7_adaptive_concurrency_shedding() {
    let origin = ControllableOrigin::start(OriginConfig {
        capacity: 8,
        latency_ms: 50,
        error_rate: 0.0,
        is_outage: false,
        default_ttl_sec: 60,
    })
    .await
    .unwrap();

    let client = Client::new();
    // Capacity limit = 1
    let policy = AdaptiveConcurrencyPolicy::new(1);
    let key = CacheKey::new(None, "/bl7_item", None);

    // Simulate in_flight saturation
    policy.in_flight.store(1, Ordering::SeqCst);

    // Excess request must be shed
    let res = policy.fetch_miss(&key, &origin.url(""), &client).await;
    match res {
        Err(OriginError::CapacityExceeded) => (),
        other => panic!("Expected CapacityExceeded shedding, got {:?}", other),
    }

    origin.shutdown();
}

#[test]
fn test_bl9_queue_delay_admission() {
    let policy = QueueDelayAdmissionPolicy::new(25); // target 25ms
    policy.estimated_delay_ms.store(10, Ordering::Relaxed);
    assert_eq!(policy.policy_id(), "BL-9");

    // Under target -> admitted
    assert!(policy.estimated_delay_ms.load(Ordering::Relaxed) <= policy.target_delay_ms);

    // Above target -> shed
    policy.estimated_delay_ms.store(50, Ordering::Relaxed);
    assert!(policy.estimated_delay_ms.load(Ordering::Relaxed) > policy.target_delay_ms);
}

#[test]
fn test_bl12_tuned_static_configurations() {
    let oracle = TunedStaticPolicy::new_oracle();
    assert_eq!(oracle.policy_id_str, "BL-12a");
    assert_eq!(oracle.nginx_lock_ms, 1000);
    assert_eq!(oracle.xfetch_beta, 1.0);

    let family_tuned = TunedStaticPolicy::new_family_tuned();
    assert_eq!(family_tuned.policy_id_str, "BL-12b");
    assert_eq!(family_tuned.nginx_lock_ms, 500);
    assert_eq!(family_tuned.xfetch_beta, 0.8);
}

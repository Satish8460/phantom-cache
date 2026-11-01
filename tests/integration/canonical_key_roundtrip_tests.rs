//! Findings M1 (oversized key vs. the 4096-byte cluster frame cap) and M2 (canonical key round
//! trip), both fixed in `src/core/cache.rs` (`CacheKey::validate`) and enforced at the HTTP edge in
//! `src/api/server.rs`.
//!
//! RULES (decided by the implementation, see `CacheKey::validate`):
//!  * tenant id: 1..=128 bytes of `[A-Za-z0-9._-]` only (so the FIRST ':' of a canonical key is
//!    always the tenant separator);
//!  * path must not contain the two-byte sequence ":?" (it is the path/query separator of the
//!    canonical string; a path containing it is ambiguous with "path + query");
//!  * the canonical string must not exceed `MAX_CANONICAL_KEY_BYTES` (= the cluster frame cap
//!    `dist::frame::MAX_KEY_LENGTH_BYTES`, pinned equal by a test below).
//!
//! Violations are rejected at the edge (GET and DELETE use the same validation) BEFORE any origin
//! fetch, cache mutation or cluster call.

use phantom_cache_demo::api::server::{create_app, ApiState, SimulationSnapshot};
use phantom_cache_demo::core::cache::{
    CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache, MAX_CANONICAL_KEY_BYTES,
};
use phantom_cache_demo::core::coalesce::{FinalizeOutcome, OriginError};
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::dist::router::RendezvousRouter;
use phantom_cache_demo::intel::diagnostics::DiagnosticBuffer;
use phantom_cache_demo::security::{JwtClaims, JwtValidator, SecurityConfig};
use proptest::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// ------------------------------------------------------------------ unit level (M2)

#[test]
fn m2_tenant_ids_outside_the_safe_alphabet_are_rejected_by_validate() {
    for bad in ["acme:eu", "a:b:c", ":lead", "trail:", "", "has space", "caf\u{e9}", "a/b", "a?b", "a\nb"] {
        let k = CacheKey::new(Some(bad), "/p", None);
        assert!(
            matches!(k.validate(), Err(OriginError::InvalidTenantId) | Err(OriginError::TenantIdTooLong)),
            "tenant {bad:?} must be rejected, got {:?}",
            k.validate()
        );
    }
    let too_long = "a".repeat(129);
    assert!(matches!(
        CacheKey::new(Some(&too_long), "/p", None).validate(),
        Err(OriginError::TenantIdTooLong)
    ));
    for good in ["plain", "__default__", "Acme.EU-1_x", &"a".repeat(128)] {
        assert!(CacheKey::new(Some(good), "/p", None).validate().is_ok(), "tenant {good:?}");
    }
}

#[test]
fn m2_path_containing_the_query_separator_is_rejected() {
    for bad in ["/a:?b", "/:?", "/x:?y:?z"] {
        let k = CacheKey::new(Some("t"), bad, None);
        assert!(k.validate().is_err(), "path {bad:?} is ambiguous with path+query and must be rejected");
    }
    // ':' and '?' individually are fine.
    for ok in ["/a:b", "/a?b", "/a:", "/:", "/?", "/a::b?c"] {
        assert!(CacheKey::new(Some("t"), ok, None).validate().is_ok(), "path {ok:?}");
    }
}

/// The previously ignored desired-property test, now enforced for every VALID key.
#[test]
fn m2_canonical_key_round_trip_is_lossless_for_valid_keys() {
    for tenant in ["plain", "acme.eu", "a-b_c", "__default__", "T9"] {
        for path in ["/p/x", "/", "/a:b", "/a:", "/a?b", "/a::b", "/x:y?z=1"] {
            for query in [None, Some(""), Some("q=1"), Some("a:?b"), Some("?x"), Some("::"), Some("a=b:c?d")] {
                let original = CacheKey::new(Some(tenant), path, query);
                original.validate().expect("fixture must be a valid key");
                let canonical = original.to_canonical_string();
                let parsed = CacheKey::from_canonical_string(&canonical).unwrap();
                assert_eq!(parsed, original, "tenant {tenant:?} path {path:?} query {query:?} canonical {canonical:?}");
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(4000))]

    /// For ALL keys accepted by `validate`, from_canonical_string(to_canonical_string(k)) == k.
    #[test]
    fn m2_prop_round_trip_for_every_valid_key(
        tenant in proptest::option::of("[A-Za-z0-9._-]{1,128}"),
        // Biased towards the troublesome characters so ':' / '?' / ":?" occur constantly.
        path in "/(:|\\?|[a-z/]|%3A){0,40}",
        query in proptest::option::of("(:|\\?|=|&|[a-z0-9]){0,40}"),
    ) {
        let key = CacheKey::new(tenant.as_deref(), &path, query.as_deref());
        if key.validate().is_ok() {
            let parsed = CacheKey::from_canonical_string(&key.to_canonical_string()).unwrap();
            prop_assert_eq!(parsed, key);
        }
    }

    /// Every key containing ':' or '?' in the tenant, or ":?" in the path, is rejected (so the
    /// property above cannot be vacuously satisfied by rejecting everything: see the next test).
    #[test]
    fn m2_prop_invalid_tenants_never_validate(
        prefix in "[A-Za-z0-9]{0,5}",
        bad in "[:?/ ]",
        suffix in "[A-Za-z0-9]{0,5}",
    ) {
        let tenant = format!("{prefix}{bad}{suffix}");
        prop_assert!(CacheKey::new(Some(&tenant), "/p", None).validate().is_err());
    }
}

#[test]
fn m2_property_is_not_vacuous_most_generated_keys_are_accepted() {
    // Sanity: a typical key and one with ':' and '?' in path/query validate and round-trip.
    let k = CacheKey::new(Some("acme"), "/a:b?c", Some("x=1:?2"));
    assert!(k.validate().is_ok());
    assert_eq!(CacheKey::from_canonical_string(&k.to_canonical_string()).unwrap(), k);
}

// ------------------------------------------------------------------ unit level (M1)

#[test]
fn m1_canonical_cap_equals_cluster_frame_cap() {
    assert_eq!(
        MAX_CANONICAL_KEY_BYTES,
        phantom_cache_demo::dist::frame::MAX_KEY_LENGTH_BYTES,
        "edge key cap and cluster frame cap must stay equal"
    );
}

#[test]
fn m1_canonical_key_over_frame_cap_is_rejected_at_the_boundary() {
    let tenant = "t";
    // canonical = "t:" + path + ":?" + query -> overhead 2 + 2 bytes
    let path = format!("/{}", "p".repeat(2000)); // 2001
    let q_ok = "q".repeat(MAX_CANONICAL_KEY_BYTES - 2 - path.len() - 2);
    let ok = CacheKey::new(Some(tenant), &path, Some(&q_ok));
    assert_eq!(ok.to_canonical_string().len(), MAX_CANONICAL_KEY_BYTES);
    assert!(ok.validate().is_ok(), "exactly at the cap is accepted");
    let q_bad = format!("{q_ok}q");
    let bad = CacheKey::new(Some(tenant), &path, Some(&q_bad));
    assert_eq!(bad.to_canonical_string().len(), MAX_CANONICAL_KEY_BYTES + 1);
    assert!(matches!(bad.validate(), Err(OriginError::UriTooLong)), "one byte over the cap is rejected");
    // Without a query: path alone is limited to 2048, tenant to 128: always under the cap.
    assert!(CacheKey::new(Some(&"a".repeat(128)), &format!("/{}", "p".repeat(2047)), None).validate().is_ok());
}

// ------------------------------------------------------------------ HTTP level helpers

struct CountingOrigin {
    addr: std::net::SocketAddr,
    requests: Arc<AtomicU64>,
}

async fn spawn_counting_origin() -> CountingOrigin {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicU64::new(0));
    let r = requests.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else { break };
            let r = r.clone();
            tokio::spawn(async move {
                let mut buf = vec![0u8; 16384];
                let _ = sock.read(&mut buf).await;
                r.fetch_add(1, Ordering::SeqCst);
                let body = "{\"ok\":true}";
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: max-age=60\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    CountingOrigin { addr, requests }
}

fn snapshot() -> SimulationSnapshot {
    SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 0.0,
        p50_latency_ms: 0,
        p95_latency_ms: 0,
        p99_latency_ms: 0,
        db_load_ratio: 0.0,
        avoided_compute_dollars: 0.0,
        nodes: vec![],
    }
}

async fn serve(state: ApiState) -> String {
    let app = create_app(Arc::new(state));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}

fn plain_state(origin: std::net::SocketAddr, cache: Arc<PartitionedCache>) -> ApiState {
    ApiState {
        snapshot: Mutex::new(snapshot()),
        nodes: vec![],
        workload_type: Mutex::new("static".to_string()),
        cache: Some(cache),
        origin_url: Some(format!("http://{}", origin)),
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    }
}

fn seed(cache: &Arc<PartitionedCache>, key: &CacheKey) {
    let now = DiagnosticBuffer::now_ms();
    let g = match cache.begin_fetch_atomic(key, now) {
        FetchRegistrationResult::Initiate(r) => r.into_guard(),
        _ => panic!("expected Initiate"),
    };
    let res = CachedResource {
        status: 200,
        content_type: "text/plain".into(),
        body: Arc::new(b"seed".to_vec()),
        etag: None,
        created_at_ms: now,
        ttl_ms: 600_000,
    };
    if let Some(mut b) = g.complete(FinalizeOutcome::Success(Arc::new(res))) {
        b.deliver();
    }
}

fn resident(cache: &Arc<PartitionedCache>, key: &CacheKey) -> bool {
    cache.get_resource(key, DiagnosticBuffer::now_ms()).is_some()
}

// ------------------------------------------------------------------ HTTP: M1

/// 2000-byte path + 3000-byte query: each part is under its own limit (2048 / 4096) but the
/// canonical key (~5 KB) exceeds the 4096-byte cluster frame cap.
#[tokio::test]
async fn m1_http_oversized_canonical_key_get_and_delete_are_4xx_without_side_effects() {
    let origin = spawn_counting_origin().await;
    let node1 = Arc::new(VirtualNode::new(1));
    let a1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
    node1.set_addr(a1);
    node1.set_state(NodeState::Active);
    let node2 = Arc::new(VirtualNode::new(2));
    let a2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
    node2.set_addr(a2);
    node2.set_state(NodeState::Active);
    let router = Arc::new(RendezvousRouter::new(vec![node1.clone(), node2.clone()]));

    let path = "p".repeat(2000);
    let query = "q".repeat(3000);
    let key = CacheKey::new(None, &format!("/{path}"), Some(&query));
    assert!(key.to_canonical_string().len() > MAX_CANONICAL_KEY_BYTES);

    // Plant the key on the gateway cache AND on the peer: a DELETE that reaches the node or the
    // cluster would remove them.
    seed(&node1.cache, &key);
    seed(&node2.cache, &key);

    let state = plain_state(origin.addr, node1.cache.clone()).with_router(router.clone());
    let gw = serve(state).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/{}?{}", gw, path, query);

    let g = c.get(&url).send().await.unwrap();
    assert!((400..500).contains(&g.status().as_u16()), "GET must be 4xx, got {}", g.status());
    let d = c.delete(&url).send().await.unwrap();
    assert!((400..500).contains(&d.status().as_u16()), "DELETE must be 4xx, got {}", d.status());

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert_eq!(origin.requests.load(Ordering::SeqCst), 0, "no origin fetch");
    assert!(resident(&node1.cache, &key), "gateway cache entry untouched (no node state change)");
    assert!(resident(&node2.cache, &key), "peer entry untouched (no cluster call was made)");
    assert_eq!(node1.cache.counters.requests_total.load(Ordering::Relaxed), 0, "rejected before accounting");

    // A key under the cap still works end to end (GET miss then hit).
    let ok_url = format!("{}/api/v1/cache/{}?{}", gw, "p".repeat(100), "q".repeat(100));
    assert_eq!(c.get(&ok_url).send().await.unwrap().status().as_u16(), 200);
    assert_eq!(origin.requests.load(Ordering::SeqCst), 1);
    node1.stop_tcp_service();
    node2.stop_tcp_service();
    router.stop();
}

// ------------------------------------------------------------------ HTTP: M2

#[tokio::test]
async fn m2_http_non_secure_tenant_header_outside_alphabet_is_400_no_origin_fetch() {
    let origin = spawn_counting_origin().await;
    let cache = Arc::new(PartitionedCache::new(4));
    let gw = serve(plain_state(origin.addr, cache.clone())).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/m2/item", gw);

    for bad in ["acme:eu", "a b", "x/y", "a?b"] {
        let g = c.get(&url).header("x-tenant-id", bad).send().await.unwrap();
        assert_eq!(g.status().as_u16(), 400, "GET tenant {bad:?}");
        let d = c.delete(&url).header("x-tenant-id", bad).send().await.unwrap();
        assert_eq!(d.status().as_u16(), 400, "DELETE tenant {bad:?}");
    }
    // Empty and non-UTF-8 header values are rejected too (previously silently mapped to the
    // default tenant or accepted as an empty tenant).
    let empty = c.get(&url).header("x-tenant-id", "").send().await.unwrap();
    assert_eq!(empty.status().as_u16(), 400, "empty tenant header");
    let non_ascii = reqwest::header::HeaderValue::from_bytes(b"caf\xe9").unwrap();
    let r = c.get(&url).header("x-tenant-id", non_ascii).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 400, "non-visible-ASCII tenant header must not fall back to the default tenant");

    assert_eq!(origin.requests.load(Ordering::SeqCst), 0, "no origin fetch for any rejected tenant");
    // Valid tenants still work and are isolated.
    let ok = c.get(&url).header("x-tenant-id", "acme.eu-1").send().await.unwrap();
    assert_eq!(ok.status().as_u16(), 200);
    assert_eq!(origin.requests.load(Ordering::SeqCst), 1);
}

const ISS: &str = "phantom-cache-auth";
const AUD: &str = "phantom-cache-api";
const SECRET: &[u8] = b"m2-roundtrip-secret-key-0123456789abcdef";

#[tokio::test]
async fn m2_http_secure_mode_jwt_tenant_claim_outside_alphabet_is_400_no_origin_fetch() {
    let origin = spawn_counting_origin().await;
    let cache = Arc::new(PartitionedCache::new(4));
    let validator = JwtValidator::new(ISS, AUD, SECRET);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let tok = |tenant: &str| {
        validator
            .create_token(&JwtClaims::new("u", ISS, AUD, now + 3600, Some(tenant), vec!["admin"]))
            .unwrap()
    };
    let mut state = plain_state(origin.addr, cache.clone());
    state.security = Some(Arc::new(SecurityConfig::new_with_jwt(validator.clone())));
    let gw = serve(state).await;
    let c = reqwest::Client::new();
    let url = format!("{}/api/v1/cache/m2/item", gw);

    // Round-2 M1: an EMPTY tenant claim is now rejected earlier with 403 (authenticated principal
    // without a usable tenant) instead of 400; claims outside the alphabet are still 400.
    for bad in ["acme:eu", "a b"] {
        let t = tok(bad);
        let g = c.get(&url).bearer_auth(&t).send().await.unwrap();
        assert_eq!(g.status().as_u16(), 400, "GET with tenant claim {bad:?}");
        let d = c.delete(&url).bearer_auth(&t).send().await.unwrap();
        assert_eq!(d.status().as_u16(), 400, "DELETE with tenant claim {bad:?}");
    }
    let t = tok("");
    assert_eq!(c.get(&url).bearer_auth(&t).send().await.unwrap().status().as_u16(), 403, "empty tenant claim: GET");
    assert_eq!(c.delete(&url).bearer_auth(&t).send().await.unwrap().status().as_u16(), 403, "empty tenant claim: DELETE");
    assert_eq!(origin.requests.load(Ordering::SeqCst), 0, "no origin fetch for a rejected tenant claim");

    // A well-formed claim passes the validation (authorization header forces a bypass fetch).
    let g = c.get(&url).bearer_auth(tok("acme.eu")).send().await.unwrap();
    assert_eq!(g.status().as_u16(), 200);
    assert_eq!(origin.requests.load(Ordering::SeqCst), 1);
}

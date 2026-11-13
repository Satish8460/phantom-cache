//! SEC-M7-01 / SEC-M7-02 / SEC-M7-03 regression tests (Alternative A, owner approved).
//!
//! Every HTTP test drives the real router (`create_app` / `create_app_with_cors`, the same
//! functions `start_server*` serve) over a real TCP socket with real HS256 JWTs signed by the test
//! secret. Origins are raw-TCP servers that count the requests they receive, so "denied requests
//! cause zero origin fetches" is measured at the origin, not inferred.
//!
//! The intelligence executor/audit store is process-global (`ApiState::set_intelligence`), so it
//! is installed once for this test binary; every test in this file that touches intelligence uses
//! the same installed store.

use phantom_cache_demo::api::server::{create_app, create_app_with_cors, ApiState, CorsConfig, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::intelligence::audit::AuditStore;
use phantom_cache_demo::intelligence::executor::DefaultPolicyExecutor;
use phantom_cache_demo::intelligence::{PolicyExecutor, DecisionRecord, IntelligenceMode, PolicyCandidate, PolicyKind, Verdict};
use phantom_cache_demo::security::{JwtClaims, JwtValidator, SecurityConfig};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Once};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ISS: &str = "phantom-cache-auth";
const AUD: &str = "phantom-cache-api";
const SECRET: &[u8] = b"sec-fixes-test-secret-0123456789abcdef";

fn validator() -> JwtValidator {
    JwtValidator::new(ISS, AUD, SECRET)
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

fn tok(roles: Vec<&str>, tenant: Option<&str>) -> String {
    validator().create_token(&JwtClaims::new("u", ISS, AUD, now() + 3600, tenant, roles)).unwrap()
}

fn expired_tok() -> String {
    validator()
        .create_token(&JwtClaims::new("u", ISS, AUD, now().saturating_sub(3600), Some("t1"), vec!["cache:read"]))
        .unwrap()
}

fn wrong_aud_tok() -> String {
    JwtValidator::new(ISS, "some-other-api", SECRET)
        .create_token(&JwtClaims::new("u", ISS, "some-other-api", now() + 3600, Some("t1"), vec!["cache:read"]))
        .unwrap()
}

fn wrong_iss_tok() -> String {
    JwtValidator::new("some-other-issuer", AUD, SECRET)
        .create_token(&JwtClaims::new("u", "some-other-issuer", AUD, now() + 3600, Some("t1"), vec!["cache:read"]))
        .unwrap()
}

fn bad_sig_tok() -> String {
    JwtValidator::new(ISS, AUD, b"a-different-secret-0123456789abcdefghij")
        .create_token(&JwtClaims::new("u", ISS, AUD, now() + 3600, Some("t1"), vec!["cache:read"]))
        .unwrap()
}

// ------------------------------------------------------------------ raw TCP origin

struct Origin {
    url: String,
    requests: Arc<AtomicU64>,
}

async fn spawn_origin() -> Origin {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(AtomicU64::new(0));
    let r = requests.clone();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let r = r.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                let seq = r.fetch_add(1, Ordering::SeqCst) + 1;
                let body = format!("origin-body-{}", seq);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: public, max-age=60\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    Origin { url: format!("http://127.0.0.1:{}", port), requests }
}

// ------------------------------------------------------------------ gateway

fn api_state(security: Option<SecurityConfig>, origin: Option<&str>) -> Arc<ApiState> {
    let nodes = vec![Arc::new(VirtualNode::new(1))];
    for n in &nodes {
        n.set_state(NodeState::Active);
    }
    Arc::new(ApiState {
        snapshot: Mutex::new(SimulationSnapshot {
            version: "1.0".into(),
            timestamp_ms: 0,
            throughput_req_sec: 0.0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            db_load_ratio: 0.0,
            avoided_compute_dollars: 0.0,
            nodes: vec![NodeSnapshot { node_id: 1, state: "Active".into(), cpu_utilization: 0.0, memory_used_bytes: 0 }],
        }),
        nodes,
        workload_type: Mutex::new("Normal".into()),
        cache: Some(Arc::new(PartitionedCache::new(4))),
        origin_url: origin.map(|s| s.to_string()),
        telemetry: None,
        pie: None,
        security: security.map(Arc::new),
        upstream_policy: None,
        router: None,
    })
}

/// JWT security with shared-cache semantics for authenticated reads. (Default config forces
/// `BYPASS-AUTH` for any request carrying Authorization; see the dedicated characterization test.)
fn secure_cfg() -> SecurityConfig {
    let mut c = SecurityConfig::new_with_jwt(validator());
    c.enforce_rfc9111_auth_bypass = false;
    c
}

async fn serve_app(app: axum::Router) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://{}", a)
}

struct Gw {
    base: String,
    state: Arc<ApiState>,
    c: reqwest::Client,
}

impl Gw {
    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.c.get(format!("{}{}", self.base, path))
    }
    fn counters(&self) -> (u64, u64, u64, u64, (usize, usize)) {
        let cache = self.state.cache.as_ref().unwrap();
        let s = cache.counters.snapshot();
        (s.requests_total, s.cache_hits, s.cache_misses, s.cache_bypasses, cache.audit_bytes())
    }
}

async fn gw(security: Option<SecurityConfig>, origin: Option<&str>) -> Gw {
    let state = api_state(security, origin);
    let base = serve_app(create_app(state.clone())).await;
    Gw { base, state, c: reqwest::Client::builder().pool_max_idle_per_host(0).build().unwrap() }
}

fn bearer(t: &str) -> String {
    format!("Bearer {}", t)
}

// ==================================================================== SEC-M7-01

#[tokio::test]
async fn sec01_valid_token_reads_and_second_read_is_a_hit() {
    let origin = spawn_origin().await;
    let g = gw(Some(secure_cfg()), Some(&origin.url)).await;
    let t = tok(vec!["cache:read"], Some("t1"));
    let r1 = g.get("/api/v1/cache/data/a").header("Authorization", bearer(&t)).send().await.unwrap();
    assert_eq!(r1.status(), 200);
    assert_eq!(r1.headers().get("x-cache-status").unwrap(), "MISS");
    let r2 = g.get("/api/v1/cache/data/a").header("Authorization", bearer(&t)).send().await.unwrap();
    assert_eq!(r2.status(), 200);
    assert_eq!(r2.headers().get("x-cache-status").unwrap(), "HIT");
    assert_eq!(origin.requests.load(Ordering::SeqCst), 1, "second read must be served from cache");
}

#[tokio::test]
async fn sec01_every_credential_failure_is_401_with_challenge_and_causes_no_side_effects() {
    let origin = spawn_origin().await;
    let g = gw(Some(secure_cfg()), Some(&origin.url)).await;
    let before = g.counters();

    let cases: Vec<(&str, Option<String>)> = vec![
        ("missing", None),
        ("malformed", Some("Bearer not.a.jwt".to_string())),
        ("garbage", Some("Bearer ???".to_string())),
        ("empty bearer", Some("Bearer ".to_string())),
        ("expired", Some(bearer(&expired_tok()))),
        ("wrong audience", Some(bearer(&wrong_aud_tok()))),
        ("wrong issuer", Some(bearer(&wrong_iss_tok()))),
        ("bad signature", Some(bearer(&bad_sig_tok()))),
    ];
    for (name, auth) in cases {
        let mut req = g.get("/api/v1/cache/secret/path");
        if let Some(a) = auth {
            req = req.header("Authorization", a);
        }
        let r = req.send().await.unwrap();
        assert_eq!(r.status(), 401, "{}", name);
        assert_eq!(
            r.headers().get("www-authenticate").map(|v| v.to_str().unwrap()),
            Some("Bearer"),
            "{}: 401 must carry a Bearer challenge",
            name
        );
        assert!(r.headers().get("x-cache-status").is_none(), "{}", name);
    }
    assert_eq!(origin.requests.load(Ordering::SeqCst), 0, "denied requests must cause zero origin fetches");
    assert_eq!(g.counters(), before, "denied requests must not touch counters or cache state");
}

#[tokio::test]
async fn sec01_role_without_cache_read_is_403_and_causes_no_side_effects() {
    let origin = spawn_origin().await;
    let g = gw(Some(secure_cfg()), Some(&origin.url)).await;
    let before = g.counters();
    for roles in [vec!["telemetry:read"], vec!["cache:invalidate"], vec!["scenario:operate"], vec![]] {
        let t = tok(roles.clone(), Some("t1"));
        let r = g.get("/api/v1/cache/x").header("Authorization", bearer(&t)).send().await.unwrap();
        assert_eq!(r.status(), 403, "roles {:?}", roles);
        assert!(r.headers().get("www-authenticate").is_none(), "403 is not a challenge");
    }
    assert_eq!(origin.requests.load(Ordering::SeqCst), 0);
    assert_eq!(g.counters(), before);
}

#[tokio::test]
async fn sec01_cache_bypass_header_does_not_skip_authentication() {
    let origin = spawn_origin().await;
    let g = gw(Some(secure_cfg()), Some(&origin.url)).await;
    let before = g.counters();
    // no token + bypass -> 401, origin untouched, cache_bypasses not incremented
    let r = g.get("/api/v1/cache/bypass/me").header("x-cache-bypass", "true").send().await.unwrap();
    assert_eq!(r.status(), 401);
    // bad token + bypass -> 401
    let r = g
        .get("/api/v1/cache/bypass/me")
        .header("x-cache-bypass", "TRUE")
        .header("Authorization", bearer(&bad_sig_tok()))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 401);
    // insufficient role + bypass -> 403
    let r = g
        .get("/api/v1/cache/bypass/me")
        .header("x-cache-bypass", "true")
        .header("Authorization", bearer(&tok(vec!["telemetry:read"], None)))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);
    assert_eq!(origin.requests.load(Ordering::SeqCst), 0, "bypass without authorization must never reach the origin");
    assert_eq!(g.counters(), before);

    // control: an authorized bypass still works and does reach the origin
    let r = g
        .get("/api/v1/cache/bypass/me")
        .header("x-cache-bypass", "true")
        .header("Authorization", bearer(&tok(vec!["cache:read"], Some("t1"))))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers().get("x-cache-status").unwrap(), "BYPASS");
    assert_eq!(origin.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn sec01_root_cache_path_is_also_protected() {
    let origin = spawn_origin().await;
    let g = gw(Some(secure_cfg()), Some(&origin.url)).await;
    let r = g.get("/api/v1/cache").send().await.unwrap();
    assert_eq!(r.status(), 401);
    assert_eq!(origin.requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn sec01_tenant_boundary_and_anonymous_cannot_read_authenticated_entries() {
    let origin = spawn_origin().await;
    let g = gw(Some(secure_cfg()), Some(&origin.url)).await;
    let ta = tok(vec!["cache:read"], Some("tenant-a"));
    let tb = tok(vec!["cache:read"], Some("tenant-b"));

    // A fills the entry.
    let r = g.get("/api/v1/cache/shared/path").header("Authorization", bearer(&ta)).send().await.unwrap();
    assert_eq!((r.status().as_u16(), r.headers().get("x-cache-status").unwrap().to_str().unwrap()), (200, "MISS"));
    assert_eq!(origin.requests.load(Ordering::SeqCst), 1);
    // A again: HIT.
    let r = g.get("/api/v1/cache/shared/path").header("Authorization", bearer(&ta)).send().await.unwrap();
    assert_eq!(r.headers().get("x-cache-status").unwrap(), "HIT");

    // B, same path, must NOT see A's entry: a MISS and its own origin fetch.
    let r = g.get("/api/v1/cache/shared/path").header("Authorization", bearer(&tb)).send().await.unwrap();
    assert_eq!(r.headers().get("x-cache-status").unwrap(), "MISS", "tenant B must not read tenant A's entry");
    assert_eq!(origin.requests.load(Ordering::SeqCst), 2);

    // A spoofed x-tenant-id never changes the tenant: B's token + header "tenant-a" still MISSes/HITs under B.
    let r = g
        .get("/api/v1/cache/shared/path")
        .header("Authorization", bearer(&tb))
        .header("x-tenant-id", "tenant-a")
        .send()
        .await
        .unwrap();
    assert_eq!(r.headers().get("x-cache-status").unwrap(), "HIT", "B's own entry, not A's");
    assert_eq!(origin.requests.load(Ordering::SeqCst), 2);

    // Anonymous (no token, even with x-tenant-id of the victim) cannot read it.
    let before = g.counters();
    let r = g.get("/api/v1/cache/shared/path").header("x-tenant-id", "tenant-a").send().await.unwrap();
    assert_eq!(r.status(), 401);
    assert_eq!(origin.requests.load(Ordering::SeqCst), 2);
    assert_eq!(g.counters(), before);
}

#[tokio::test]
async fn sec01_raw_token_without_bearer_prefix_is_still_accepted_characterization_sec_m7_09() {
    // CHARACTERIZATION of open finding SEC-M7-09 (not a fix): a bare token without the
    // `Bearer ` prefix is accepted by authenticate_request. Kept unchanged on purpose; this test
    // documents it so a future change to the policy is a deliberate, visible decision.
    let origin = spawn_origin().await;
    let g = gw(Some(secure_cfg()), Some(&origin.url)).await;
    let t = tok(vec!["cache:read"], Some("t1"));
    let r = g.get("/api/v1/cache/raw/token").header("Authorization", t).send().await.unwrap();
    assert_eq!(r.status(), 200);
}

#[tokio::test]
async fn sec01_enforce_rfc9111_bypass_true_serves_bypass_auth_forwarding_variant() {
    // Round 2 (M2): this REPLACES the former default-config characterization test. main.rs now sets
    // enforce_rfc9111_auth_bypass = PHANTOM_FORWARD_UPSTREAM_AUTH, so enforce=true is the
    // credential-forwarding configuration: every authenticated read is BYPASS-AUTH. The default
    // (non-forwarding) behaviour is covered end to end through the real binary by
    // m7_sec_round2_tests::e2e_m2_*.
    let origin = spawn_origin().await;
    let g = gw(Some(SecurityConfig::new_with_jwt(validator())), Some(&origin.url)).await;
    let t = tok(vec!["cache:read"], Some("t1"));
    for _ in 0..2 {
        let r = g.get("/api/v1/cache/auth/bypass").header("Authorization", bearer(&t)).send().await.unwrap();
        assert_eq!(r.status(), 200);
        assert_eq!(r.headers().get("x-cache-status").unwrap(), "BYPASS-AUTH");
    }
    assert_eq!(origin.requests.load(Ordering::SeqCst), 2);
    // ... and authentication is still enforced before that branch:
    let r = g.get("/api/v1/cache/auth/bypass").send().await.unwrap();
    assert_eq!(r.status(), 401);
    assert_eq!(origin.requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn sec01_security_disabled_dev_mode_is_unchanged() {
    let origin = spawn_origin().await;
    let g = gw(None, Some(&origin.url)).await;
    let r1 = g.get("/api/v1/cache/dev/a").send().await.unwrap();
    assert_eq!(r1.status(), 200);
    assert_eq!(r1.headers().get("x-cache-status").unwrap(), "MISS");
    let r2 = g.get("/api/v1/cache/dev/a").send().await.unwrap();
    assert_eq!(r2.headers().get("x-cache-status").unwrap(), "HIT");
    // x-tenant-id still selects the tenant in dev mode (no security configured)
    let r3 = g.get("/api/v1/cache/dev/a").header("x-tenant-id", "other").send().await.unwrap();
    assert_eq!(r3.headers().get("x-cache-status").unwrap(), "MISS");
    assert_eq!(origin.requests.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn sec01_health_endpoints_stay_public_in_secure_mode() {
    let g = gw(Some(secure_cfg()), None).await;
    for p in ["/health", "/health/live", "/health/ready"] {
        let r = g.get(p).send().await.unwrap();
        assert_ne!(r.status().as_u16(), 401, "{}", p);
        assert_ne!(r.status().as_u16(), 403, "{}", p);
    }
    assert_eq!(g.get("/health").send().await.unwrap().status(), 200);
    assert_eq!(g.get("/health/live").send().await.unwrap().status(), 200);
}

// ==================================================================== SEC-M7-02

static INTEL_ONCE: Once = Once::new();

fn candidate(id: &str) -> PolicyCandidate {
    PolicyCandidate {
        candidate_id: id.to_string(),
        kind: PolicyKind::Eviction,
        parameters: BTreeMap::new(),
        bounds: BTreeMap::new(),
        expected_effect: "test".to_string(),
    }
}

/// Installs the process-global audit store with `N_AUDIT` records, once.
const N_AUDIT: u64 = 130; // > default capacity 100: retained 100, lifetime 130
fn install_intelligence() {
    INTEL_ONCE.call_once(|| {
        let exec = DefaultPolicyExecutor::new(None);
        exec.set_mode(IntelligenceMode::Advisory).ok();
        let mut store = AuditStore::new(100);
        for i in 1..=N_AUDIT {
            store.append(DecisionRecord {
                decision_id: i,
                timestamp_ms: 1_000 + i,
                mode: IntelligenceMode::Advisory,
                trigger_reason: "test".into(),
                candidate: candidate(&format!("C{}", i)),
                estimated_effect: 0.1,
                confidence_interval: (0.0, 0.2),
                guardrails_passed: true,
                verdict: Verdict::Commit,
                explanation: "e".into(),
                config_hash_before: "b".into(),
                config_hash_after: "a".into(),
            });
        }
        ApiState::set_intelligence(Arc::new(exec), Arc::new(Mutex::new(store)));
    });
}

async fn intel_gw(security: Option<SecurityConfig>) -> Gw {
    install_intelligence();
    gw(security, None).await
}

#[tokio::test]
async fn sec02_observer_gets_200_anonymous_401_other_roles_403() {
    let g = intel_gw(Some(secure_cfg())).await;
    let obs = tok(vec!["telemetry:read"], None);
    for p in ["/api/v1/intelligence/status", "/api/v1/intelligence/audit"] {
        let r = g.get(p).header("Authorization", bearer(&obs)).send().await.unwrap();
        assert_eq!(r.status(), 200, "{} observer", p);
        let r = g.get(p).send().await.unwrap();
        assert_eq!(r.status(), 401, "{} anonymous", p);
        assert_eq!(r.headers().get("www-authenticate").unwrap(), "Bearer");
        for bad in [expired_tok(), wrong_aud_tok(), wrong_iss_tok(), bad_sig_tok(), "x.y.z".to_string()] {
            let r = g.get(p).header("Authorization", bearer(&bad)).send().await.unwrap();
            assert_eq!(r.status(), 401, "{} invalid token", p);
        }
        for roles in [vec!["cache:read"], vec!["cache:invalidate"], vec!["scenario:operate"], vec![]] {
            let t = tok(roles.clone(), None);
            let r = g.get(p).header("Authorization", bearer(&t)).send().await.unwrap();
            assert_eq!(r.status(), 403, "{} roles {:?}", p, roles);
        }
        // admin satisfies Observer
        let r = g.get(p).header("Authorization", bearer(&tok(vec!["admin"], None))).send().await.unwrap();
        assert_eq!(r.status(), 200, "{} admin", p);
    }
}

#[tokio::test]
async fn sec02_security_disabled_is_unchanged_and_keeps_response_shape() {
    let g = intel_gw(None).await;
    let r = g.get("/api/v1/intelligence/status").send().await.unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = r.json().await.unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["intelligence_mode"], "Advisory");
    let r = g.get("/api/v1/intelligence/audit").send().await.unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = r.json().await.unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["total_records"], N_AUDIT);
    assert_eq!(v["retained_records"], 100);
}

#[tokio::test]
async fn sec02_status_exposes_only_the_documented_fields() {
    let g = intel_gw(Some(secure_cfg())).await;
    let r = g
        .get("/api/v1/intelligence/status")
        .header("Authorization", bearer(&tok(vec!["telemetry:read"], None)))
        .send()
        .await
        .unwrap();
    let v: serde_json::Value = r.json().await.unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(|k| k.as_str()).collect();
    keys.sort();
    assert_eq!(
        keys,
        vec!["active_candidate", "intelligence_mode", "revert_count", "status", "truthful", "truthful_basis"],
        "status response fields changed: update docs/user/security.md and re-review for sensitive data"
    );
    let text = v.to_string();
    assert!(!text.contains(std::str::from_utf8(SECRET).unwrap()), "secret must never appear");
}

async fn audit(g: &Gw, query: &str) -> reqwest::Response {
    g.get(&format!("/api/v1/intelligence/audit{}", query))
        .header("Authorization", bearer(&tok(vec!["telemetry:read"], None)))
        .send()
        .await
        .unwrap()
}

#[tokio::test]
async fn sec02_audit_pagination_boundaries() {
    let g = intel_gw(Some(secure_cfg())).await;

    // default: limit 50, offset 0, oldest-first. Retained are decision_id 31..=130.
    let v: serde_json::Value = audit(&g, "").await.json().await.unwrap();
    assert_eq!(v["limit"], 50);
    assert_eq!(v["offset"], 0);
    assert_eq!(v["returned"], 50);
    assert_eq!(v["total_records"], N_AUDIT);
    assert_eq!(v["retained_records"], 100);
    assert_eq!(v["ordering"], "oldest_first");
    assert_eq!(v["records"][0]["decision_id"], 31);
    assert_eq!(v["records"][49]["decision_id"], 80);

    // limit 1, limit 100 accepted
    let v: serde_json::Value = audit(&g, "?limit=1").await.json().await.unwrap();
    assert_eq!(v["returned"], 1);
    assert_eq!(v["records"][0]["decision_id"], 31);
    let v: serde_json::Value = audit(&g, "?limit=100").await.json().await.unwrap();
    assert_eq!(v["returned"], 100);
    assert_eq!(v["records"][99]["decision_id"], 130);

    // stable pages: offset walks the same ordering without gaps or overlap
    let v: serde_json::Value = audit(&g, "?limit=50&offset=50").await.json().await.unwrap();
    assert_eq!(v["returned"], 50);
    assert_eq!(v["records"][0]["decision_id"], 81);
    let v: serde_json::Value = audit(&g, "?limit=30&offset=90").await.json().await.unwrap();
    assert_eq!(v["returned"], 10, "partial last page");
    assert_eq!(v["records"][9]["decision_id"], 130);

    // offset at / past the end: 200 with empty records and totals still present
    for q in ["?offset=100", "?offset=101", "?offset=999999999999"] {
        let r = audit(&g, q).await;
        assert_eq!(r.status(), 200, "{}", q);
        let v: serde_json::Value = r.json().await.unwrap();
        assert_eq!(v["returned"], 0, "{}", q);
        assert_eq!(v["records"].as_array().unwrap().len(), 0);
        assert_eq!(v["total_records"], N_AUDIT);
        assert_eq!(v["retained_records"], 100);
    }

    // invalid limit / offset -> 400
    for q in [
        "?limit=0",
        "?limit=101",
        "?limit=999999999999",
        "?limit=abc",
        "?limit=",
        "?limit=-1",
        "?limit=1.5",
        "?limit=%2B5",
        "?limit=99999999999999999999999",
        "?offset=abc",
        "?offset=-1",
        "?offset=",
        "?offset=1.5",
        "?offset=99999999999999999999999",
        "?limit=10&offset=x",
        "?limit",
    ] {
        assert_eq!(audit(&g, q).await.status(), 400, "{}", q);
    }
}

#[tokio::test]
async fn sec02_unauthorized_requests_are_rejected_before_query_validation() {
    let g = intel_gw(Some(secure_cfg())).await;
    // invalid pagination but no token: must be 401, not 400 (auth first, no information leak)
    let r = g.get("/api/v1/intelligence/audit?limit=abc").send().await.unwrap();
    assert_eq!(r.status(), 401);
}

// ==================================================================== SEC-M7-03

fn cors_app(origins: &[&str]) -> CorsConfig {
    CorsConfig::with_origins(origins.iter().map(|s| s.to_string())).unwrap()
}

async fn cors_gw(cfg: &CorsConfig, security: Option<SecurityConfig>) -> Gw {
    let state = api_state(security, None);
    let base = serve_app(create_app_with_cors(state.clone(), cfg)).await;
    Gw { base, state, c: reqwest::Client::new() }
}

#[tokio::test]
async fn sec03_default_create_app_allows_no_cross_origin_access() {
    // Before the fix: CorsLayer::permissive() answered `access-control-allow-origin: *` for any
    // Origin (the old m7_finding_cors_is_permissive asserted exactly that).
    let base = serve_app(create_app(api_state(None, None))).await;
    let c = reqwest::Client::new();
    for origin in ["https://evil.example", "http://localhost:3000", "http://127.0.0.1:3000", "null"] {
        let r = c.get(format!("{}/health", base)).header("Origin", origin).send().await.unwrap();
        assert_eq!(r.status(), 200);
        assert!(r.headers().get("access-control-allow-origin").is_none(), "default must not allow {}", origin);
        assert!(r.headers().get("access-control-allow-credentials").is_none());
        // preflight too
        let r = c
            .request(reqwest::Method::OPTIONS, format!("{}/api/v1/cache/x", base))
            .header("Origin", origin)
            .header("Access-Control-Request-Method", "GET")
            .header("Access-Control-Request-Headers", "authorization")
            .send()
            .await
            .unwrap();
        assert!(r.headers().get("access-control-allow-origin").is_none(), "preflight default must not allow {}", origin);
    }
}

#[tokio::test]
async fn sec03_allowed_origin_gets_acao_and_vary_and_exposed_headers() {
    let cfg = cors_app(&["http://localhost:3000", "https://ui.example.com"]);
    let g = cors_gw(&cfg, None).await;
    for o in ["http://localhost:3000", "https://ui.example.com"] {
        let r = g.get("/health").header("Origin", o).send().await.unwrap();
        assert_eq!(r.headers().get("access-control-allow-origin").unwrap(), o);
        assert!(r.headers().get("access-control-allow-credentials").is_none(), "no credentials, ever");
        let vary = r.headers().get_all("vary").iter().map(|v| v.to_str().unwrap().to_ascii_lowercase()).collect::<Vec<_>>().join(",");
        assert!(vary.contains("origin"), "Vary: Origin required, got {:?}", vary);
        let expose = r.headers().get("access-control-expose-headers").unwrap().to_str().unwrap().to_ascii_lowercase();
        for h in ["x-cache-status", "age", "retry-after", "x-cache-error", "content-type"] {
            assert!(expose.contains(h), "expose-headers must list {}: {}", h, expose);
        }
    }
}

#[tokio::test]
async fn sec03_disallowed_origins_get_nothing() {
    let cfg = cors_app(&["http://localhost:3000"]);
    let g = cors_gw(&cfg, None).await;
    for o in [
        "https://evil.example",
        "http://localhost:3001",
        "https://localhost:3000",
        "http://localhost:3000.evil.example",
        "http://LOCALHOST:3000",
        "null",
    ] {
        let r = g.get("/health").header("Origin", o).send().await.unwrap();
        assert!(r.headers().get("access-control-allow-origin").is_none(), "{} must not be allowed", o);
        assert!(r.headers().get("access-control-allow-credentials").is_none());
    }
}

#[tokio::test]
async fn sec03_preflight_for_authorization_succeeds_only_for_allowed_origins() {
    let cfg = cors_app(&["http://localhost:3000"]);
    let g = cors_gw(&cfg, Some(secure_cfg())).await;
    let pre = |origin: &'static str, method: &'static str, headers: &'static str| {
        g.c.request(reqwest::Method::OPTIONS, format!("{}/api/v1/cache/x", g.base))
            .header("Origin", origin)
            .header("Access-Control-Request-Method", method)
            .header("Access-Control-Request-Headers", headers)
    };
    let r = pre("http://localhost:3000", "GET", "authorization,content-type,x-tenant-id,x-cache-bypass").send().await.unwrap();
    assert!(r.status().is_success(), "preflight status {}", r.status());
    assert_eq!(r.headers().get("access-control-allow-origin").unwrap(), "http://localhost:3000");
    let allow_h = r.headers().get("access-control-allow-headers").unwrap().to_str().unwrap().to_ascii_lowercase();
    for h in ["authorization", "content-type", "x-tenant-id", "x-cache-bypass"] {
        assert!(allow_h.contains(h), "allow-headers must list {}: {}", h, allow_h);
    }
    let allow_m = r.headers().get("access-control-allow-methods").unwrap().to_str().unwrap().to_ascii_uppercase();
    for m in ["GET", "POST", "DELETE", "OPTIONS"] {
        assert!(allow_m.contains(m), "allow-methods must list {}: {}", m, allow_m);
    }
    assert!(!allow_m.contains("PUT") && !allow_m.contains("PATCH"));
    assert!(r.headers().get("access-control-allow-credentials").is_none());
    let max_age: u64 = r.headers().get("access-control-max-age").unwrap().to_str().unwrap().parse().unwrap();
    assert!((1..=3600).contains(&max_age), "modest max-age, got {}", max_age);

    // disallowed origin: no ACAO on the preflight
    let r = pre("https://evil.example", "GET", "authorization").send().await.unwrap();
    assert!(r.headers().get("access-control-allow-origin").is_none());
}

#[tokio::test]
async fn sec03_requests_without_origin_are_unaffected() {
    let cfg = cors_app(&["http://localhost:3000"]);
    let g = cors_gw(&cfg, Some(secure_cfg())).await;
    let r = g.get("/health").send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers().get("access-control-allow-origin").is_none());
    // CORS never substitutes for authentication: an allowed Origin without a token is still 401.
    let r = g.get("/api/v1/cache/x").header("Origin", "http://localhost:3000").send().await.unwrap();
    assert_eq!(r.status(), 401);
}

#[tokio::test]
async fn sec03_cors_origin_does_not_bypass_authz_on_real_router() {
    // Allowed browser origin + valid token works; the CORS header is present on the 200 too.
    let origin = spawn_origin().await;
    let cfg = cors_app(&["http://localhost:3000"]);
    let state = api_state(Some(secure_cfg()), Some(&origin.url));
    let base = serve_app(create_app_with_cors(state, &cfg)).await;
    let r = reqwest::Client::new()
        .get(format!("{}/api/v1/cache/ui/x", base))
        .header("Origin", "http://localhost:3000")
        .header("Authorization", bearer(&tok(vec!["cache:read"], Some("t1"))))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers().get("access-control-allow-origin").unwrap(), "http://localhost:3000");
    assert!(r.headers().get("access-control-expose-headers").unwrap().to_str().unwrap().to_ascii_lowercase().contains("x-cache-status"));
}

#[test]
fn sec03_cors_config_validation_rejects_bad_entries() {
    let bad = [
        "*",
        "http://*.example.com",
        "http://localhost:3000/",
        "http://localhost:3000/path",
        "http://localhost:3000?x=1",
        "http://localhost:3000#f",
        "ftp://example.com",
        "file:///etc/passwd",
        "localhost:3000",
        "javascript:alert(1)",
        "http://user:pw@example.com",
        "http://",
        "http://a.com,,http://b.com",
        "http://a.com,",
        ",http://a.com",
        "http://exa mple.com",
        "null",
        "",
    ];
    for b in bad {
        let r = CorsConfig::with_origins(vec![b.to_string()]);
        if b.is_empty() {
            // empty means "no origins" (same-origin) at this API level; the config layer treats an
            // empty value as unset, and entries with empty segments are rejected below.
            assert!(r.is_ok());
        } else {
            assert!(r.is_err(), "{:?} must be rejected", b);
        }
    }
    let many: Vec<String> = (0..17).map(|i| format!("http://h{}.example.com", i)).collect();
    assert!(CorsConfig::with_origins(many).is_err(), "17 entries must be rejected");
    let ok16: Vec<String> = (0..16).map(|i| format!("http://h{}.example.com", i)).collect();
    assert_eq!(CorsConfig::with_origins(ok16).unwrap().allowed_origins().len(), 16);
    // normalization: default port dropped, host lowercased, duplicates removed
    let c = CorsConfig::with_origins(vec!["HTTPS://UI.Example.com:443,https://ui.example.com".to_string()]).unwrap();
    assert_eq!(c.allowed_origins(), &["https://ui.example.com".to_string()]);
}

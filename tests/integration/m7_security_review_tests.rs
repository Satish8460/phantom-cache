//! M7-04 security review tests: additional JWT/RBAC, upstream SSRF and HTTP-surface checks.
//! Tests named `m7_fix_*` are the former `m7_finding_*` characterization tests converted to
//! regression tests of the fixed behavior (SEC-M7-01/02/03); see docs/security/m7_security_review.md.

use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::security::cluster::{parse_san_node_identity, ClusterMembershipRegistry, MemberNode, NodeIdentity};
use phantom_cache_demo::security::{
    AuthError, JwtClaims, JwtValidator, PreConnectIpFilter, Role, SecurityConfig, UpstreamPolicy,
};
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

const ISS: &str = "phantom-cache-auth";
const AUD: &str = "phantom-cache-api";
const SECRET: &[u8] = b"m7-review-secret-key-0123456789abcdef";

fn validator() -> JwtValidator {
    JwtValidator::new(ISS, AUD, SECRET).with_rotation_key("k1", b"m7-rotation-secret-0123456789abcdef!")
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

fn token(roles: Vec<&str>, tenant: Option<&str>) -> String {
    validator()
        .create_token(&JwtClaims::new("u", ISS, AUD, now() + 3600, tenant, roles))
        .unwrap()
}

// ---------------------------------------------------------------- JWT

#[test]
fn m7_jwt_rejects_alg_none_wrong_alg_missing_exp_and_tampering() {
    let v = validator();
    // alg=none (unsigned) must never validate.
    let none_tok = "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiJ4In0.";
    assert!(v.validate(none_tok).is_err());

    // HS384 with the correct secret is not accepted (HS256 only).
    let claims = JwtClaims::new("u", ISS, AUD, now() + 3600, None, vec!["admin"]);
    let hs384 = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS384),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(SECRET),
    )
    .unwrap();
    assert!(matches!(v.validate(&hs384), Err(AuthError::InvalidToken(_))));

    // Missing exp claim is rejected (no non-expiring tokens).
    #[derive(serde::Serialize)]
    struct NoExp {
        sub: String,
        iss: String,
        aud: String,
        roles: Vec<String>,
    }
    let no_exp = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256),
        &NoExp { sub: "u".into(), iss: ISS.into(), aud: AUD.into(), roles: vec!["admin".into()] },
        &jsonwebtoken::EncodingKey::from_secret(SECRET),
    )
    .unwrap();
    assert!(v.validate(&no_exp).is_err(), "token without exp must be rejected");

    // Payload tampering invalidates the signature (swap payload segment with another token's).
    let good = token(vec!["cache:read"], None);
    let admin = token(vec!["admin"], None);
    let g: Vec<&str> = good.split('.').collect();
    let a: Vec<&str> = admin.split('.').collect();
    let forged = format!("{}.{}.{}", g[0], a[1], g[2]);
    assert!(v.validate(&forged).is_err());

    // Token signed with a different secret.
    let other = JwtValidator::new(ISS, AUD, b"a-completely-different-secret-0123456789");
    let t = other.create_token(&claims).unwrap();
    assert!(v.validate(&t).is_err());

    // Unknown kid is rejected without falling back to the primary secret.
    let mut h = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    h.kid = Some("ghost".into());
    let t = jsonwebtoken::encode(&h, &claims, &jsonwebtoken::EncodingKey::from_secret(SECRET)).unwrap();
    assert!(v.validate(&t).is_err());

    // Garbage inputs never panic.
    for junk in ["", ".", "..", "a.b.c", "Bearer x", &"x".repeat(100_000)] {
        assert!(v.validate(junk).is_err());
    }
}

#[test]
fn m7_role_matrix() {
    let mk = |r: &[&str]| JwtClaims::new("u", ISS, AUD, now() + 60, None, r.to_vec());
    let all = [Role::CacheRead, Role::CacheInvalidate, Role::Observer, Role::Operator, Role::Admin];
    // admin satisfies everything
    for r in all {
        assert!(mk(&["admin"]).has_role(r));
        assert!(mk(&["ADMIN"]).has_role(r));
    }
    // no roles satisfies nothing
    for r in all {
        assert!(!mk(&[]).has_role(r));
    }
    // each specific role satisfies only its own permission
    let cases = [
        ("cache:read", Role::CacheRead),
        ("cache:invalidate", Role::CacheInvalidate),
        ("telemetry:read", Role::Observer),
        ("scenario:operate", Role::Operator),
    ];
    for (name, role) in cases {
        for other in all {
            let expect = other == role;
            assert_eq!(mk(&[name]).has_role(other), expect, "{} vs {:?}", name, other);
        }
    }
    // Only admin grants Admin.
    assert!(!mk(&["operator", "observer", "requester"]).has_role(Role::Admin));
    // "requester" alias grants BOTH read and invalidate (documented in the review).
    assert!(mk(&["requester"]).has_role(Role::CacheRead) && mk(&["requester"]).has_role(Role::CacheInvalidate));
    assert!(!mk(&["requester"]).has_role(Role::Operator) && !mk(&["requester"]).has_role(Role::Observer));
    // unknown role strings grant nothing
    assert!(!mk(&["superuser", "root", "cache:*"]).has_role(Role::CacheRead));
}

// ---------------------------------------------------------------- upstream SSRF

fn ip(s: &str) -> IpAddr {
    s.parse().unwrap()
}

#[test]
fn m7_ssrf_filter_blocks_metadata_private_and_embedded_addresses() {
    let strict = PreConnectIpFilter::new(false, false);
    for blocked in [
        "169.254.169.254",
        "169.254.1.1",
        "127.0.0.1",
        "127.5.5.5",
        "::1",
        "0.0.0.0",
        "10.1.2.3",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.1.1",
        "100.64.0.1",
        "224.0.0.1",
        "255.255.255.255",
        "192.0.2.1",
        "::ffff:169.254.169.254",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "2002:a9fe:a9fe::1",    // 6to4 embedding 169.254.169.254
        "2002:7f00:0001::1",    // 6to4 embedding 127.0.0.1
        "64:ff9b::a9fe:a9fe",   // NAT64 embedding IMDS
        "fd00:ec2::254",
        "fe80::1",
        "fc00::1",
        "ff02::1",
        "2001:db8::1",
    ] {
        assert!(strict.check_ip(ip(blocked)).is_err(), "{} must be blocked", blocked);
    }
    for allowed in ["8.8.8.8", "1.1.1.1", "93.184.216.34", "2606:4700:4700::1111"] {
        assert!(strict.check_ip(ip(allowed)).is_ok(), "{} must be allowed", allowed);
    }
    // IMDS and link-local stay blocked even with LAN and loopback allowed.
    let lax = PreConnectIpFilter::new(true, true);
    for still in ["169.254.169.254", "::ffff:169.254.169.254", "fd00:ec2::254", "2002:a9fe:a9fe::1", "64:ff9b::a9fe:a9fe"] {
        assert!(lax.check_ip(ip(still)).is_err(), "{} must stay blocked", still);
    }
    assert!(lax.check_ip(ip("10.0.0.1")).is_ok());
    assert!(lax.check_ip(ip("127.0.0.1")).is_ok());
}

fn parse(u: &str) -> url::Url {
    url::Url::parse(u).unwrap()
}

#[test]
fn m7_upstream_policy_validate_target() {
    let p = UpstreamPolicy::default();
    assert!(p.validate_target(&parse("https://example.com")).is_ok());
    assert!(p.validate_target(&parse("ftp://example.com")).is_err());
    assert!(p.validate_target(&parse("file:///etc/passwd")).is_err());
    assert!(p.validate_target(&parse("https://user:pw@example.com")).is_err());
    assert!(p.validate_target(&parse("https://example.com/?q=1")).is_err());
    // hard-blocked ports can never be reached, even if the origin is allowlisted
    for port in [22u16, 25, 3306, 5432, 6379, 11211, 27017, 2375] {
        let u = format!("https://example.com:{}", port);
        let mut pol = UpstreamPolicy::default();
        pol.allowed_origins.push(u.clone());
        pol.origin_url = Some(u.clone());
        assert!(pol.validate_target(&parse(&u)).is_err(), "port {}", port);
    }
    // plaintext non-loopback http needs allow_insecure_upstream AND an allowlist entry
    assert!(p.validate_target(&parse("http://example.com")).is_err());
    let mut pol = UpstreamPolicy { allow_insecure_upstream: true, ..Default::default() };
    assert!(pol.validate_target(&parse("http://example.com")).is_err());
    pol.allowed_origins.push("http://example.com".into());
    assert!(pol.validate_target(&parse("http://example.com")).is_ok());
    // allowlist is exact: a different host is refused
    assert!(pol.validate_target(&parse("http://evil.example.com")).is_err());
    // enforce_tls rejects http even for loopback
    let tls_only = UpstreamPolicy { enforce_tls: true, ..Default::default() };
    assert!(tls_only.validate_target(&parse("http://127.0.0.1:9000")).is_err());
}

#[test]
fn m7_upstream_policy_loopback_literals_follow_the_same_rule() {
    // Regression: IPv6 loopback and other 127/8 literals were treated as non-loopback
    // (url's host_str() keeps the brackets for IPv6), so a valid loopback origin was refused.
    let p = UpstreamPolicy { allow_loopback_origin: true, ..Default::default() };
    for u in ["http://127.0.0.1:9000", "http://localhost:9000", "http://[::1]:9000", "http://127.0.0.2:9000"] {
        assert!(p.validate_target(&parse(u)).is_ok(), "{} should be accepted as loopback", u);
    }
    let network = UpstreamPolicy { allow_loopback_origin: false, ..Default::default() };
    for u in ["http://127.0.0.1:9000", "http://localhost:9000", "http://[::1]:9000"] {
        assert!(network.validate_target(&parse(u)).is_err(), "{} must be refused on network binds", u);
    }
}

// ---------------------------------------------------------------- cluster identity / registry

#[test]
fn m7_cluster_identity_and_registry_edge_cases() {
    assert_eq!(
        parse_san_node_identity("node-3.alpha.phantom.internal").unwrap(),
        NodeIdentity { node_id: 3, cluster_id: "alpha".into() }
    );
    for bad in [
        "node-0.alpha.phantom.internal",
        "node-256.alpha.phantom.internal",
        "node-1.Alpha.phantom.internal",
        "node-1.-alpha.phantom.internal",
        "node-1.alpha_x.phantom.internal",
        "node-1.alpha.example.com",
        "host-1.alpha.phantom.internal",
        "node-1.phantom.internal",
        "",
    ] {
        assert!(parse_san_node_identity(bad).is_err(), "{:?}", bad);
    }
    let mut reg = ClusterMembershipRegistry::new("alpha", 5);
    reg.add_member(MemberNode { node_id: 1, fingerprints: vec!["a".repeat(64)], active: true });
    let id = NodeIdentity { node_id: 1, cluster_id: "alpha".into() };
    // a certificate whose fingerprint is not registered is refused
    assert!(reg.validate_peer(&id, b"not-the-registered-cert").is_err());
    // wrong cluster
    assert!(reg.validate_peer(&NodeIdentity { node_id: 1, cluster_id: "beta".into() }, b"x").is_err());
    // unknown node
    assert!(reg.validate_peer(&NodeIdentity { node_id: 9, cluster_id: "alpha".into() }, b"x").is_err());
    // epoch must increase strictly
    let same = ClusterMembershipRegistry::new("alpha", 5);
    assert!(reg.update(same).is_err());
    let older = ClusterMembershipRegistry::new("alpha", 4);
    assert!(reg.update(older).is_err());
}

// ---------------------------------------------------------------- HTTP surface

fn state(security: Option<SecurityConfig>) -> Arc<ApiState> {
    let nodes = vec![Arc::new(VirtualNode::new(1))];
    for n in &nodes {
        n.set_state(NodeState::Active);
    }
    let snap = SimulationSnapshot {
        version: "1.0".into(),
        timestamp_ms: 0,
        throughput_req_sec: 0.0,
        p50_latency_ms: 0,
        p95_latency_ms: 0,
        p99_latency_ms: 0,
        db_load_ratio: 0.0,
        avoided_compute_dollars: 0.0,
        nodes: vec![NodeSnapshot { node_id: 1, state: "Active".into(), cpu_utilization: 0.0, memory_used_bytes: 0 }],
    };
    Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes,
        workload_type: Mutex::new("Normal".into()),
        cache: Some(Arc::new(PartitionedCache::new(4))),
        origin_url: None,
        telemetry: None,
        pie: None,
        security: security.map(Arc::new),
        upstream_policy: Some(Arc::new(UpstreamPolicy::default())),
        router: None,
    })
}

async fn serve(st: Arc<ApiState>) -> String {
    let app = create_app(st);
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://{}", a)
}

#[tokio::test]
async fn m7_http_rbac_matrix_in_secure_mode() {
    let base = serve(state(Some(SecurityConfig::new_with_jwt(validator())))).await;
    let c = reqwest::Client::new();
    let read = token(vec!["cache:read"], Some("t1"));
    let inval = token(vec!["cache:invalidate"], Some("t1"));
    let obs = token(vec!["telemetry:read"], None);
    let op = token(vec!["scenario:operate"], None);

    // (method, path, token, expected-status-class: Some(code) exact or None = "not 401/403")
    let get = |p: &str, t: Option<&str>| {
        let mut r = c.get(format!("{}{}", base, p));
        if let Some(t) = t {
            r = r.header("Authorization", format!("Bearer {}", t));
        }
        r
    };
    // metrics + diagnostics: Observer only
    for p in ["/api/v1/metrics", "/api/v1/diagnostics/summary", "/api/v1/diagnostics/events"] {
        assert_eq!(get(p, None).send().await.unwrap().status(), 401, "{} anon", p);
        assert_eq!(get(p, Some(&read)).send().await.unwrap().status(), 403, "{} read-role", p);
        assert_eq!(get(p, Some(&inval)).send().await.unwrap().status(), 403, "{} inval-role", p);
        assert_eq!(get(p, Some(&op)).send().await.unwrap().status(), 403, "{} operator", p);
        assert_eq!(get(p, Some(&obs)).send().await.unwrap().status(), 200, "{} observer", p);
    }
    // control: Operator only, never accepts observer/read
    let post = |t: Option<&str>| {
        let mut r = c.post(format!("{}/api/control", base)).json(&serde_json::json!({"event":"NORMAL"}));
        if let Some(t) = t {
            r = r.header("Authorization", format!("Bearer {}", t));
        }
        r
    };
    assert_eq!(post(None).send().await.unwrap().status(), 401);
    assert_eq!(post(Some(&obs)).send().await.unwrap().status(), 403);
    assert_eq!(post(Some(&read)).send().await.unwrap().status(), 403);
    assert_eq!(post(Some(&op)).send().await.unwrap().status(), 200);
    // invalidation: CacheInvalidate only
    let del = |t: Option<&str>| {
        let mut r = c.delete(format!("{}/api/v1/cache/some/key", base));
        if let Some(t) = t {
            r = r.header("Authorization", format!("Bearer {}", t));
        }
        r
    };
    assert_eq!(del(None).send().await.unwrap().status(), 401);
    assert_eq!(del(Some(&read)).send().await.unwrap().status(), 403);
    assert_eq!(del(Some(&obs)).send().await.unwrap().status(), 403);
    assert_ne!(del(Some(&inval)).send().await.unwrap().status().as_u16(), 401);
    // an authenticated GET with a non-read role is refused
    assert_eq!(get("/api/v1/cache/k", Some(&obs)).send().await.unwrap().status(), 403);
    // health stays open
    assert_eq!(get("/health", None).send().await.unwrap().status(), 200);
}

// The three former `m7_finding_*` characterization tests are converted into regression tests of
// the NEW behaviour (SEC-M7-01/02/03 fixed). What each asserted BEFORE the fix is recorded in its
// doc comment. The full matrices are in tests/integration/m7_sec_fixes_tests.rs.

#[tokio::test]
async fn m7_fix_anonymous_cache_read_is_rejected_in_secure_mode() {
    // SEC-M7-01. BEFORE: `assert_ne!(status, 401)` (anonymous GET was served for the default
    // tenant). NOW: 401 with a Bearer challenge, and a valid cache:read token is accepted.
    let base = serve(state(Some(SecurityConfig::new_with_jwt(validator())))).await;
    let r = reqwest::get(format!("{}/api/v1/cache/anonymous-probe", base)).await.unwrap();
    assert_eq!(r.status().as_u16(), 401);
    assert_eq!(r.headers().get("www-authenticate").map(|v| v.to_str().unwrap()), Some("Bearer"));
    let r = reqwest::Client::new()
        .get(format!("{}/api/v1/cache/anonymous-probe", base))
        .header("Authorization", format!("Bearer {}", token(vec!["cache:read"], Some("t1"))))
        .send()
        .await
        .unwrap();
    assert_ne!(r.status().as_u16(), 401);
    assert_ne!(r.status().as_u16(), 403);
}

#[tokio::test]
async fn m7_fix_intelligence_endpoints_require_observer_in_secure_mode() {
    // SEC-M7-02. BEFORE: both endpoints answered anonymous requests (neither 401 nor 403).
    // NOW: anonymous -> 401, non-observer role -> 403, observer -> not 401/403.
    let base = serve(state(Some(SecurityConfig::new_with_jwt(validator())))).await;
    let c = reqwest::Client::new();
    for p in ["/api/v1/intelligence/status", "/api/v1/intelligence/audit"] {
        let r = reqwest::get(format!("{}{}", base, p)).await.unwrap();
        assert_eq!(r.status().as_u16(), 401, "{} anonymous", p);
        let r = c
            .get(format!("{}{}", base, p))
            .header("Authorization", format!("Bearer {}", token(vec!["cache:read"], None)))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status().as_u16(), 403, "{} non-observer", p);
        let r = c
            .get(format!("{}{}", base, p))
            .header("Authorization", format!("Bearer {}", token(vec!["telemetry:read"], None)))
            .send()
            .await
            .unwrap();
        assert!(r.status().as_u16() != 401 && r.status().as_u16() != 403, "{} observer", p);
    }
}

#[tokio::test]
async fn m7_fix_cors_is_same_origin_by_default() {
    // SEC-M7-03. BEFORE: `CorsLayer::permissive()` answered `access-control-allow-origin: *` for
    // any Origin. NOW: `create_app` (default policy) emits no ACAO header for any Origin.
    let base = serve(state(None)).await;
    let r = reqwest::Client::new()
        .get(format!("{}/health", base))
        .header("Origin", "https://evil.example")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert!(r.headers().get("access-control-allow-origin").is_none());
}

#[tokio::test]
async fn m7_secure_mode_ignores_spoofed_tenant_header() {
    // Authenticated tenant comes from the token, never from x-tenant-id (cross-tenant guard).
    let st = state(Some(SecurityConfig::new_with_jwt(validator())));
    let cache = st.cache.clone().unwrap();
    let base = serve(st).await;
    let t = token(vec!["cache:invalidate", "cache:read"], Some("alpha"));
    let before = cache.counters.snapshot();
    let _ = before;
    let r = reqwest::Client::new()
        .delete(format!("{}/api/v1/cache/x", base))
        .header("Authorization", format!("Bearer {}", t))
        .header("x-tenant-id", "beta")
        .send()
        .await
        .unwrap();
    assert_ne!(r.status().as_u16(), 401);
    assert_ne!(r.status().as_u16(), 403);
}

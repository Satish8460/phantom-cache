//! Round-2 security review fixes (H1, M1, M2, M3 docs, lows) for SEC-M7-01/02/03.
//!
//! In-process tests drive the real router (`create_app*`) over TCP with real HS256 JWTs. The
//! `e2e_*` tests spawn the REAL gateway binary (`CARGO_BIN_EXE_phantom-cache-demo`) with
//! environment configuration, so the wiring in `main.rs` (enforce_rfc9111_auth_bypass, CORS
//! allowlist) is exercised, not only library functions. Origins are raw-TCP servers that record the
//! exact request bytes.

use phantom_cache_demo::api::server::{create_app, create_app_with_cors, ApiState, CorsConfig, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::{CacheKey, PartitionedCache};
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::intel::telemetry::TelemetryCollector;
use phantom_cache_demo::security::{JwtClaims, JwtValidator, SecurityConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ISS: &str = "phantom-cache-auth";
const AUD: &str = "phantom-cache-api";
const SECRET: &str = "sec2-test-secret-0123456789abcdefghij";

fn validator() -> JwtValidator {
    JwtValidator::new(ISS, AUD, SECRET.as_bytes())
}
fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}
fn tok(roles: Vec<&str>, tenant: Option<&str>) -> String {
    validator().create_token(&JwtClaims::new("u", ISS, AUD, now() + 3600, tenant, roles)).unwrap()
}
fn bearer(t: &str) -> String {
    format!("Bearer {}", t)
}

// ------------------------------------------------------------------ recording origin

struct Origin {
    url: String,
    requests: Arc<AtomicU64>,
    heads: Arc<Mutex<Vec<Vec<u8>>>>,
}

async fn spawn_origin(cache_control: &'static str, delay_ms: u64) -> Origin {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(AtomicU64::new(0));
    let heads: Arc<Mutex<Vec<Vec<u8>>>> = Arc::new(Mutex::new(Vec::new()));
    let (r, h) = (requests.clone(), heads.clone());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let (r, h) = (r.clone(), h.clone());
            tokio::spawn(async move {
                let mut buf = vec![0u8; 16384];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                if n == 0 {
                    return;
                }
                buf.truncate(n);
                h.lock().unwrap().push(buf);
                let seq = r.fetch_add(1, Ordering::SeqCst) + 1;
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                let body = format!("origin-body-{}", seq);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    cache_control,
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.flush().await;
            });
        }
    });
    Origin { url: format!("http://127.0.0.1:{}", port), requests, heads }
}

impl Origin {
    fn count(&self) -> u64 {
        self.requests.load(Ordering::SeqCst)
    }
    fn request_line(&self, i: usize) -> Vec<u8> {
        let h = self.heads.lock().unwrap();
        let first = h[i].split(|b| *b == b'\n').next().unwrap().to_vec();
        first.into_iter().filter(|b| *b != b'\r').collect()
    }
    fn head_text(&self, i: usize) -> String {
        String::from_utf8_lossy(&self.heads.lock().unwrap()[i]).to_string()
    }
}

// ------------------------------------------------------------------ in-process gateway

struct Gw {
    base: String,
    state: Arc<ApiState>,
    c: reqwest::Client,
}

fn state(security: Option<SecurityConfig>, origin: Option<&str>, telemetry: bool) -> Arc<ApiState> {
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
        telemetry: if telemetry { Some(Arc::new(TelemetryCollector::new(1000))) } else { None },
        pie: None,
        security: security.map(Arc::new),
        upstream_policy: None,
        router: None,
    })
}

async fn serve(app: axum::Router) -> String {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    format!("http://{}", a)
}

fn sec_cfg() -> SecurityConfig {
    let mut c = SecurityConfig::new_with_jwt(validator());
    c.enforce_rfc9111_auth_bypass = false;
    c
}

async fn gw(security: Option<SecurityConfig>, origin: Option<&str>) -> Gw {
    let st = state(security, origin, true);
    let base = serve(create_app(st.clone())).await;
    Gw { base, state: st, c: reqwest::Client::builder().pool_max_idle_per_host(0).build().unwrap() }
}

impl Gw {
    fn get(&self, p: &str) -> reqwest::RequestBuilder {
        self.c.get(format!("{}{}", self.base, p))
    }
    /// counters + byte audit + telemetry + diagnostics fingerprint
    fn fingerprint(&self) -> (u64, u64, u64, u64, (usize, usize), usize, u64, u64, usize) {
        let cache = self.state.cache.as_ref().unwrap();
        let s = cache.counters.snapshot();
        let tel = self.state.telemetry.as_ref().unwrap();
        (
            s.requests_total,
            s.cache_hits,
            s.cache_misses,
            s.cache_bypasses,
            cache.audit_bytes(),
            tel.len(),
            tel.error_count(),
            tel.histogram().count(),
            cache.diagnostics.events.lock().unwrap().len(),
        )
    }
}

// ==================================================================== H1

#[tokio::test]
async fn h1_illegal_bytes_in_path_are_400_and_origin_receives_nothing() {
    for secure in [false, true] {
        let origin = spawn_origin("public, max-age=60", 0).await;
        let g = gw(if secure { Some(sec_cfg()) } else { None }, Some(&origin.url)).await;
        let t = tok(vec!["cache:read"], Some("t1"));
        let paths = [
            "/api/v1/cache/a%0d%0aX-Injected:%20yes",
            "/api/v1/cache/a%0aX",
            "/api/v1/cache/a%0dX",
            "/api/v1/cache/a%00b",
            "/api/v1/cache/a%20b",
            "/api/v1/cache/a%7fb",
            "/api/v1/cache/a%09b",
            "/api/v1/cache/a%1fb",
            "/api/v1/cache/a%20HTTP/1.1%0d%0aHost:%20x%0d%0a%0d%0aGET%20/admin",
        ];
        for p in paths {
            let mut req = g.get(p);
            if secure {
                req = req.header("Authorization", bearer(&t));
            }
            let r = req.send().await.unwrap();
            assert_eq!(r.status(), 400, "secure={} path {} must be rejected", secure, p);
        }
        assert_eq!(origin.count(), 0, "secure={}: rejected keys must never reach the origin", secure);
    }
}

#[test]
fn h1_validate_rejects_control_space_del_in_path_query_and_tenant() {
    let bad = ["a\r\nb", "a\nb", "a\rb", "a\0b", "a b", "a\tb", "a\x7fb", "a\x1fb"];
    for b in bad {
        assert!(CacheKey::new(Some("t"), &format!("/{}", b), None).validate().is_err(), "path {:?}", b);
        assert!(CacheKey::new(Some("t"), "/ok", Some(&format!("q={}", b))).validate().is_err(), "query {:?}", b);
        assert!(CacheKey::new(Some(&format!("t{}", b)), "/ok", None).validate().is_err(), "tenant {:?}", b);
    }
    // legal, including non-ASCII UTF-8 and percent text
    for (p, q) in [("/a/b-c_d.e~f", Some("k=v&x=1")), ("/caf\u{e9}", Some("q=\u{e9}")), ("/a%41", Some("x=%0d%0a")), ("/", None)] {
        assert!(CacheKey::new(Some("t"), p, q).validate().is_ok(), "{:?} {:?}", p, q);
    }
}

#[tokio::test]
async fn h1_encoded_query_crlf_stays_encoded_and_cannot_inject() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let g = gw(None, Some(&origin.url)).await;
    let r = g.get("/api/v1/cache/q?x=%0d%0aHost:%20evil%0d%0a%0d%0aGET%20/admin").send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(origin.count(), 1);
    assert_eq!(
        String::from_utf8_lossy(&origin.request_line(0)),
        "GET /q?x=%0d%0aHost:%20evil%0d%0a%0d%0aGET%20/admin HTTP/1.1"
    );
    let head = origin.head_text(0);
    assert_eq!(head.to_ascii_lowercase().matches("\r\nhost:").count(), 1, "exactly one Host header: {:?}", head);
    assert!(!head.contains("GET /admin"));
}

#[tokio::test]
async fn h1_double_encoding_stays_literal_percent_text() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let g = gw(None, Some(&origin.url)).await;
    let r = g.get("/api/v1/cache/a%250d%250aX").send().await.unwrap();
    assert_eq!(r.status(), 200);
    // Round 3 (L-1): the faithful behaviour. The client sent %250d%250a (literal text "%0d%0a" after one
    // decode); the origin must receive that same text, i.e. %250d%250a, not %0d%0a (which an origin would
    // decode to CR LF). Before round 3 the gateway sent %0d%0a.
    assert_eq!(String::from_utf8_lossy(&origin.request_line(0)), "GET /a%250d%250aX HTTP/1.1");
}

#[tokio::test]
async fn h1_non_ascii_is_percent_encoded_upstream_and_legal_characters_arrive_unchanged() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let g = gw(None, Some(&origin.url)).await;
    let r = g.get("/api/v1/cache/caf%C3%A9?q=%C3%A9").send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(String::from_utf8_lossy(&origin.request_line(0)), "GET /caf%C3%A9?q=%C3%A9 HTTP/1.1");
    assert!(origin.heads.lock().unwrap()[0].is_ascii(), "the upstream request must be pure ASCII");

    let r = g.get("/api/v1/cache/a-b_c.d~e/!$&(x)*+,;=:@/z?k=v&n=1").send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(String::from_utf8_lossy(&origin.request_line(1)), "GET /a-b_c.d~e/!$&(x)*+,;=:@/z?k=v&n=1 HTTP/1.1");
    // brackets are outside the RFC 3986 query set and are encoded; the key still caches (HIT)
    let r = g.get("/api/v1/cache/br?z[0]=1").send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(String::from_utf8_lossy(&origin.request_line(2)), "GET /br?z%5B0%5D=1 HTTP/1.1");
    let r = g.get("/api/v1/cache/br?z[0]=1").send().await.unwrap();
    assert_eq!(r.headers().get("x-cache-status").unwrap(), "HIT");
    assert_eq!(origin.count(), 3);
}

#[tokio::test]
async fn h1_percent_decoded_question_mark_in_path_cannot_become_a_query() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let g = gw(None, Some(&origin.url)).await;
    let r = g.get("/api/v1/cache/a%3Fb=1").send().await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(String::from_utf8_lossy(&origin.request_line(0)), "GET /a%3Fb=1 HTTP/1.1");
}

// ==================================================================== M1

#[tokio::test]
async fn m1_tenantless_and_reserved_tenant_tokens_are_403_for_cache_read_and_invalidate() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let g = gw(Some(sec_cfg()), Some(&origin.url)).await;
    let before = g.fingerprint();
    for (name, t) in [
        ("no tenant claim", tok(vec!["cache:read"], None)),
        ("empty tenant claim", tok(vec!["cache:read"], Some(""))),
        ("reserved __default__", tok(vec!["cache:read"], Some("__default__"))),
    ] {
        let r = g.get("/api/v1/cache/x").header("Authorization", bearer(&t)).send().await.unwrap();
        assert_eq!(r.status(), 403, "read: {}", name);
        let body = r.text().await.unwrap().to_lowercase();
        assert!(body.contains("tenant"), "{}: clear error expected, got {:?}", name, body);
    }
    for (name, t) in [
        ("no tenant claim", tok(vec!["cache:invalidate"], None)),
        ("reserved __default__", tok(vec!["cache:invalidate"], Some("__default__"))),
    ] {
        let r = g.c.delete(format!("{}/api/v1/cache/x", g.base)).header("Authorization", bearer(&t)).send().await.unwrap();
        assert_eq!(r.status(), 403, "invalidate: {}", name);
    }
    assert_eq!(origin.count(), 0);
    assert_eq!(g.fingerprint(), before);
    // a proper tenant still works, and a tenant-less Observer token still reads metrics
    let ok = tok(vec!["cache:read"], Some("t1"));
    assert_eq!(g.get("/api/v1/cache/x").header("Authorization", bearer(&ok)).send().await.unwrap().status(), 200);
    let obs = tok(vec!["telemetry:read"], None);
    assert_eq!(g.get("/api/v1/metrics").header("Authorization", bearer(&obs)).send().await.unwrap().status(), 200);
}

// ==================================================================== lows (in-process)

#[tokio::test]
async fn low_zero_side_effects_include_telemetry_and_diagnostics() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let g = gw(Some(sec_cfg()), Some(&origin.url)).await;
    // one legitimate request first so the fingerprint is non-trivial
    let ok = tok(vec!["cache:read"], Some("t1"));
    assert_eq!(g.get("/api/v1/cache/warm").header("Authorization", bearer(&ok)).send().await.unwrap().status(), 200);
    let before = g.fingerprint();
    let n = origin.count();
    for a in [None, Some("Bearer x.y.z".to_string()), Some(bearer(&tok(vec!["telemetry:read"], Some("t1")))), Some(bearer(&tok(vec!["cache:read"], None)))] {
        let mut req = g.get("/api/v1/cache/warm");
        if let Some(a) = a {
            req = req.header("Authorization", a);
        }
        assert!(matches!(req.send().await.unwrap().status().as_u16(), 401 | 403));
    }
    let r = g.get("/api/v1/cache/warm").header("x-cache-bypass", "true").send().await.unwrap();
    assert_eq!(r.status(), 401);
    assert_eq!(origin.count(), n);
    assert_eq!(g.fingerprint(), before, "denied requests must not touch counters, telemetry or diagnostics");
}

#[tokio::test]
async fn low_401_body_is_generic_and_does_not_echo_kid_or_parser_detail() {
    let g = gw(Some(sec_cfg()), None).await;
    // header with an attacker-controlled kid
    let mut h = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    h.kid = Some("ATTACKER-KID-<script>".into());
    let t = jsonwebtoken::encode(
        &h,
        &JwtClaims::new("u", ISS, AUD, now() + 3600, Some("t1"), vec!["cache:read"]),
        &jsonwebtoken::EncodingKey::from_secret(SECRET.as_bytes()),
    )
    .unwrap();
    for auth in [bearer(&t), "Bearer not.a.jwt".to_string(), "Bearer ???".to_string()] {
        for path in ["/api/v1/cache/x", "/api/v1/metrics", "/api/v1/intelligence/status"] {
            let r = g.get(path).header("Authorization", auth.clone()).send().await.unwrap();
            assert_eq!(r.status(), 401);
            let xa = r.headers().get("x-auth-error").map(|v| v.to_str().unwrap().to_string());
            let body = r.text().await.unwrap();
            assert!(!body.contains("ATTACKER") && !body.contains("kid"), "body leaks detail: {:?}", body);
            assert!(!body.to_lowercase().contains("unknown") && !body.contains("base64") && !body.contains("InvalidToken("), "{:?}", body);
            assert_eq!(xa.as_deref(), Some("InvalidToken"), "unsigned/garbage tokens get only the generic code");
        }
    }
}

#[tokio::test]
async fn low_cors_exposes_auth_headers_and_error_responses_carry_acao_only_for_allowed_origin() {
    let cfg = CorsConfig::with_origins(vec!["http://localhost:3000".to_string()]).unwrap();
    let st = state(Some(sec_cfg()), None, false);
    let base = serve(create_app_with_cors(st, &cfg)).await;
    let c = reqwest::Client::new();
    let long = "a".repeat(5000);
    let t = tok(vec!["cache:read"], Some("t1"));
    // (path, token, expected status)
    let cases: Vec<(String, Option<String>, u16)> = vec![
        ("/api/v1/cache/x".into(), None, 401),
        ("/api/v1/cache/x".into(), Some(tok(vec!["telemetry:read"], Some("t1"))), 403),
        ("/api/v1/cache/a%20b".into(), Some(t.clone()), 400),
        (format!("/api/v1/cache/{}", long), Some(t.clone()), 414),
    ];
    for (p, token, status) in cases {
        for (origin, allowed) in [("http://localhost:3000", true), ("https://evil.example", false)] {
            let mut req = c.get(format!("{}{}", base, p)).header("Origin", origin);
            if let Some(tk) = &token {
                req = req.header("Authorization", bearer(tk));
            }
            let r = req.send().await.unwrap();
            assert_eq!(r.status().as_u16(), status, "{} from {}", &p[..p.len().min(30)], origin);
            assert_eq!(r.headers().get("access-control-allow-origin").is_some(), allowed, "{} {}", status, origin);
            if allowed {
                let ex = r.headers().get("access-control-expose-headers").unwrap().to_str().unwrap().to_ascii_lowercase();
                for h in ["www-authenticate", "x-auth-error", "x-cache-error", "x-cache-status"] {
                    assert!(ex.contains(h), "expose-headers must list {}: {}", h, ex);
                }
            }
        }
    }
}

#[tokio::test]
async fn low_pagination_duplicate_parameters_first_wins() {
    let g = gw(None, None).await;
    // no intelligence store installed in this binary: 404 "disabled" proves validation passed and
    // first-wins semantics: the second (invalid) limit is ignored, the first (invalid) one is not.
    let ok = g.get("/api/v1/intelligence/audit?limit=5&limit=abc").send().await.unwrap();
    assert_ne!(ok.status(), 400, "first limit=5 wins; the later invalid duplicate is ignored");
    let bad = g.get("/api/v1/intelligence/audit?limit=abc&limit=5").send().await.unwrap();
    assert_eq!(bad.status(), 400, "first limit=abc wins and is invalid");
}

#[tokio::test]
async fn low_events_stream_is_observer_gated() {
    let g = gw(Some(sec_cfg()), None).await;
    assert_eq!(g.get("/api/events").send().await.unwrap().status(), 401);
    assert_eq!(g.get("/api/events").header("Authorization", bearer(&tok(vec!["cache:read"], Some("t1")))).send().await.unwrap().status(), 403);
    let r = g.get("/api/events").header("Authorization", bearer(&tok(vec!["telemetry:read"], None))).send().await.unwrap();
    assert_eq!(r.status(), 200);
}

#[tokio::test]
async fn low_embedder_case_auth_for_control_disabled_still_validates_presented_tokens() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let mut c = sec_cfg();
    c.require_auth_for_control = false;
    let g = gw(Some(c), Some(&origin.url)).await;
    // no token: allowed (embedder explicitly disabled enforcement)
    assert_eq!(g.get("/api/v1/cache/e1").send().await.unwrap().status(), 200);
    // a presented token must still be valid and carry cache:read
    assert_eq!(g.get("/api/v1/cache/e2").header("Authorization", "Bearer garbage").send().await.unwrap().status(), 401);
    assert_eq!(g.get("/api/v1/cache/e3").header("Authorization", bearer(&tok(vec!["telemetry:read"], Some("t1")))).send().await.unwrap().status(), 403);
    assert_eq!(g.get("/api/v1/cache/e4").header("Authorization", bearer(&tok(vec!["cache:read"], Some("t1")))).send().await.unwrap().status(), 200);
    assert_eq!(origin.count(), 2);
}

// ==================================================================== e2e: the real binary

struct Proc {
    child: std::process::Child,
    base: String,
}
impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn free_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

fn spawn_cmd(env: &[(&str, &str)]) -> (std::process::Child, u16) {
    let port = free_port();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_phantom-cache-demo"));
    for (k, _) in std::env::vars() {
        if k.starts_with("PHANTOM_") || k == "ORIGIN_URL" {
            cmd.env_remove(k);
        }
    }
    cmd.env("PHANTOM_BIND_ADDR", format!("127.0.0.1:{}", port));
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    (cmd.spawn().unwrap(), port)
}

async fn start_gateway(env: &[(&str, &str)]) -> Proc {
    let (mut child, port) = spawn_cmd(env);
    for _ in 0..200 {
        if let Ok(Some(st)) = child.try_wait() {
            panic!("gateway exited early with {:?}", st);
        }
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return Proc { child, base: format!("http://127.0.0.1:{}", port) };
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    panic!("gateway did not start");
}

fn secure_env<'a>(origin: &'a str, extra: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut v = vec![("PHANTOM_JWT_SECRET", SECRET), ("ORIGIN_URL", origin)];
    v.extend_from_slice(extra);
    v
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().pool_max_idle_per_host(0).build().unwrap()
}

#[tokio::test]
async fn e2e_m2_authenticated_public_read_is_cached_and_coalesced() {
    for cc in ["public, max-age=60", "s-maxage=60", "max-age=60, must-revalidate"] {
        let origin = spawn_origin(cc, 300).await;
        let gwp = start_gateway(&secure_env(&origin.url, &[])).await;
        let c = client();
        let t = tok(vec!["cache:read"], Some("t1"));
        // coalescing: 8 concurrent authenticated reads of one key -> one origin request
        let mut hs = Vec::new();
        for _ in 0..8 {
            let (c, u, a) = (c.clone(), format!("{}/api/v1/cache/co", gwp.base), bearer(&t));
            hs.push(tokio::spawn(async move { c.get(u).header("Authorization", a).send().await.unwrap().status().as_u16() }));
        }
        for h in hs {
            assert_eq!(h.await.unwrap(), 200, "{}", cc);
        }
        assert_eq!(origin.count(), 1, "{}: coalescing/caching of authenticated reads", cc);
        let r = c.get(format!("{}/api/v1/cache/co", gwp.base)).header("Authorization", bearer(&t)).send().await.unwrap();
        assert_eq!(r.headers().get("x-cache-status").unwrap(), "HIT", "{}", cc);
        assert_eq!(origin.count(), 1);
        // the client credential is not forwarded upstream by default
        assert!(!origin.head_text(0).to_ascii_lowercase().contains("authorization"), "credential must not reach the origin");
    }
}

#[tokio::test]
async fn e2e_m2_response_without_explicit_shared_cacheability_is_not_stored_for_authenticated_reads() {
    let origin = spawn_origin("max-age=60", 0).await;
    let gwp = start_gateway(&secure_env(&origin.url, &[])).await;
    let c = client();
    let t = tok(vec!["cache:read"], Some("t1"));
    for _ in 0..2 {
        let r = c.get(format!("{}/api/v1/cache/priv", gwp.base)).header("Authorization", bearer(&t)).send().await.unwrap();
        assert_eq!(r.status(), 200);
        assert_ne!(r.headers().get("x-cache-status").unwrap(), "HIT");
    }
    assert_eq!(origin.count(), 2, "RFC 9111 3.5: not stored without public/s-maxage/must-revalidate");
}

#[tokio::test]
async fn e2e_m2_tenant_isolation_and_denials_on_the_cached_path() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let gwp = start_gateway(&secure_env(&origin.url, &[])).await;
    let c = client();
    let (ta, tb) = (tok(vec!["cache:read"], Some("tenant-a")), tok(vec!["cache:read"], Some("tenant-b")));
    let u = format!("{}/api/v1/cache/iso", gwp.base);
    assert_eq!(c.get(&u).header("Authorization", bearer(&ta)).send().await.unwrap().headers().get("x-cache-status").unwrap(), "MISS");
    assert_eq!(c.get(&u).header("Authorization", bearer(&ta)).send().await.unwrap().headers().get("x-cache-status").unwrap(), "HIT");
    assert_eq!(c.get(&u).header("Authorization", bearer(&tb)).send().await.unwrap().headers().get("x-cache-status").unwrap(), "MISS");
    assert_eq!(origin.count(), 2);
    // denied/anonymous requests never touch the cache or the origin, hit or miss
    assert_eq!(c.get(&u).send().await.unwrap().status(), 401);
    assert_eq!(c.get(&u).header("Authorization", bearer(&tok(vec!["telemetry:read"], Some("tenant-a")))).send().await.unwrap().status(), 403);
    assert_eq!(c.get(&u).header("Authorization", bearer(&tok(vec!["cache:read"], None))).send().await.unwrap().status(), 403);
    assert_eq!(c.get(&u).header("x-cache-bypass", "true").send().await.unwrap().status(), 401);
    assert_eq!(origin.count(), 2);
}

#[tokio::test]
async fn e2e_m2_forwarding_client_authorization_keeps_bypass_auth() {
    let origin = spawn_origin("public, max-age=60", 0).await;
    let gwp = start_gateway(&secure_env(&origin.url, &[("PHANTOM_FORWARD_UPSTREAM_AUTH", "true")])).await;
    let c = client();
    let t = tok(vec!["cache:read"], Some("t1"));
    for _ in 0..2 {
        let r = c.get(format!("{}/api/v1/cache/fw", gwp.base)).header("Authorization", bearer(&t)).send().await.unwrap();
        assert_eq!(r.status(), 200);
        assert_eq!(r.headers().get("x-cache-status").unwrap(), "BYPASS-AUTH");
    }
    assert_eq!(origin.count(), 2);
    assert_eq!(c.get(format!("{}/api/v1/cache/fw", gwp.base)).send().await.unwrap().status(), 401);
    assert_eq!(origin.count(), 2);
}

async fn acao(base: &str, origin: &str) -> (Option<String>, Option<String>) {
    let r = client().get(format!("{}/health", base)).header("Origin", origin).send().await.unwrap();
    (
        r.headers().get("access-control-allow-origin").map(|v| v.to_str().unwrap().to_string()),
        r.headers().get("access-control-allow-credentials").map(|v| v.to_str().unwrap().to_string()),
    )
}

#[tokio::test]
async fn e2e_cors_wiring_dev_default_explicit_empty_and_production() {
    // development profile (default mode): the Vite origins only
    let p = start_gateway(&[]).await;
    assert_eq!(acao(&p.base, "http://localhost:3000").await.0.as_deref(), Some("http://localhost:3000"));
    assert_eq!(acao(&p.base, "http://127.0.0.1:3000").await.0.as_deref(), Some("http://127.0.0.1:3000"));
    assert_eq!(acao(&p.base, "https://evil.example").await, (None, None));
    drop(p);
    // explicit list replaces the default
    let p = start_gateway(&[("PHANTOM_CORS_ORIGINS", "https://ui.example.com")]).await;
    assert_eq!(acao(&p.base, "https://ui.example.com").await.0.as_deref(), Some("https://ui.example.com"));
    assert_eq!(acao(&p.base, "http://localhost:3000").await, (None, None));
    drop(p);
    // set-but-empty means no cross-origin access even in the development profile
    let p = start_gateway(&[("PHANTOM_CORS_ORIGINS", "")]).await;
    assert_eq!(acao(&p.base, "http://localhost:3000").await, (None, None));
    drop(p);
    // production default: same-origin only
    let p = start_gateway(&[("PHANTOM_MODE", "production"), ("PHANTOM_REQUIRE_AUTH", "true"), ("PHANTOM_JWT_SECRET", SECRET)]).await;
    assert_eq!(acao(&p.base, "http://localhost:3000").await, (None, None));
    assert_eq!(acao(&p.base, "https://evil.example").await, (None, None));
}

fn run_once(env: &[(&str, &str)]) -> std::process::Output {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_phantom-cache-demo"));
    for (k, _) in std::env::vars() {
        if k.starts_with("PHANTOM_") || k == "ORIGIN_URL" {
            cmd.env_remove(k);
        }
    }
    cmd.env("PHANTOM_BIND_ADDR", format!("127.0.0.1:{}", free_port()));
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.output().unwrap()
}

#[test]
fn e2e_cors_malformed_value_aborts_startup_with_config_exit_code_without_echoing_userinfo() {
    let out = run_once(&[("PHANTOM_CORS_ORIGINS", "https://user:s3cretpw@ui.example.com")]);
    assert_eq!(out.status.code(), Some(2));
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(text.contains("PHANTOM_CORS_ORIGINS"));
    assert!(!text.contains("s3cretpw"), "userinfo must be redacted from the error: {}", text);
    let out = run_once(&[
        ("PHANTOM_MODE", "production"),
        ("PHANTOM_REQUIRE_AUTH", "true"),
        ("PHANTOM_JWT_SECRET", SECRET),
        ("PHANTOM_CORS_ORIGINS", "*"),
    ]);
    assert_eq!(out.status.code(), Some(2), "wildcard in production must fail fast");
}

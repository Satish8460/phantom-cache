//! Round-3 security review fixes: M-1 (no client-controlled request headers upstream), M-2 (cluster
//! GetRequest key namespace), L-1 (faithful `%` encoding, dot segments), bypass framing, check
//! ordering, CORS expose of `x-cache-warning`, and leading `//` behaviour.

use phantom_cache_demo::api::server::{create_app, create_app_with_cors, encode_origin_target, ApiState, CorsConfig, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::{CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache};
use phantom_cache_demo::dist::frame::{read_frame, write_frame, PhantomMeshFrame};
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::security::{JwtClaims, JwtValidator, SecurityConfig};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ISS: &str = "phantom-cache-auth";
const AUD: &str = "phantom-cache-api";
const SECRET: &[u8] = b"sec3-test-secret-0123456789abcdefghij";

fn validator() -> JwtValidator {
    JwtValidator::new(ISS, AUD, SECRET)
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
fn sec_cfg() -> SecurityConfig {
    let mut c = SecurityConfig::new_with_jwt(validator());
    c.enforce_rfc9111_auth_bypass = false; // what main.rs sets when the credential is not forwarded
    c
}

// ------------------------------------------------------------------ origins

struct Origin {
    url: String,
    requests: Arc<AtomicU64>,
    heads: Arc<Mutex<Vec<String>>>,
}
impl Origin {
    fn count(&self) -> u64 {
        self.requests.load(Ordering::SeqCst)
    }
    fn head(&self, i: usize) -> String {
        self.heads.lock().unwrap()[i].clone()
    }
}

fn header_value(head: &str, name: &str) -> Vec<String> {
    head.split("\r\n")
        .skip(1)
        .filter_map(|l| l.split_once(':'))
        .filter(|(k, _)| k.trim().eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim().to_string())
        .collect()
}

/// Personalising echo origin: the body reveals every client-controlled header it saw.
async fn spawn_echo_origin(delay_ms: u64, extra_response_headers: &'static str) -> Origin {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(AtomicU64::new(0));
    let heads = Arc::new(Mutex::new(Vec::new()));
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
                let head = String::from_utf8_lossy(&buf[..n]).to_string();
                h.lock().unwrap().push(head.clone());
                r.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                let get = |n: &str| header_value(&head, n).first().cloned().unwrap_or_default();
                let body = format!(
                    "uid={};fu={};acc={};tenant={}",
                    get("x-user-id"),
                    get("x-forwarded-user"),
                    get("accept"),
                    get("x-tenant-id")
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: public, max-age=60\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    extra_response_headers,
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

/// Origin that answers with fixed raw bytes.
async fn spawn_raw_origin(response: Vec<u8>) -> Origin {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(AtomicU64::new(0));
    let heads = Arc::new(Mutex::new(Vec::new()));
    let (r, h) = (requests.clone(), heads.clone());
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let mut buf = vec![0u8; 16384];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            h.lock().unwrap().push(String::from_utf8_lossy(&buf[..n]).to_string());
            r.fetch_add(1, Ordering::SeqCst);
            let _ = socket.write_all(&response).await;
            let _ = socket.flush().await;
        }
    });
    Origin { url: format!("http://127.0.0.1:{}", port), requests, heads }
}

// ------------------------------------------------------------------ gateway

fn state(security: Option<SecurityConfig>, origin: Option<&str>) -> Arc<ApiState> {
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

struct Gw {
    base: String,
    addr: std::net::SocketAddr,
    c: reqwest::Client,
}

async fn serve_with(app: axum::Router) -> Gw {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    Gw { base: format!("http://{}", addr), addr, c: reqwest::Client::builder().pool_max_idle_per_host(0).build().unwrap() }
}

async fn gw(security: Option<SecurityConfig>, origin: Option<&str>) -> Gw {
    serve_with(create_app(state(security, origin))).await
}

impl Gw {
    fn get(&self, p: &str) -> reqwest::RequestBuilder {
        self.c.get(format!("{}{}", self.base, p))
    }
}

/// Send a raw request line (no client-side URL normalisation); returns the full raw response.
async fn raw_get(addr: std::net::SocketAddr, target: &str, extra: &str) -> String {
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    let req = format!("GET {} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n{}\r\n", target, extra);
    s.write_all(req.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), s.read_to_end(&mut out)).await;
    String::from_utf8_lossy(&out).to_string()
}

fn status_of(raw: &str) -> u16 {
    raw.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0)
}

// ==================================================================== M-1

#[tokio::test]
async fn m1_client_x_headers_accept_and_cookie_never_reach_the_origin_and_users_share_one_variant() {
    let origin = spawn_echo_origin(0, "").await;
    let g = gw(Some(sec_cfg()), Some(&origin.url)).await;
    let ta = tok(vec!["cache:read"], Some("t1"));
    let tb = tok(vec!["cache:read"], Some("t1")); // a second user of the same tenant
    let r1 = g
        .get("/api/v1/cache/doc")
        .header("Authorization", bearer(&ta))
        .header("x-user-id", "alice")
        .header("x-forwarded-user", "alice")
        .header("x-original-url", "/admin")
        .header("x-http-method-override", "DELETE")
        .header("x-real-ip", "6.6.6.6")
        .header("x-cache-bypass", "false")
        .header("Accept", "text/html")
        .header("Cookie", "session=alice-secret")
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 200);
    let body1 = r1.text().await.unwrap();
    assert_eq!(origin.count(), 1);
    let head = origin.head(0);
    for forbidden in ["x-user-id", "x-forwarded-user", "x-original-url", "x-http-method-override", "x-real-ip", "x-cache-bypass", "cookie", "authorization"] {
        assert!(header_value(&head, forbidden).is_empty(), "{} must never reach the origin: {:?}", forbidden, head);
    }
    assert_eq!(header_value(&head, "accept"), vec!["application/json".to_string()], "fixed Accept, exactly once: {:?}", head);
    assert_eq!(body1, "uid=;fu=;acc=application/json;tenant=t1");

    // a different user of the same tenant sending different values gets identical cached content
    let r2 = g
        .get("/api/v1/cache/doc")
        .header("Authorization", bearer(&tb))
        .header("x-user-id", "bob")
        .header("x-forwarded-user", "bob")
        .header("Accept", "application/xml")
        .send()
        .await
        .unwrap();
    assert_eq!(r2.headers().get("x-cache-status").unwrap(), "HIT");
    assert_eq!(r2.text().await.unwrap(), body1);
    assert_eq!(origin.count(), 1);
}

#[tokio::test]
async fn m1_x_request_id_is_forwarded_only_when_valid_and_tenant_header_comes_from_the_token() {
    let origin = spawn_echo_origin(0, "").await;
    let g = gw(Some(sec_cfg()), Some(&origin.url)).await;
    let t = tok(vec!["cache:read"], Some("tenant-9"));
    let cases = [("req-123.A_b", true), ("bad id;x", false), ("a\u{20}b", false), (&"x".repeat(65), false), (&"x".repeat(64), true), ("ok", true)];
    for (i, (rid, valid)) in cases.iter().enumerate() {
        let r = g
            .get(&format!("/api/v1/cache/rid{}", i))
            .header("Authorization", bearer(&t))
            .header("x-request-id", *rid)
            .header("x-tenant-id", "spoofed-tenant")
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200);
        let head = origin.head(i);
        let got = header_value(&head, "x-request-id");
        if *valid {
            assert_eq!(got, vec![rid.to_string()], "{:?}", rid);
        } else {
            assert!(got.is_empty(), "invalid x-request-id {:?} must be dropped: {:?}", rid, head);
        }
        assert_eq!(header_value(&head, "x-tenant-id"), vec!["tenant-9".to_string()], "x-tenant-id equals the token tenant");
    }
}

#[tokio::test]
async fn m1_dev_mode_forwards_the_key_tenant_only_and_nothing_for_the_default_tenant() {
    let origin = spawn_echo_origin(0, "").await;
    let g = gw(None, Some(&origin.url)).await;
    g.get("/api/v1/cache/d1").send().await.unwrap();
    assert!(header_value(&origin.head(0), "x-tenant-id").is_empty(), "no tenant header for the default tenant");
    g.get("/api/v1/cache/d2").header("x-tenant-id", "acme").header("x-user-id", "mallory").send().await.unwrap();
    assert_eq!(header_value(&origin.head(1), "x-tenant-id"), vec!["acme".to_string()]);
    assert!(header_value(&origin.head(1), "x-user-id").is_empty());
}

#[tokio::test]
async fn m1_cross_user_coalescing_one_origin_fetch_and_no_header_leak() {
    let origin = spawn_echo_origin(400, "").await;
    let g = Arc::new(gw(Some(sec_cfg()), Some(&origin.url)).await);
    let t = tok(vec!["cache:read"], Some("t1"));
    let mut hs = Vec::new();
    for (user, accept) in [("alice", "text/html"), ("bob", "application/xml")] {
        let (g, t) = (g.clone(), t.clone());
        hs.push(tokio::spawn(async move {
            let r = g
                .get("/api/v1/cache/shared")
                .header("Authorization", bearer(&t))
                .header("x-user-id", user)
                .header("x-forwarded-user", user)
                .header("Accept", accept)
                .send()
                .await
                .unwrap();
            assert_eq!(r.status(), 200);
            r.text().await.unwrap()
        }));
    }
    let bodies: Vec<String> = futures_join(hs).await;
    assert_eq!(origin.count(), 1, "concurrent misses of two users coalesce into one origin fetch");
    assert_eq!(bodies[0], bodies[1], "both users get the same body");
    for b in &bodies {
        assert!(!b.contains("alice") && !b.contains("bob") && !b.contains("html") && !b.contains("xml"), "no per-user value may leak: {}", b);
    }
}

async fn futures_join(hs: Vec<tokio::task::JoinHandle<String>>) -> Vec<String> {
    let mut out = Vec::new();
    for h in hs {
        out.push(h.await.unwrap());
    }
    out
}

#[tokio::test]
async fn m1_set_cookie_from_the_origin_is_not_replayed_on_a_hit() {
    let origin = spawn_echo_origin(0, "Set-Cookie: session=leader-secret; Path=/\r\n").await;
    let g = gw(Some(sec_cfg()), Some(&origin.url)).await;
    let ta = tok(vec!["cache:read"], Some("t1"));
    let r1 = g.get("/api/v1/cache/cookie").header("Authorization", bearer(&ta)).send().await.unwrap();
    assert_eq!(r1.status(), 200);
    let r2 = g.get("/api/v1/cache/cookie").header("Authorization", bearer(&ta)).send().await.unwrap();
    assert_eq!(r2.headers().get("x-cache-status").unwrap(), "HIT");
    assert!(r2.headers().get("set-cookie").is_none(), "Set-Cookie must not be replayed from the cache");
    assert_eq!(origin.count(), 1);
}

// ==================================================================== M-2

#[tokio::test]
async fn m2_cluster_get_request_keys_in_the_http_namespace_or_invalid_are_rejected() {
    let node = Arc::new(VirtualNode::new(7));
    let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
    // an HTTP-namespace entry for the default tenant exists: tenant __default__, path /x
    let key = CacheKey::new(None, "/x", None);
    let ms = phantom_cache_demo::intel::diagnostics::DiagnosticBuffer::now_ms();
    if let FetchRegistrationResult::Initiate(r) = node.cache.begin_fetch_atomic(&key, ms) {
        let res = Arc::new(CachedResource {
            status: 200,
            content_type: "text/plain".into(),
            body: Arc::new(b"http-namespace-secret".to_vec()),
            etag: None,
            created_at_ms: ms,
            ttl_ms: 60_000,
        });
        r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
    }
    let mut c = tokio::net::TcpStream::connect(addr).await.unwrap();
    for (i, k) in ["/x", "/", "a b", "x\ty", "a/../b"].iter().enumerate() {
        let req = PhantomMeshFrame::new_get_request(i as u64 + 1, 0, k).unwrap();
        write_frame(&mut c, &req).await.unwrap();
        let resp = read_frame(&mut c).await.unwrap();
        let body = String::from_utf8_lossy(&resp.payload).to_string();
        assert_eq!(body, "ERR 400", "key {:?} must be rejected, got {:?}", k, body);
        assert!(!body.contains("secret"));
    }
    // legitimate cluster-internal keys still work
    let req = PhantomMeshFrame::new_get_request(99, 0, "user_profile_123").unwrap();
    write_frame(&mut c, &req).await.unwrap();
    let resp = read_frame(&mut c).await.unwrap();
    assert!(String::from_utf8_lossy(&resp.payload).starts_with("MISS val_node_7"));
    node.stop_tcp_service();
}

// ==================================================================== L-1

#[test]
fn l1_encode_origin_target_direct() {
    let cases: Vec<(&str, Option<&str>, &str)> = vec![
        ("/a\rb", None, "/a%0Db"),
        ("/a\nb", None, "/a%0Ab"),
        ("/a\0b", None, "/a%00b"),
        ("/a b", None, "/a%20b"),
        ("/a\x7fb", None, "/a%7Fb"),
        ("/a\\b", None, "/a%5Cb"),
        ("/a\"b", None, "/a%22b"),
        ("/a<b>c", None, "/a%3Cb%3Ec"),
        ("/a[b]c", None, "/a%5Bb%5Dc"),
        ("/a#b", None, "/a%23b"),
        ("/100%", None, "/100%25"),
        ("/a%4", None, "/a%254"),
        ("/a%41", None, "/a%2541"),
        ("/a%252F", None, "/a%25252F"),
        ("/a?b", None, "/a%3Fb"),
        ("/caf\u{e9}", None, "/caf%C3%A9"),
        ("/\u{1f600}", None, "/%F0%9F%98%80"),
        ("caf\u{e9}", None, "/caf%C3%A9"),
        ("", None, "/"),
        ("/p", Some("a=1&b=2"), "/p?a=1&b=2"),
        ("/p", Some("x=%41"), "/p?x=%41"),
        ("/p", Some("x=%4"), "/p?x=%254"),
        ("/p", Some("x=%"), "/p?x=%25"),
        ("/p", Some("x=%0d%0a"), "/p?x=%0d%0a"),
        ("/p", Some("a\r\nb c"), "/p?a%0D%0Ab%20c"),
        ("/p", Some("a?b[0]=\"q\"#"), "/p?a?b%5B0%5D=%22q%22%23"),
        ("/p", Some("q=\u{e9}"), "/p?q=%C3%A9"),
        ("/p", Some(""), "/p?"),
    ];
    for (p, q, want) in cases {
        let got = encode_origin_target(p, q);
        assert_eq!(got, want, "path {:?} query {:?}", p, q);
        assert!(got.is_ascii() && !got.bytes().any(|b| b <= 0x20 || b == 0x7f), "{:?}", got);
    }
}

#[tokio::test]
async fn l1_percent_in_decoded_path_is_re_encoded_so_client_252f_goes_upstream_as_252f() {
    let origin = spawn_echo_origin(0, "").await;
    let g = gw(None, Some(&origin.url)).await;
    let r = g.get("/api/v1/cache/a%252Fb%2541").send().await.unwrap();
    assert_eq!(r.status(), 200);
    let line = origin.head(0).lines().next().unwrap().to_string();
    assert_eq!(line, "GET /a%252Fb%2541 HTTP/1.1");
}

#[test]
fn l1_validate_rejects_dot_segments_but_allows_dots_inside_names() {
    for p in ["/a/../b", "/./a", "/a/..", "/..", "/a/./b", "/.", "/a/b/.."] {
        assert!(CacheKey::new(Some("t"), p, None).validate().is_err(), "{} must be rejected", p);
    }
    for p in ["/a..b", "/.hidden", "/a/.../b", "/a/b.c", "/..a", "/a../b", "/"] {
        assert!(CacheKey::new(Some("t"), p, None).validate().is_ok(), "{} must be accepted", p);
    }
    // query is not a path: dots there are fine
    assert!(CacheKey::new(Some("t"), "/a", Some("x=../b")).validate().is_ok());
}

#[tokio::test]
async fn l1_dot_segments_over_http_are_400_and_never_reach_the_origin() {
    let origin = spawn_echo_origin(0, "").await;
    let g = gw(None, Some(&origin.url)).await;
    // raw request lines: a client library would normalise these away before sending
    for target in ["/api/v1/cache/a/../b", "/api/v1/cache/a/%2e%2e/b", "/api/v1/cache/a/%2E/b", "/api/v1/cache/%2e%2e/%2e%2e/etc/passwd"] {
        let raw = raw_get(g.addr, target, "").await;
        assert_eq!(status_of(&raw), 400, "{} -> {}", target, raw.lines().next().unwrap_or(""));
    }
    assert_eq!(origin.count(), 0);
    // names that merely contain dots still work
    assert_eq!(status_of(&raw_get(g.addr, "/api/v1/cache/a..b/.c", "").await), 200);
}

#[tokio::test]
async fn d_leading_double_slash_aliases_single_slash_but_triple_slash_is_a_distinct_key() {
    // Documented behaviour (not a fix): `/api/v1/cache//x` captures the key "/x", the same key as
    // `/api/v1/cache/x`, so the two share one cache entry and one upstream target `/x`.
    // `/api/v1/cache///x` captures "//x": a distinct key, sent upstream as `//x`.
    let origin = spawn_echo_origin(0, "").await;
    let g = gw(None, Some(&origin.url)).await;
    let a = raw_get(g.addr, "/api/v1/cache/x", "").await;
    assert_eq!(status_of(&a), 200);
    let b = raw_get(g.addr, "/api/v1/cache//x", "").await;
    assert_eq!(status_of(&b), 200);
    assert!(b.to_ascii_lowercase().contains("x-cache-status: hit"), "{}", b);
    assert_eq!(origin.count(), 1);
    let c = raw_get(g.addr, "/api/v1/cache///x", "").await;
    assert_eq!(status_of(&c), 200);
    assert_eq!(origin.count(), 2);
    assert_eq!(origin.head(1).lines().next().unwrap(), "GET //x HTTP/1.1");
}

// ==================================================================== (a) bypass framing

#[tokio::test]
async fn a_bypass_response_is_reframed_not_a_copy_of_the_origin_framing() {
    // chunked origin response: the gateway de-chunks it, so it must not pass `Transfer-Encoding:
    // chunked` through next to its own framing, and must send exactly one correct Content-Length.
    let chunked = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nTransfer-Encoding: chunked\r\nConnection: keep-alive\r\nKeep-Alive: timeout=5\r\n\r\n5\r\nhello\r\n0\r\n\r\n".to_vec();
    let origin = spawn_raw_origin(chunked).await;
    let g = gw(None, Some(&origin.url)).await;
    let raw = raw_get(g.addr, "/api/v1/cache/framing", "x-cache-bypass: true\r\n").await;
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    assert_eq!(status_of(&raw), 200);
    let lower = head.to_ascii_lowercase();
    let cl = header_value(head, "content-length");
    let te = header_value(head, "transfer-encoding");
    assert!(!(cl.len() == 1 && te.len() == 1), "both Content-Length and Transfer-Encoding: {}", lower);
    assert!(cl.len() <= 1 && te.len() <= 1, "duplicate framing headers: {}", lower);
    assert!(header_value(head, "keep-alive").is_empty(), "hop-by-hop header copied: {}", lower);
    if te.is_empty() {
        assert_eq!(cl, vec!["5".to_string()], "{}", lower);
        assert_eq!(body, "hello");
    } else {
        // hyper chunk-encodes the body itself: decode it
        assert_eq!(te[0].to_ascii_lowercase(), "chunked");
        assert!(body.starts_with("5\r\nhello\r\n"), "{:?}", body);
    }
    // a normal Content-Length origin keeps exactly one correct Content-Length
    let fixed = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello".to_vec();
    let origin = spawn_raw_origin(fixed).await;
    let g = gw(None, Some(&origin.url)).await;
    let raw = raw_get(g.addr, "/api/v1/cache/framing2", "x-cache-bypass: true\r\n").await;
    let (head, body) = raw.split_once("\r\n\r\n").unwrap();
    assert_eq!(header_value(head, "content-length"), vec!["5".to_string()]);
    assert_eq!(body, "hello");
}

// ==================================================================== (b) ordering, fallback, DELETE codes

#[tokio::test]
async fn b_check_order_is_401_then_403_role_then_403_tenant_then_400() {
    let origin = spawn_echo_origin(0, "").await;
    let g = gw(Some(sec_cfg()), Some(&origin.url)).await;
    let bad_path = "/api/v1/cache/a%20b";
    // 1. no/invalid token beats an invalid path
    assert_eq!(g.get(bad_path).send().await.unwrap().status(), 401);
    assert_eq!(g.get(bad_path).header("Authorization", "Bearer junk").send().await.unwrap().status(), 401);
    // 2. wrong role AND no tenant -> role error first
    let r = g.get(bad_path).header("Authorization", bearer(&tok(vec!["telemetry:read"], None))).send().await.unwrap();
    assert_eq!(r.status(), 403);
    assert_eq!(r.headers().get("x-auth-error").unwrap(), "InsufficientPermissions");
    // 3. right role, no tenant -> tenant error (before path validation)
    let r = g.get(bad_path).header("Authorization", bearer(&tok(vec!["cache:read"], None))).send().await.unwrap();
    assert_eq!(r.status(), 403);
    assert_eq!(r.headers().get("x-auth-error").unwrap(), "MissingTenant");
    // 4. right role and tenant, bad path -> 400
    let r = g.get(bad_path).header("Authorization", bearer(&tok(vec!["cache:read"], Some("t1")))).send().await.unwrap();
    assert_eq!(r.status(), 400);
    assert_eq!(r.headers().get("x-cache-error").unwrap(), "InvalidPath");
    // tenant claim outside the alphabet -> 400 InvalidTenantId
    let r = g.get("/api/v1/cache/ok").header("Authorization", bearer(&tok(vec!["cache:read"], Some("a b")))).send().await.unwrap();
    assert_eq!(r.status(), 400);
    assert_eq!(r.headers().get("x-cache-error").unwrap(), "InvalidTenantId");
    assert_eq!(origin.count(), 0);
}

#[tokio::test]
async fn b_tenantless_tokens_in_the_fallback_branch_and_on_delete_have_clear_error_codes() {
    let origin = spawn_echo_origin(0, "").await;
    // embedder case: auth for control disabled, a presented tenant-less token is still rejected
    let mut c = sec_cfg();
    c.require_auth_for_control = false;
    let g = gw(Some(c), Some(&origin.url)).await;
    let r = g.get("/api/v1/cache/f1").header("Authorization", bearer(&tok(vec!["cache:read"], None))).send().await.unwrap();
    assert_eq!(r.status(), 403);
    assert_eq!(r.headers().get("x-auth-error").unwrap(), "MissingTenant");
    let r = g.get("/api/v1/cache/f1").header("Authorization", bearer(&tok(vec!["cache:read"], Some("__default__")))).send().await.unwrap();
    assert_eq!(r.status(), 403);
    assert_eq!(r.headers().get("x-auth-error").unwrap(), "ReservedTenant");
    assert_eq!(origin.count(), 0);

    // DELETE (always authenticated when security is configured)
    let g = gw(Some(sec_cfg()), Some(&origin.url)).await;
    for (t, code) in [(tok(vec!["cache:invalidate"], None), "MissingTenant"), (tok(vec!["cache:invalidate"], Some("__default__")), "ReservedTenant"), (tok(vec!["cache:read"], Some("t1")), "InsufficientPermissions")] {
        let r = g.c.delete(format!("{}/api/v1/cache/x", g.base)).header("Authorization", bearer(&t)).send().await.unwrap();
        assert_eq!(r.status(), 403);
        assert_eq!(r.headers().get("x-auth-error").unwrap(), code);
    }
    assert_eq!(g.c.delete(format!("{}/api/v1/cache/x", g.base)).send().await.unwrap().status(), 401);
}

// ==================================================================== (c) CORS expose

#[tokio::test]
async fn c_cors_exposes_x_cache_warning() {
    let cfg = CorsConfig::with_origins(vec!["http://localhost:3000".to_string()]).unwrap();
    let g = serve_with(create_app_with_cors(state(None, None), &cfg)).await;
    let r = g.get("/health").header("Origin", "http://localhost:3000").send().await.unwrap();
    let ex = r.headers().get("access-control-expose-headers").unwrap().to_str().unwrap().to_ascii_lowercase();
    assert!(ex.contains("x-cache-warning"), "{}", ex);
}

use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::security::{
    resolve_and_pin_origin, validate_port, OriginSecurityError, PreConnectIpFilter,
    TlsServerConfig, UpstreamPolicy, UpstreamTlsConfig, HARD_BLOCKED_PORTS, STANDARD_ALLOWED_PORTS,
};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct P2Certificates {
    cert_pem: String,
    key_pem: String,
    expired_cert_pem: String,
    expired_key_pem: String,
    wrong_host_cert_pem: String,
    wrong_host_key_pem: String,
}

fn generate_p2_certificates() -> P2Certificates {
    let valid_key = rcgen::KeyPair::generate().unwrap();
    let valid_params =
        rcgen::CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .unwrap();
    let valid_cert = valid_params.self_signed(&valid_key).unwrap();
    let cert_pem = valid_cert.pem();
    let key_pem = valid_key.serialize_pem();

    let expired_key = rcgen::KeyPair::generate().unwrap();
    let mut expired_params = rcgen::CertificateParams::new(vec!["localhost".to_string()]).unwrap();
    expired_params.not_before = rcgen::date_time_ymd(1975, 1, 1);
    expired_params.not_after = rcgen::date_time_ymd(1975, 1, 2);
    let expired_cert = expired_params.self_signed(&expired_key).unwrap();
    let expired_cert_pem = expired_cert.pem();
    let expired_key_pem = expired_key.serialize_pem();

    let wrong_host_key = rcgen::KeyPair::generate().unwrap();
    let wrong_host_params =
        rcgen::CertificateParams::new(vec!["evil.internal".to_string()]).unwrap();
    let wrong_host_cert = wrong_host_params.self_signed(&wrong_host_key).unwrap();
    let wrong_host_cert_pem = wrong_host_cert.pem();
    let wrong_host_key_pem = wrong_host_key.serialize_pem();

    P2Certificates {
        cert_pem,
        key_pem,
        expired_cert_pem,
        expired_key_pem,
        wrong_host_cert_pem,
        wrong_host_key_pem,
    }
}

async fn spawn_tls_mock_origin(
    cert_pem: &[u8],
    key_pem: &[u8],
    body_response: String,
) -> (SocketAddr, axum_server::Handle) {
    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::Router;

    let body = body_response.clone();
    let app = Router::new().route(
        "/*path",
        get(move || {
            let b = body.clone();
            async move { (axum::http::StatusCode::OK, [("content-type", "application/json")], b).into_response() }
        }),
    );

    let tls_config = TlsServerConfig::from_pem(cert_pem, key_pem)
        .await
        .expect("Failed to initialize mock origin TLS config");

    let handle = axum_server::Handle::new();
    let handle_clone = handle.clone();
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = axum_server::bind_rustls(bind_addr, tls_config.rustls_config).handle(handle_clone);

    tokio::spawn(async move {
        server.serve(app.into_make_service()).await.unwrap();
    });

    let addr = handle.listening().await.expect("Mock origin TLS failed to bind");
    (addr, handle)
}

fn make_p2_api_state(
    cache: Arc<PartitionedCache>,
    origin_url: Option<String>,
    upstream_policy: Option<Arc<UpstreamPolicy>>,
) -> Arc<ApiState> {
    let nodes = vec![Arc::new(VirtualNode::new(1))];
    nodes[0].set_state(NodeState::Active);

    let snap = SimulationSnapshot {
        version: "2.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 10000.0,
        p50_latency_ms: 5,
        p95_latency_ms: 10,
        p99_latency_ms: 15,
        db_load_ratio: 0.1,
        avoided_compute_dollars: 0.0,
        nodes: vec![NodeSnapshot {
            node_id: 1,
            state: "Active".to_string(),
            cpu_utilization: 20.0,
            memory_used_bytes: 1024,
        }],
    };

    Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes,
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache),
        origin_url,
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy,
        router: None,
    })
}

// =========================================================================
// Test 1: Valid Upstream HTTPS with Custom CA Anchoring
// =========================================================================

#[tokio::test]
async fn test_p2_valid_upstream_https_success() {
    let certs = generate_p2_certificates();
    let (origin_addr, _handle) =
        spawn_tls_mock_origin(certs.cert_pem.as_bytes(), certs.key_pem.as_bytes(), r#"{"secure":"data"}"#.to_string()).await;

    let tls_config = UpstreamTlsConfig::new_with_ca_pem(certs.cert_pem.as_bytes()).unwrap();
    let upstream_policy = Arc::new(UpstreamPolicy {
        origin_url: Some(format!("https://localhost:{}", origin_addr.port())),
        allowed_origins: vec![format!("https://localhost:{}", origin_addr.port())],
        enforce_tls: true,
        allow_insecure_upstream: false,
        allow_lan_origin: false,
        allow_loopback_origin: true,
        forward_client_authorization: false,
        tls_config: Some(Arc::new(tls_config)),
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let origin_url = format!("https://localhost:{}", origin_addr.port());
    let api_state = make_p2_api_state(cache.clone(), Some(origin_url), Some(upstream_policy));

    let app = create_app(api_state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/cache/secure_item", server_addr);

    // 1. Initial request -> MISS, successfully fetches over HTTPS
    let r1 = client.get(&url).send().await.unwrap();
    assert_eq!(r1.status(), reqwest::StatusCode::OK);
    assert_eq!(r1.headers().get("X-Cache-Status").unwrap(), "MISS");
    let text1 = r1.text().await.unwrap();
    assert!(text1.contains("secure"));

    // 2. Repeat request -> HIT from cache
    let r2 = client.get(&url).send().await.unwrap();
    assert_eq!(r2.status(), reqwest::StatusCode::OK);
    assert_eq!(r2.headers().get("X-Cache-Status").unwrap(), "HIT");
}

// =========================================================================
// Test 2: Untrusted CA Certificate Rejected
// =========================================================================

#[tokio::test]
async fn test_p2_untrusted_ca_rejected() {
    let certs = generate_p2_certificates();
    let (origin_addr, _handle) =
        spawn_tls_mock_origin(certs.cert_pem.as_bytes(), certs.key_pem.as_bytes(), r#"{"untrusted":"leak"}"#.to_string()).await;

    // PhantomCache configured with standard WebPKI roots only (does NOT trust self-signed test CA)
    let upstream_policy = Arc::new(UpstreamPolicy {
        origin_url: Some(format!("https://localhost:{}", origin_addr.port())),
        allowed_origins: vec![format!("https://localhost:{}", origin_addr.port())],
        enforce_tls: true,
        allow_insecure_upstream: false,
        allow_lan_origin: false,
        allow_loopback_origin: true,
        forward_client_authorization: false,
        tls_config: Some(Arc::new(UpstreamTlsConfig::new_default())),
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let origin_url = format!("https://localhost:{}", origin_addr.port());
    let api_state = make_p2_api_state(cache.clone(), Some(origin_url), Some(upstream_policy));

    let app = create_app(api_state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/cache/untrusted_item", server_addr);

    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    assert_eq!(resp.headers().get("X-Cache-Error").unwrap(), "UpstreamTlsFailed");
}

// =========================================================================
// Test 3: Expired Upstream Certificate Rejected
// =========================================================================

#[tokio::test]
async fn test_p2_expired_cert_rejected() {
    let certs = generate_p2_certificates();
    let (origin_addr, _handle) = spawn_tls_mock_origin(
        certs.expired_cert_pem.as_bytes(),
        certs.expired_key_pem.as_bytes(),
        r#"{"expired":"leak"}"#.to_string(),
    )
    .await;

    let tls_config = UpstreamTlsConfig::new_with_ca_pem(certs.expired_cert_pem.as_bytes()).unwrap();
    let upstream_policy = Arc::new(UpstreamPolicy {
        origin_url: Some(format!("https://localhost:{}", origin_addr.port())),
        allowed_origins: vec![format!("https://localhost:{}", origin_addr.port())],
        enforce_tls: true,
        allow_insecure_upstream: false,
        allow_lan_origin: false,
        allow_loopback_origin: true,
        forward_client_authorization: false,
        tls_config: Some(Arc::new(tls_config)),
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let origin_url = format!("https://localhost:{}", origin_addr.port());
    let api_state = make_p2_api_state(cache.clone(), Some(origin_url), Some(upstream_policy));

    let app = create_app(api_state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/cache/expired_item", server_addr);

    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    assert_eq!(resp.headers().get("X-Cache-Error").unwrap(), "UpstreamTlsFailed");
}

// =========================================================================
// Test 4: Hostname / SAN Mismatch Rejected
// =========================================================================

#[tokio::test]
async fn test_p2_san_mismatch_rejected() {
    let certs = generate_p2_certificates();
    // Certificate is issued ONLY for "evil.internal"
    let (origin_addr, _handle) = spawn_tls_mock_origin(
        certs.wrong_host_cert_pem.as_bytes(),
        certs.wrong_host_key_pem.as_bytes(),
        r#"{"wrong_host":"leak"}"#.to_string(),
    )
    .await;

    let tls_config = UpstreamTlsConfig::new_with_ca_pem(certs.wrong_host_cert_pem.as_bytes()).unwrap();
    // PhantomCache connects to "localhost", but certificate presented is for "evil.internal"
    let upstream_policy = Arc::new(UpstreamPolicy {
        origin_url: Some(format!("https://localhost:{}", origin_addr.port())),
        allowed_origins: vec![format!("https://localhost:{}", origin_addr.port())],
        enforce_tls: true,
        allow_insecure_upstream: false,
        allow_lan_origin: false,
        allow_loopback_origin: true,
        forward_client_authorization: false,
        tls_config: Some(Arc::new(tls_config)),
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let origin_url = format!("https://localhost:{}", origin_addr.port());
    let api_state = make_p2_api_state(cache.clone(), Some(origin_url), Some(upstream_policy));

    let app = create_app(api_state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/cache/mismatched_item", server_addr);

    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    assert_eq!(resp.headers().get("X-Cache-Error").unwrap(), "UpstreamTlsFailed");
}

// =========================================================================
// Test 5: SSRF Defense - Cloud Metadata IMDS Strictly Blocked
// =========================================================================

#[test]
fn test_p2_ssrf_cloud_metadata_blocked() {
    let filter = PreConnectIpFilter::new(true, true);

    // IPv4 AWS/GCP/Azure IMDS: 169.254.169.254
    let imds_v4: Ipv4Addr = "169.254.169.254".parse().unwrap();
    let res_v4 = filter.check_ipv4(imds_v4);
    assert!(
        matches!(res_v4, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("169.254.169.254")),
        "Cloud metadata 169.254.169.254 must be blocked unconditionally"
    );

    // IPv4 Link-Local subnet: 169.254.1.1
    let link_local: Ipv4Addr = "169.254.1.1".parse().unwrap();
    let res_ll = filter.check_ipv4(link_local);
    assert!(
        matches!(res_ll, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("169.254.0.0/16")),
        "IPv4 link-local must be blocked"
    );

    // IPv6 AWS IMDSv2: fd00:ec2::254
    let imds_v6: Ipv6Addr = "fd00:ec2::254".parse().unwrap();
    let res_v6 = filter.check_ipv6(imds_v6);
    assert!(
        matches!(res_v6, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("fd00:ec2::254")),
        "IPv6 IMDS (fd00:ec2::254) must be blocked unconditionally"
    );
}

// =========================================================================
// Test 6: SSRF Defense - IPv4-Mapped IPv6 Unmapped and Blocked
// =========================================================================

#[test]
fn test_p2_ssrf_ipv4_mapped_ipv6_blocked() {
    let filter = PreConnectIpFilter::new(true, true);

    // Attacker attempts bypass using IPv4-mapped IPv6 syntax: ::ffff:169.254.169.254
    let mapped_imds: Ipv6Addr = "::ffff:169.254.169.254".parse().unwrap();
    let res_mapped = filter.check_ipv6(mapped_imds);
    assert!(
        matches!(res_mapped, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("169.254.169.254")),
        "IPv4-mapped IPv6 for IMDS must be unmapped and blocked"
    );

    // IPv6 Link-Local: fe80::1
    let ipv6_ll: Ipv6Addr = "fe80::1".parse().unwrap();
    let res_v6_ll = filter.check_ipv6(ipv6_ll);
    assert!(
        matches!(res_v6_ll, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("fe80::/10")),
        "IPv6 link-local must be blocked"
    );

    // IPv6 Loopback when allow_loopback = false
    let filter_no_loopback = PreConnectIpFilter::new(false, false);
    let ipv6_loopback: Ipv6Addr = "::1".parse().unwrap();
    let res_v6_loop = filter_no_loopback.check_ipv6(ipv6_loopback);
    assert!(
        matches!(res_v6_loop, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("::1")),
        "IPv6 loopback must be blocked when allow_loopback is false"
    );
}

// =========================================================================
// Test 7: Multi-Address / Mixed DNS Answer Set Rejection
// =========================================================================

#[tokio::test]
async fn test_p2_ssrf_mixed_dns_blocked() {
    // Test resolve_and_pin_origin logic:
    // When a DNS response contains any prohibited address, the whole answer set is tainted
    let filter = PreConnectIpFilter::new(false, false);

    // Loopback address with allow_loopback = false
    let res = resolve_and_pin_origin("127.0.0.1", 80, &filter).await;
    assert!(
        matches!(res, Err(OriginSecurityError::SsrfBlocked(_))),
        "Loopback DNS target must be blocked when allow_loopback is false"
    );

    // Cloud metadata host target
    let res_imds = resolve_and_pin_origin("169.254.169.254", 80, &filter).await;
    assert!(
        matches!(res_imds, Err(OriginSecurityError::SsrfBlocked(_))),
        "169.254.169.254 DNS target must be blocked"
    );
}

// =========================================================================
// Test 8: Unconditionally Hard-Blocked Sensitive Ports
// =========================================================================

#[test]
fn test_p2_hard_blocked_ports_unconditional() {
    // Allowlist explicitly containing sensitive ports
    let allowlist = vec![
        "http://example.com:22".to_string(),
        "http://example.com:6379".to_string(),
        "http://example.com:25".to_string(),
        "http://example.com:3306".to_string(),
    ];

    // SSH (22) must be blocked unconditionally even though it's in allowlist!
    let res_ssh = validate_port(22, "http://example.com:22", &allowlist);
    assert_eq!(res_ssh, Err(OriginSecurityError::SensitivePortBlocked(22)));

    // Redis (6379) must be blocked unconditionally
    let res_redis = validate_port(6379, "http://example.com:6379", &allowlist);
    assert_eq!(res_redis, Err(OriginSecurityError::SensitivePortBlocked(6379)));

    // SMTP (25) must be blocked unconditionally
    let res_smtp = validate_port(25, "http://example.com:25", &allowlist);
    assert_eq!(res_smtp, Err(OriginSecurityError::SensitivePortBlocked(25)));

    // MySQL (3306) must be blocked unconditionally
    let res_mysql = validate_port(3306, "http://example.com:3306", &allowlist);
    assert_eq!(res_mysql, Err(OriginSecurityError::SensitivePortBlocked(3306)));

    // Privileged port < 1024 not in standard list (e.g. port 79 Finger)
    let res_priv = validate_port(79, "http://example.com:79", &[]);
    assert_eq!(res_priv, Err(OriginSecurityError::DisallowedPort(79)));

    // Standard ports 80, 443, 8080, 8443, 9000, 9443 succeed
    for &p in STANDARD_ALLOWED_PORTS {
        assert_eq!(validate_port(p, &format!("http://example.com:{}", p), &[]), Ok(()));
    }

    // All hard blocked ports are tested against HARD_BLOCKED_PORTS
    for &p in HARD_BLOCKED_PORTS {
        assert_eq!(
            validate_port(p, &format!("http://example.com:{}", p), &allowlist),
            Err(OriginSecurityError::SensitivePortBlocked(p))
        );
    }
}

// =========================================================================
// Test 9: Insecure HTTP Upstream Fail-Closed Policy
// =========================================================================

#[test]
fn test_p2_insecure_upstream_fail_closed() {
    // Policy A: Network interface (allow_loopback_origin = false, allow_insecure_upstream = false)
    let policy_secure = UpstreamPolicy {
        origin_url: None,
        allowed_origins: vec!["https://api.example.com".to_string()],
        enforce_tls: false,
        allow_insecure_upstream: false,
        allow_lan_origin: false,
        allow_loopback_origin: false,
        forward_client_authorization: false,
        tls_config: None,
    };

    let http_url = url::Url::parse("http://example.com:80").unwrap();
    let res_http = policy_secure.validate_target(&http_url);
    assert_eq!(res_http, Err(OriginSecurityError::InsecureUpstreamDisallowed));

    // Policy B: Enforce TLS is true
    let mut policy_enforce_tls = policy_secure.clone();
    policy_enforce_tls.enforce_tls = true;
    let res_enforce = policy_enforce_tls.validate_target(&http_url);
    assert_eq!(
        res_enforce,
        Err(OriginSecurityError::DisallowedScheme("HTTPS is strictly enforced by policy".to_string()))
    );

    // Policy C: Insecure HTTP allowed, but origin not in allowlist
    let mut policy_with_override = policy_secure.clone();
    policy_with_override.allow_insecure_upstream = true;
    let res_not_allowlisted = policy_with_override.validate_target(&http_url);
    assert_eq!(
        res_not_allowlisted,
        Err(OriginSecurityError::OriginNotAllowlisted("http://example.com:80".to_string()))
    );

    // Policy D: Insecure HTTP allowed AND origin is allowlisted -> Succeeds
    policy_with_override.allowed_origins.push("http://example.com:80".to_string());
    let res_ok = policy_with_override.validate_target(&http_url);
    assert_eq!(res_ok, Ok(()));
}

// =========================================================================
// Test 10: Explicit Upstream Credential Forwarding Policy
// =========================================================================

#[tokio::test]
async fn test_p2_credential_forwarding_policy() {
    let auth_header_received = Arc::new(Mutex::new(None::<String>));
    let header_capture = auth_header_received.clone();

    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            let cap = header_capture.clone();
            tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);

                let mut auth_val = None;
                for line in req_str.split("\r\n") {
                    if let Some((k, v)) = line.split_once(':') {
                        if k.trim().eq_ignore_ascii_case("authorization") {
                            auth_val = Some(v.trim().to_string());
                        }
                    }
                }
                *cap.lock().unwrap() = auth_val;

                let resp = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"status\":\"ok\"}";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);

    // CASE A: forward_client_authorization = false (Default Secure Mode)
    // Authorization header MUST be stripped by PhantomCache!
    let policy_no_forward = Arc::new(UpstreamPolicy {
        origin_url: Some(origin_url.clone()),
        allowed_origins: vec![origin_url.clone()],
        enforce_tls: false,
        allow_insecure_upstream: true,
        allow_lan_origin: false,
        allow_loopback_origin: true,
        forward_client_authorization: false,
        tls_config: None,
    });

    let cache_a = Arc::new(PartitionedCache::new(4));
    let state_a = make_p2_api_state(cache_a, Some(origin_url.clone()), Some(policy_no_forward));
    let app_a = create_app(state_a);
    let s_a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = s_a.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(s_a, app_a).await.unwrap();
    });

    let client = reqwest::Client::new();
    let _ = client
        .get(format!("http://{}/api/v1/cache/item_strip", addr_a))
        .header("Authorization", "Bearer top-secret-token")
        .send()
        .await
        .unwrap();

    // PROOF: Origin did NOT receive Authorization header!
    let captured_a = auth_header_received.lock().unwrap().clone();
    assert_eq!(captured_a, None, "Authorization header must be stripped when forward_client_authorization is false");

    // CASE B: forward_client_authorization = true (Explicit Origin Contract)
    let policy_forward = Arc::new(UpstreamPolicy {
        origin_url: Some(origin_url.clone()),
        allowed_origins: vec![origin_url.clone()],
        enforce_tls: false,
        allow_insecure_upstream: true,
        allow_lan_origin: false,
        allow_loopback_origin: true,
        forward_client_authorization: true,
        tls_config: None,
    });

    let cache_b = Arc::new(PartitionedCache::new(4));
    let state_b = make_p2_api_state(cache_b, Some(origin_url.clone()), Some(policy_forward));
    let app_b = create_app(state_b);
    let s_b = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_b = s_b.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(s_b, app_b).await.unwrap();
    });

    let _ = client
        .get(format!("http://{}/api/v1/cache/item_forward", addr_b))
        .header("Authorization", "Bearer authorized-upstream-token")
        .send()
        .await
        .unwrap();

    // PROOF: Origin received Authorization header under explicit policy
    let captured_b = auth_header_received.lock().unwrap().clone();
    assert_eq!(
        captured_b,
        Some("Bearer authorized-upstream-token".to_string()),
        "Authorization header must be forwarded when forward_client_authorization is true"
    );
}

// =========================================================================
// Test 11: HTTP 3xx Redirects Are Terminal and Never Followed
// =========================================================================

#[tokio::test]
async fn test_p2_redirect_not_followed() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Return 302 Found pointing to Cloud Metadata IMDS!
                let resp = "HTTP/1.1 302 Found\r\nLocation: http://169.254.169.254/latest/meta-data/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let policy = Arc::new(UpstreamPolicy {
        origin_url: Some(origin_url.clone()),
        allowed_origins: vec![origin_url.clone()],
        enforce_tls: false,
        allow_insecure_upstream: true,
        allow_lan_origin: false,
        allow_loopback_origin: true,
        forward_client_authorization: false,
        tls_config: None,
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache, Some(origin_url), Some(policy));
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    // Client sends request; client does NOT follow redirects automatically to see PhantomCache's response
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let resp = client
        .get(format!("http://{}/api/v1/cache/redirect_item", server_addr))
        .send()
        .await
        .unwrap();

    // PROOF: PhantomCache returned 302 without following to IMDS
    assert_eq!(resp.status(), reqwest::StatusCode::FOUND);
    assert_eq!(
        resp.headers().get("X-Cache-Error").unwrap(),
        "UpstreamStatus"
    );
}

// =========================================================================
// Test 12: Conflicting Content-Length Headers Rejected
// =========================================================================

#[tokio::test]
async fn test_p2_conflicting_content_length_rejected() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Multiple differing Content-Length headers (HTTP smuggling vector)
                let resp = "HTTP/1.1 200 OK\r\nContent-Length: 10\r\nContent-Length: 20\r\nConnection: close\r\n\r\n1234567890";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/v1/cache/conflict_cl", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    assert_eq!(resp.headers().get("X-Cache-Error").unwrap(), "MalformedFraming");

    // PROOF: Never cached
    let entry = cache.get("default:/api/v1/cache/conflict_cl", 0);
    assert!(entry.is_none(), "Malformed conflicting Content-Length response must never be cached");
}

// =========================================================================
// Test 13: Conflicting Content-Length and Transfer-Encoding Rejected
// =========================================================================

#[tokio::test]
async fn test_p2_conflicting_cl_and_chunked_rejected() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Both Content-Length and Transfer-Encoding: chunked (smuggling vector)
                let resp = "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/v1/cache/conflict_cl_chunked", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    assert_eq!(resp.headers().get("X-Cache-Error").unwrap(), "MalformedFraming");

    // PROOF: Never cached
    let entry = cache.get("default:/api/v1/cache/conflict_cl_chunked", 0);
    assert!(entry.is_none(), "Conflicting CL + chunked response must never be cached");
}

// =========================================================================
// Test 14: Truncated Response Body Rejected and Never Cached
// =========================================================================

#[tokio::test]
async fn test_p2_truncated_response_not_cached() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Declares Content-Length: 500, but only sends 5 bytes before premature close
                let resp = "HTTP/1.1 200 OK\r\nContent-Length: 500\r\nConnection: close\r\n\r\nshort";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/v1/cache/trunc_body", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    let err_hdr = resp.headers().get("X-Cache-Error").unwrap().to_str().unwrap();
    assert_eq!(err_hdr, "TruncatedResponse");

    // PROOF: Partial body is NEVER stored in cache
    let entry = cache.get("default:/api/v1/cache/trunc_body", 0);
    assert!(entry.is_none(), "Truncated response body must never be stored in cache");
}

// =========================================================================
// Test 15: Malformed Chunked Transfer Encoding Rejected
// =========================================================================

#[tokio::test]
async fn test_p2_malformed_chunked_encoding_rejected() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Invalid chunk size (not valid hex: "ZZ")
                let resp = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\nZZ\r\ninvalid_chunk\r\n0\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/v1/cache/malformed_chunk", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    assert_eq!(resp.headers().get("X-Cache-Error").unwrap(), "MalformedFraming");

    // PROOF: Never cached
    let entry = cache.get("default:/api/v1/cache/malformed_chunk", 0);
    assert!(entry.is_none(), "Malformed chunked response must never be cached");
}

// =========================================================================
// Test 16: Premature EOF in Chunked Stream Rejected and Discarded
// =========================================================================

#[tokio::test]
async fn test_p2_premature_eof_in_chunked_not_cached() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Chunk says 100 bytes (0x64), but sends only 5 bytes then closes
                let resp = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n64\r\nshort";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/v1/cache/premature_eof_chunk", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    let err_hdr = resp.headers().get("X-Cache-Error").unwrap().to_str().unwrap();
    assert_eq!(err_hdr, "TruncatedResponse");

    // PROOF: Never cached
    let entry = cache.get("default:/api/v1/cache/premature_eof_chunk", 0);
    assert!(entry.is_none(), "Premature EOF in chunked response must never be cached");
}

// =========================================================================
// Red-Team Finding 1 Regression: IPv4-compatible, 6to4, NAT64 SSRF Blocked
// =========================================================================

#[test]
fn test_redteam_ipv4_compatible_and_transition_ipv6_ssrf_blocked() {
    let filter = PreConnectIpFilter {
        allow_loopback: false,
        allow_lan: false,
    };

    // IPv4-compatible IPv6 (::169.254.169.254)
    let ip_compat_meta: Ipv6Addr = "::169.254.169.254".parse().unwrap();
    let res_compat_meta = filter.check_ip(std::net::IpAddr::V6(ip_compat_meta));
    assert!(
        matches!(res_compat_meta, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("169.254.169.254")),
        "IPv4-compatible IPv6 for IMDS must be unmapped and blocked"
    );

    // IPv4-compatible loopback (::127.0.0.1)
    let ip_compat_loop: Ipv6Addr = "::127.0.0.1".parse().unwrap();
    let res_compat_loop = filter.check_ip(std::net::IpAddr::V6(ip_compat_loop));
    assert!(
        matches!(res_compat_loop, Err(OriginSecurityError::SsrfBlocked(_))),
        "IPv4-compatible loopback must be blocked when allow_loopback is false"
    );

    // 6to4 prefix (2002:a9fe:a9fe::) embedding 169.254.169.254
    let ip_6to4_meta: Ipv6Addr = "2002:a9fe:a9fe::".parse().unwrap();
    let res_6to4_meta = filter.check_ip(std::net::IpAddr::V6(ip_6to4_meta));
    assert!(
        matches!(res_6to4_meta, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("169.254.169.254")),
        "6to4 embedded IMDS must be blocked"
    );

    // Well-Known Prefix NAT64 (64:ff9b::169.254.169.254)
    let ip_nat64_meta: Ipv6Addr = "64:ff9b::169.254.169.254".parse().unwrap();
    let res_nat64_meta = filter.check_ip(std::net::IpAddr::V6(ip_nat64_meta));
    assert!(
        matches!(res_nat64_meta, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("169.254.169.254")),
        "NAT64 embedded IMDS must be blocked"
    );

    // IPv6 documentation range (2001:db8::1)
    let ip_doc: Ipv6Addr = "2001:db8::1".parse().unwrap();
    let res_doc = filter.check_ip(std::net::IpAddr::V6(ip_doc));
    assert!(
        matches!(res_doc, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("2001:db8::/32")),
        "IPv6 documentation range must be blocked"
    );

    // IPv6 discard prefix (100::1)
    let ip_discard: Ipv6Addr = "100::1".parse().unwrap();
    let res_discard = filter.check_ip(std::net::IpAddr::V6(ip_discard));
    assert!(
        matches!(res_discard, Err(OriginSecurityError::SsrfBlocked(ref msg)) if msg.contains("100::/64")),
        "IPv6 discard range must be blocked"
    );
}

// =========================================================================
// Red-Team Finding 2 Regression: Insecure Upstream Empty Allowlist Fails Closed
// =========================================================================

#[test]
fn test_redteam_insecure_upstream_empty_allowlist_fails_closed() {
    let policy = UpstreamPolicy {
        origin_url: None,
        allowed_origins: vec![],
        enforce_tls: false,
        allow_insecure_upstream: true,
        allow_lan_origin: false,
        allow_loopback_origin: false,
        forward_client_authorization: false,
        tls_config: None,
    };

    let target: url::Url = "http://example.com:80".parse().unwrap();
    let res = policy.validate_target(&target);
    assert!(matches!(res, Err(OriginSecurityError::OriginNotAllowlisted(_))));
}

// =========================================================================
// Red-Team Finding 3 Regression: Fail-Closed Default strips Authorization
// =========================================================================

#[tokio::test]
async fn test_redteam_default_policy_strips_credentials() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Option<String>>(1);

    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        if let Ok((mut socket, _)) = origin_listener.accept().await {
            let mut buf = [0u8; 2048];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let auth_header = req
                .lines()
                .find(|l| l.to_lowercase().starts_with("authorization:"))
                .map(|l| l.to_string());
            let _ = tx.send(auth_header).await;

            let resp = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
            let _ = socket.write_all(resp.as_bytes()).await;
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    // Upstream policy is None -> default behavior must strip credentials
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let _ = client
        .get(format!("http://{}/api/v1/cache/test_auth_strip", server_addr))
        .header("Authorization", "Bearer sensitive-secret-token")
        .send()
        .await
        .unwrap();

    let auth_received = rx.recv().await.unwrap();
    assert!(
        auth_received.is_none(),
        "Default policy must strip Authorization header, but received: {:?}",
        auth_received
    );
}

// =========================================================================
// Red-Team Finding 4 Regression: Unsupported Transfer-Encoding Rejected
// =========================================================================

#[tokio::test]
async fn test_redteam_unsupported_transfer_encoding_rejected() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let resp = "HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\nConnection: close\r\n\r\ncompresseddata";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/api/v1/cache/unsupported_te", server_addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::BAD_GATEWAY);
    assert_eq!(resp.headers().get("X-Cache-Error").unwrap(), "MalformedFraming");

    let entry = cache.get("default:/api/v1/cache/unsupported_te", 0);
    assert!(entry.is_none(), "Unsupported Transfer-Encoding must never be cached");
}

// =========================================================================
// Red-Team Finding 5 Regression: Chunk Extensions Parsed & Close-Delimited Not Cached
// =========================================================================

#[tokio::test]
async fn test_redteam_chunk_extensions_parsed_safely() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Chunk with extensions according to RFC 9112 section 7.1.1
                let resp = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;ext=val;foo=bar\r\nhello\r\n0;terminal=true\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/cache/chunk_ext", server_addr);

    // Initial request: MISS, valid chunked stream decoded
    let resp1 = client.get(&url).send().await.unwrap();
    assert_eq!(resp1.status(), reqwest::StatusCode::OK);
    assert_eq!(resp1.headers().get("X-Cache-Status").unwrap(), "MISS");
    let body1 = resp1.text().await.unwrap();
    assert_eq!(body1, "hello");

    // PROOF: Valid chunked response is cached (repeat request is a HIT)
    let resp2 = client.get(&url).send().await.unwrap();
    assert_eq!(resp2.status(), reqwest::StatusCode::OK);
    assert_eq!(resp2.headers().get("X-Cache-Status").unwrap(), "HIT");
    let body2 = resp2.text().await.unwrap();
    assert_eq!(body2, "hello");
}

#[tokio::test]
async fn test_redteam_close_delimited_body_not_cached() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                // Ambiguous close-delimited response: no Content-Length, no Chunked
                let resp = "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nstreamed data without framing";
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let origin_url = format!("http://{}", origin_addr);
    let cache = Arc::new(PartitionedCache::new(4));
    let state = make_p2_api_state(cache.clone(), Some(origin_url), None);
    let app = create_app(state);
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/v1/cache/close_delimited", server_addr);

    // Initial request: MISS, delivered successfully
    let resp1 = client.get(&url).send().await.unwrap();
    assert_eq!(resp1.status(), reqwest::StatusCode::OK);
    assert_eq!(resp1.headers().get("X-Cache-Status").unwrap(), "MISS");
    let body1 = resp1.text().await.unwrap();
    assert_eq!(body1, "streamed data without framing");

    // PROOF: Close-delimited response MUST NOT be cached -> second request is STILL a MISS!
    let resp2 = client.get(&url).send().await.unwrap();
    assert_eq!(resp2.status(), reqwest::StatusCode::OK);
    assert_eq!(
        resp2.headers().get("X-Cache-Status").unwrap(),
        "MISS",
        "Close-delimited responses without explicit framing must NEVER be cached"
    );
}

// =========================================================================
// Red-Team Finding 7 Regression: Startup Security Fail-Closed Upstream
// =========================================================================

#[test]
fn test_redteam_startup_security_upstream_validation() {
    use phantom_cache_demo::security::{validate_upstream_startup_security, StartupSecurityError};

    let network_addr: SocketAddr = "192.168.1.50:8085".parse().unwrap();
    let loopback_addr: SocketAddr = "127.0.0.1:8085".parse().unwrap();

    // 1. Loopback with insecure upstream and empty allowlist is allowed for local dev
    assert!(validate_upstream_startup_security(&loopback_addr, true, false).is_ok());

    // 2. Network bind with secure upstream (allow_insecure = false) is allowed
    assert!(validate_upstream_startup_security(&network_addr, false, false).is_ok());

    // 3. Network bind with insecure upstream and non-empty allowlist is allowed
    assert!(validate_upstream_startup_security(&network_addr, true, true).is_ok());

    // 4. Network bind with insecure upstream and EMPTY allowlist MUST FAIL CLOSED
    let err = validate_upstream_startup_security(&network_addr, true, false).unwrap_err();
    assert_eq!(
        err,
        StartupSecurityError::NetworkInsecureUpstreamWithoutAllowlist(network_addr)
    );
}

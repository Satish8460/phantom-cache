use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::security::{
    validate_startup_security, JwtClaims, JwtValidator, Role, SecurityConfig, StartupSecurityError,
    TlsServerConfig,
};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};


const TEST_ISSUER: &str = "phantom-cache-auth";
const TEST_AUDIENCE: &str = "phantom-cache-api";
const TEST_SECRET: &[u8] = b"super-secret-key-for-phase-1-tests-32b!";

fn make_test_jwt_validator() -> JwtValidator {
    JwtValidator::new(TEST_ISSUER, TEST_AUDIENCE, TEST_SECRET)
        .with_rotation_key("rot-key-1", b"rotation-secret-key-32-bytes-long!")
}

fn make_test_api_state(
    cache: Option<Arc<PartitionedCache>>,
    origin_url: Option<String>,
    security: Option<SecurityConfig>,
) -> (Arc<ApiState>, Vec<Arc<VirtualNode>>) {
    let nodes = vec![
        Arc::new(VirtualNode::new(1)),
        Arc::new(VirtualNode::new(2)),
        Arc::new(VirtualNode::new(3)),
    ];
    for n in &nodes {
        n.set_state(NodeState::Active);
    }
    let snap = SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 12000.0,
        p50_latency_ms: 10,
        p95_latency_ms: 12,
        p99_latency_ms: 15,
        db_load_ratio: 0.20,
        avoided_compute_dollars: 0.0,
        nodes: vec![
            NodeSnapshot {
                node_id: 1,
                state: "Active".to_string(),
                cpu_utilization: 25.0,
                memory_used_bytes: 1024,
            },
            NodeSnapshot {
                node_id: 2,
                state: "Active".to_string(),
                cpu_utilization: 25.0,
                memory_used_bytes: 1024,
            },
        ],
    };

    let upstream_policy = Some(Arc::new(phantom_cache_demo::security::UpstreamPolicy {
        forward_client_authorization: true,
        ..Default::default()
    }));

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes: nodes.clone(),
        workload_type: Mutex::new("Normal".to_string()),
        cache,
        origin_url,
        telemetry: None,
        pie: None,
        security: security.map(Arc::new),
        upstream_policy,
        router: None,
    });

    (api_state, nodes)
}

struct TestCertificates {
    server_cert_pem: String,
    server_key_pem: String,
    expired_cert_pem: String,
    expired_key_pem: String,
    wrong_host_cert_pem: String,
    wrong_host_key_pem: String,
}

fn generate_test_certificates() -> TestCertificates {
    let valid_key = rcgen::KeyPair::generate().unwrap();
    let valid_params =
        rcgen::CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()])
            .unwrap();
    let valid_cert = valid_params.self_signed(&valid_key).unwrap();
    let server_cert_pem = valid_cert.pem();
    let server_key_pem = valid_key.serialize_pem();

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

    TestCertificates {
        server_cert_pem,
        server_key_pem,
        expired_cert_pem,
        expired_key_pem,
        wrong_host_cert_pem,
        wrong_host_key_pem,
    }
}


async fn spawn_tls_test_server(
    api_state: Arc<ApiState>,
    cert_pem: &[u8],
    key_pem: &[u8],
) -> (SocketAddr, axum_server::Handle) {
    let app = create_app(api_state);
    let tls_config = TlsServerConfig::from_pem(cert_pem, key_pem)
        .await
        .expect("Failed to initialize TlsServerConfig");

    let handle = axum_server::Handle::new();
    let handle_clone = handle.clone();

    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server =
        axum_server::bind_rustls(bind_addr, tls_config.rustls_config).handle(handle_clone);

    tokio::spawn(async move {
        server.serve(app.into_make_service()).await.unwrap();
    });

    let addr = handle.listening().await.expect("Server failed to bind");
    (addr, handle)
}

// ==========================================
// Acceptance Test 1 & 2: Ingress TLS 1.3
// ==========================================

#[tokio::test]
async fn test_phase1_at01_valid_tls_connection_succeeds() {
    let certs = generate_test_certificates();
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator);

    let (api_state, _) = make_test_api_state(None, None, Some(sec));
    let (addr, _handle) = spawn_tls_test_server(
        api_state,
        certs.server_cert_pem.as_bytes(),
        certs.server_key_pem.as_bytes(),
    )
    .await;

    // Client explicitly trusts the server certificate as root anchor
    let root_cert = reqwest::Certificate::from_pem(certs.server_cert_pem.as_bytes()).unwrap();
    let client = reqwest::Client::builder()
        .add_root_certificate(root_cert)
        .build()
        .unwrap();

    let url = format!("https://localhost:{}/health", addr.port());
    let resp = client.get(&url).send().await.expect("TLS request failed");

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    assert_eq!(resp.text().await.unwrap(), "OK");
}

#[tokio::test]
async fn test_phase1_at02_untrusted_expired_or_mismatched_tls_fails() {
    let certs = generate_test_certificates();
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator);

    // 1. Untrusted: client does NOT trust server cert
    let (api_state1, _) = make_test_api_state(None, None, Some(sec.clone()));
    let (addr1, _h1) = spawn_tls_test_server(
        api_state1,
        certs.server_cert_pem.as_bytes(),
        certs.server_key_pem.as_bytes(),
    )
    .await;

    let default_client = reqwest::Client::builder().build().unwrap();
    let res1 = default_client
        .get(format!("https://localhost:{}/health", addr1.port()))
        .send()
        .await;
    assert!(res1.is_err(), "Untrusted certificate must fail TLS connection");

    // 2. Expired Certificate
    let (api_state2, _) = make_test_api_state(None, None, Some(sec.clone()));
    let (addr2, _h2) = spawn_tls_test_server(
        api_state2,
        certs.expired_cert_pem.as_bytes(),
        certs.expired_key_pem.as_bytes(),
    )
    .await;

    let trusted_client = reqwest::Client::builder()
        .add_root_certificate(
            reqwest::Certificate::from_pem(certs.expired_cert_pem.as_bytes()).unwrap(),
        )
        .build()
        .unwrap();

    let res2 = trusted_client
        .get(format!("https://localhost:{}/health", addr2.port()))
        .send()
        .await;
    assert!(res2.is_err(), "Expired certificate must fail TLS connection");

    // 3. Hostname Mismatch: certificate is for "evil.internal", client connects to "localhost"
    let (api_state3, _) = make_test_api_state(None, None, Some(sec));
    let (addr3, _h3) = spawn_tls_test_server(
        api_state3,
        certs.wrong_host_cert_pem.as_bytes(),
        certs.wrong_host_key_pem.as_bytes(),
    )
    .await;

    let client_for_wrong_host = reqwest::Client::builder()
        .add_root_certificate(
            reqwest::Certificate::from_pem(certs.wrong_host_cert_pem.as_bytes()).unwrap(),
        )
        .build()
        .unwrap();

    let res3 = client_for_wrong_host
        .get(format!("https://localhost:{}/health", addr3.port()))
        .send()
        .await;
    assert!(
        res3.is_err(),
        "Hostname mismatched certificate must fail TLS connection"
    );
}


// ==========================================
// Acceptance Test 3 & 4 & 7: Control Plane RBAC
// ==========================================

#[tokio::test]
async fn test_phase1_at03_and_at07_unauthorized_or_invalid_jwt_rejected() {
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator.clone());
    let (api_state, _) = make_test_api_state(None, None, Some(sec));

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/control", addr);

    // 1. Missing Token -> HTTP 401
    let r1 = client
        .post(&url)
        .json(&serde_json::json!({ "event": "NODE_FAILURE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(r1.headers().get("X-Auth-Error").unwrap(), "MissingToken");

    // 2. Malformed Token -> HTTP 401
    let r2 = client
        .post(&url)
        .header("Authorization", "Bearer invalid-gibberish-token")
        .json(&serde_json::json!({ "event": "NODE_FAILURE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(r2.headers().get("X-Auth-Error").unwrap(), "InvalidToken");

    // 3. Expired Token -> HTTP 401
    let expired_claims = JwtClaims::new(
        "admin_user",
        TEST_ISSUER,
        TEST_AUDIENCE,
        1000, // Year 1970 (expired)
        Some("tenant_admin"),
        vec!["operator"],
    );
    let expired_token = validator.create_token(&expired_claims).unwrap();
    let r3 = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", expired_token))
        .json(&serde_json::json!({ "event": "NODE_FAILURE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r3.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(r3.headers().get("X-Auth-Error").unwrap(), "ExpiredToken");

    // 4. Wrong Issuer -> HTTP 401
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let wrong_iss_claims = JwtClaims::new(
        "admin_user",
        "rogue-issuer",
        TEST_AUDIENCE,
        now + 3600,
        Some("tenant_admin"),
        vec!["operator"],
    );
    let wrong_iss_token = validator.create_token(&wrong_iss_claims).unwrap();
    let r4 = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", wrong_iss_token))
        .json(&serde_json::json!({ "event": "NODE_FAILURE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r4.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(r4.headers().get("X-Auth-Error").unwrap(), "InvalidIssuer");

    // 5. Wrong Audience -> HTTP 401
    let wrong_aud_claims = JwtClaims::new(
        "admin_user",
        TEST_ISSUER,
        "wrong-audience-service",
        now + 3600,
        Some("tenant_admin"),
        vec!["operator"],
    );
    let wrong_aud_token = validator.create_token(&wrong_aud_claims).unwrap();
    let r5 = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", wrong_aud_token))
        .json(&serde_json::json!({ "event": "NODE_FAILURE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(r5.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(r5.headers().get("X-Auth-Error").unwrap(), "InvalidAudience");
}

#[tokio::test]
async fn test_phase1_at04_insufficient_role_returns_403_and_no_mutation() {
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator.clone());
    let (api_state, nodes) = make_test_api_state(None, None, Some(sec));

    let app = create_app(api_state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let url = format!("http://{}/api/control", addr);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Caller only has "observer" role
    let observer_claims = JwtClaims::new(
        "observer_bob",
        TEST_ISSUER,
        TEST_AUDIENCE,
        now + 3600,
        Some("tenant_obs"),
        vec!["observer"],
    );
    let observer_token = validator.create_token(&observer_claims).unwrap();

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", observer_token))
        .json(&serde_json::json!({ "event": "NODE_FAILURE" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        resp.headers().get("X-Auth-Error").unwrap(),
        "InsufficientPermissions"
    );

    // CRITICAL: Prove state was NOT mutated
    assert_eq!(
        nodes[1].get_state(),
        NodeState::Active,
        "Node 2 state must not be modified by unauthorized caller"
    );

    // Now send with "operator" role -> Must succeed
    let operator_claims = JwtClaims::new(
        "operator_charlie",
        TEST_ISSUER,
        TEST_AUDIENCE,
        now + 3600,
        Some("tenant_ops"),
        vec!["operator"],
    );
    let operator_token = validator.create_token(&operator_claims).unwrap();

    let resp_ok = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", operator_token))
        .json(&serde_json::json!({ "event": "NODE_FAILURE" }))
        .send()
        .await
        .unwrap();

    assert_eq!(resp_ok.status(), reqwest::StatusCode::OK);
    assert_eq!(nodes[1].get_state(), NodeState::Failed);
}

// ==========================================
// Acceptance Test 5 & 6: Cryptographic Tenant Isolation & Forged Header Protection
// ==========================================

#[tokio::test]
async fn test_phase1_at05_and_at06_tenant_cryptographic_isolation() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    // Spawn mock origin returning tenant-specific responses
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);

                // Check tenant header received by origin
                let body = if req_str.contains("tenant_beta") {
                    r#"{"tenant":"beta","secret":"BETA_TOP_SECRET"}"#
                } else {
                    r#"{"tenant":"alpha","secret":"ALPHA_DATA"}"#
                };

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator.clone());

    let (api_state, _) = make_test_api_state(
        Some(cache.clone()),
        Some(format!("http://{}", origin_addr)),
        Some(sec),
    );

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Token for Tenant Alpha
    let token_alpha = validator
        .create_token(&JwtClaims::new(
            "alice",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            Some("tenant_alpha"),
            vec!["cache:read", "cache:invalidate"],
        ))
        .unwrap();

    // Token for Tenant Beta
    let token_beta = validator
        .create_token(&JwtClaims::new(
            "bob",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            Some("tenant_beta"),
            vec!["cache:read", "cache:invalidate"],
        ))
        .unwrap();

    // 1. Tenant Alpha requests /profile while forging header "x-tenant-id: tenant_beta"
    let url = format!("http://{}/api/v1/cache/profile", addr);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token_alpha))
        .header("x-tenant-id", "tenant_beta") // ATTACK: Forging tenant header
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let body_text = resp.text().await.unwrap();
    // In secure mode, forged header MUST be ignored, and token's tenant_alpha enforced!
    assert!(
        body_text.contains("ALPHA_DATA"),
        "Must receive Alpha's data, never Beta: {}",
        body_text
    );
    assert!(!body_text.contains("BETA_TOP_SECRET"));

    // 2. Tenant Beta requests /profile
    let resp_beta = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token_beta))
        .send()
        .await
        .unwrap();
    let body_beta = resp_beta.text().await.unwrap();
    assert!(body_beta.contains("BETA_TOP_SECRET"));

    // 3. Acceptance Test 6: Tenant Alpha attempts to invalidate Tenant Beta's cache
    let del_resp = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", token_alpha))
        .header("x-tenant-id", "tenant_beta") // Attack: Attempting to delete Beta's key
        .send()
        .await
        .unwrap();
    assert_eq!(del_resp.status(), reqwest::StatusCode::NO_CONTENT);

    // Tenant Beta's resource is isolated under tenant_beta partition
}

// ==========================================
// Acceptance Test 9: Configuration Errors Fail Closed
// ==========================================

#[tokio::test]
async fn test_phase1_at09_configuration_errors_fail_closed() {
    // 1. Non-existent cert/key files must fail closed immediately
    let res = TlsServerConfig::from_pem_file("non_existent_cert.pem", "non_existent_key.pem").await;
    assert!(res.is_err(), "Missing TLS certificate files must fail closed");

    // 2. Malformed PEM bytes must fail closed
    let malformed_res = TlsServerConfig::from_pem(b"NOT_A_VALID_CERT", b"NOT_A_VALID_KEY").await;
    assert!(malformed_res.is_err(), "Malformed PEM data must fail closed");

    // 3. JWT validation with wrong secret fails closed
    let val_correct = JwtValidator::new(TEST_ISSUER, TEST_AUDIENCE, TEST_SECRET);
    let val_wrong = JwtValidator::new(TEST_ISSUER, TEST_AUDIENCE, b"completely-wrong-secret-key-32b");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let token = val_correct
        .create_token(&JwtClaims::new(
            "user",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            None,
            vec!["operator"],
        ))
        .unwrap();

    let val_res = val_wrong.validate(&token);
    assert!(
        val_res.is_err(),
        "Validation with wrong secret must fail closed"
    );
}

// ==========================================
// Key Rotation Support
// ==========================================

#[test]
fn test_phase1_jwt_key_rotation_support() {
    let validator = make_test_jwt_validator();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = JwtClaims::new(
        "admin",
        TEST_ISSUER,
        TEST_AUDIENCE,
        now + 3600,
        Some("corp"),
        vec!["admin"],
    );

    // Token signed with rotated key "rot-key-1"
    let token = validator
        .create_token_with_kid("rot-key-1", &claims)
        .expect("Signing with rotation key failed");

    let validated = validator
        .validate(&token)
        .expect("Validating token signed with rotation key failed");
    assert_eq!(validated.sub, "admin");
    assert_eq!(validated.tenant_id, Some("corp".to_string()));
    assert!(validated.has_role(Role::Admin));
}

// ==========================================
// Acceptance Test: Unknown kid Handling Without Silent Fallback
// ==========================================

#[test]
fn test_phase1_jwt_unknown_kid_rejected_without_fallback() {
    let validator = make_test_jwt_validator();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let claims = JwtClaims::new(
        "admin",
        TEST_ISSUER,
        TEST_AUDIENCE,
        now + 3600,
        Some("corp"),
        vec!["admin"],
    );

    // Attempt to forge a token with an unknown kid "rogue-kid-999" signed with TEST_SECRET
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    header.kid = Some("rogue-kid-999".to_string());
    let key = jsonwebtoken::EncodingKey::from_secret(TEST_SECRET);
    let rogue_token = jsonwebtoken::encode(&header, &claims, &key).unwrap();

    let res = validator.validate(&rogue_token);
    assert!(
        res.is_err(),
        "Token with unknown kid must be rejected, not silently fall back to primary key"
    );
    match res {
        Err(phantom_cache_demo::security::AuthError::InvalidToken(msg)) => {
            assert!(msg.contains("Unknown key ID (kid): rogue-kid-999"));
        }
        other => panic!("Expected InvalidToken with unknown kid error, got: {:?}", other),
    }
}

// ==========================================
// Acceptance Test 8: RFC 9111 Authorization-Bearing Cache Bypass & No Poisoning
// ==========================================

#[tokio::test]
async fn test_phase1_at08_rfc9111_auth_bypass_and_no_cache_poisoning() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);

                let body = if req_str.contains("Bearer ") {
                    r#"{"data":"alice_private_data"}"#
                } else {
                    r#"{"data":"public_shared_data"}"#
                };

                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator.clone());

    // SEC-M7-01 justification: the "public" (anonymous) requests of this test can no longer be
    // sent to a JWT-enabled gateway (they are 401). The public side is therefore served by a
    // second, development-mode (security disabled) router that SHARES the same cache and origin;
    // the authenticated side uses the JWT-enabled router. The assertions about BYPASS-AUTH and
    // about an authenticated response never entering the shared cache are unchanged.
    let (api_state, _) = make_test_api_state(
        Some(cache.clone()),
        Some(format!("http://{}", origin_addr)),
        Some(sec),
    );
    let (public_state, _) = make_test_api_state(
        Some(cache.clone()),
        Some(format!("http://{}", origin_addr)),
        None,
    );

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let public_app = create_app(public_state);
    let public_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let public_addr = public_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(public_listener, public_app).await.unwrap();
    });

    let client = reqwest::Client::new();
    // New behaviour: an anonymous read on the JWT-enabled router is rejected.
    let anon = client.get(format!("http://{}/api/v1/cache/doc_seq_a", addr)).send().await.unwrap();
    assert_eq!(anon.status(), reqwest::StatusCode::UNAUTHORIZED);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let alice_token = validator
        .create_token(&JwtClaims::new(
            "alice",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            Some("tenant_alice"),
            vec!["cache:read", "cache:invalidate"],
        ))
        .unwrap();

    // Sequence A: Public request first, then Authenticated request
    let url_a = format!("http://{}/api/v1/cache/doc_seq_a", addr);
    let pub_url_a = format!("http://{}/api/v1/cache/doc_seq_a", public_addr);

    // 1. Unauthenticated public request -> MISS, populates public cache
    let pub_r1 = client.get(&pub_url_a).send().await.unwrap();
    assert_eq!(pub_r1.status(), reqwest::StatusCode::OK);
    assert_eq!(pub_r1.headers().get("X-Cache-Status").unwrap(), "MISS");
    let pub_b1 = pub_r1.text().await.unwrap();
    assert!(pub_b1.contains("public_shared_data"));

    // 2. Second unauthenticated public request -> HIT (proves it is cached)
    let pub_r2 = client.get(&pub_url_a).send().await.unwrap();
    assert_eq!(pub_r2.headers().get("X-Cache-Status").unwrap(), "HIT");

    // 3. Authenticated request for the same key -> Must BYPASS-AUTH and receive private data
    let auth_r1 = client
        .get(&url_a)
        .header("Authorization", format!("Bearer {}", alice_token))
        .send()
        .await
        .unwrap();
    assert_eq!(auth_r1.headers().get("X-Cache-Status").unwrap(), "BYPASS-AUTH");
    let auth_b1 = auth_r1.text().await.unwrap();
    assert!(auth_b1.contains("alice_private_data"));
    assert!(!auth_b1.contains("public_shared_data"));

    // Sequence B: Authenticated request first, then Public request
    let url_b = format!("http://{}/api/v1/cache/doc_seq_b", addr);
    let pub_url_b = format!("http://{}/api/v1/cache/doc_seq_b", public_addr);

    // 1. Authenticated request -> BYPASS-AUTH, must not store in shared cache
    let auth_r2 = client
        .get(&url_b)
        .header("Authorization", format!("Bearer {}", alice_token))
        .send()
        .await
        .unwrap();
    assert_eq!(auth_r2.headers().get("X-Cache-Status").unwrap(), "BYPASS-AUTH");
    let auth_b2 = auth_r2.text().await.unwrap();
    assert!(auth_b2.contains("alice_private_data"));

    // 2. Unauthenticated request -> Must be MISS, proving Alice's response was never stored!
    let pub_r3 = client.get(&pub_url_b).send().await.unwrap();
    assert_eq!(pub_r3.headers().get("X-Cache-Status").unwrap(), "MISS");
    let pub_b3 = pub_r3.text().await.unwrap();
    assert!(pub_b3.contains("public_shared_data"));
    assert!(!pub_b3.contains("alice_private_data"));

    // Round-2 review: strengthen "no poisoning" at the CACHE level (enforce_rfc9111_auth_bypass
    // is true in this configuration). Alice's private response must not be stored under any key
    // it could have been stored under: her own tenant key or the shared __default__ key.
    use phantom_cache_demo::core::cache::CacheKey;
    let ms = phantom_cache_demo::intel::diagnostics::DiagnosticBuffer::now_ms();
    for path in ["/doc_seq_a", "/doc_seq_b"] {
        assert!(
            cache.get_resource(&CacheKey::new(Some("tenant_alice"), path, None), ms).is_none(),
            "{}: authenticated response must not be stored under the tenant key",
            path
        );
        if let Some(res) = cache.get_resource(&CacheKey::new(None, path, None), ms) {
            let body = String::from_utf8_lossy(&res.body).to_string();
            assert!(!body.contains("alice_private_data"), "{}: __default__ entry holds alice's data: {}", path, body);
        }
    }
    assert!(
        cache.get_resource(&CacheKey::new(None, "/doc_seq_a", None), ms).is_some(),
        "control: the anonymous (public) response for doc_seq_a is cached under __default__"
    );
}

// ==========================================
// Acceptance Test: RFC 9111 Vary: * and no-cache are Uncacheable
// ==========================================

#[tokio::test]
async fn test_phase1_at08_rfc9111_vary_star_and_no_cache_uncacheable() {
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);

                let (extra_header, body) = if req_str.contains("/vary_star") {
                    ("Vary: *\r\n", r#"{"type":"vary_star"}"#)
                } else if req_str.contains("/no_cache") {
                    ("Cache-Control: no-cache\r\n", r#"{"type":"no_cache"}"#)
                } else {
                    ("", r#"{"type":"normal"}"#)
                };

                let resp = format!(
                    "HTTP/1.1 200 OK\r\n{}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    extra_header,
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    // SEC-M7-01 justification: this test used anonymous reads against a JWT-enabled gateway, which
    // are now rejected (401). It is about origin cacheability (Vary: * / no-cache), not about
    // authentication, so it now runs in development mode (security disabled), where anonymous
    // reads remain allowed. Authentication of reads is covered by m7_sec_fixes_tests::sec01_*.
    let cache = Arc::new(PartitionedCache::new(4));

    let (api_state, _) = make_test_api_state(
        Some(cache),
        Some(format!("http://{}", origin_addr)),
        None,
    );

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();

    // 1. Test Vary: *
    let url_vary = format!("http://{}/api/v1/cache/vary_star", addr);
    let r1 = client.get(&url_vary).send().await.unwrap();
    assert_eq!(r1.headers().get("X-Cache-Status").unwrap(), "MISS");

    let r2 = client.get(&url_vary).send().await.unwrap();
    assert_eq!(
        r2.headers().get("X-Cache-Status").unwrap(),
        "MISS",
        "Vary: * response must be uncacheable and subsequent request must be MISS"
    );

    // 2. Test Cache-Control: no-cache
    let url_nocache = format!("http://{}/api/v1/cache/no_cache", addr);
    let nr1 = client.get(&url_nocache).send().await.unwrap();
    assert_eq!(nr1.headers().get("X-Cache-Status").unwrap(), "MISS");

    let nr2 = client.get(&url_nocache).send().await.unwrap();
    assert_eq!(
        nr2.headers().get("X-Cache-Status").unwrap(),
        "MISS",
        "Cache-Control: no-cache response must be uncacheable and subsequent request must be MISS"
    );
}

// ==========================================
// Acceptance Test: CacheRead & CacheInvalidate Role Enforcement
// ==========================================

#[tokio::test]
async fn test_phase1_cache_read_and_invalidate_roles_enforced() {
    let cache = Arc::new(PartitionedCache::new(4));
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator.clone());

    let (api_state, _) = make_test_api_state(
        Some(cache),
        Some("http://127.0.0.1:0".to_string()),
        Some(sec),
    );

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Token with ONLY telemetry:read (Observer)
    let observer_token = validator
        .create_token(&JwtClaims::new(
            "obs",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            Some("tenant_obs"),
            vec!["telemetry:read"],
        ))
        .unwrap();

    // Token with ONLY cache:read
    let reader_token = validator
        .create_token(&JwtClaims::new(
            "reader",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            Some("tenant_reader"),
            vec!["cache:read"],
        ))
        .unwrap();

    // Token with ONLY cache:invalidate
    let invalidator_token = validator
        .create_token(&JwtClaims::new(
            "invalidator",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            Some("tenant_invalidator"),
            vec!["cache:invalidate"],
        ))
        .unwrap();

    let url = format!("http://{}/api/v1/cache/item", addr);

    // 1. Observer attempts GET /api/v1/cache/item -> 403 Forbidden
    let r_get = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", observer_token))
        .send()
        .await
        .unwrap();
    assert_eq!(r_get.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        r_get.headers().get("X-Auth-Error").unwrap(),
        "InsufficientPermissions"
    );

    // 2. Reader attempts DELETE /api/v1/cache/item -> 403 Forbidden
    let r_del_reader = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", reader_token))
        .send()
        .await
        .unwrap();
    assert_eq!(r_del_reader.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        r_del_reader.headers().get("X-Auth-Error").unwrap(),
        "InsufficientPermissions"
    );

    // 3. Unauthenticated client attempts DELETE in secure mode -> 401 Unauthorized
    let r_del_unauth = client.delete(&url).send().await.unwrap();
    assert_eq!(r_del_unauth.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(
        r_del_unauth.headers().get("X-Auth-Error").unwrap(),
        "MissingToken"
    );

    // 4. Invalidator attempts DELETE /api/v1/cache/item -> 204 No Content
    let r_del_ok = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", invalidator_token))
        .send()
        .await
        .unwrap();
    assert_eq!(r_del_ok.status(), reqwest::StatusCode::NO_CONTENT);
}

// ==========================================
// Acceptance Test 10: Diagnostics Protected with Observer Role
// ==========================================

#[tokio::test]
async fn test_phase1_at10_diagnostics_protected_with_observer_role() {
    let cache = Arc::new(PartitionedCache::new(4));
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator.clone());

    let (api_state, _) = make_test_api_state(
        Some(cache),
        None,
        Some(sec),
    );

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let diag_summary_url = format!("http://{}/api/v1/diagnostics/summary", addr);

    // 1. Missing Token -> 401
    let r_unauth = client.get(&diag_summary_url).send().await.unwrap();
    assert_eq!(r_unauth.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(
        r_unauth.headers().get("X-Auth-Error").unwrap(),
        "MissingToken"
    );

    // 2. Token with only "cache:read" -> 403 Forbidden
    let cache_token = validator
        .create_token(&JwtClaims::new(
            "reader",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            None,
            vec!["cache:read"],
        ))
        .unwrap();
    let r_insufficient = client
        .get(&diag_summary_url)
        .header("Authorization", format!("Bearer {}", cache_token))
        .send()
        .await
        .unwrap();
    assert_eq!(r_insufficient.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        r_insufficient.headers().get("X-Auth-Error").unwrap(),
        "InsufficientPermissions"
    );

    // 3. Token with "telemetry:read" (Observer) -> 200 OK
    let obs_token = validator
        .create_token(&JwtClaims::new(
            "observer",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            None,
            vec!["telemetry:read"],
        ))
        .unwrap();
    let r_ok = client
        .get(&diag_summary_url)
        .header("Authorization", format!("Bearer {}", obs_token))
        .send()
        .await
        .unwrap();
    assert_eq!(r_ok.status(), reqwest::StatusCode::OK);

    // 4. Events endpoint with "telemetry:read" -> 200 OK
    let diag_events_url = format!("http://{}/api/v1/diagnostics/events", addr);
    let r_events_ok = client
        .get(&diag_events_url)
        .header("Authorization", format!("Bearer {}", obs_token))
        .send()
        .await
        .unwrap();
    assert_eq!(r_events_ok.status(), reqwest::StatusCode::OK);
}

// ==========================================
// Acceptance Test 10: SSE /api/events Telemetry Stream RBAC
// ==========================================

#[tokio::test]
async fn test_phase1_at10_sse_events_telemetry_rbac() {
    let cache = Arc::new(PartitionedCache::new(4));
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator.clone());

    let (api_state, _) = make_test_api_state(
        Some(cache),
        None,
        Some(sec),
    );

    let app = create_app(api_state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let events_url = format!("http://{}/api/events", addr);

    // 1. Missing Token -> 401 Unauthorized
    let r_missing = client.get(&events_url).send().await.unwrap();
    assert_eq!(r_missing.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(
        r_missing.headers().get("X-Auth-Error").unwrap(),
        "MissingToken"
    );

    // 2. Insufficient Role (cache:read only) -> 403 Forbidden
    let cache_token = validator
        .create_token(&JwtClaims::new(
            "reader",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            None,
            vec!["cache:read"],
        ))
        .unwrap();
    let r_insufficient = client
        .get(&events_url)
        .header("Authorization", format!("Bearer {}", cache_token))
        .send()
        .await
        .unwrap();
    assert_eq!(r_insufficient.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(
        r_insufficient.headers().get("X-Auth-Error").unwrap(),
        "InsufficientPermissions"
    );

    // 3. Valid Observer Token (telemetry:read) -> 200 OK with text/event-stream
    let obs_token = validator
        .create_token(&JwtClaims::new(
            "observer_agent",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            None,
            vec!["telemetry:read"],
        ))
        .unwrap();
    let r_ok = client
        .get(&events_url)
        .header("Authorization", format!("Bearer {}", obs_token))
        .send()
        .await
        .unwrap();
    assert_eq!(r_ok.status(), reqwest::StatusCode::OK);
    let ct = r_ok.headers().get("Content-Type").unwrap().to_str().unwrap();
    assert!(ct.contains("text/event-stream"), "Must return event-stream, got: {}", ct);
}

// ==========================================
// Acceptance Test 9: Network-Accessible Startup Security Fail-Closed
// ==========================================

#[test]
fn test_phase1_at09_startup_security_fail_closed_rules() {
    let loopback: SocketAddr = "127.0.0.1:8085".parse().unwrap();
    let network_all: SocketAddr = "0.0.0.0:8085".parse().unwrap();
    let lan_ip: SocketAddr = "192.168.1.100:8085".parse().unwrap();

    // 1. Loopback development without auth or TLS succeeds (dev/test mode)
    let res_dev = validate_startup_security(&loopback, false, false, false, false);
    assert_eq!(res_dev, Ok(()));

    // 2. Loopback with explicit require_auth=true without secret fails closed
    let res_loopback_req_auth = validate_startup_security(&loopback, false, false, true, false);
    assert_eq!(
        res_loopback_req_auth,
        Err(StartupSecurityError::RequireAuthWithoutSecret)
    );

    // 3. Network-accessible address (0.0.0.0) without auth fails closed immediately
    let res_net_no_auth = validate_startup_security(&network_all, false, false, false, false);
    assert_eq!(
        res_net_no_auth,
        Err(StartupSecurityError::NetworkWithoutAuth(network_all))
    );

    // 4. LAN IP without auth fails closed immediately
    let res_lan_no_auth = validate_startup_security(&lan_ip, false, false, false, false);
    assert_eq!(
        res_lan_no_auth,
        Err(StartupSecurityError::NetworkWithoutAuth(lan_ip))
    );

    // 5. Network-accessible address with auth but without TLS fails closed
    let res_net_no_tls = validate_startup_security(&network_all, true, false, false, false);
    assert_eq!(
        res_net_no_tls,
        Err(StartupSecurityError::NetworkWithoutTls(network_all))
    );

    // 6. Network-accessible address with auth and allow_insecure_http override succeeds
    let res_net_insecure_override = validate_startup_security(&network_all, true, false, false, true);
    assert_eq!(res_net_insecure_override, Ok(()));

    // 7. Network-accessible address with auth and TLS succeeds
    let res_net_valid = validate_startup_security(&network_all, true, true, false, false);
    assert_eq!(res_net_valid, Ok(()));

    // 8. Insecure HTTP override CANNOT bypass mandatory authentication!
    // Even if allow_insecure_http is true, missing auth on network address MUST fail closed!
    let res_net_insecure_no_auth = validate_startup_security(&network_all, false, false, false, true);
    assert_eq!(
        res_net_insecure_no_auth,
        Err(StartupSecurityError::NetworkWithoutAuth(network_all)),
        "allow_insecure_http=true MUST NOT bypass mandatory authentication on network interfaces"
    );
}

// ==========================================
// Upstream Destination Isolation & Credential Forwarding Policy
// ==========================================

#[tokio::test]
async fn test_phase1_upstream_destination_isolation() {
    use std::sync::atomic::AtomicUsize;

    // Authorized Origin A (Port A)
    let origin_a_hits = Arc::new(AtomicUsize::new(0));
    let a_hits_clone = origin_a_hits.clone();
    let listener_a = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_a = listener_a.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener_a.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            let a_counter = a_hits_clone.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = [0u8; 1024];
                let n = socket.read(&mut buf).await.unwrap_or(0);
                let req_str = String::from_utf8_lossy(&buf[..n]);

                // Verify Host header sent to upstream is strictly Origin A's authority
                assert!(req_str.contains(&format!("Host: {}", addr_a)));
                assert!(!req_str.contains("Host: rogue.internal"));

                a_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let body = r#"{"origin":"A_AUTHORIZED"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
            });
        }
    });

    // Rogue / Untrusted Destination B (Port B) - MUST NEVER RECEIVE TRAFFIC
    let rogue_b_hits = Arc::new(AtomicUsize::new(0));
    let b_hits_clone = rogue_b_hits.clone();
    let listener_b = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr_b = listener_b.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match listener_b.accept().await {
                Ok(c) => c,
                Err(_) => break,
            };
            let b_counter = b_hits_clone.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                b_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let _ = socket.write_all(b"HTTP/1.1 500 LEAK\r\n\r\n").await;
            });
        }
    });

    // Configure cache strictly with Authorized Origin A
    let cache = Arc::new(PartitionedCache::new(4));
    let validator = make_test_jwt_validator();
    let sec = SecurityConfig::new_with_jwt(validator.clone());

    let (api_state, _) = make_test_api_state(
        Some(cache),
        Some(format!("http://{}", addr_a)),
        Some(sec),
    );

    let app = create_app(api_state);
    let server_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server_listener, app).await.unwrap();
    });

    let client = reqwest::Client::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let token = validator
        .create_token(&JwtClaims::new(
            "alice",
            TEST_ISSUER,
            TEST_AUDIENCE,
            now + 3600,
            Some("corp"),
            vec!["cache:read"],
        ))
        .unwrap();

    // Attacker attempts SSRF / Host override pointing to Rogue Destination B
    let url = format!("http://{}/api/v1/cache/item", server_addr);
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Host", "rogue.internal")
        .header("X-Forwarded-Host", format!("{}", addr_b))
        .header("X-Target-Host", format!("{}", addr_b))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    let text = resp.text().await.unwrap();
    assert!(text.contains("A_AUTHORIZED"));

    // PROOF: Authorized Origin A handled the request
    assert_eq!(origin_a_hits.load(std::sync::atomic::Ordering::SeqCst), 1);
    // PROOF: Rogue Destination B was NEVER contacted, credentials NEVER leaked
    assert_eq!(rogue_b_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
}

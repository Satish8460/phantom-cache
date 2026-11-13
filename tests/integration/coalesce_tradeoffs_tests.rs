use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn spawn_delay_origin(delay_ms: u64, status: u16, body_str: &'static str) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                if delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
                let status_line = match status {
                    200 => "HTTP/1.1 200 OK",
                    504 => "HTTP/1.1 504 Gateway Timeout",
                    _ => "HTTP/1.1 500 Internal Server Error",
                };
                let resp = format!(
                    "{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status_line,
                    body_str.len(),
                    body_str
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });
    addr
}

async fn setup_app_with_origin(origin_addr: std::net::SocketAddr) -> (std::net::SocketAddr, Arc<PartitionedCache>) {
    let cache = Arc::new(PartitionedCache::new(4));
    let nodes = vec![Arc::new(VirtualNode::new(1))];
    nodes[0].set_state(NodeState::Active);

    let snap = SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 0,
        throughput_req_sec: 0.0,
        p50_latency_ms: 0,
        p95_latency_ms: 0,
        p99_latency_ms: 0,
        db_load_ratio: 0.0,
        avoided_compute_dollars: 0.0,
        nodes: vec![NodeSnapshot {
            node_id: 1,
            state: "Active".to_string(),
            cpu_utilization: 10.0,
            memory_used_bytes: 1024,
        }],
    };

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes,
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache.clone()),
        origin_url: Some(format!("http://{}", origin_addr)),
        telemetry: None,
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    });

    let app = create_app(api_state);
    let server_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server_listener, app).await.unwrap();
    });

    (server_addr, cache)
}

#[tokio::test]
async fn test_negative_effect_head_of_line_blocking() {
    // Upstream has an intentional 150ms delay
    let origin_addr = spawn_delay_origin(150, 200, r#"{"delayed":"data"}"#).await;
    let (server_addr, cache) = setup_app_with_origin(origin_addr).await;

    // Stampede of 10 concurrent requests to the same cold key
    let key = "hol_blocked_key";
    let client_count = 10;
    let mut handles = Vec::new();

    let start = Instant::now();
    for _ in 0..client_count {
        let handle = tokio::spawn(async move {
            let req = format!(
                "GET /api/v1/cache/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                key
            );
            let mut stream = tokio::net::TcpStream::connect(server_addr).await.unwrap();
            stream.write_all(req.as_bytes()).await.unwrap();
            let mut resp = String::new();
            stream.read_to_string(&mut resp).await.unwrap();
            resp
        });
        handles.push(handle);
    }

    let mut coalesced_count = 0;
    let mut miss_count = 0;

    for h in handles {
        let resp = h.await.unwrap();
        assert!(resp.starts_with("HTTP/1.1 200 OK"));
        if resp.to_lowercase().contains("x-cache-status: coalesced") {
            coalesced_count += 1;
        } else if resp.to_lowercase().contains("x-cache-status: miss") {
            miss_count += 1;
        }
    }

    let total_elapsed = start.elapsed();

    // ANALYSIS OF HOL BLOCKING:
    // 1 initiator + 9 coalesced waiters
    assert_eq!(miss_count, 1, "Exactly one request initiates origin fetch");
    assert_eq!(coalesced_count, 9, "Remaining 9 requests coalesced as waiters");
    assert!(
        total_elapsed >= Duration::from_millis(140),
        "Head-of-line blocking: all coalesced waiters are held for the duration of slow origin: {:?}",
        total_elapsed
    );
    assert_eq!(cache.counters.coalesced_waiters.load(Ordering::Relaxed), 9);
}

#[tokio::test]
async fn test_negative_effect_error_fanout_blast_radius() {
    // Upstream fails with 504 Gateway Timeout
    let origin_addr = spawn_delay_origin(20, 504, r#"{"error":"upstream timeout"}"#).await;
    let (server_addr, cache) = setup_app_with_origin(origin_addr).await;

    let key = "error_fanout_key";
    let client_count = 8;
    let mut handles = Vec::new();

    for _ in 0..client_count {
        let handle = tokio::spawn(async move {
            let req = format!(
                "GET /api/v1/cache/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                key
            );
            let mut stream = tokio::net::TcpStream::connect(server_addr).await.unwrap();
            stream.write_all(req.as_bytes()).await.unwrap();
            let mut resp = String::new();
            stream.read_to_string(&mut resp).await.unwrap();
            resp
        });
        handles.push(handle);
    }

    let mut failed_responses = 0;
    for h in handles {
        let resp = h.await.unwrap();
        // ANALYSIS OF ERROR FAN-OUT:
        // When origin fails, every single coalesced waiter receives the upstream 504 error simultaneously!
        if resp.starts_with("HTTP/1.1 504") {
            failed_responses += 1;
        }
    }

    assert_eq!(
        failed_responses, client_count,
        "Error fan-out blast radius: single origin failure affected all {} concurrent clients",
        client_count
    );
    assert_eq!(cache.counters.coalesced_waiters.load(Ordering::Relaxed), (client_count - 1) as u64);
}

#[tokio::test]
async fn test_concurrency_waiter_fairness_and_starvation_freedom() {
    // Upstream has modest delay
    let origin_addr = spawn_delay_origin(50, 200, r#"{"fairness":"verified"}"#).await;
    let (server_addr, _) = setup_app_with_origin(origin_addr).await;

    let key = "fairness_key";
    let num_waves = 5;
    let clients_per_wave = 6;
    let mut all_handles = Vec::new();
    let delivered_count = Arc::new(AtomicUsize::new(0));

    for wave in 0..num_waves {
        for client_idx in 0..clients_per_wave {
            let delivered = delivered_count.clone();
            let handle = tokio::spawn(async move {
                let req = format!(
                    "GET /api/v1/cache/{} HTTP/1.1\r\nx-client-id: {}_{}\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    key, wave, client_idx
                );
                let mut stream = tokio::net::TcpStream::connect(server_addr).await.unwrap();
                stream.write_all(req.as_bytes()).await.unwrap();
                let mut resp = String::new();
                stream.read_to_string(&mut resp).await.unwrap();
                assert!(resp.starts_with("HTTP/1.1 200 OK"));
                delivered.fetch_add(1, Ordering::SeqCst);
            });
            all_handles.push(handle);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    for h in all_handles {
        h.await.unwrap();
    }

    assert_eq!(
        delivered_count.load(Ordering::SeqCst),
        num_waves * clients_per_wave,
        "Zero starvation: all {} clients across all waves were successfully delivered responses",
        num_waves * clients_per_wave
    );
}

#[tokio::test]
async fn test_concurrency_fairness_competing_keys_and_tenants() {
    let origin_addr = spawn_delay_origin(40, 200, r#"{"multi_tenant":"ok"}"#).await;
    let (server_addr, _) = setup_app_with_origin(origin_addr).await;

    let tenants = ["tenant_alpha", "tenant_beta", "tenant_gamma"];
    let num_keys = 10;
    let requests_per_key = 6;
    let total_requests = tenants.len() * num_keys * requests_per_key;

    let mut handles = Vec::new();
    let latencies = Arc::new(Mutex::new(Vec::with_capacity(total_requests)));

    let start_all = Instant::now();

    for tenant in tenants {
        for key_idx in 0..num_keys {
            for _ in 0..requests_per_key {
                let latencies_clone = latencies.clone();
                let tenant_str = tenant.to_string();
                let key_str = format!("competing_key_{}", key_idx);

                let handle = tokio::spawn(async move {
                    let req = format!(
                        "GET /api/v1/cache/{} HTTP/1.1\r\nx-tenant-id: {}\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                        key_str, tenant_str
                    );
                    let req_start = Instant::now();
                    let mut stream = tokio::net::TcpStream::connect(server_addr).await.unwrap();
                    stream.write_all(req.as_bytes()).await.unwrap();
                    let mut resp = String::new();
                    stream.read_to_string(&mut resp).await.unwrap();
                    let duration_ms = req_start.elapsed().as_millis() as u64;

                    assert!(
                        resp.starts_with("HTTP/1.1 200 OK"),
                        "Failed request for tenant {}: {}",
                        tenant_str,
                        resp
                    );
                    latencies_clone.lock().unwrap().push((tenant_str, key_str, duration_ms));
                });
                handles.push(handle);
            }
        }
    }

    for h in handles {
        h.await.unwrap();
    }

    let all_elapsed = start_all.elapsed();
    let recorded = latencies.lock().unwrap().clone();
    assert_eq!(recorded.len(), total_requests);

    let mut lat_values: Vec<u64> = recorded.iter().map(|(_, _, d)| *d).collect();
    lat_values.sort_unstable();

    let min_lat = lat_values[0];
    let max_lat = lat_values[lat_values.len() - 1];
    let mean_lat = lat_values.iter().sum::<u64>() as f64 / lat_values.len() as f64;
    let p50_lat = lat_values[(lat_values.len() as f64 * 0.50) as usize];
    let p99_lat = lat_values[((lat_values.len() as f64 * 0.99) as usize).min(lat_values.len() - 1)];

    println!(
        "[Fairness Verification] Total: {} reqs in {:.3}s | Latencies: Min={}ms, Mean={:.1}ms, P50={}ms, P99={}ms, Max={}ms",
        total_requests,
        all_elapsed.as_secs_f64(),
        min_lat,
        mean_lat,
        p50_lat,
        p99_lat,
        max_lat
    );

    // STARVATION BOUND:
    // With 40ms origin latency and 180 concurrent requests across 3 tenants and 10 keys:
    // If starvation occurred, starved requests would exhibit unbounded tail latency (> 3000ms).
    // Proving starvation freedom: all 180 requests complete within 1000ms.
    assert!(
        max_lat < 1000,
        "Starvation detected! Max request latency exceeded 1000ms: {}ms",
        max_lat
    );
}

#[tokio::test]
async fn test_mitigation_waiter_client_timeout_and_abort() {
    // Upstream has an intentional 250ms slow delay
    let origin_addr = spawn_delay_origin(250, 200, r#"{"slow":"data"}"#).await;
    let (server_addr, cache) = setup_app_with_origin(origin_addr).await;

    let key = "mitigated_timeout_key";

    // Client 1: Initiator (waits full duration, up to 1000ms)
    let c1_handle = tokio::spawn(async move {
        let req = format!(
            "GET /api/v1/cache/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            key
        );
        let mut stream = tokio::net::TcpStream::connect(server_addr).await.unwrap();
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        resp
    });

    // Brief delay to guarantee Client 1 initiated the in-flight fetch
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Client 2: Coalesced Waiter with aggressive 50ms client timeout
    // Demonstrates Mitigation: Client aborts wait early when origin is slow
    let c2_handle = tokio::spawn(async move {
        let req = format!(
            "GET /api/v1/cache/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
            key
        );
        let mut stream = tokio::net::TcpStream::connect(server_addr).await.unwrap();
        stream.write_all(req.as_bytes()).await.unwrap();

        let mut buf = [0u8; 1024];
        // Timeout after 50ms: drops stream, causing waiter channel receiver to be dropped!
        let read_res = tokio::time::timeout(Duration::from_millis(50), stream.read(&mut buf)).await;
        read_res.is_err() // True if timed out as expected
    });

    let c2_timed_out = c2_handle.await.unwrap();
    assert!(c2_timed_out, "Client 2 must abort wait early after 50ms");

    // Client 1 (Initiator) must still complete cleanly at ~250ms despite Client 2's aborted receiver!
    let c1_resp = c1_handle.await.unwrap();
    assert!(
        c1_resp.starts_with("HTTP/1.1 200 OK"),
        "Initiator must succeed even if coalesced waiter aborted: {}",
        c1_resp
    );

    // Client 3: Arrives after fetch completes -> must immediately HIT the populated cache!
    let req3 = format!(
        "GET /api/v1/cache/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        key
    );
    let mut s3 = tokio::net::TcpStream::connect(server_addr).await.unwrap();
    s3.write_all(req3.as_bytes()).await.unwrap();
    let mut resp3 = String::new();
    s3.read_to_string(&mut resp3).await.unwrap();
    assert!(resp3.to_lowercase().contains("x-cache-status: hit"));

    // Verify accounting: coalesced_waiters recorded the waiter registration
    let coalesced = cache.counters.coalesced_waiters.load(Ordering::Relaxed);
    assert!(
        coalesced >= 1,
        "Coalesced waiter must be tracked cleanly in diagnostics counters (got {})",
        coalesced
    );
}


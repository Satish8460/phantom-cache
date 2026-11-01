use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::intel::telemetry::TelemetryCollector;
use phantom_cache_demo::sim::rng::SeededRngSource;
use phantom_cache_demo::sim::traffic::{KeyDistribution, RequestType, TrafficGenerator, WorkloadWeights};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn test_p3_seeded_workload_determinism() {
    let rng1 = Arc::new(SeededRngSource::new(424242));
    let rng2 = Arc::new(SeededRngSource::new(424242));
    let rng3 = Arc::new(SeededRngSource::new(999999));

    let gen1 = TrafficGenerator::new(rng1);
    let gen2 = TrafficGenerator::new(rng2);
    let gen3 = TrafficGenerator::new(rng3);

    let dist = KeyDistribution::Zipfian {
        catalog_size: 1000,
        skew: 1.1,
    };
    let weights = WorkloadWeights::default();

    let batch1 = gen1.generate_batch(500, &dist, &weights);
    let batch2 = gen2.generate_batch(500, &dist, &weights);
    let batch3 = gen3.generate_batch(500, &dist, &weights);

    // Exact seeded equality
    assert_eq!(batch1.len(), 500);
    assert_eq!(batch1, batch2, "Identical seeds must produce identical request sequences");

    // Different seed must diverge
    assert_ne!(batch1, batch3, "Different seeds must produce divergent sequences");
}

#[test]
fn test_p3_request_type_distribution_weights() {
    let rng = Arc::new(SeededRngSource::new(1337));
    let gen = TrafficGenerator::new(rng);

    let weights = WorkloadWeights {
        cacheable_get: 0.60,
        uncacheable_get: 0.15,
        delete_invalidation: 0.10,
        bypass: 0.10,
        upstream_error: 0.05,
    };

    let dist = KeyDistribution::Uniform { catalog_size: 500 };
    let total = 20_000;
    let batch = gen.generate_batch(total, &dist, &weights);

    let mut cacheable_get_count = 0;
    let mut uncacheable_get_count = 0;
    let mut delete_count = 0;
    let mut bypass_count = 0;
    let mut error_count = 0;

    for req in batch {
        match req {
            RequestType::CacheableGet { .. } => cacheable_get_count += 1,
            RequestType::UncacheableGet { .. } => uncacheable_get_count += 1,
            RequestType::DeleteInvalidation { .. } => delete_count += 1,
            RequestType::Bypass { .. } => bypass_count += 1,
            RequestType::UpstreamError { .. } => error_count += 1,
        }
    }

    let p_get = cacheable_get_count as f64 / total as f64;
    let p_uncacheable = uncacheable_get_count as f64 / total as f64;
    let p_del = delete_count as f64 / total as f64;
    let p_bypass = bypass_count as f64 / total as f64;
    let p_err = error_count as f64 / total as f64;

    // Verify statistical convergence within +/- 1.5% tolerance
    assert!((p_get - 0.60).abs() < 0.015, "p_get: {}", p_get);
    assert!((p_uncacheable - 0.15).abs() < 0.015, "p_uncacheable: {}", p_uncacheable);
    assert!((p_del - 0.10).abs() < 0.015, "p_del: {}", p_del);
    assert!((p_bypass - 0.10).abs() < 0.015, "p_bypass: {}", p_bypass);
    assert!((p_err - 0.05).abs() < 0.015, "p_err: {}", p_err);
}

#[test]
fn test_p3_key_distributions() {
    let rng = Arc::new(SeededRngSource::new(777));
    let gen = TrafficGenerator::new(rng);

    // 1. Zipfian distribution skew: top 5% of catalog should receive > 40% of traffic
    let zipf_dist = KeyDistribution::Zipfian {
        catalog_size: 1000,
        skew: 1.2,
    };
    let weights = WorkloadWeights {
        cacheable_get: 1.0,
        uncacheable_get: 0.0,
        delete_invalidation: 0.0,
        bypass: 0.0,
        upstream_error: 0.0,
    };

    let total = 10_000;
    let batch = gen.generate_batch(total, &zipf_dist, &weights);
    let mut key_counts: HashMap<String, usize> = HashMap::new();
    for req in batch {
        if let RequestType::CacheableGet { key } = req {
            *key_counts.entry(key).or_insert(0) += 1;
        }
    }

    let mut counts: Vec<usize> = key_counts.values().cloned().collect();
    counts.sort_unstable_by(|a, b| b.cmp(a));
    let top_50_sum: usize = counts.iter().take(50).sum();
    let top_50_ratio = top_50_sum as f64 / total as f64;
    assert!(
        top_50_ratio > 0.40,
        "Zipfian top 5% must capture > 40% traffic (got {})",
        top_50_ratio
    );

    // 2. FlashCrowd hotspot distribution: hotspot keys should receive majority of traffic
    let hotspot = vec!["/resource/hot1".to_string(), "/resource/hot2".to_string()];
    let flash_dist = KeyDistribution::FlashCrowd {
        catalog_size: 1000,
        hotspot_keys: hotspot.clone(),
        hotspot_probability: 0.75,
    };

    let flash_batch = gen.generate_batch(total, &flash_dist, &weights);
    let mut hotspot_hits = 0;
    for req in flash_batch {
        if let RequestType::CacheableGet { key } = req {
            if hotspot.contains(&key) {
                hotspot_hits += 1;
            }
        }
    }
    let hotspot_ratio = hotspot_hits as f64 / total as f64;
    assert!(
        (hotspot_ratio - 0.75).abs() < 0.02,
        "Flash crowd hotspot ratio must be ~0.75 (got {})",
        hotspot_ratio
    );
}

#[tokio::test]
async fn test_p3_live_telemetry_stream_and_counters_bridge() {
    // Spin up mock origin
    let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_addr = origin_listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = match origin_listener.accept().await {
                Ok(conn) => conn,
                Err(_) => break,
            };
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let _ = socket.read(&mut buf).await;
                let body = r#"{"status":"ok","val":42}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    let cache = Arc::new(PartitionedCache::new(4));
    let telemetry = Arc::new(TelemetryCollector::new(5000));
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
            cpu_utilization: 15.0,
            memory_used_bytes: 1024,
        }],
    };

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes,
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache.clone()),
        origin_url: Some(format!("http://{}", origin_addr)),
        telemetry: Some(telemetry.clone()),
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    });

    let app = create_app(api_state.clone());
    let server_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server_listener, app).await.unwrap();
    });

    // Execute real requests against the server
    // Request 1: Cold miss for /item_a
    let req1 = "GET /api/v1/cache/item_a HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let mut s1 = tokio::net::TcpStream::connect(server_addr).await.unwrap();
    s1.write_all(req1.as_bytes()).await.unwrap();
    let mut resp1 = String::new();
    s1.read_to_string(&mut resp1).await.unwrap();
    assert!(resp1.to_lowercase().contains("x-cache-status: miss"));

    // Request 2: Repeat hit for /item_a
    let mut s2 = tokio::net::TcpStream::connect(server_addr).await.unwrap();
    s2.write_all(req1.as_bytes()).await.unwrap();
    let mut resp2 = String::new();
    s2.read_to_string(&mut resp2).await.unwrap();
    assert!(resp2.to_lowercase().contains("x-cache-status: hit"));

    // Request 3: Repeat hit for /item_a
    let mut s3 = tokio::net::TcpStream::connect(server_addr).await.unwrap();
    s3.write_all(req1.as_bytes()).await.unwrap();
    let mut resp3 = String::new();
    s3.read_to_string(&mut resp3).await.unwrap();
    assert!(resp3.to_lowercase().contains("x-cache-status: hit"));

    // Verify TelemetryCollector and CacheCounters
    assert_eq!(cache.counters.cache_hits.load(std::sync::atomic::Ordering::Relaxed), 2);
    assert_eq!(cache.counters.cache_misses.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(telemetry.len(), 3);

    // Verify live SSE stream connects to live counters
    let sse_req = "GET /api/events HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    let mut s_sse = tokio::net::TcpStream::connect(server_addr).await.unwrap();
    s_sse.write_all(sse_req.as_bytes()).await.unwrap();

    let mut sse_str = String::new();
    let start_read = std::time::Instant::now();
    while !sse_str.contains("data:") && start_read.elapsed() < Duration::from_secs(3) {
        let mut chunk = [0u8; 1024];
        if let Ok(Ok(n)) = tokio::time::timeout(Duration::from_millis(300), s_sse.read(&mut chunk)).await {
            if n > 0 {
                sse_str.push_str(&String::from_utf8_lossy(&chunk[..n]));
            }
        }
    }

    assert!(sse_str.contains("data:"), "SSE output: {}", sse_str);
    assert!(sse_str.contains(r#""cache_hits":2"#), "SSE missing live cache_hits:2: {}", sse_str);
    assert!(sse_str.contains(r#""cache_misses":1"#), "SSE missing live cache_misses:1: {}", sse_str);
    assert!(sse_str.contains(r#""hit_ratio":0.666"#), "SSE hit_ratio missing: {}", sse_str);
}

#[tokio::test]
async fn test_p3_workload_generator_executes_against_live_server() {
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
                let body = r#"{"generated":"content","status":200}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                let _ = socket.shutdown().await;
            });
        }
    });

    let cache = Arc::new(PartitionedCache::new(4));
    cache.set_capacity_limits(30, 100);
    let telemetry = Arc::new(TelemetryCollector::new(1000));
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
            cpu_utilization: 20.0,
            memory_used_bytes: 2048,
        }],
    };

    let api_state = Arc::new(ApiState {
        snapshot: Mutex::new(snap),
        nodes,
        workload_type: Mutex::new("Normal".to_string()),
        cache: Some(cache.clone()),
        origin_url: Some(format!("http://{}", origin_addr)),
        telemetry: Some(telemetry.clone()),
        pie: None,
        security: None,
        upstream_policy: None,
        router: None,
    });

    let app = create_app(api_state.clone());
    let server_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server_listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(server_listener, app).await.unwrap();
    });

    // Generate real mixed traffic using SeededRngSource
    let rng = Arc::new(SeededRngSource::new(777123));
    let gen = TrafficGenerator::new(rng);
    let dist = KeyDistribution::Zipfian {
        catalog_size: 100,
        skew: 1.1,
    };
    let weights = WorkloadWeights {
        cacheable_get: 0.60,
        uncacheable_get: 0.15,
        delete_invalidation: 0.10,
        bypass: 0.10,
        upstream_error: 0.05,
    };

    let batch = gen.generate_batch(150, &dist, &weights);
    let start_time = std::time::Instant::now();
    let mut completed_requests = 0;

    for req in batch {
        let (method, path, headers_extra) = match req {
            RequestType::CacheableGet { key } => ("GET", format!("/api/v1/cache{}", key), ""),
            RequestType::UncacheableGet { key, .. } => (
                "GET",
                format!("/api/v1/cache{}", key),
                "Cache-Control: no-cache\r\n",
            ),
            RequestType::DeleteInvalidation { key } => ("DELETE", format!("/api/v1/cache{}", key), ""),
            RequestType::Bypass { key } => (
                "GET",
                format!("/api/v1/cache{}", key),
                "X-Cache-Bypass: true\r\n",
            ),
            RequestType::UpstreamError { key, .. } => ("GET", format!("/api/v1/cache{}", key), ""),
        };

        let raw_req = format!(
            "{} {} HTTP/1.1\r\nHost: localhost\r\n{}Connection: close\r\n\r\n",
            method, path, headers_extra
        );

        if let Ok(mut stream) = tokio::net::TcpStream::connect(server_addr).await {
            if stream.write_all(raw_req.as_bytes()).await.is_ok() {
                let mut resp_str = String::new();
                let _ = stream.read_to_string(&mut resp_str).await;
                completed_requests += 1;
            }
        }
    }

    let elapsed = start_time.elapsed();
    let throughput = completed_requests as f64 / elapsed.as_secs_f64().max(0.001);

    let hits = cache.counters.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
    let misses = cache.counters.cache_misses.load(std::sync::atomic::Ordering::Relaxed);
    let evictions = cache.counters.evictions_total.load(std::sync::atomic::Ordering::Relaxed);
    let coalesced = cache.counters.coalesced_waiters.load(std::sync::atomic::Ordering::Relaxed);
    let total_cache_lookups = hits + misses;
    let hit_ratio = if total_cache_lookups > 0 {
        hits as f64 / total_cache_lookups as f64
    } else {
        0.0
    };
    let (p50, p90, p99) = telemetry.calculate_percentiles();
    let errors = telemetry.error_count();

    println!(
        "[Live Workload Telemetry] Completed: {} reqs in {:.3}s ({:.1} req/s) | Hit Ratio: {:.1}% (Hits: {}, Misses: {}) | Coalesced: {} | Evictions: {} | Errors: {} | Latency: P50={}ms, P90={}ms, P99={}ms",
        completed_requests,
        elapsed.as_secs_f64(),
        throughput,
        hit_ratio * 100.0,
        hits,
        misses,
        coalesced,
        evictions,
        errors,
        p50,
        p90,
        p99
    );

    assert_eq!(completed_requests, 150);
    assert!(total_cache_lookups > 50, "Should have executed cache lookups");
    assert!(hit_ratio > 0.10, "Zipfian workload should achieve repeat hits");
}

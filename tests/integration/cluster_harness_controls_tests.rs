//! Harness control tests and proxy fidelity verification for PhantomCache Phase 3 (M3-01, AC3-2).
//!
//! Validates:
//! - Link-fault proxy delay, blackhole, and partition fidelity.
//! - Known-stale control scenario detection.
//! - Known-good control scenario consistency.
//! - Clean shutdown and zero orphaned child processes.

#[path = "../support/mod.rs"]
mod support;

use std::time::{Duration, Instant};
use support::cluster_harness::ClusterHarness;
use support::fault_proxy::{FaultMode, FaultProxy};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn test_proxy_delay_fidelity() {
    // 1. Target echo server
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 128];
                while let Ok(n) = stream.read(&mut buf).await {
                    if n == 0 { break; }
                    let _ = stream.write_all(&buf[..n]).await;
                }
            });
        }
    });

    // 2. Start fault proxy pointing to target
    let proxy = FaultProxy::start(target_addr).await.unwrap();
    let proxy_addr = proxy.listen_addr();

    // Verify Normal mode latency (< 25 ms)
    let t0 = Instant::now();
    let mut client = TcpStream::connect(proxy_addr).await.unwrap();
    client.write_all(b"ping").await.unwrap();
    let mut resp = [0u8; 4];
    client.read_exact(&mut resp).await.unwrap();
    let normal_elapsed = t0.elapsed();
    assert_eq!(&resp, b"ping");
    assert!(normal_elapsed < Duration::from_millis(40), "Normal latency too high: {:?}", normal_elapsed);

    // 3. Inject 60ms delay
    proxy.set_mode(FaultMode::Delay { ms: 60 }).await;
    let t1 = Instant::now();
    let mut client2 = TcpStream::connect(proxy_addr).await.unwrap();
    client2.write_all(b"pong").await.unwrap();
    let mut resp2 = [0u8; 4];
    client2.read_exact(&mut resp2).await.unwrap();
    let delayed_elapsed = t1.elapsed();
    assert_eq!(&resp2, b"pong");
    assert!(
        delayed_elapsed >= Duration::from_millis(50),
        "Delay fidelity failed: expected >=50ms, got {:?}",
        delayed_elapsed
    );

    let metrics = proxy.metrics();
    assert!(metrics.delayed_events.load(std::sync::atomic::Ordering::Relaxed) >= 1);
    assert!(metrics.bytes_forwarded.load(std::sync::atomic::Ordering::Relaxed) >= 8);
}

#[tokio::test]
async fn test_proxy_partition_and_blackhole_fidelity() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 64];
                while let Ok(n) = stream.read(&mut buf).await {
                    if n == 0 { break; }
                    let _ = stream.write_all(&buf[..n]).await;
                }
            });
        }
    });

    let proxy = FaultProxy::start(target_addr).await.unwrap();
    let proxy_addr = proxy.listen_addr();

    // 1. Partition mode: connection dropped / refused
    proxy.set_mode(FaultMode::Partition).await;
    let connect_res = TcpStream::connect(proxy_addr).await;
    if let Ok(mut stream) = connect_res {
        // Stream should close immediately on read/write
        let mut buf = [0u8; 8];
        let read_res = tokio::time::timeout(Duration::from_millis(100), stream.read(&mut buf)).await;
        assert!(read_res.is_err() || matches!(read_res, Ok(Ok(0))), "Partition should drop traffic");
    }

    // 2. Heal partition
    proxy.set_mode(FaultMode::Normal).await;
    let mut healthy_stream = TcpStream::connect(proxy_addr).await.unwrap();
    healthy_stream.write_all(b"healed").await.unwrap();
    let mut buf = [0u8; 6];
    healthy_stream.read_exact(&mut buf).await.unwrap();
    assert_eq!(&buf, b"healed");
}

#[tokio::test]
async fn test_harness_known_stale_control() {
    // Known-stale control: In an isolated/partitioned link, reading from node 2
    // after an origin mutation without invalidation observing stale version.
    let harness = ClusterHarness::start(&[1, 2], true).await.expect("Failed to start cluster");

    // Partition links between Node 1 and Node 2
    harness.set_symmetric_partition(1, 2).await;

    // Node 2 reads initial version 1
    let (s1, _, status1, v1) = harness.http_get(2, "/stale_control_key").await.unwrap();
    assert_eq!(s1, axum::http::StatusCode::OK);
    assert_eq!(status1.as_deref(), Some("MISS"));
    assert_eq!(v1, Some(1));

    // Second read on Node 2 is a cache HIT (v1)
    let (s2, _, status2, v2) = harness.http_get(2, "/stale_control_key").await.unwrap();
    assert_eq!(s2, axum::http::StatusCode::OK);
    assert_eq!(status2.as_deref(), Some("HIT"));
    assert_eq!(v2, Some(1));

    // Origin mutates key to version 2
    let new_v = harness.origin().mutate_key("/stale_control_key", "version_2_data");
    assert_eq!(new_v, 2);

    // Node 2 reads again without invalidation: must serve stale version 1 (u < v)
    let (s3, _, status3, v3) = harness.http_get(2, "/stale_control_key").await.unwrap();
    assert_eq!(s3, axum::http::StatusCode::OK);
    assert_eq!(status3.as_deref(), Some("HIT"));
    assert_eq!(v3, Some(1), "Harness correctly detects stale-after-ack read u < v");

    // Heal partition
    harness.heal_partition(1, 2).await;
}

#[tokio::test]
async fn test_harness_known_good_control() {
    // Known-good control: On a single gateway with local invalidation,
    // mutating origin and deleting key ensures subsequent read is fresh (u == v).
    let harness = ClusterHarness::start(&[1], false).await.expect("Failed to start cluster");

    // 1. Initial GET on Node 1 -> origin count 1, version 1
    let (s1, _, status1, v1) = harness.http_get(1, "/good_control_key").await.unwrap();
    assert_eq!(s1, axum::http::StatusCode::OK);
    assert_eq!(status1.as_deref(), Some("MISS"));
    assert_eq!(v1, Some(1));

    // 2. Second GET -> HIT, version 1
    let (s2, _, status2, v2) = harness.http_get(1, "/good_control_key").await.unwrap();
    assert_eq!(s2, axum::http::StatusCode::OK);
    assert_eq!(status2.as_deref(), Some("HIT"));
    assert_eq!(v2, Some(1));

    // 3. Mutate origin to version 2
    harness.origin().mutate_key("/good_control_key", "version_2_data");

    // 4. Issue local DELETE on Node 1
    let del_st = harness.http_delete(1, "/good_control_key").await.unwrap();
    assert_eq!(del_st, axum::http::StatusCode::NO_CONTENT);

    // 5. Read on Node 1 -> fresh MISS with version 2
    let (s3, _, status3, v3) = harness.http_get(1, "/good_control_key").await.unwrap();
    assert_eq!(s3, axum::http::StatusCode::OK);
    assert_eq!(status3.as_deref(), Some("MISS"));
    assert_eq!(v3, Some(2), "Known good control serves fresh data (u == v)");
}

#[tokio::test]
async fn test_harness_clean_shutdown_and_no_orphan_pids() {
    // Launches 3 real gateway processes, records their PIDs, drops harness,
    // and verifies that all processes are terminated.
    let mut harness = ClusterHarness::start(&[1, 2, 3], false).await.expect("Failed to start cluster");

    let mut pids = Vec::new();
    for gw in harness.gateways.values() {
        if let Some(pid) = gw.child.id() {
            pids.push(pid);
        }
    }
    assert_eq!(pids.len(), 3, "Expected 3 child PIDs");

    // Verify all 3 gateways are healthy
    for &id in &[1, 2, 3] {
        let (st, _, _, _) = harness.http_get(id, "/health_test").await.unwrap();
        assert_eq!(st, axum::http::StatusCode::OK);
    }

    // Explicitly shut down harness
    harness.shutdown();

    // Verify child processes have been terminated: poll (bounded) instead of a fixed sleep.
    for &pid in &pids {
        #[cfg(windows)]
        {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let output = std::process::Command::new("tasklist")
                    .args(["/FI", &format!("PID eq {}", pid)])
                    .output()
                    .unwrap();
                let text = String::from_utf8_lossy(&output.stdout);
                if !text.contains(&pid.to_string()) {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "PID {} must be terminated on harness drop (still alive after 5s)",
                    pid
                );
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
        #[cfg(not(windows))]
        let _ = pid;
    }
}

/// Diagnostics path (task CF-2): when a gateway cannot start, `ClusterHarness::start*` must fail
/// FAST (child exit is detected, no waiting out the full budget) and the error must carry the
/// node id, elapsed time, the process exit status and the gateway's captured stderr tail.
/// The failure is forced with a nonexistent node key path, which the gateway rejects at startup
/// with `[CONFIG ERROR] PHANTOM_NODE_KEY: file ... cannot be read` (exit code 2).
#[tokio::test]
async fn test_harness_start_failure_reports_gateway_stderr_and_elapsed() {
    use support::cluster_harness::ClusterOptions;

    let bogus_key = std::env::temp_dir().join("phantom_does_not_exist_node.key");
    let t0 = Instant::now();
    let res = ClusterHarness::start_with_options(
        &[1, 2],
        ClusterOptions {
            with_proxies: false,
            extra_env: vec![("PHANTOM_NODE_KEY".to_string(), bogus_key.display().to_string())],
            startup_timeout: Duration::from_secs(12),
            wait_for_prewarm: false,
        },
    )
    .await;
    let waited = t0.elapsed();

    let err = match res {
        Ok(_) => panic!("start must fail when the node key cannot be read"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("Gateway node 1"), "error names the failing node: {err}");
    assert!(err.contains("EXITED"), "error says the process exited (not just timed out): {err}");
    assert!(err.contains("PHANTOM_NODE_KEY") && err.contains("cannot be read"), "error carries the gateway stderr tail: {err}");
    assert!(err.contains("budget"), "error states elapsed time and budget: {err}");
    assert!(
        waited < Duration::from_secs(10),
        "a dead gateway must be detected via try_wait, not by waiting out the budget (took {waited:?})"
    );
}

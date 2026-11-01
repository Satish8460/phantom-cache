//! Cluster-aware lab scenario validation for Phantom Lab (M4-13).
//!
//! Validates:
//! - Multi-gateway cluster deployment under ADR-003 Option A1 contract.
//! - Per-node linearizability (Guarantee G1): immediate miss on deleting node.
//! - Cluster-wide invalidation broadcast to peer nodes (Guarantee G2).
//! - Clean shutdown and zero orphan processes.

#[path = "../support/fault_proxy.rs"]
mod fault_proxy;

#[path = "../support/cluster_harness.rs"]
mod cluster_harness;

use cluster_harness::ClusterHarness;
use reqwest::StatusCode;

#[tokio::test]
async fn test_lab_cluster_scenario_invalidation_broadcast() {
    let mut harness = ClusterHarness::start(&[1, 2, 3], true)
        .await
        .expect("Failed to start cluster harness");
    harness.warm_invalidation_paths().await.expect("invalidation-path warm-up failed");

    let key = "/lab_cluster_key";

    // 1. Initial GET on Node 2 and Node 3 caches the item
    let (s2, _, c2, v2) = harness.http_get(2, key).await.unwrap();
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(c2.as_deref(), Some("MISS"));
    assert_eq!(v2, Some(1));

    let (s3, _, c3, v3) = harness.http_get(3, key).await.unwrap();
    assert_eq!(s3, StatusCode::OK);
    assert_eq!(c3.as_deref(), Some("MISS"));
    assert_eq!(v3, Some(1));

    // 2. Warm HIT verification
    let (_, _, c2_hit, _) = harness.http_get(2, key).await.unwrap();
    assert_eq!(c2_hit.as_deref(), Some("HIT"));

    // 3. Mutate key at origin to v2, then DELETE via Node 1
    harness.origin().mutate_key(key, "v2");
    let s_del = harness.http_delete(1, key).await.unwrap();
    assert_eq!(s_del, StatusCode::NO_CONTENT);

    // 4. Guarantee G1: Node 1 immediately misses on next GET
    let (s1, _, c1, v1) = harness.http_get(1, key).await.unwrap();
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(c1.as_deref(), Some("MISS"));
    assert_eq!(v1, Some(2)); // New version served

    // 5. Guarantee G2: Node 2 and Node 3 observe invalidation and serve fresh v=2. Poll until the
    //    peer stops answering HIT (hard cap 2 s: 4 broadcaster attempts x 300 ms + 175 ms backoff =
    //    1375 ms worst case) instead of sleeping a fixed 50 ms.
    for peer in [2u8, 3u8] {
        let obs = harness
            .wait_until_invalidated(peer, key, std::time::Duration::from_millis(2000))
            .await
            .unwrap_or_else(|e| panic!("cluster-wide invalidation not observed: {e}"));
        assert_eq!(obs.version_after, Some(2), "Node {peer} must serve fresh v=2");
    }

    harness.shutdown();
}

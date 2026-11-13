#[cfg(test)]
mod tests {
    use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
    use phantom_cache_demo::dist::router::RendezvousRouter;
    use std::sync::Arc;

    #[test]
    fn test_normal_routing() {
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Active);

        let router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);
        let primary = router.route_key("hotel_123");
        assert!(primary.is_some());
    }

    #[test]
    fn test_node_failure_exclusion() {
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Failed); // Node 2 is marked offline

        let router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);

        // Ensure keys always map to active Node 1
        for i in 0..10 {
            let key = format!("hotel_{}", i);
            let primary = router.route_key(&key).unwrap();
            assert_eq!(primary.node_id, 1);
        }
    }

    #[test]
    fn test_replica_selection() {
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        let n3 = Arc::new(VirtualNode::new(3));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Active);
        n3.set_state(NodeState::Active);

        let router = RendezvousRouter::new(vec![n1, n2, n3]);
        let replicas = router.get_replicas("hotel_123", 1);
        assert_eq!(replicas.len(), 1);
    }

    #[test]
    fn test_fnv1a_deterministic_golden_vectors() {
        // Documented 64-bit FNV-1a golden test vectors
        // (offset_basis = 0xcbf29ce484222325, prime = 0x100000001b3)
        // Encoding: key.as_bytes() followed by node_id as single u8 byte.
        assert_eq!(
            RendezvousRouter::calculate_hash("hotel_123", 1),
            0x395fe85ed8f6c3d9,
            "Golden vector 1 mismatch for key='hotel_123', node_id=1"
        );
        assert_eq!(
            RendezvousRouter::calculate_hash("hotel_123", 2),
            0x395fe55ed8f6bec0,
            "Golden vector 2 mismatch for key='hotel_123', node_id=2"
        );
        assert_eq!(
            RendezvousRouter::calculate_hash("user:1001", 1),
            0xc08552b57b6f2d55,
            "Golden vector 3 mismatch for key='user:1001', node_id=1"
        );
        assert_eq!(
            RendezvousRouter::calculate_hash("", 0),
            0xaf63bd4c8601b7df,
            "Golden vector 4 mismatch for empty key='', node_id=0"
        );

        // Verification of byte-level encoding definition:
        // Hash calculated over a concatenated byte buffer [key_bytes, node_id_byte]
        let manual_hash = |key: &str, node_id: u8| -> u64 {
            let mut h = 0xcbf29ce484222325u64;
            let mut bytes = key.as_bytes().to_vec();
            bytes.push(node_id);
            for b in bytes {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        };
        assert_eq!(
            manual_hash("hotel_123", 1),
            RendezvousRouter::calculate_hash("hotel_123", 1)
        );
        assert_eq!(
            manual_hash("user:1001", 1),
            RendezvousRouter::calculate_hash("user:1001", 1)
        );
    }

    #[test]
    fn test_hrw_shared_golden_vectors_json() {
        let path = std::path::Path::new("tests/vectors/hrw_vectors.json");
        assert!(path.exists(), "tests/vectors/hrw_vectors.json missing");
        let content = std::fs::read_to_string(path).expect("read hrw_vectors.json");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("parse json");
        let vectors = parsed["vectors"].as_array().expect("vectors array");

        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        let n3 = Arc::new(VirtualNode::new(3));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Active);
        n3.set_state(NodeState::Active);
        let router = RendezvousRouter::new(vec![n1, n2, n3]);

        for vec in vectors {
            let key = vec["key"].as_str().unwrap();
            let expected_primary = vec["primary_node_id"].as_u64().unwrap() as u8;
            let node_hashes = vec["node_hashes"].as_object().unwrap();

            for (node_str, hash_obj) in node_hashes {
                let node_id: u8 = node_str.parse().unwrap();
                let expected_hash: u64 = hash_obj["dec"].as_str().unwrap().parse().unwrap();
                let computed = RendezvousRouter::calculate_hash(key, node_id);
                assert_eq!(
                    computed, expected_hash,
                    "Hash mismatch for key='{}', node_id={}",
                    key, node_id
                );
            }

            let route_res = router.route_key(key).expect("routed node");
            assert_eq!(
                route_res.node_id, expected_primary,
                "Primary mismatch for key='{}'",
                key
            );

            let replicas = router.get_replicas(key, 2);
            let expected_replicas = vec["replicas"].as_array().unwrap();
            assert_eq!(replicas.len(), expected_replicas.len());
            for (idx, r) in replicas.iter().enumerate() {
                assert_eq!(
                    r.node_id, expected_replicas[idx].as_u64().unwrap() as u8,
                    "Replica mismatch at index {} for key='{}'",
                    idx, key
                );
            }
        }
    }

    #[test]
    fn test_deterministic_routing_across_independent_instances() {
        // Verify cross-process consensus: two distinct router instances
        // must compute byte-for-byte identical primary ownership assignments.
        let n1_a = Arc::new(VirtualNode::new(1));
        let n2_a = Arc::new(VirtualNode::new(2));
        let n3_a = Arc::new(VirtualNode::new(3));
        n1_a.set_state(NodeState::Active);
        n2_a.set_state(NodeState::Active);
        n3_a.set_state(NodeState::Active);
        let router_a = RendezvousRouter::new(vec![n1_a, n2_a, n3_a]);

        let n1_b = Arc::new(VirtualNode::new(1));
        let n2_b = Arc::new(VirtualNode::new(2));
        let n3_b = Arc::new(VirtualNode::new(3));
        n1_b.set_state(NodeState::Active);
        n2_b.set_state(NodeState::Active);
        n3_b.set_state(NodeState::Active);
        let router_b = RendezvousRouter::new(vec![n1_b, n2_b, n3_b]);

        for i in 0..100 {
            let key = format!("tenantA:resource:{}", i);
            let owner_a = router_a.route_key(&key).unwrap().node_id;
            let owner_b = router_b.route_key(&key).unwrap().node_id;
            assert_eq!(
                owner_a, owner_b,
                "Routing discrepancy across instances for key {}",
                key
            );
        }
    }

    #[test]
    fn test_rendezvous_hrw_invariant_on_node_addition() {
        // Core HRW Invariant on Node Addition:
        // When a new node is added, an existing key must EITHER:
        // 1. Retain its original owner (new_owner == old_owner), OR
        // 2. Migrate to the newly added node (new_owner == added_node_id).
        // Under no circumstances may a key migrate between existing nodes.
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        let n3 = Arc::new(VirtualNode::new(3));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Active);
        n3.set_state(NodeState::Active);
        let router_3 = RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]);

        let total_keys = 500;
        let mut initial_placements = std::collections::HashMap::with_capacity(total_keys);
        for i in 0..total_keys {
            let key = format!("cache:key:{}", i);
            let owner = router_3.route_key(&key).unwrap().node_id;
            initial_placements.insert(key, owner);
        }

        // Add 4th node (node_id = 4)
        let n4 = Arc::new(VirtualNode::new(4));
        n4.set_state(NodeState::Active);
        let router_4 = RendezvousRouter::new(vec![n1, n2, n3, n4]);

        let mut migrated_to_new_node = 0;
        let mut retained_previous_owner = 0;

        for (key, old_owner) in &initial_placements {
            let new_owner = router_4.route_key(key).unwrap().node_id;
            if new_owner == 4 {
                migrated_to_new_node += 1;
            } else {
                // Strong HRW invariant: must not migrate between existing nodes
                assert_eq!(
                    new_owner, *old_owner,
                    "HRW violation: key {} migrated from node {} to node {} instead of staying or moving to 4",
                    key, old_owner, new_owner
                );
                retained_previous_owner += 1;
            }
        }

        assert_eq!(
            migrated_to_new_node + retained_previous_owner,
            total_keys,
            "All keys must be accounted for"
        );
        assert!(
            migrated_to_new_node > 0,
            "Added node must capture a non-zero portion of keys"
        );
    }

    #[test]
    fn test_rendezvous_hrw_invariant_on_node_removal() {
        // Core HRW Invariant on Node Failure/Removal:
        // When a node fails or is removed, keys that were mapped to surviving nodes
        // must remain strictly on their original nodes. Only keys mapped to the failed
        // node are reassigned among surviving nodes.
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        let n3 = Arc::new(VirtualNode::new(3));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Active);
        n3.set_state(NodeState::Active);
        let router_before = RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]);

        let total_keys = 500;
        let mut initial_placements = std::collections::HashMap::with_capacity(total_keys);
        for i in 0..total_keys {
            let key = format!("cache:key:{}", i);
            let owner = router_before.route_key(&key).unwrap().node_id;
            initial_placements.insert(key, owner);
        }

        // Fail Node 2
        n2.set_state(NodeState::Failed);
        let router_after = RendezvousRouter::new(vec![n1, n2, n3]);

        for (key, old_owner) in &initial_placements {
            let new_owner = router_after.route_key(key).unwrap().node_id;
            assert_ne!(new_owner, 2, "Failed node 2 must not be assigned any keys");
            if *old_owner != 2 {
                // If the key was on node 1 or 3, it MUST remain on node 1 or 3 respectively
                assert_eq!(
                    new_owner, *old_owner,
                    "HRW violation: surviving node key {} moved from node {} to node {}",
                    key, old_owner, new_owner
                );
            }
        }
    }

    #[tokio::test]
    async fn test_shared_cache_node_peer_get_genuine_miss_no_synthetic_data() {
        use phantom_cache_demo::core::cache::{CacheKey, PartitionedCache};
        use phantom_cache_demo::dist::frame::{read_frame, write_frame, PhantomMeshFrame};

        let cache = Arc::new(PartitionedCache::new(4));
        let node = Arc::new(VirtualNode::new_with_cache(10, cache.clone()));
        node.set_state(NodeState::Active);
        let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = PhantomMeshFrame::new_get_request(101, 1, "missing_key_123").unwrap();
        write_frame(&mut stream, &req).await.unwrap();

        let resp = read_frame(&mut stream).await.unwrap();
        let body = resp.extract_response().unwrap();

        // Must return genuine "MISS", never synthetic "val_node_10"
        assert_eq!(body, "MISS", "Expected genuine MISS from shared-cache node");
        assert!(!body.contains("val_node_"), "Synthetic values must not be returned");

        // Cache must remain 100% clean: no entries, zero bytes resident
        let now_ms = phantom_cache_demo::intel::diagnostics::DiagnosticBuffer::now_ms();
        let key = CacheKey::new(None, "missing_key_123", None);
        assert!(
            cache.get_resource(&key, now_ms).is_none(),
            "Uncached peer GET must not insert into serving cache"
        );
        let (resident, _) = cache.audit_bytes();
        assert_eq!(resident, 0, "No bytes must be charged on cache miss");

        node.stop_tcp_service();
    }

    #[tokio::test]
    async fn test_shared_cache_node_peer_get_genuine_hit() {
        use phantom_cache_demo::core::cache::{CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache};
        use phantom_cache_demo::dist::frame::{read_frame, write_frame, PhantomMeshFrame};

        let cache = Arc::new(PartitionedCache::new(4));
        let node = Arc::new(VirtualNode::new_with_cache(11, cache.clone()));
        node.set_state(NodeState::Active);

        // Pre-populate legitimate cached resource
        let now_ms = phantom_cache_demo::intel::diagnostics::DiagnosticBuffer::now_ms();
        let key = CacheKey::new(None, "resident_article_99", None);
        if let FetchRegistrationResult::Initiate(reg) = cache.begin_fetch_atomic(&key, now_ms) {
            let res = Arc::new(CachedResource {
                status: 200,
                content_type: "application/json".to_string(),
                body: Arc::new(b"{\"title\":\"Distributed Caching\"}".to_vec()),
                etag: Some("\"etag-abc-123\"".to_string()),
                created_at_ms: now_ms,
                ttl_ms: 60_000,
            });
            reg.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
        }

        let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = PhantomMeshFrame::new_get_request(102, 1, "resident_article_99").unwrap();
        write_frame(&mut stream, &req).await.unwrap();

        let resp = read_frame(&mut stream).await.unwrap();
        let body = resp.extract_response().unwrap();

        assert_eq!(
            body, "HIT {\"title\":\"Distributed Caching\"}",
            "Must return authentic cached body for resident entry"
        );

        node.stop_tcp_service();
    }

    #[tokio::test]
    async fn test_private_cache_harness_node_preserves_legacy_stub() {
        use phantom_cache_demo::dist::frame::{read_frame, write_frame, PhantomMeshFrame};

        // Legacy harness node without shared cache
        let node = Arc::new(VirtualNode::new(25));
        node.set_state(NodeState::Active);
        let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = PhantomMeshFrame::new_get_request(103, 1, "harness_probe_key").unwrap();
        write_frame(&mut stream, &req).await.unwrap();

        let resp = read_frame(&mut stream).await.unwrap();
        let body = resp.extract_response().unwrap();

        // Legacy harness stub behavior preserved
        assert_eq!(body, "MISS val_node_25");

        node.stop_tcp_service();
    }

    #[tokio::test]
    async fn test_peer_read_and_peer_fill_flow() {
        use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
        use phantom_cache_demo::core::cache::{CacheKey, CachedResource, FetchRegistrationResult, PartitionedCache};
        use phantom_cache_demo::core::coalesce::FinalizeOutcome;
        use phantom_cache_demo::intel::diagnostics::DiagnosticBuffer;
        use std::sync::Mutex;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Node 1 (Remote Primary): resident cache
        let cache1 = Arc::new(PartitionedCache::new(4));
        let node1 = Arc::new(VirtualNode::new_with_cache(1, cache1.clone()));
        node1.set_state(NodeState::Active);
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);

        // Node 2 (Local Gateway): starts with cold cache
        let cache2 = Arc::new(PartitionedCache::new(4));
        let node2 = Arc::new(VirtualNode::new_with_cache(2, cache2.clone()));
        node2.set_state(NodeState::Active);
        let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
        node2.set_addr(addr2);

        // Configure Node 2's router with both nodes, designated local node 2
        let router2 = Arc::new(
            RendezvousRouter::new(vec![node1.clone(), node2.clone()]).with_local_node_id(2),
        );

        // Find a key that HRW deterministically routes to Node 1 as primary
        let mut target_path = String::new();
        for i in 0..100 {
            let candidate_path = format!("/api/item_{}", i);
            let canonical_str = format!("__default__:{}", candidate_path);
            if let Some(owner) = router2.route_key(&canonical_str) {
                if owner.node_id == 1 {
                    target_path = candidate_path;
                    break;
                }
            }
        }
        assert!(!target_path.is_empty(), "Must find a key mapping to Node 1");

        // Seed Node 1's cache with authentic data
        let ms = DiagnosticBuffer::now_ms();
        let target_key = CacheKey::new(None, &target_path, None);
        if let FetchRegistrationResult::Initiate(reg) = cache1.begin_fetch_atomic(&target_key, ms) {
            let res = Arc::new(CachedResource {
                status: 200,
                content_type: "text/plain".into(),
                body: Arc::new(b"authentic_peer_payload_42".to_vec()),
                etag: None,
                created_at_ms: ms,
                ttl_ms: 60_000,
            });
            let _ = reg.into_guard().complete(FinalizeOutcome::Success(res));
        }

        // Verify Node 1 has it, Node 2 does not
        assert!(cache1.get_resource(&target_key, ms).is_some());
        assert!(cache2.get_resource(&target_key, ms).is_none());

        // Spawn Node 2's HTTP gateway server
        let snap = SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: 0,
            throughput_req_sec: 1000.0,
            p50_latency_ms: 5,
            p95_latency_ms: 10,
            p99_latency_ms: 15,
            db_load_ratio: 0.1,
            avoided_compute_dollars: 0.0,
            nodes: vec![
                NodeSnapshot { node_id: 1, state: "Active".into(), cpu_utilization: 10.0, memory_used_bytes: 512 },
                NodeSnapshot { node_id: 2, state: "Active".into(), cpu_utilization: 10.0, memory_used_bytes: 512 },
            ],
        };
        let api_state2 = Arc::new(ApiState {
            snapshot: Mutex::new(snap),
            nodes: vec![node1.clone(), node2.clone()],
            workload_type: Mutex::new("Normal".into()),
            cache: Some(cache2.clone()),
            origin_url: None, // No origin! Request MUST be satisfied by peer read!
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router2.clone()),
        });

        let app2 = create_app(api_state2);
        let listener2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http_addr2 = listener2.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener2, app2).await;
        });

        // Client sends GET to Node 2. Node 2 misses locally, queries Node 1 over TCP,
        // receives HIT, and peer-fills its local cache without any origin contact.
        let mut stream = tokio::net::TcpStream::connect(http_addr2).await.unwrap();
        let http_req = format!("GET /api/v1/cache{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", target_path);
        stream.write_all(http_req.as_bytes()).await.unwrap();
        let mut resp_str = String::new();
        stream.read_to_string(&mut resp_str).await.unwrap();

        assert!(resp_str.contains("HTTP/1.1 200 OK"), "Response must be 200 OK: {}", resp_str);
        assert!(resp_str.contains("authentic_peer_payload_42"), "Response must contain peer data: {}", resp_str);

        // Verify Node 2 local cache is now populated (peer-fill succeeded!)
        let cached_at_node2 = cache2.get_resource(&target_key, DiagnosticBuffer::now_ms());
        assert!(cached_at_node2.is_some(), "Node 2 cache must contain peer-filled entry");
        assert_eq!(*cached_at_node2.unwrap().body, b"authentic_peer_payload_42");

        // Second GET on Node 2 must be an immediate local cache HIT
        let mut stream2 = tokio::net::TcpStream::connect(http_addr2).await.unwrap();
        stream2.write_all(http_req.as_bytes()).await.unwrap();
        let mut resp_str2 = String::new();
        stream2.read_to_string(&mut resp_str2).await.unwrap();

        assert!(resp_str2.contains("HTTP/1.1 200 OK"));
        assert!(resp_str2.to_lowercase().contains("x-cache-status: hit"));

        node1.stop_tcp_service();
        node2.stop_tcp_service();
    }

    #[tokio::test]
    async fn test_peer_miss_falls_back_to_origin() {
        use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
        use phantom_cache_demo::core::cache::PartitionedCache;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Mutex;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Mock origin server
        let origin_fetch_count = Arc::new(AtomicUsize::new(0));
        let count_clone = origin_fetch_count.clone();
        let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin_addr = origin_listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = origin_listener.accept().await {
                    let c = count_clone.clone();
                    tokio::spawn(async move {
                        let mut buf = [0u8; 1024];
                        let _ = socket.read(&mut buf).await;
                        c.fetch_add(1, Ordering::SeqCst);
                        let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 22\r\nCache-Control: max-age=60\r\nConnection: close\r\n\r\norigin_fallback_data42";
                        let _ = socket.write_all(resp.as_bytes()).await;
                    });
                }
            }
        });

        // Node 1 (Primary): empty cache (will return genuine MISS)
        let cache1 = Arc::new(PartitionedCache::new(4));
        let node1 = Arc::new(VirtualNode::new_with_cache(1, cache1.clone()));
        node1.set_state(NodeState::Active);
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);

        // Node 2 (Gateway)
        let cache2 = Arc::new(PartitionedCache::new(4));
        let node2 = Arc::new(VirtualNode::new_with_cache(2, cache2.clone()));
        node2.set_state(NodeState::Active);
        let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
        node2.set_addr(addr2);

        let router2 = Arc::new(
            RendezvousRouter::new(vec![node1.clone(), node2.clone()]).with_local_node_id(2),
        );

        let mut target_path = String::new();
        for i in 0..100 {
            let candidate_path = format!("/api/item_{}", i);
            let canonical_str = format!("__default__:{}", candidate_path);
            if let Some(owner) = router2.route_key(&canonical_str) {
                if owner.node_id == 1 {
                    target_path = candidate_path;
                    break;
                }
            }
        }
        assert!(!target_path.is_empty());

        let snap = SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: 0,
            throughput_req_sec: 1000.0,
            p50_latency_ms: 5,
            p95_latency_ms: 10,
            p99_latency_ms: 15,
            db_load_ratio: 0.1,
            avoided_compute_dollars: 0.0,
            nodes: vec![
                NodeSnapshot { node_id: 1, state: "Active".into(), cpu_utilization: 10.0, memory_used_bytes: 512 },
                NodeSnapshot { node_id: 2, state: "Active".into(), cpu_utilization: 10.0, memory_used_bytes: 512 },
            ],
        };
        let api_state2 = Arc::new(ApiState {
            snapshot: Mutex::new(snap),
            nodes: vec![node1.clone(), node2.clone()],
            workload_type: Mutex::new("Normal".into()),
            cache: Some(cache2.clone()),
            origin_url: Some(format!("http://{}", origin_addr)),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router2.clone()),
        });

        let app2 = create_app(api_state2);
        let listener2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http_addr2 = listener2.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener2, app2).await;
        });

        // Client sends GET to Node 2: Node 2 queries Node 1 over TCP, receives MISS,
        // falls back to mock origin, caches response, and returns 200 OK.
        let mut stream = tokio::net::TcpStream::connect(http_addr2).await.unwrap();
        let http_req = format!("GET /api/v1/cache{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", target_path);
        stream.write_all(http_req.as_bytes()).await.unwrap();
        let mut resp_str = String::new();
        stream.read_to_string(&mut resp_str).await.unwrap();

        assert!(resp_str.contains("HTTP/1.1 200 OK"), "Must be 200 OK: {}", resp_str);
        assert!(resp_str.contains("origin_fallback_data42"), "Must contain origin data: {}", resp_str);
        assert_eq!(origin_fetch_count.load(Ordering::SeqCst), 1, "Origin must be fetched exactly once");

        // Second GET must hit cache2; origin count stays 1
        let mut stream2 = tokio::net::TcpStream::connect(http_addr2).await.unwrap();
        stream2.write_all(http_req.as_bytes()).await.unwrap();
        let mut resp_str2 = String::new();
        stream2.read_to_string(&mut resp_str2).await.unwrap();

        assert!(resp_str2.contains("HTTP/1.1 200 OK"));
        assert!(resp_str2.to_lowercase().contains("x-cache-status: hit"));
        assert_eq!(origin_fetch_count.load(Ordering::SeqCst), 1, "Origin must NOT be called on second GET");

        node1.stop_tcp_service();
        node2.stop_tcp_service();
    }

    #[tokio::test]
    async fn test_peer_unreachable_falls_back_to_origin() {
        use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
        use phantom_cache_demo::core::cache::PartitionedCache;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Mutex;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Mock origin server
        let origin_fetch_count = Arc::new(AtomicUsize::new(0));
        let count_clone = origin_fetch_count.clone();
        let origin_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin_addr = origin_listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = origin_listener.accept().await {
                    let c = count_clone.clone();
                    tokio::spawn(async move {
                        let mut buf = [0u8; 1024];
                        let _ = socket.read(&mut buf).await;
                        c.fetch_add(1, Ordering::SeqCst);
                        let resp = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 25\r\nCache-Control: max-age=60\r\nConnection: close\r\n\r\norigin_unreachable_fail42";
                        let _ = socket.write_all(resp.as_bytes()).await;
                    });
                }
            }
        });

        // Node 1 (Primary): Suspect state and no active TCP listener
        let cache1 = Arc::new(PartitionedCache::new(4));
        let node1 = Arc::new(VirtualNode::new_with_cache(1, cache1.clone()));
        node1.set_state(NodeState::Suspect);
        // Do NOT start TCP service on Node 1; TCP connect will fail immediately.

        // Node 2 (Gateway)
        let cache2 = Arc::new(PartitionedCache::new(4));
        let node2 = Arc::new(VirtualNode::new_with_cache(2, cache2.clone()));
        node2.set_state(NodeState::Active);
        let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
        node2.set_addr(addr2);

        let router2 = Arc::new(
            RendezvousRouter::new(vec![node1.clone(), node2.clone()]).with_local_node_id(2),
        );

        let target_path = "/api/any_item_unreachable";

        let snap = SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: 0,
            throughput_req_sec: 1000.0,
            p50_latency_ms: 5,
            p95_latency_ms: 10,
            p99_latency_ms: 15,
            db_load_ratio: 0.1,
            avoided_compute_dollars: 0.0,
            nodes: vec![
                NodeSnapshot { node_id: 1, state: "Suspect".into(), cpu_utilization: 0.0, memory_used_bytes: 0 },
                NodeSnapshot { node_id: 2, state: "Active".into(), cpu_utilization: 10.0, memory_used_bytes: 512 },
            ],
        };
        let api_state2 = Arc::new(ApiState {
            snapshot: Mutex::new(snap),
            nodes: vec![node1.clone(), node2.clone()],
            workload_type: Mutex::new("Normal".into()),
            cache: Some(cache2.clone()),
            origin_url: Some(format!("http://{}", origin_addr)),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router2.clone()),
        });

        let app2 = create_app(api_state2);
        let listener2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http_addr2 = listener2.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener2, app2).await;
        });

        // Client sends GET: Peer connect fails/skips, falls back to origin seamlessly
        let mut stream = tokio::net::TcpStream::connect(http_addr2).await.unwrap();
        let http_req = format!("GET /api/v1/cache{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n", target_path);
        stream.write_all(http_req.as_bytes()).await.unwrap();
        let mut resp_str = String::new();
        stream.read_to_string(&mut resp_str).await.unwrap();

        assert!(resp_str.contains("HTTP/1.1 200 OK"), "Must be 200 OK: {}", resp_str);
        assert!(resp_str.contains("origin_unreachable_fail42"), "Must contain origin data: {}", resp_str);
        assert_eq!(origin_fetch_count.load(Ordering::SeqCst), 1);

        node2.stop_tcp_service();
    }

    #[test]
    fn test_node_health_state_machine_transitions() {
        let node = VirtualNode::new(10);
        node.set_state(NodeState::Active);

        // 1 probe failure -> transitions Active to Suspect
        let state1 = node.record_probe_failure(3);
        assert_eq!(state1, NodeState::Suspect);
        assert_eq!(node.get_state(), NodeState::Suspect);

        // 2nd probe failure -> remains Suspect
        let state2 = node.record_probe_failure(3);
        assert_eq!(state2, NodeState::Suspect);

        // 3rd consecutive probe failure -> transitions to Failed
        let state3 = node.record_probe_failure(3);
        assert_eq!(state3, NodeState::Failed);
        assert_eq!(node.get_state(), NodeState::Failed);

        // Subsequent failure keeps it Failed
        let state4 = node.record_probe_failure(3);
        assert_eq!(state4, NodeState::Failed);

        // 1st successful probe -> transitions Failed to Recovering
        let s_rec1 = node.record_probe_success(3);
        assert_eq!(s_rec1, NodeState::Recovering);
        assert_eq!(node.get_state(), NodeState::Recovering);

        // 2nd successful probe -> remains Recovering (recovery_threshold = 3)
        let s_rec2 = node.record_probe_success(3);
        assert_eq!(s_rec2, NodeState::Recovering);

        // 3rd consecutive successful probe -> transitions to Active
        let s_rec3 = node.record_probe_success(3);
        assert_eq!(s_rec3, NodeState::Active);
        assert_eq!(node.get_state(), NodeState::Active);
    }

    #[tokio::test]
    async fn test_proactive_failure_detection_and_dynamic_rerouting() {
        // Node 1
        let node1 = Arc::new(VirtualNode::new(1));
        node1.set_state(NodeState::Active);
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);

        // Node 2 (local observer router)
        let node2 = Arc::new(VirtualNode::new(2));
        node2.set_state(NodeState::Active);

        let router = Arc::new(
            RendezvousRouter::new(vec![node1.clone(), node2.clone()]).with_local_node_id(2),
        );

        // Probing active node1 succeeds
        let probe_ok = router.probe_node(&node1, 1000, 3).await;
        assert!(probe_ok.is_ok());
        assert_eq!(node1.get_state(), NodeState::Active);

        // Find a key owned by Node 1
        let mut key_for_node1 = String::new();
        for i in 0..100 {
            let k = format!("test_key_{}", i);
            if router.route_key(&k).map(|n| n.node_id) == Some(1) {
                key_for_node1 = k;
                break;
            }
        }
        assert!(!key_for_node1.is_empty());
        assert_eq!(router.route_key(&key_for_node1).unwrap().node_id, 1);

        // Simulate crash: stop Node 1's TCP listener
        node1.stop_tcp_service();

        // 1st probe fails -> Suspect
        let res1 = router.probe_node(&node1, 200, 3).await;
        assert!(res1.is_err());
        assert_eq!(node1.get_state(), NodeState::Suspect);

        // 2nd probe fails -> Suspect
        let res2 = router.probe_node(&node1, 200, 3).await;
        assert!(res2.is_err());
        assert_eq!(node1.get_state(), NodeState::Suspect);

        // 3rd probe fails -> Failed!
        let res3 = router.probe_node(&node1, 200, 3).await;
        assert!(res3.is_err());
        assert_eq!(node1.get_state(), NodeState::Failed);

        // Dynamic rerouting: key_for_node1 MUST now route to healthy Node 2!
        let failover_owner = router.route_key(&key_for_node1).expect("Must fail over to active node");
        assert_eq!(failover_owner.node_id, 2, "Failed Node 1 must be excluded from routing");

        // Simulate recovery: restart Node 1's TCP service
        let new_addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(new_addr1);

        // Probing recovering node1 succeeds
        let rec_res1 = router.probe_node(&node1, 1000, 1).await;
        assert!(rec_res1.is_ok());
        // Since recovery_threshold was 1, it transitions to Active (or Recovering then Active)
        node1.record_probe_success(1);
        assert_eq!(node1.get_state(), NodeState::Active);

        // Rendezvous routing dynamically restores Node 1 as primary for key_for_node1
        let recovered_owner = router.route_key(&key_for_node1).expect("Must route to restored node");
        assert_eq!(recovered_owner.node_id, 1, "Restored Node 1 must resume primary ownership");

        node1.stop_tcp_service();
    }

    #[tokio::test]
    async fn test_cluster_observability_schema_metrics() {
        use phantom_cache_demo::api::server::{build_schema_v2_payload, ApiState, NodeSnapshot, SimulationSnapshot};
        use phantom_cache_demo::core::cache::PartitionedCache;
        use std::sync::Mutex;

        let node1 = Arc::new(VirtualNode::new(1));
        node1.set_state(NodeState::Active);
        let node2 = Arc::new(VirtualNode::new(2));
        node2.set_state(NodeState::Active);

        let router = Arc::new(
            RendezvousRouter::new(vec![node1.clone(), node2.clone()]).with_local_node_id(1),
        );

        let snap = SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: 0,
            throughput_req_sec: 1000.0,
            p50_latency_ms: 5,
            p95_latency_ms: 10,
            p99_latency_ms: 15,
            db_load_ratio: 0.1,
            avoided_compute_dollars: 0.0,
            nodes: vec![
                NodeSnapshot { node_id: 1, state: "Active".into(), cpu_utilization: 10.0, memory_used_bytes: 512 },
                NodeSnapshot { node_id: 2, state: "Active".into(), cpu_utilization: 10.0, memory_used_bytes: 512 },
            ],
        };

        let state = Arc::new(ApiState {
            snapshot: Mutex::new(snap),
            nodes: vec![node1, node2],
            workload_type: Mutex::new("Normal".into()),
            cache: Some(Arc::new(PartitionedCache::new(4))),
            origin_url: None,
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router.clone()),
        });

        let payload = build_schema_v2_payload(&state);
        let cluster = &payload["metrics"]["cluster"];
        assert!(cluster.is_object(), "metrics.cluster must be an object");

        let broadcaster = &cluster["broadcaster"];
        assert_eq!(broadcaster["sent"]["src"], "measured");
        assert_eq!(broadcaster["retried"]["src"], "measured");
        assert_eq!(broadcaster["acked"]["src"], "measured");
        assert_eq!(broadcaster["dropped"]["src"], "measured");
        // PC-AUTO-034: queue overflow (`dropped`) and give-up (`exhausted`) are separate counters.
        assert_eq!(broadcaster["enqueued"]["src"], "measured");
        assert_eq!(broadcaster["exhausted"]["src"], "measured");
        assert_eq!(broadcaster["exhausted"]["v"], 0);

        // PC-AUTO-039: `peers` lists remote members only, the local node is reported separately,
        // and every state is labelled like its siblings.
        let peers = cluster["peers"].as_array().expect("cluster.peers must be an array");
        assert_eq!(peers.len(), 1, "Only the remote member is a peer; the local node is not");
        assert_eq!(peers[0]["node_id"], 2);
        assert_eq!(peers[0]["state"]["src"], "measured");
        assert_eq!(peers[0]["state"]["v"], "Active");
        assert_eq!(peers[0]["idle_connections"]["src"], "measured");
        assert_eq!(peers[0]["active_connections"]["src"], "measured");
        assert_eq!(cluster["local"]["node_id"], 1);
        assert_eq!(cluster["local"]["state"]["src"], "measured");
    }

    // =====================================================================================
    // REV-003-FIXES (PC-AUTO-034, 035, 037, 038, 039). Helpers shared by the tests below.
    // =====================================================================================

    use phantom_cache_demo::core::cache::{CacheKey as RfKey, CachedResource as RfRes, FetchRegistrationResult as RfReg, PartitionedCache as RfCache};
    use phantom_cache_demo::dist::frame::{InvalidateRecord as RfRecord, InvalidationScope as RfScope};
    use phantom_cache_demo::dist::router::{DistError as RfDistError, ReplicaInvalidationTask as RfTask};
    use phantom_cache_demo::intel::diagnostics::DiagnosticBuffer as RfClock;
    use std::time::Duration as RfDuration;

    /// In-process plaintext loopback node (same router/pool/frame code paths as the gateway;
    /// mTLS-specific behaviour is covered by the multi-process scripts).
    async fn rf_plain_node(id: u8) -> Arc<VirtualNode> {
        let n = Arc::new(VirtualNode::new(id));
        let addr = n.start_tcp_service("127.0.0.1:0").await.unwrap();
        n.set_addr(addr);
        n
    }

    /// Node whose address points at a closed port (connect refused).
    fn rf_dead_node(id: u8) -> Arc<VirtualNode> {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        drop(l);
        let n = Arc::new(VirtualNode::new(id));
        n.set_addr(addr);
        n.set_state(NodeState::Active);
        n
    }

    fn rf_put(cache: &Arc<RfCache>, key: &RfKey, body: &[u8], created_at_ms: u64, ttl_ms: u64) {
        let now = RfClock::now_ms();
        let res = Arc::new(RfRes {
            status: 200,
            content_type: "text/plain".to_string(),
            body: Arc::new(body.to_vec()),
            etag: None,
            created_at_ms,
            ttl_ms,
        });
        match cache.begin_fetch_atomic(key, now) {
            RfReg::Initiate(r) => {
                let _ = r.into_guard().complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
            }
            _ => panic!("expected Initiate for a fresh key"),
        }
    }

    fn rf_has(cache: &RfCache, key: &RfKey) -> bool {
        cache.get_resource(key, RfClock::now_ms()).is_some()
    }

    async fn rf_wait_until(timeout_ms: u64, mut cond: impl FnMut() -> bool) -> bool {
        let deadline = std::time::Instant::now() + RfDuration::from_millis(timeout_ms);
        while std::time::Instant::now() < deadline {
            if cond() {
                return true;
            }
            tokio::time::sleep(RfDuration::from_millis(10)).await;
        }
        cond()
    }

    fn rf_no_tls() -> Option<Arc<phantom_cache_demo::security::ClusterTlsConfig>> {
        None
    }

    // -------------------------------------------------------------------------------------
    // PC-AUTO-037 (P1). Regression for "a burst of concurrent invalidations leaves healthy
    // nodes stuck Recovering and later DELETEs stop reaching replicas". Fails without the fix:
    // before it, concurrent callers beyond the 8-slot pool got NodeUnreachable (not Busy),
    // the primary was marked Suspect and the node never returned to Active.
    // -------------------------------------------------------------------------------------
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pc_auto_037_burst_does_not_strand_peers_and_next_delete_reaches_replica() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let n3 = rf_plain_node(3).await;
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]).with_local_node_id(1));

        // 40 keys whose primary is NOT node 3, so node 3 is a replica for every one of them.
        let mut keys: Vec<RfKey> = Vec::new();
        let mut i = 0;
        while keys.len() < 40 {
            let k = RfKey::new(None, &format!("/burst_{i}"), None);
            if router.route_key(&k.to_canonical_string()).unwrap().node_id != 3 {
                rf_put(&n3.cache, &k, b"stale-on-3", RfClock::now_ms(), 600_000);
                keys.push(k);
            }
            i += 1;
        }

        let mut handles = Vec::new();
        for k in &keys {
            let r = router.clone();
            let canon = k.to_canonical_string();
            handles.push(tokio::spawn(async move { r.invalidate_key_across_cluster(&canon, RfScope::ExactKey).await }));
        }
        let mut results = Vec::new();
        for h in handles {
            results.push(h.await.unwrap());
        }
        // Local pool pressure may surface as Busy (retryable) but never as a peer failure.
        for res in &results {
            assert!(
                matches!(res, Ok(_) | Err(RfDistError::Busy(_))),
                "burst DELETE must succeed or report Busy, got {:?}",
                res
            );
        }
        // Back-pressure must not mark any healthy node Suspect/Recovering.
        for n in [&n1, &n2, &n3] {
            assert_eq!(n.get_state(), NodeState::Active, "node {} must stay Active after the burst", n.node_id);
        }
        // Every key is eventually purged on node 3 (delivered by the primary path or the fan-out).
        let all_gone = rf_wait_until(5_000, || keys.iter().all(|k| !rf_has(&n3.cache, k))).await;
        assert!(all_gone, "all 40 invalidations must reach node 3");

        // Reconcile cycles (what the 250 ms worker does) must leave every node Active.
        for _ in 0..5 {
            router.reconcile_all_peers().await;
        }
        for n in [&n1, &n2, &n3] {
            assert_eq!(n.get_state(), NodeState::Active);
        }

        // The very next DELETE still reaches node 3.
        let probe = RfKey::new(None, "/burst_probe", None);
        rf_put(&n3.cache, &probe, b"stale-on-3", RfClock::now_ms(), 600_000);
        let _ = router.invalidate_key_across_cluster(&probe.to_canonical_string(), RfScope::ExactKey).await;
        assert!(
            rf_wait_until(2_000, || !rf_has(&n3.cache, &probe)).await,
            "the DELETE after the burst must reach node 3"
        );

        let c = router.get_broadcaster_counters();
        assert_eq!(c.dropped, 0, "no queue overflow");
        assert_eq!(c.exhausted, 0, "no fan-out gave up on a healthy cluster");
        router.stop();
    }

    /// PC-AUTO-037 (b): `Recovering` is transitional. A peer left Recovering by an earlier partial
    /// reconciliation must be reconciled again and promoted. The peer owes this node a replay, so
    /// the sync answer is `ReplayBatch` (the case that used to dead-end: `InSync` needs equal ring
    /// digests, which nodes that both send and receive invalidations do not have).
    /// Fails without the fix: reconcile_all_peers ignored Recovering peers, and ReplayBatch never promoted.
    #[tokio::test]
    async fn test_pc_auto_037_recovering_peer_converges_to_active_via_replay_batch() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let key = RfKey::new(None, "/replay_me", None);
        rf_put(&n1.cache, &key, b"stale", RfClock::now_ms(), 600_000);
        n2.ring_buffer.lock().unwrap().push_outbound(
            RfRecord { stream_seq: 1, scope: RfScope::ExactKey, key: key.to_canonical_string() },
            1,
        );
        n1.set_state(NodeState::Recovering);
        n2.set_state(NodeState::Recovering);

        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]).with_local_node_id(1));
        for _ in 0..5 {
            router.reconcile_all_peers().await;
            if n1.get_state() == NodeState::Active && n2.get_state() == NodeState::Active {
                break;
            }
        }
        assert_eq!(n2.get_state(), NodeState::Active, "peer must be promoted out of Recovering");
        assert_eq!(n1.get_state(), NodeState::Active, "local node must be promoted out of Recovering");
        assert!(!rf_has(&n1.cache, &key), "the replayed invalidation must have been applied");
        router.stop();
    }

    /// PC-AUTO-037 (a): invalidation fan-out targets every configured member regardless of health,
    /// while read routing/failover candidates keep the health filter.
    #[test]
    fn test_pc_auto_037_invalidation_targets_ignore_health_but_read_replicas_do_not() {
        let n1 = Arc::new(VirtualNode::new(1));
        let n2 = Arc::new(VirtualNode::new(2));
        let n3 = Arc::new(VirtualNode::new(3));
        n1.set_state(NodeState::Active);
        n2.set_state(NodeState::Suspect);
        n3.set_state(NodeState::Recovering);
        let router = RendezvousRouter::new(vec![n1, n2, n3]);
        let key = "__default__:/targets_probe";
        let targets: Vec<u8> = router.invalidation_targets(key, 1).iter().map(|n| n.node_id).collect();
        let mut sorted = targets.clone();
        sorted.sort();
        assert_eq!(sorted, vec![2, 3], "Suspect and Recovering members must stay invalidation targets");
        assert!(router.get_replicas(key, 2).is_empty(), "read routing still excludes unhealthy members");
        router.stop();
    }

    /// PC-AUTO-037 (c): a full local pool is back-pressure (`Busy`), not peer failure. Fails without
    /// the fix (the old code returned NodeUnreachable and the caller marked the primary Suspect).
    #[tokio::test]
    async fn test_pc_auto_037_pool_exhaustion_reports_busy_and_never_marks_peer_suspect() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]).with_local_node_id(1));

        // Occupy every connection slot (max_total_per_node = 8) to node 2.
        let mut held = Vec::new();
        for _ in 0..8 {
            held.push(
                RendezvousRouter::acquire_connection_to_node(router.pool(), &rf_no_tls(), &n2, 500)
                    .await
                    .expect("slot"),
            );
        }
        let res = RendezvousRouter::acquire_connection_to_node(router.pool(), &rf_no_tls(), &n2, 50).await;
        assert!(matches!(res, Err(RfDistError::Busy(2))), "expected Busy(2), got {:?}", res.err());
        assert_eq!(n2.get_state(), NodeState::Active, "Busy must not change the peer's state");

        // Same through the invalidation entry point, for a key whose primary is node 2.
        let key = (0..500)
            .map(|i| RfKey::new(None, &format!("/busy_{i}"), None))
            .find(|k| router.route_key(&k.to_canonical_string()).unwrap().node_id == 2)
            .expect("key owned by node 2");
        rf_put(&n2.cache, &key, b"stale", RfClock::now_ms(), 600_000);
        let out = router.invalidate_key_across_cluster(&key.to_canonical_string(), RfScope::ExactKey).await;
        assert!(matches!(out, Err(RfDistError::Busy(2))), "expected Busy(2), got {:?}", out);
        assert_eq!(n2.get_state(), NodeState::Active, "primary must not be marked Suspect by local pressure");
        assert_eq!(n1.get_state(), NodeState::Active);

        // The invalidation is not lost: once slots free up the broadcaster delivers it to the primary.
        for c in held {
            router.pool().checkin(c);
        }
        assert!(rf_wait_until(3_000, || !rf_has(&n2.cache, &key)).await, "background delivery to the primary");
        router.stop();
    }

    /// PC-AUTO-037 follow-up found in batch B1 (dead primary + saturated acting primary): when the
    /// FAILOVER leg is Busy, the DELETE is answered Busy but the invalidation must still be delivered in
    /// the background. Before B1 this arm returned Busy and dropped the invalidation on the floor.
    #[tokio::test]
    async fn test_pc_auto_037_busy_failover_leg_still_delivers_in_background() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let n3 = rf_dead_node(3);
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]).with_local_node_id(1));
        // A key owned by the dead node 3; its acting primary is the first healthy replica.
        let key = (0..2000)
            .map(|i| RfKey::new(None, &format!("/failover_busy_{i}"), None))
            .find(|k| router.route_key(&k.to_canonical_string()).unwrap().node_id == 3)
            .expect("key owned by node 3");
        let acting = router.get_replicas(&key.to_canonical_string(), 2)[0].clone();
        let other = if acting.node_id == 1 { n2.clone() } else { n1.clone() };
        rf_put(&acting.cache, &key, b"stale", RfClock::now_ms(), 600_000);
        rf_put(&other.cache, &key, b"stale", RfClock::now_ms(), 600_000);

        // Saturate the pool to the acting primary so the failover leg is Busy.
        let mut held = Vec::new();
        for _ in 0..8 {
            held.push(RendezvousRouter::acquire_connection_to_node(router.pool(), &rf_no_tls(), &acting, 500).await.expect("slot"));
        }
        let out = router.invalidate_key_across_cluster(&key.to_canonical_string(), RfScope::ExactKey).await;
        assert!(matches!(out, Err(RfDistError::Busy(_))), "expected Busy, got {out:?}");
        for c in held {
            router.pool().checkin(c);
        }
        assert!(
            rf_wait_until(5_000, || !rf_has(&acting.cache, &key) && !rf_has(&other.cache, &key)).await,
            "the invalidation must still reach the other members in the background"
        );
        router.stop();
    }

    // -------------------------------------------------------------------------------------
    // PC-AUTO-034. `exhausted` counts deliveries that gave up; `dropped` stays queue overflow.
    // -------------------------------------------------------------------------------------
    #[tokio::test]
    async fn test_pc_auto_034_exhausted_counts_gave_up_deliveries_and_clean_run_shows_zero() {
        // Clean run: live peer, every delivery acked, nothing exhausted.
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]).with_local_node_id(1));
        for i in 0..3 {
            assert!(router.broadcaster().enqueue(RfTask {
                key: format!("__default__:/clean_{i}"),
                scope: RfScope::ExactKey,
                replica_node_ids: vec![2],
                membership_epoch: 1,
            }));
        }
        assert!(rf_wait_until(3_000, || router.get_broadcaster_counters().acked == 3).await);
        let c = router.get_broadcaster_counters();
        assert_eq!((c.enqueued, c.acked, c.exhausted, c.dropped, c.retried), (3, 3, 0, 0, 0));
        router.stop();

        // Forced failure: the destination refuses connections; every delivery uses 4 attempts and gives up.
        let dead = rf_dead_node(2);
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), dead]).with_local_node_id(1));
        for i in 0..3 {
            assert!(router.broadcaster().enqueue(RfTask {
                key: format!("__default__:/lost_{i}"),
                scope: RfScope::ExactKey,
                replica_node_ids: vec![2],
                membership_epoch: 1,
            }));
        }
        assert!(rf_wait_until(5_000, || router.get_broadcaster_counters().exhausted == 3).await);
        let c = router.get_broadcaster_counters();
        assert_eq!(c.exhausted, 3, "three deliveries gave up");
        assert_eq!(c.dropped, 0, "`dropped` keeps meaning queue overflow only");
        assert_eq!(c.acked, 0);
        assert_eq!((c.sent, c.retried), (12, 9), "4 attempts per delivery, 3 of them retries");
        router.stop();
    }

    // -------------------------------------------------------------------------------------
    // PC-AUTO-038. After a peer restart every idle pooled connection to it is dead. The stale
    // retry must evict them all, so the immediate retry opens a genuinely new connection.
    // Fails without `evict_node`: the retry takes another stale idle connection, so the delivery
    // needs 5 attempts instead of 2.
    // -------------------------------------------------------------------------------------
    #[tokio::test]
    async fn test_pc_auto_038_stale_pooled_connections_are_evicted_and_retry_uses_a_new_connection() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]).with_local_node_id(1));

        // Fill the idle pool with 4 reused connections to node 2.
        let mut conns = Vec::new();
        for _ in 0..4 {
            conns.push(
                RendezvousRouter::acquire_connection_to_node(router.pool(), &rf_no_tls(), &n2, 500)
                    .await
                    .unwrap(),
            );
        }
        for c in conns {
            router.pool().checkin(c);
        }
        assert_eq!(router.pool().idle_count_for_node(2), 4);

        // "Restart" of node 2 as seen by the client: every established session is closed server-side,
        // while the listener keeps accepting new connections.
        assert!(n2.session_tracker.evict_all() >= 4);
        tokio::time::sleep(RfDuration::from_millis(150)).await;

        let key = RfKey::new(None, "/stale_conn", None);
        rf_put(&n2.cache, &key, b"stale", RfClock::now_ms(), 600_000);
        assert!(router.broadcaster().enqueue(RfTask {
            key: key.to_canonical_string(),
            scope: RfScope::ExactKey,
            replica_node_ids: vec![2],
            membership_epoch: 1,
        }));
        assert!(
            rf_wait_until(5_000, || {
                let c = router.get_broadcaster_counters();
                c.acked + c.exhausted >= 1
            })
            .await
        );
        let c = router.get_broadcaster_counters();
        assert_eq!(c.acked, 1, "delivered");
        assert_eq!(c.exhausted, 0);
        assert_eq!(c.sent, 2, "one stale attempt plus one immediate attempt on a new connection");
        assert_eq!(c.retried, 1, "the extra attempt is counted honestly");
        assert!(!rf_has(&n2.cache, &key));
        router.stop();
    }
    // =====================================================================================
    // BATCH B1: peer-read fidelity and freshness (PC-AUTO-041 / 042), X-Cache-Source,
    // fan-out hardening (PC-AUTO-043), honest reachability (PC-AUTO-044).
    // =====================================================================================

    /// Seeds `cache` with a full entry (content type, ETag, status, validators) created at `created_at_ms`.
    #[allow(clippy::too_many_arguments)]
    fn rf_put_full(
        cache: &Arc<RfCache>,
        key: &RfKey,
        status: u16,
        content_type: &str,
        etag: Option<&str>,
        body: &[u8],
        created_at_ms: u64,
        ttl_ms: u64,
    ) {
        let now = RfClock::now_ms();
        let res = Arc::new(RfRes {
            status,
            content_type: content_type.to_string(),
            body: Arc::new(body.to_vec()),
            etag: etag.map(|s| s.to_string()),
            created_at_ms,
            ttl_ms,
        });
        match cache.begin_fetch_atomic(key, now) {
            RfReg::Initiate(r) => {
                let guard = r.into_guard();
                guard.cache.update_freshness_for_guard(
                    &guard,
                    phantom_cache_demo::core::freshness::FreshnessMetadata {
                        must_revalidate: true,
                        stale_while_revalidate_ms: Some(30_000),
                        stale_if_error_ms: Some(120_000),
                        last_modified: Some("Wed, 21 Oct 2026 07:28:00 GMT".to_string()),
                        date_ms: Some(created_at_ms),
                        initial_age_ms: 0,
                    },
                );
                let _ = guard.complete(phantom_cache_demo::core::coalesce::FinalizeOutcome::Success(res));
            }
            _ => panic!("expected Initiate for a fresh key"),
        }
    }

    /// Raw HTTP/1.1 GET against an in-process gateway; returns the whole response text (lower-cased headers kept as sent).
    async fn rf_http_get(addr: std::net::SocketAddr, path: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = format!("GET /api/v1/cache{path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.unwrap();
        String::from_utf8_lossy(&raw).into_owned()
    }

    fn rf_header(resp: &str, name: &str) -> Option<String> {
        let lower_name = format!("{}:", name.to_lowercase());
        resp.split("\r\n\r\n")
            .next()
            .unwrap_or("")
            .lines()
            .find(|l| l.to_lowercase().starts_with(&lower_name))
            .map(|l| l[lower_name.len()..].trim().to_string())
    }

    /// Two in-process gateways sharing nothing but the cluster transport: node 1 (primary of a key) and
    /// node 2 (asks node 1 on a local miss). Returns everything a test needs.
    struct RfPair {
        cache1: Arc<RfCache>,
        cache2: Arc<RfCache>,
        node1: Arc<VirtualNode>,
        node2: Arc<VirtualNode>,
        router2: Arc<RendezvousRouter>,
        http2: std::net::SocketAddr,
        path: String,
        key: RfKey,
    }

    async fn rf_pair() -> RfPair {
        use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
        use std::sync::Mutex;

        let cache1 = Arc::new(RfCache::new(4));
        let node1 = Arc::new(VirtualNode::new_with_cache(1, cache1.clone()));
        let addr1 = node1.start_tcp_service("127.0.0.1:0").await.unwrap();
        node1.set_addr(addr1);
        let cache2 = Arc::new(RfCache::new(4));
        let node2 = Arc::new(VirtualNode::new_with_cache(2, cache2.clone()));
        let addr2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
        node2.set_addr(addr2);
        let router2 = Arc::new(RendezvousRouter::new(vec![node1.clone(), node2.clone()]).with_local_node_id(2));

        let path = (0..200)
            .map(|i| format!("/api/fresh_{i}"))
            .find(|p| router2.route_key(&format!("__default__:{p}")).unwrap().node_id == 1)
            .expect("key owned by node 1");
        let key = RfKey::new(None, &path, None);

        let snap = SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: 0,
            throughput_req_sec: 0.0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            db_load_ratio: 0.0,
            avoided_compute_dollars: 0.0,
            nodes: vec![
                NodeSnapshot { node_id: 1, state: "Active".into(), cpu_utilization: 0.0, memory_used_bytes: 0 },
                NodeSnapshot { node_id: 2, state: "Active".into(), cpu_utilization: 0.0, memory_used_bytes: 0 },
            ],
        };
        let api_state2 = Arc::new(ApiState {
            snapshot: Mutex::new(snap),
            nodes: vec![node1.clone(), node2.clone()],
            workload_type: Mutex::new("Normal".into()),
            cache: Some(cache2.clone()),
            origin_url: None,
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router2.clone()),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http2 = listener.local_addr().unwrap();
        let app = create_app(api_state2);
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        RfPair { cache1, cache2, node1, node2, router2, http2, path, key }
    }

    // -------------------------------------------------------------------------------------
    // PC-AUTO-041 / 042 (WP9, batch B1). Peer read-through through the REAL gateway HTTP handler and a
    // REAL peer read over the cluster transport, with NON-JSON content. The clock is simulated only in
    // the sense that the source entry is created 40 s "in the past" and expiry is evaluated by passing
    // `now_ms` to the cache; nothing is slept. Before B1 the filled copy was `application/json`, had no
    // ETag, `Age: 0`, a fresh `created_at` and a fixed 60 s lifetime (outliving its source by 40 s).
    // -------------------------------------------------------------------------------------
    #[tokio::test]
    async fn test_peer_fill_staleness_and_invalidation() {
        let p = rf_pair().await;

        // Source entry on node 1: text/plain + ETag + validators, created 40 s ago, TTL 60 s => 20 s left.
        let t0 = RfClock::now_ms();
        let source_created = t0 - 40_000;
        let source_expiry = source_created + 60_000;
        rf_put_full(&p.cache1, &p.key, 200, "text/plain; charset=utf-8", Some("\"v1-abc\""), b"plain text body", source_created, 60_000);

        // GET at node 2: local miss, peer read from node 1, peer fill.
        let resp = rf_http_get(p.http2, &p.path).await;
        let t1 = RfClock::now_ms();
        assert!(resp.contains("200 OK") && resp.contains("plain text body"), "peer read must serve the body: {resp}");

        // Fidelity (PC-AUTO-041): headers equal the primary's.
        assert_eq!(rf_header(&resp, "content-type").as_deref(), Some("text/plain; charset=utf-8"), "{resp}");
        assert_eq!(rf_header(&resp, "etag").as_deref(), Some("\"v1-abc\""), "{resp}");
        assert_eq!(rf_header(&resp, "x-cache-source").as_deref(), Some("peer"), "{resp}");
        assert_eq!(rf_header(&resp, "x-cache-status").as_deref(), Some("MISS"), "X-Cache-Status semantics unchanged: {resp}");

        // Truthful Age (PC-AUTO-042): the source entry is 40 s old.
        let age: u64 = rf_header(&resp, "age").expect("Age header").parse().unwrap();
        assert!((40..=42).contains(&age), "Age must reflect the source entry, got {age}");

        // The stored copy keeps status, content type, ETag, validators, and expires WITH its source.
        let (peer, meta) = p.cache2.get_resource_with_metadata(&p.key, t1).expect("peer-filled copy resident on node 2");
        assert_eq!(peer.status, 200);
        assert_eq!(peer.content_type, "text/plain; charset=utf-8");
        assert_eq!(peer.etag.as_deref(), Some("\"v1-abc\""));
        let meta = meta.expect("validators carried to the peer copy");
        assert!(meta.must_revalidate);
        assert_eq!(meta.stale_while_revalidate_ms, Some(30_000));
        assert_eq!(meta.stale_if_error_ms, Some(120_000));
        assert_eq!(meta.last_modified.as_deref(), Some("Wed, 21 Oct 2026 07:28:00 GMT"));
        let peer_expiry = peer.created_at_ms + peer.ttl_ms;
        let drift = peer_expiry as i64 - source_expiry as i64;
        assert!(drift.abs() <= 1_500, "peer copy must expire with its source, drift {drift} ms");
        assert!(peer.created_at_ms <= source_created + 1_500, "created_at keeps the source age");

        let probe_now = source_expiry + 2_000;
        assert!(p.cache1.get_resource(&p.key, probe_now).is_none(), "source entry is expired");
        assert!(p.cache2.get_resource(&p.key, probe_now).is_none(), "peer copy must NOT outlive its source");

        // Second GET is a local HIT with the same headers and X-Cache-Source: local.
        let again = rf_http_get(p.http2, &p.path).await;
        assert_eq!(rf_header(&again, "x-cache-status").as_deref(), Some("HIT"), "{again}");
        assert_eq!(rf_header(&again, "x-cache-source").as_deref(), Some("local"), "{again}");
        assert_eq!(rf_header(&again, "content-type").as_deref(), Some("text/plain; charset=utf-8"), "{again}");
        assert_eq!(rf_header(&again, "etag").as_deref(), Some("\"v1-abc\""), "{again}");

        // Real invalidation path (not cache.delete): the cluster invalidation purges both copies.
        let out = p.router2.invalidate_key_across_cluster(&p.key.to_canonical_string(), RfScope::ExactKey).await;
        assert!(out.is_ok(), "{:?}", out);
        assert!(
            rf_wait_until(3_000, || !rf_has(&p.cache1, &p.key) && !rf_has(&p.cache2, &p.key)).await,
            "cluster invalidation must purge the source and the peer-filled copy"
        );

        p.router2.stop();
        p.node1.stop_tcp_service();
        p.node2.stop_tcp_service();
    }

    /// PC-AUTO-041: a 404 and a binary (non-UTF-8) body survive the peer read byte for byte (the legacy
    /// string protocol turned invalid UTF-8 into replacement characters).
    #[tokio::test]
    async fn test_peer_fill_preserves_status_and_binary_body() {
        let p = rf_pair().await;
        let body: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x00, 0xFF, 0xFE, 0x80, 0x01];
        rf_put_full(&p.cache1, &p.key, 404, "image/png", Some("W/\"img-1\""), &body, RfClock::now_ms(), 60_000);
        let resp = rf_http_get(p.http2, &p.path).await;
        assert!(resp.starts_with("HTTP/1.1 404"), "status must be carried: {}", &resp[..resp.len().min(80)]);
        assert_eq!(rf_header(&resp, "content-type").as_deref(), Some("image/png"));
        assert_eq!(rf_header(&resp, "x-cache-source").as_deref(), Some("peer"));
        let peer = p.cache2.get_resource(&p.key, RfClock::now_ms()).expect("peer copy");
        assert_eq!(peer.status, 404);
        assert_eq!(*peer.body, body, "binary body must be identical");
        p.router2.stop();
        p.node1.stop_tcp_service();
        p.node2.stop_tcp_service();
    }

    /// Wire level: round trip, version guard (a peer that speaks another format is refused, never
    /// interpreted), and a node that is not a shared-cache node answers Miss.
    #[tokio::test]
    async fn test_peer_read_wire_roundtrip_and_version_guard() {
        use phantom_cache_demo::dist::frame::{
            read_frame, write_frame, MessageType, PeerEntry, PhantomMeshFrame, PEER_ENTRY_FORMAT,
        };
        // 1. Encode/decode round trip with every optional field.
        let entry = PeerEntry {
            status: 301,
            content_type: "text/html".into(),
            etag: Some("\"e\"".into()),
            last_modified: Some("lm".into()),
            must_revalidate: true,
            stale_while_revalidate_ms: Some(5),
            stale_if_error_ms: Some(6),
            date_ms: Some(7),
            initial_age_ms: 8,
            age_ms: 9,
            remaining_ms: 10,
            body: vec![0, 255, 1],
        };
        let frame = PhantomMeshFrame::new_peer_read_hit(1, 2, &entry).unwrap();
        assert_eq!(frame.extract_peer_read_response().unwrap(), Some(entry.clone()));
        let minimal = PeerEntry { etag: None, last_modified: None, must_revalidate: false, stale_while_revalidate_ms: None, stale_if_error_ms: None, date_ms: None, ..entry.clone() };
        let f2 = PhantomMeshFrame::new_peer_read_hit(1, 2, &minimal).unwrap();
        assert_eq!(f2.extract_peer_read_response().unwrap(), Some(minimal));

        // 2. A response in a format this build does not speak is refused.
        let mut alien = frame.clone();
        alien.payload[0] = PEER_ENTRY_FORMAT + 1;
        assert!(alien.extract_peer_read_response().is_err(), "unknown format must be refused");
        let unsupported = PhantomMeshFrame::new_peer_read_unsupported(1, 2).unwrap();
        assert!(unsupported.extract_peer_read_response().is_err(), "Unsupported answer must be refused");
        let miss = PhantomMeshFrame::new_peer_read_miss(1, 2).unwrap();
        assert_eq!(miss.extract_peer_read_response().unwrap(), None);

        // 3. A live shared-cache node: a request in an unknown format gets Unsupported, not a guess.
        let cache = Arc::new(RfCache::new(4));
        let node = Arc::new(VirtualNode::new_with_cache(1, cache.clone()));
        let addr = node.start_tcp_service("127.0.0.1:0").await.unwrap();
        let key = RfKey::new(None, "/wire", None);
        rf_put_full(&cache, &key, 200, "text/plain", Some("\"w\""), b"wire", RfClock::now_ms(), 60_000);
        let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut req = PhantomMeshFrame::new_peer_read_request(7, 1, &key.to_canonical_string()).unwrap();
        req.payload[0] = PEER_ENTRY_FORMAT + 1;
        write_frame(&mut s, &req).await.unwrap();
        let resp = read_frame(&mut s).await.unwrap();
        assert_eq!(resp.header.msg_type, MessageType::PeerReadResponse);
        assert!(resp.extract_peer_read_response().is_err(), "unknown request format must be answered Unsupported");
        // Same connection, correct format: a hit with the metadata.
        let req_ok = PhantomMeshFrame::new_peer_read_request(8, 2, &key.to_canonical_string()).unwrap();
        write_frame(&mut s, &req_ok).await.unwrap();
        let hit = read_frame(&mut s).await.unwrap().extract_peer_read_response().unwrap().expect("hit");
        assert_eq!((hit.status, hit.content_type.as_str(), hit.etag.as_deref()), (200, "text/plain", Some("\"w\"")));
        assert_eq!(hit.body, b"wire");
        assert!(hit.remaining_ms > 0 && hit.remaining_ms <= 60_000);
        // A Recovering node never serves.
        node.set_state(NodeState::Recovering);
        write_frame(&mut s, &PhantomMeshFrame::new_peer_read_request(9, 3, &key.to_canonical_string()).unwrap()).await.unwrap();
        assert_eq!(read_frame(&mut s).await.unwrap().extract_peer_read_response().unwrap(), None);
        node.stop_tcp_service();
    }

    // -------------------------------------------------------------------------------------
    // PC-AUTO-043. A dead destination must not delay deliveries to a healthy one, must not grow an
    // unbounded task backlog, and is capped to one attempt once known unhealthy.
    // Before B1: one shared 16-permit semaphore, 4 attempts with backoff per delivery for the dead peer
    // => the healthy replica was purged only after seconds.
    // -------------------------------------------------------------------------------------
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pc_auto_043_dead_destination_does_not_delay_healthy_replica() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let n3 = rf_dead_node(3);
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]).with_local_node_id(1));

        let mut keys = Vec::new();
        for i in 0..400 {
            let k = RfKey::new(None, &format!("/storm_{i}"), None);
            rf_put(&n2.cache, &k, b"stale-on-2", RfClock::now_ms(), 600_000);
            keys.push(k);
        }
        let t0 = std::time::Instant::now();
        for k in &keys {
            assert!(router.broadcaster().enqueue(RfTask {
                key: k.to_canonical_string(),
                scope: RfScope::ExactKey,
                replica_node_ids: vec![3, 2], // the dead peer is listed FIRST on purpose
                membership_epoch: 1,
            }));
        }
        let purged = rf_wait_until(10_000, || keys.iter().all(|k| !rf_has(&n2.cache, k))).await;
        let took = t0.elapsed();
        assert!(purged, "healthy replica must be purged");
        assert!(took < RfDuration::from_millis(1_500), "healthy replica purged in {took:?}, a dead peer must not delay it");

        // The dead peer's backlog drains on its own queue; every delivery to it is counted exhausted.
        assert!(rf_wait_until(15_000, || router.get_broadcaster_counters().exhausted == 400).await);
        let c = router.get_broadcaster_counters();
        assert_eq!(c.acked, 400, "all healthy deliveries acked");
        assert_eq!(c.dropped, 0);
        // At most FANOUT_DEST_CONCURRENCY (4) deliveries start while the peer is still believed Active and
        // use the full 4 attempts; once it is unreachable only one probe per 500 ms is attempted. Before:
        // 400 x 4 = 1600 dead attempts, each up to 150 ms.
        assert!(c.sent <= 400 + 16 + 40, "attempts to a known-unhealthy peer must be capped, sent={}", c.sent);

        // PC-AUTO-044: the dead peer is reported unreachable, the healthy one reachable.
        let reach: std::collections::HashMap<u8, &str> = router.get_peer_reachability().into_iter().map(|(id, l, _)| (id, l)).collect();
        assert_eq!(reach[&3], "unreachable");
        assert_eq!(reach[&2], "reachable");
        router.stop();
    }

    /// Isolation: a destination that is slow but never fails (so no health signal ever reacts) is served
    /// from its OWN queue; deliveries to a healthy replica are not queued behind it.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pc_auto_043_slow_destination_does_not_delay_healthy_replica() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let n3 = Arc::new(VirtualNode::new(3));
        n3.set_artificial_delay(120); // answers every frame after 120 ms (< the 150 ms timeout): slow, not failed
        let addr3 = n3.start_tcp_service("127.0.0.1:0").await.unwrap();
        n3.set_addr(addr3);
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]).with_local_node_id(1));
        let mut keys = Vec::new();
        for i in 0..200 {
            let k = RfKey::new(None, &format!("/slowdest_{i}"), None);
            rf_put(&n2.cache, &k, b"stale-on-2", RfClock::now_ms(), 600_000);
            keys.push(k);
        }
        let t0 = std::time::Instant::now();
        for k in &keys {
            assert!(router.broadcaster().enqueue(RfTask {
                key: k.to_canonical_string(),
                scope: RfScope::ExactKey,
                replica_node_ids: vec![3, 2],
                membership_epoch: 1,
            }));
        }
        let purged = rf_wait_until(10_000, || keys.iter().all(|k| !rf_has(&n2.cache, k))).await;
        let took = t0.elapsed();
        assert!(purged);
        assert!(took < RfDuration::from_millis(1_500), "healthy replica purged in {took:?}; a slow peer must not delay it");
        // The slow peer is still being served (200 x 120 ms / 4 in flight = about 6 s): not failed, not unreachable.
        assert_eq!(router.get_peer_reachability().into_iter().find(|(id, _, _)| *id == 3).unwrap().1, "reachable");
        router.stop();
    }

    /// Bounded queue: a destination that never answers cannot make the process spawn unbounded tasks;
    /// overflow is counted as `dropped` per delivery (and the ring buffer keeps the record for repair).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_pc_auto_043_destination_queue_is_bounded_and_overflow_is_counted() {
        // A peer that accepts TCP and never answers: each attempt waits for the 150 ms read timeout.
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        tokio::spawn(async move {
            let mut held = Vec::new();
            while let Ok((s, _)) = l.accept().await {
                held.push(s);
            }
        });
        let n1 = rf_plain_node(1).await;
        let slow = Arc::new(VirtualNode::new(2));
        slow.set_addr(addr);
        slow.set_state(NodeState::Active);
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), slow.clone()]).with_local_node_id(1));
        // 10_000 deliveries in batches (the dispatcher moves each batch to the destination queue before
        // the next one), against a peer that drains only 4 deliveries per ~150 ms. The per-destination
        // queue holds 8192, so about 1800 deliveries must be counted as overflow, not silently buffered.
        for batch in 0..20 {
            for i in 0..500 {
                assert!(router.broadcaster().enqueue(RfTask {
                    key: format!("__default__:/slow_{batch}_{i}"),
                    scope: RfScope::ExactKey,
                    replica_node_ids: vec![2],
                    membership_epoch: 1,
                }));
            }
            tokio::time::sleep(RfDuration::from_millis(30)).await;
        }
        tokio::time::sleep(RfDuration::from_millis(300)).await;
        let c = router.get_broadcaster_counters();
        assert_eq!(c.enqueued, 10_000, "the task queue itself never overflowed");
        assert!(
            (1_500..=1_900).contains(&c.dropped),
            "destination queue overflow must be counted (expected about 10000 - 8192), got {c:?}"
        );
        router.stop();
    }

    // -------------------------------------------------------------------------------------
    // X-Cache-Source / outcome counters (D-016) for the cases an in-process gateway can produce
    // without an origin: peer fill, local hit. Origin, coalesced and stale are covered by the live
    // multi-process script (scripts/cluster_outcomes_check.mjs).
    // -------------------------------------------------------------------------------------
    #[tokio::test]
    async fn test_x_cache_source_counters_in_metrics() {
        use phantom_cache_demo::api::server::{build_schema_v2_payload, outcome_counters};
        let before = outcome_counters();
        let (peer0, local0) = (
            before.peer.load(std::sync::atomic::Ordering::Relaxed),
            before.local.load(std::sync::atomic::Ordering::Relaxed),
        );
        let p = rf_pair().await;
        rf_put_full(&p.cache1, &p.key, 200, "text/plain", None, b"x", RfClock::now_ms(), 60_000);
        let r1 = rf_http_get(p.http2, &p.path).await;
        let r2 = rf_http_get(p.http2, &p.path).await;
        assert_eq!(rf_header(&r1, "x-cache-source").as_deref(), Some("peer"));
        assert_eq!(rf_header(&r2, "x-cache-source").as_deref(), Some("local"));
        let after = outcome_counters();
        assert!(after.peer.load(std::sync::atomic::Ordering::Relaxed) > peer0);
        assert!(after.local.load(std::sync::atomic::Ordering::Relaxed) > local0);

        // Schema: every outcome value is labelled measured.
        let state = std::sync::Arc::new(phantom_cache_demo::api::server::ApiState {
            snapshot: std::sync::Mutex::new(phantom_cache_demo::api::server::SimulationSnapshot {
                version: "1.0".into(), timestamp_ms: 0, throughput_req_sec: 0.0, p50_latency_ms: 0, p95_latency_ms: 0,
                p99_latency_ms: 0, db_load_ratio: 0.0, avoided_compute_dollars: 0.0, nodes: vec![],
            }),
            nodes: vec![],
            workload_type: std::sync::Mutex::new("Normal".into()),
            cache: Some(Arc::new(RfCache::new(4))),
            origin_url: None,
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });
        let payload = build_schema_v2_payload(&state);
        let o = &payload["metrics"]["outcomes"];
        for k in ["local", "peer", "origin", "coalesced", "stale"] {
            assert_eq!(o["responses"][k]["src"], "measured", "{k}");
        }
        for k in ["miss", "failed", "refused_format"] {
            assert_eq!(o["peer_reads_without_hit"][k]["src"], "measured", "{k}");
        }
        p.router2.stop();
        p.node1.stop_tcp_service();
        p.node2.stop_tcp_service();
    }

    // =====================================================================================
    // BATCH B2: CORS expose (PC-AUTO-048), peer lifetime clamp (PC-AUTO-051), read-your-delete and
    // lagging replica (PC-AUTO-050), sync-leg pool wait (PC-AUTO-049).
    // =====================================================================================

    async fn rf_http(addr: std::net::SocketAddr, method: &str, path: &str, extra_headers: &str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let req = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n{extra_headers}Connection: close\r\n\r\n");
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.unwrap();
        String::from_utf8_lossy(&raw).into_owned()
    }

    /// PC-AUTO-048: a browser client must be able to read ETag, Last-Modified and X-Cache-Source.
    #[tokio::test]
    async fn test_pc_auto_048_cors_exposes_etag_last_modified_and_source() {
        use phantom_cache_demo::api::server::{create_app_with_cors, ApiState, CorsConfig, SimulationSnapshot};
        let state = Arc::new(ApiState {
            snapshot: std::sync::Mutex::new(SimulationSnapshot {
                version: "1.0".into(), timestamp_ms: 0, throughput_req_sec: 0.0, p50_latency_ms: 0, p95_latency_ms: 0,
                p99_latency_ms: 0, db_load_ratio: 0.0, avoided_compute_dollars: 0.0, nodes: vec![],
            }),
            nodes: vec![],
            workload_type: std::sync::Mutex::new("Normal".into()),
            cache: Some(Arc::new(RfCache::new(4))),
            origin_url: None,
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });
        let cfg = CorsConfig::with_origins(["http://localhost:3000"]).unwrap();
        let app = create_app_with_cors(state, &cfg);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let resp = rf_http(addr, "GET", "/health", "Origin: http://localhost:3000\r\n").await;
        let expose = rf_header(&resp, "access-control-expose-headers").expect("expose header").to_lowercase();
        for h in ["etag", "last-modified", "x-cache-source", "x-cache-status", "age"] {
            assert!(expose.contains(h), "Access-Control-Expose-Headers must list {h}: {expose}");
        }
    }

    /// PC-AUTO-051 / D-036: a long peer-supplied lifetime is clamped to 24 h, an absurd one is ignored.
    /// Fails without the clamp (the copy would live 10 days / the absurd entry would be stored).
    #[tokio::test]
    async fn test_pc_auto_051_peer_supplied_lifetime_is_clamped_and_absurd_is_ignored() {
        let p = rf_pair().await;
        let day = 86_400_000u64;
        let now = RfClock::now_ms();

        // 10-day source lifetime: stored, but the peer copy expires within 24 h.
        rf_put_full(&p.cache1, &p.key, 200, "text/plain", Some("\"long\""), b"long-lived", now, 10 * day);
        let resp = rf_http_get(p.http2, &p.path).await;
        assert_eq!(rf_header(&resp, "x-cache-source").as_deref(), Some("peer"), "{resp}");
        let (peer, _) = p.cache2.get_resource_with_metadata(&p.key, RfClock::now_ms()).expect("peer copy");
        let expiry = peer.created_at_ms + peer.ttl_ms;
        assert!(expiry <= now + day + 5_000, "peer copy lifetime must be clamped to 24 h, expiry is {} ms away", expiry - now);
        assert!(expiry >= now + day - 5_000, "clamp should be exactly 24 h, expiry is {} ms away", expiry as i64 - now as i64);

        // Absurd lifetime (400 days): the entry is ignored, nothing is stored on the asking gateway.
        let path2 = (0..400)
            .map(|i| format!("/api/absurd_{i}"))
            .find(|q| p.router2.route_key(&format!("__default__:{q}")).unwrap().node_id == 1)
            .unwrap();
        let key2 = RfKey::new(None, &path2, None);
        rf_put_full(&p.cache1, &key2, 200, "text/plain", None, b"absurd", now, 400 * day);
        let resp2 = rf_http_get(p.http2, &path2).await;
        assert_ne!(rf_header(&resp2, "x-cache-source").as_deref(), Some("peer"), "an absurd peer lifetime must not be used: {resp2}");
        assert!(p.cache2.get_resource(&key2, RfClock::now_ms()).is_none(), "nothing may be stored from an absurd peer entry");
        p.router2.stop();
        p.node1.stop_tcp_service();
        p.node2.stop_tcp_service();
    }

    /// PC-AUTO-050 (seed B, REAL): a GET that starts after this gateway purged the key but before the
    /// primary applied the invalidation peer-fills the entry that is being deleted. Once DELETE has
    /// returned, this gateway must not hold it (read-your-delete). Fails without the second purge.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_pc_auto_050_get_racing_a_delete_cannot_leave_a_stale_peer_fill() {
        let p = rf_pair().await;
        // Node 2's own delivery of the cluster fan-out (to itself) is slow, so only the second purge can help.
        p.node2.set_artificial_delay(500);
        rf_put_full(&p.cache1, &p.key, 200, "text/plain", Some("\"old\""), b"old body", RfClock::now_ms(), 60_000);

        // Two pooled connections to node 1 (the primary): the first checked out is SLOW (100 ms per frame),
        // the second is fast. The DELETE's synchronous leg takes the slow one, the racing peer read the fast one.
        p.node1.set_artificial_delay(100);
        let slow = RendezvousRouter::acquire_connection_to_node(p.router2.pool(), &rf_no_tls(), &p.node1, 1_000).await.unwrap();
        tokio::time::sleep(RfDuration::from_millis(150)).await;
        p.node1.set_artificial_delay(0);
        let fast = RendezvousRouter::acquire_connection_to_node(p.router2.pool(), &rf_no_tls(), &p.node1, 1_000).await.unwrap();
        tokio::time::sleep(RfDuration::from_millis(150)).await;
        p.router2.pool().checkin(fast);
        p.router2.pool().checkin(slow); // LIFO: the slow connection is handed out first

        let http = p.http2;
        let path = p.path.clone();
        let del = tokio::spawn(async move { rf_http(http, "DELETE", &format!("/api/v1/cache{path}"), "").await });
        tokio::time::sleep(RfDuration::from_millis(30)).await;
        let racing = rf_http_get(p.http2, &p.path).await;
        assert!(racing.contains("200 OK") && racing.contains("old body"), "the racing GET must have read the not-yet-invalidated primary: {racing}");
        assert_eq!(rf_header(&racing, "x-cache-source").as_deref(), Some("peer"), "{racing}");
        assert!(rf_has(&p.cache2, &p.key), "the racing GET stored the entry (this is the race)");

        let del_resp = del.await.unwrap();
        assert!(del_resp.starts_with("HTTP/1.1 204"), "{del_resp}");
        // Immediately after DELETE returned: no stale copy on this gateway.
        assert!(!rf_has(&p.cache2, &p.key), "read-your-delete: the stale peer fill must be gone once DELETE returned");
        let after = rf_http_get(p.http2, &p.path).await;
        assert_ne!(rf_header(&after, "x-cache-status").as_deref(), Some("HIT"), "{after}");
        p.router2.stop();
        p.node1.stop_tcp_service();
        p.node2.stop_tcp_service();
    }

    /// PC-AUTO-050 (seed A): when the primary cannot be reached, a lagging replica must NOT be used as a
    /// source for a peer fill. Fails with replica fallback (`max_retries` 1).
    #[tokio::test]
    async fn test_pc_auto_050_unreachable_primary_does_not_fall_back_to_a_lagging_replica() {
        use phantom_cache_demo::api::server::{create_app, ApiState, NodeSnapshot, SimulationSnapshot};
        let cache2 = Arc::new(RfCache::new(4));
        let node2 = Arc::new(VirtualNode::new_with_cache(2, cache2.clone()));
        let a2 = node2.start_tcp_service("127.0.0.1:0").await.unwrap();
        node2.set_addr(a2);
        let cache3 = Arc::new(RfCache::new(4));
        let node3 = Arc::new(VirtualNode::new_with_cache(3, cache3.clone()));
        let a3 = node3.start_tcp_service("127.0.0.1:0").await.unwrap();
        node3.set_addr(a3);
        let node1 = rf_dead_node(1); // the primary of the key: unreachable
        let router2 = Arc::new(RendezvousRouter::new(vec![node1.clone(), node2.clone(), node3.clone()]).with_local_node_id(2));
        let path = (0..2000)
            .map(|i| format!("/api/lag_{i}"))
            // primary = node 1 (dead) and the first-ranked replica = node 3, so a replica fallback WOULD pick node 3
            .find(|q| {
                let c = format!("__default__:{q}");
                let h = |id: u8| RendezvousRouter::calculate_hash(&c, id);
                h(1) > h(3) && h(3) > h(2)
            })
            .unwrap();
        assert_eq!(router2.route_key(&format!("__default__:{path}")).unwrap().node_id, 1);
        let key = RfKey::new(None, &path, None);
        // The replica still holds the entry the cluster has already deleted elsewhere.
        rf_put_full(&cache3, &key, 200, "text/plain", Some("\"lag\""), b"lagging copy", RfClock::now_ms(), 60_000);

        let state = Arc::new(ApiState {
            snapshot: std::sync::Mutex::new(SimulationSnapshot {
                version: "1.0".into(), timestamp_ms: 0, throughput_req_sec: 0.0, p50_latency_ms: 0, p95_latency_ms: 0,
                p99_latency_ms: 0, db_load_ratio: 0.0, avoided_compute_dollars: 0.0,
                nodes: vec![NodeSnapshot { node_id: 2, state: "Active".into(), cpu_utilization: 0.0, memory_used_bytes: 0 }],
            }),
            nodes: vec![node1.clone(), node2.clone(), node3.clone()],
            workload_type: std::sync::Mutex::new("Normal".into()),
            cache: Some(cache2.clone()),
            origin_url: None,
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: Some(router2.clone()),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http = listener.local_addr().unwrap();
        let app = create_app(state);
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let resp = rf_http_get(http, &path).await;
        assert!(!resp.contains("lagging copy"), "a lagging replica must not be used as a peer-fill source: {resp}");
        assert_ne!(rf_header(&resp, "x-cache-source").as_deref(), Some("peer"), "{resp}");
        assert!(cache2.get_resource(&key, RfClock::now_ms()).is_none());
        router2.stop();
        node2.stop_tcp_service();
        node3.stop_tcp_service();
    }

    /// PC-AUTO-049: the synchronous leg of a DELETE waits (up to its own budget) for a pool slot instead of
    /// answering Busy after the 150 ms I/O deadline. Slots are freed after 400 ms: the DELETE must succeed.
    #[tokio::test]
    async fn test_pc_auto_049_sync_leg_waits_for_a_pool_slot_instead_of_answering_busy() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]).with_local_node_id(1));
        let key = (0..500)
            .map(|i| RfKey::new(None, &format!("/syncwait_{i}"), None))
            .find(|k| router.route_key(&k.to_canonical_string()).unwrap().node_id == 2)
            .unwrap();
        rf_put(&n2.cache, &key, b"stale", RfClock::now_ms(), 600_000);
        let mut held = Vec::new();
        for _ in 0..8 {
            held.push(RendezvousRouter::acquire_connection_to_node(router.pool(), &rf_no_tls(), &n2, 500).await.unwrap());
        }
        let pool = router.pool().clone();
        let releaser = tokio::spawn(async move {
            tokio::time::sleep(RfDuration::from_millis(400)).await;
            for c in held {
                pool.checkin(c);
            }
        });
        let started = std::time::Instant::now();
        let out = router.invalidate_key_across_cluster(&key.to_canonical_string(), RfScope::ExactKey).await;
        releaser.await.unwrap();
        assert!(out.is_ok(), "the sync leg must outwait a 400 ms pool shortage, got {out:?} after {:?}", started.elapsed());
        assert!(!rf_has(&n2.cache, &key));
        router.stop();
    }

    /// PC-AUTO-049: when the pool is full because connects to a DEAD peer hang, waiters must stop waiting once
    /// the peer is known unreachable and let the caller fail over, instead of waiting the whole budget and
    /// answering 503. Fails without the early exit (the call would wait 1 s and return Busy).
    #[tokio::test]
    async fn test_pc_auto_049_waiters_stop_waiting_for_a_peer_that_is_known_unreachable() {
        let n1 = rf_plain_node(1).await;
        let n2 = rf_plain_node(2).await;
        let router = Arc::new(RendezvousRouter::new(vec![n1.clone(), n2.clone()]).with_local_node_id(1));
        let mut held = Vec::new();
        for _ in 0..8 {
            held.push(RendezvousRouter::acquire_connection_to_node(router.pool(), &rf_no_tls(), &n2, 500).await.unwrap());
        }
        // Slots full, peer healthy: the caller waits its budget and reports Busy.
        let started = std::time::Instant::now();
        let busy = RendezvousRouter::acquire_connection_waiting(router.pool(), &rf_no_tls(), &n2, 100, 300).await;
        assert!(matches!(busy, Err(RfDistError::Busy(2))), "{:?}", busy.err());
        assert!(started.elapsed() >= RfDuration::from_millis(250));
        // The peer proves unreachable: the waiter must not wait any more.
        for _ in 0..3 {
            n2.note_unreachable();
        }
        let started = std::time::Instant::now();
        let res = RendezvousRouter::acquire_connection_waiting(router.pool(), &rf_no_tls(), &n2, 100, 5_000).await;
        assert!(matches!(res, Err(RfDistError::NodeUnreachable(2))), "{:?}", res.err());
        assert!(started.elapsed() < RfDuration::from_millis(500), "must return at once, took {:?}", started.elapsed());
        // PC-AUTO-052: the early exit made no attempt, so it must not add to the failed-attempt streak.
        let streak = n2.unreachable_streak.load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(streak, 3, "the early NodeUnreachable must not be counted as a failed attempt");
        let _ = RendezvousRouter::acquire_connection_waiting(router.pool(), &rf_no_tls(), &n2, 100, 5_000).await;
        assert_eq!(n2.unreachable_streak.load(std::sync::atomic::Ordering::Relaxed), 3);
        for c in held {
            router.pool().checkin(c);
        }
        router.stop();
    }

    // =====================================================================================
    // Lane L4: static origin cap in every mode.
    // =====================================================================================

    /// Slow origin that records the maximum number of requests it served at the same time.
    async fn l4_slow_origin(delay_ms: u64) -> (std::net::SocketAddr, Arc<std::sync::atomic::AtomicUsize>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let in_flight = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let max_seen = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let m = max_seen.clone();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = listener.accept().await {
                let in_flight = in_flight.clone();
                let max_seen = max_seen.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let _ = sock.read(&mut buf).await;
                    let now = in_flight.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    max_seen.fetch_max(now, std::sync::atomic::Ordering::SeqCst);
                    tokio::time::sleep(RfDuration::from_millis(delay_ms)).await;
                    in_flight.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    let body = "origin-body";
                    let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: max-age=60\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });
        (addr, m)
    }

    /// The configured origin cap (`PHANTOM_ORIGIN_CAP_MAX`) must protect the origin in EVERY intelligence mode,
    /// here mode Off. REAL gateway process (the cap is process-global, so it must not run inside the test process):
    /// 12 concurrent requests for distinct cold keys against a slow origin must never put more than 3 requests in
    /// flight at the origin; the rest get 503 and are counted; permits are released afterwards. A control process
    /// without the cap shows the same burst reaching the origin in full, so the test fails without the change.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_l4_static_origin_cap_limits_origin_concurrency_in_every_mode() {
        async fn burst(cap: Option<u32>, mode: &str) -> (usize, usize, usize, serde_json::Value) {
            let (origin_addr, max_seen) = l4_slow_origin(300).await;
            let gw_port = {
                let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
                l.local_addr().unwrap().port()
            };
            let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_phantom-cache-demo"));
            cmd.env("ORIGIN_URL", format!("http://{origin_addr}"))
                .env("PHANTOM_BIND_ADDR", format!("127.0.0.1:{gw_port}"))
                .env("PHANTOM_INTELLIGENCE", mode)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true);
            if let Some(c) = cap {
                cmd.env("PHANTOM_ORIGIN_CAP_MAX", c.to_string());
            }
            let mut child = cmd.spawn().expect("spawn gateway");
            let client = reqwest::Client::builder().timeout(RfDuration::from_secs(10)).build().unwrap();
            let base = format!("http://127.0.0.1:{gw_port}");
            let mut up = false;
            for _ in 0..100 {
                if let Ok(r) = client.get(format!("{base}/health/live")).send().await {
                    if r.status().is_success() {
                        up = true;
                        break;
                    }
                }
                tokio::time::sleep(RfDuration::from_millis(100)).await;
            }
            assert!(up, "gateway process did not come up");
            let mut hs = Vec::new();
            for i in 0..12 {
                let c = client.clone();
                let u = format!("{base}/api/v1/cache/l4/cap_{i}");
                hs.push(tokio::spawn(async move { c.get(u).send().await.unwrap().status().as_u16() }));
            }
            let (mut ok, mut limited) = (0, 0);
            for h in hs {
                match h.await.unwrap() {
                    200 => ok += 1,
                    503 => limited += 1,
                    other => panic!("unexpected status {other}"),
                }
            }
            let metrics: serde_json::Value = client.get(format!("{base}/api/v1/metrics")).send().await.unwrap().json().await.unwrap();
            let _ = child.kill().await;
            (ok, limited, max_seen.load(std::sync::atomic::Ordering::SeqCst), metrics["metrics"]["origin_static_cap"].clone())
        }
        let (ok, limited, peak, cap_metrics) = burst(Some(3), "off").await;
        assert!(peak <= 3, "the origin saw {peak} concurrent requests with a cap of 3 in mode off");
        assert!(ok >= 3 && limited >= 1 && ok + limited == 12, "ok {ok}, limited {limited}");
        assert_eq!(cap_metrics["limit"]["v"], 3);
        assert_eq!(cap_metrics["rejections"]["v"], limited as u64, "every 503 is one counted cap rejection");
        assert_eq!(cap_metrics["in_flight"]["v"], 0, "all permits released");
        // Same in observe mode (no executor enforcement there either).
        let (_, limited_obs, peak_obs, _) = burst(Some(3), "observe").await;
        assert!(peak_obs <= 3 && limited_obs >= 1, "observe: peak {peak_obs}, limited {limited_obs}");
        // Control: no cap configured, the same burst reaches the origin in full.
        let (ok_free, limited_free, peak_free, _) = burst(None, "off").await;
        assert_eq!((ok_free, limited_free), (12, 0));
        assert!(peak_free > 3, "uncapped burst must exceed 3 in flight, saw {peak_free}");
    }

    /// Lane L4 / PIE design section 8: canary and active are rejected unless explicitly allowed.
    #[test]
    fn test_l4_canary_and_active_are_rejected_without_the_experimental_opt_in() {
        use phantom_cache_demo::config::{resolve, Inputs};
        let r = |pairs: &[(&str, &str)]| {
            resolve(&Inputs {
                env: pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
                ..Default::default()
            })
        };
        for mode in ["canary", "active"] {
            let e = r(&[("PHANTOM_INTELLIGENCE", mode)]).expect_err("must be rejected");
            let text = format!("{e:?}");
            assert!(text.contains("PHANTOM_INTELLIGENCE") && text.contains("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE"), "{text}");
            assert!(r(&[("PHANTOM_INTELLIGENCE", mode), ("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE", "false")]).is_err());
            assert!(r(&[("PHANTOM_INTELLIGENCE", mode), ("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE", "true")]).is_ok(), "explicit opt-in works");
        }
        for mode in ["off", "observe", "advisory"] {
            assert!(r(&[("PHANTOM_INTELLIGENCE", mode)]).is_ok(), "{mode} stays valid");
        }
        // The opt-in alone changes nothing.
        assert!(r(&[("PHANTOM_ALLOW_EXPERIMENTAL_INTELLIGENCE", "true")]).is_ok());
    }

    /// Lane L4 item 3: the documented access control and tenant isolation (docs/user/security.md) against a REAL
    /// secure gateway PROCESS (the in-process equivalents are in m7_sec_fixes_tests and security_phase1_tests).
    /// Origin echoes the tenant header it receives, so a cross-tenant leak would be visible in the body.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_l4_secure_gateway_process_401_403_and_tenant_isolation() {
        use phantom_cache_demo::security::{JwtClaims, JwtValidator};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        const ISS: &str = "phantom-cache-auth";
        const AUD: &str = "phantom-cache-api";
        const SECRET: &[u8] = b"l4-process-test-secret-0123456789abcdef-not-a-real-secret";
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let tok = |roles: Vec<&str>, tenant: &str| {
            JwtValidator::new(ISS, AUD, SECRET)
                .create_token(&JwtClaims::new("u", ISS, AUD, now + 3600, Some(tenant), roles))
                .unwrap()
        };

        // Origin: replies with the x-tenant-id it was sent and counts requests.
        let origin = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin_port = origin.local_addr().unwrap().port();
        let origin_hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let hits = origin_hits.clone();
        tokio::spawn(async move {
            while let Ok((mut sock, _)) = origin.accept().await {
                let hits = hits.clone();
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 8192];
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]).to_lowercase();
                    hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let tenant = req.lines().find_map(|l| l.strip_prefix("x-tenant-id:")).map(|v| v.trim().to_string()).unwrap_or_default();
                    let body = format!("tenant={tenant}");
                    let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nCache-Control: public, max-age=60\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                    let _ = sock.write_all(resp.as_bytes()).await;
                });
            }
        });

        let gw_port = {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            l.local_addr().unwrap().port()
        };
        let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_phantom-cache-demo"))
            .env("ORIGIN_URL", format!("http://127.0.0.1:{origin_port}"))
            .env("PHANTOM_BIND_ADDR", format!("127.0.0.1:{gw_port}"))
            .env("PHANTOM_JWT_SECRET", std::str::from_utf8(SECRET).unwrap())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn gateway");
        let client = reqwest::Client::builder().timeout(RfDuration::from_secs(5)).build().unwrap();
        let base = format!("http://127.0.0.1:{gw_port}");
        let mut up = false;
        for _ in 0..100 {
            if let Ok(r) = client.get(format!("{base}/health/live")).send().await {
                if r.status().is_success() {
                    up = true;
                    break;
                }
            }
            tokio::time::sleep(RfDuration::from_millis(100)).await;
        }
        assert!(up, "secure gateway process did not come up");

        let url = format!("{base}/api/v1/cache/l4sec/item");
        // 401: no token, garbage token, wrong signature; with a Bearer challenge and no origin contact.
        let r = client.get(&url).send().await.unwrap();
        assert_eq!(r.status().as_u16(), 401);
        assert!(r.headers().get("www-authenticate").is_some());
        let r = client.get(&url).bearer_auth("not.a.jwt").send().await.unwrap();
        assert_eq!(r.status().as_u16(), 401);
        let bad = JwtValidator::new(ISS, AUD, b"a-different-secret-0123456789abcdefghij")
            .create_token(&JwtClaims::new("u", ISS, AUD, now + 3600, Some("a"), vec!["cache:read"]))
            .unwrap();
        assert_eq!(client.get(&url).bearer_auth(bad).send().await.unwrap().status().as_u16(), 401);
        assert_eq!(origin_hits.load(std::sync::atomic::Ordering::SeqCst), 0, "unauthenticated requests never reach the origin");
        // 403: valid token without the role. Reader cannot invalidate; invalidator cannot read.
        let reader_a = tok(vec!["cache:read"], "tenant_a");
        let invalidator_a = tok(vec!["cache:invalidate"], "tenant_a");
        assert_eq!(client.get(&url).bearer_auth(&invalidator_a).send().await.unwrap().status().as_u16(), 403);
        assert_eq!(client.delete(&url).bearer_auth(&reader_a).send().await.unwrap().status().as_u16(), 403);
        assert_eq!(origin_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
        // Public health stays public.
        assert!(client.get(format!("{base}/health/live")).send().await.unwrap().status().is_success());

        // Tenant isolation: the same path under two tenants is two cache entries; bodies show which tenant fetched.
        let reader_b = tok(vec!["cache:read"], "tenant_b");
        let a1 = client.get(&url).bearer_auth(&reader_a).send().await.unwrap();
        assert_eq!(a1.status().as_u16(), 200);
        assert_eq!(a1.headers().get("x-cache-status").unwrap(), "MISS");
        assert_eq!(a1.text().await.unwrap(), "tenant=tenant_a");
        let b1 = client.get(&url).bearer_auth(&reader_b).send().await.unwrap();
        assert_eq!(b1.headers().get("x-cache-status").unwrap(), "MISS", "tenant B must not be served tenant A's entry");
        assert_eq!(b1.text().await.unwrap(), "tenant=tenant_b");
        let a2 = client.get(&url).bearer_auth(&reader_a).send().await.unwrap();
        assert_eq!(a2.headers().get("x-cache-status").unwrap(), "HIT");
        assert_eq!(a2.text().await.unwrap(), "tenant=tenant_a");
        // Tenant A's DELETE purges A only.
        assert_eq!(client.delete(&url).bearer_auth(&invalidator_a).send().await.unwrap().status().as_u16(), 204);
        let a3 = client.get(&url).bearer_auth(&reader_a).send().await.unwrap();
        assert_eq!(a3.headers().get("x-cache-status").unwrap(), "MISS", "A's entry was purged");
        let b2 = client.get(&url).bearer_auth(&reader_b).send().await.unwrap();
        assert_eq!(b2.headers().get("x-cache-status").unwrap(), "HIT", "B's entry survives A's DELETE");
        let _ = child.kill().await;
    }

    /// Lane L4 item 4: PIE v2 wired into the gateway per docs/lanes/CONTRACT-pie-api-v1.md. In-process gateway with
    /// PIE v2 installed in Advisory mode (the only modes that exist are Off, Observe, Advisory); requests feed it
    /// through the non-blocking sink, `tick` is driven by the test with a synthetic clock.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_l4_pie_v2_routes_and_event_wiring() {
        use phantom_cache_demo::api::server::{create_app, install_pie_v2, ApiState, SimulationSnapshot};
        use phantom_cache_demo::intelligence_v2::{Mode, Pie, PieConfig};
        let pie = Arc::new(Pie::new(PieConfig { mode: Mode::Advisory, ..Default::default() }).unwrap());
        install_pie_v2(pie.clone());
        let (origin_addr, _max) = l4_slow_origin(1).await;
        let state = Arc::new(ApiState {
            snapshot: std::sync::Mutex::new(SimulationSnapshot {
                version: "1.0".into(), timestamp_ms: 0, throughput_req_sec: 0.0, p50_latency_ms: 0, p95_latency_ms: 0,
                p99_latency_ms: 0, db_load_ratio: 0.0, avoided_compute_dollars: 0.0, nodes: vec![],
            }),
            nodes: vec![],
            workload_type: std::sync::Mutex::new("Normal".into()),
            cache: Some(Arc::new(RfCache::new(4))),
            origin_url: Some(format!("http://{origin_addr}")),
            telemetry: None,
            pie: None,
            security: None,
            upstream_policy: None,
            router: None,
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let http = listener.local_addr().unwrap();
        let app = create_app(state);
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let get = |path: &str| {
            let p = path.to_string();
            async move {
                let r = rf_http(http, "GET", &p, "").await;
                let status: u16 = r[9..12].parse().unwrap();
                let body = r.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
                (status, serde_json::from_str::<serde_json::Value>(&body).unwrap_or(serde_json::Value::Null), r)
            }
        };
        // 600 requests over 20 keys: 20 misses and 580 hits; every response stays a normal 200.
        for round in 0..30 {
            for k in 0..20 {
                let r = rf_http_get(http, &format!("/pie/k{k}")).await;
                assert!(r.starts_with("HTTP/1.1 200"), "round {round} key {k}: {}", &r[..r.len().min(60)]);
            }
        }
        pie.tick(1000);
        pie.tick(2000);
        let (st, sig, _) = get("/api/v1/intelligence/signals").await;
        assert_eq!(st, 200);
        assert!(sig["window"].is_object());
        assert!(sig["history"].as_array().unwrap().iter().any(|w| w["events"].as_u64().unwrap_or(0) > 0), "a closed window saw the requests: {}", sig["history"]);
        for t in 3..=12u64 {
            pie.tick(t * 1000);
        }
        let (st, reg, _) = get("/api/v1/intelligence/registry").await;
        assert_eq!(st, 200);
        assert_eq!(reg["schema"], "pie-api-v1");
        assert_eq!(reg["installed"], true);
        assert!(reg["policies"].as_array().unwrap().len() >= 5);
        assert!(reg["policies"].as_array().unwrap().iter().all(|p| p["max_runtime_state"] != "canary" && p["max_runtime_state"] != "active"));
        let (st, status, _) = get("/api/v1/intelligence/status").await;
        assert_eq!(st, 200);
        assert_eq!(status["intelligence_mode"], "Advisory");
        assert_eq!(status["v2"]["mode"]["value"], "advisory");
        assert_eq!(status["v2"]["claims"]["benefit_claim"], "none");
        assert!(status["v2"]["queue"]["enqueued_total"]["value"].as_u64().unwrap() >= 600, "events reached PIE: {}", status["v2"]["queue"]);
        assert_eq!(status["v2"]["queue"]["dropped_total"]["value"], 0);
        assert!(status["revert_count"].is_number(), "existing status fields are kept");
        let (st, dec, _) = get("/api/v1/intelligence/decisions?limit=5").await;
        assert_eq!(st, 200);
        assert_eq!(dec["ordering"], "oldest_first");
        let (st, bad, _) = get("/api/v1/intelligence/decisions?limit=0").await;
        assert_eq!(st, 400);
        assert_eq!(bad["error"], "bad_request");
        let (st, pol, _) = get("/api/v1/intelligence/policy/POL-CLASS-1").await;
        assert_eq!(st, 200);
        assert_eq!(pol["policy"]["policy_id"], "POL-CLASS-1");
        let (st, nope, _) = get("/api/v1/intelligence/policy/NOPE").await;
        assert_eq!(st, 404);
        assert_eq!(nope["error"], "unknown_policy");
        let (st, nod, _) = get("/api/v1/intelligence/decisions/999999").await;
        assert_eq!(st, 404);
        assert_eq!(nod["error"], "unknown_decision");
        // Nothing applied: no key material in any body.
        let (_, whole, raw) = get("/api/v1/intelligence/signals").await;
        assert!(!raw.contains("/pie/k"), "no request key in PIE output");
        let _ = whole;
    }
}

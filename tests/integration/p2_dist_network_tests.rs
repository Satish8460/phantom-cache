use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::dist::router::{DistError, RendezvousRouter};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_p2_multi_node_tcp_communication() {
    let n1 = Arc::new(VirtualNode::new(1));
    let n2 = Arc::new(VirtualNode::new(2));
    let n3 = Arc::new(VirtualNode::new(3));

    // Bind real TCP services on ephemeral ports
    n1.start_tcp_service("127.0.0.1:0").await.unwrap();
    n2.start_tcp_service("127.0.0.1:0").await.unwrap();
    n3.start_tcp_service("127.0.0.1:0").await.unwrap();

    let router = RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]);

    let key = "hotel_del_luna";
    let (node_id, resp1) = router
        .forward_key_over_tcp(key, 1000, 2)
        .await
        .expect("TCP forwarding to primary node failed");

    assert!(
        resp1.starts_with("MISS val_node_"),
        "First request must be cold miss: {}",
        resp1
    );

    // Second request to the same key must hit the cache on that specific node
    let (node_id_2, resp2) = router
        .forward_key_over_tcp(key, 1000, 2)
        .await
        .expect("Second TCP request failed");

    assert_eq!(node_id, node_id_2, "Key must route to the same primary node");
    assert!(
        resp2.starts_with("HIT val_node_"),
        "Second request must be a cache HIT: {}",
        resp2
    );
}

#[tokio::test]
async fn test_p2_actual_tcp_connection_refused_socket_failover() {
    let n1 = Arc::new(VirtualNode::new(1));
    let n2 = Arc::new(VirtualNode::new(2));

    n1.start_tcp_service("127.0.0.1:0").await.unwrap();
    n2.start_tcp_service("127.0.0.1:0").await.unwrap();

    let router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);
    let test_key = "socket_disconnect_test_key";

    let primary = router.route_key(test_key).unwrap();
    let primary_id = primary.node_id;
    let expected_replica_id = if primary_id == 1 { 2 } else { 1 };

    // ABRUPT PROCESS/SOCKET CRASH: stop the TCP listener, dropping the socket
    // Notice: we DO NOT call primary.set_state(NodeState::Failed).
    // In-memory router still sees primary as Active, forcing a REAL TCP connection attempt!
    assert_eq!(primary.get_state(), NodeState::Active);
    primary.stop_tcp_service();

    // Give OS brief moment to release listening socket
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Router must attempt TCP connect to primary, receive actual OS ConnectionRefused,
    // mark primary as Suspect, and immediately failover to replica!
    let (served_id, resp) = router
        .forward_key_over_tcp(test_key, 500, 1)
        .await
        .expect("Failover after actual socket disconnect failed");

    assert_eq!(
        served_id, expected_replica_id,
        "Must failover to secondary replica upon actual TCP socket connection refused"
    );
    assert!(
        resp.starts_with("MISS val_node_") || resp.starts_with("HIT val_node_"),
        "Replica must return usable cache data: {}",
        resp
    );
    assert_eq!(
        primary.get_state(),
        NodeState::Suspect,
        "Primary must be marked Suspect after socket connection failure"
    );
}

#[tokio::test]
async fn test_p2_tcp_socket_timeout_failover() {
    let n1 = Arc::new(VirtualNode::new(1));
    let n2 = Arc::new(VirtualNode::new(2));

    n1.start_tcp_service("127.0.0.1:0").await.unwrap();
    n2.start_tcp_service("127.0.0.1:0").await.unwrap();

    let router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);
    let test_key = "timeout_test_key";

    let primary = router.route_key(test_key).unwrap();
    let primary_id = primary.node_id;
    let expected_replica_id = if primary_id == 1 { 2 } else { 1 };

    // Inject 300ms artificial network delay into primary
    primary.set_artificial_delay(300);

    // Call with aggressive timeout of 60ms
    // Router must time out on primary, detect network delay, and failover to replica (0ms delay)
    let (served_id, resp) = router
        .forward_key_over_tcp(test_key, 60, 1)
        .await
        .expect("Failover on timeout failed");

    assert_eq!(
        served_id, expected_replica_id,
        "Must failover to replica when primary exceeds network timeout"
    );
    assert!(resp.contains("val_node_"), "Replica served valid response: {}", resp);
}

#[tokio::test]
async fn test_p2_replica_data_usability_and_consistency() {
    let n1 = Arc::new(VirtualNode::new(1));
    let n2 = Arc::new(VirtualNode::new(2));

    n1.start_tcp_service("127.0.0.1:0").await.unwrap();
    n2.start_tcp_service("127.0.0.1:0").await.unwrap();

    let router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);
    let key = "consistent_product_data";

    // Primary serves cold miss
    let (p_id, p_resp) = router.forward_key_over_tcp(key, 1000, 1).await.unwrap();
    assert!(p_resp.starts_with("MISS val_node_"));

    // Primary now has cached data
    let (_, p_hit) = router.forward_key_over_tcp(key, 1000, 1).await.unwrap();
    assert!(p_hit.starts_with("HIT val_node_"));

    // Crash primary
    let primary = if p_id == 1 { n1.clone() } else { n2.clone() };
    primary.stop_tcp_service();
    tokio::time::sleep(Duration::from_millis(20)).await;

    // Failover to replica: replica admits cold miss, fetches origin, caches, and returns valid data
    let (r_id, r_resp) = router.forward_key_over_tcp(key, 500, 1).await.unwrap();
    assert_ne!(r_id, p_id);
    assert!(r_resp.starts_with("MISS val_node_"));

    // Subsequent request to replica hits replica's cache
    let (r_id_2, r_hit) = router.forward_key_over_tcp(key, 500, 1).await.unwrap();
    assert_eq!(r_id_2, r_id);
    assert!(r_hit.starts_with("HIT val_node_"));
}

#[tokio::test]
async fn test_p2_node_failure_and_replica_failover() {
    let n1 = Arc::new(VirtualNode::new(1));
    let n2 = Arc::new(VirtualNode::new(2));
    let n3 = Arc::new(VirtualNode::new(3));

    n1.start_tcp_service("127.0.0.1:0").await.unwrap();
    n2.start_tcp_service("127.0.0.1:0").await.unwrap();
    n3.start_tcp_service("127.0.0.1:0").await.unwrap();

    let router = RendezvousRouter::new(vec![n1.clone(), n2.clone(), n3.clone()]);
    let test_key = "room_inventory_999";

    let primary = router.route_key(test_key).unwrap();
    let primary_id = primary.node_id;

    primary.set_state(NodeState::Failed);

    let (repl_id, resp) = router
        .forward_key_over_tcp(test_key, 1000, 2)
        .await
        .expect("Failover to replica node failed");

    assert_ne!(
        repl_id, primary_id,
        "Request must failover away from failed primary node"
    );
    assert!(
        resp.starts_with("MISS val_node_") || resp.starts_with("HIT val_node_"),
        "Replica must return valid response: {}",
        resp
    );
}

#[tokio::test]
async fn test_p2_partition_and_recovery() {
    let n1 = Arc::new(VirtualNode::new(1));
    let n2 = Arc::new(VirtualNode::new(2));

    n1.start_tcp_service("127.0.0.1:0").await.unwrap();
    n2.start_tcp_service("127.0.0.1:0").await.unwrap();

    let router = RendezvousRouter::new(vec![n1.clone(), n2.clone()]);
    let key = "partition_test_key";

    // 1. Initial healthy query
    let (initial_id, _) = router.forward_key_over_tcp(key, 1000, 1).await.unwrap();

    // 2. Partition / Offline the active node
    let active_node = if initial_id == 1 { n1.clone() } else { n2.clone() };
    active_node.set_state(NodeState::Offline);

    // Failover to survivor
    let (survivor_id, _) = router.forward_key_over_tcp(key, 1000, 1).await.unwrap();
    assert_ne!(survivor_id, initial_id, "Must failover during partition");

    // 3. Recover node
    active_node.set_state(NodeState::Active);

    // Key routes back to its primary owner
    let (recovered_id, _) = router.forward_key_over_tcp(key, 1000, 1).await.unwrap();
    assert_eq!(
        recovered_id, initial_id,
        "Recovered primary must resume ownership"
    );
}

#[tokio::test]
async fn test_p2_all_nodes_unreachable_returns_error() {
    let n1 = Arc::new(VirtualNode::new(1));
    let router = RendezvousRouter::new(vec![n1.clone()]);

    n1.set_state(NodeState::Failed);

    let res = router.forward_key_over_tcp("test", 100, 1).await;
    assert_eq!(res, Err(DistError::NoActiveNodes));
}

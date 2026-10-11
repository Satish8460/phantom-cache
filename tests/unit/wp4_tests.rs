#[cfg(test)]
mod tests {
    use phantom_cache_demo::api::server::{ApiState, NodeSnapshot, SimulationSnapshot};
    use phantom_cache_demo::dist::node::VirtualNode;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_sse_state_projection() {
        let clock_now = 12345u64;
        let snap = SimulationSnapshot {
            version: "1.0".to_string(),
            timestamp_ms: clock_now,
            throughput_req_sec: 15000.0,
            p50_latency_ms: 10,
            p95_latency_ms: 12,
            p99_latency_ms: 12,
            db_load_ratio: 0.15,
            avoided_compute_dollars: 42.50,
            nodes: vec![NodeSnapshot {
                node_id: 1,
                state: "Active".to_string(),
                cpu_utilization: 22.4,
                memory_used_bytes: 1048576,
            }],
        };

        let nodes = vec![Arc::new(VirtualNode::new(1))];

        let api_state = Arc::new(ApiState {
            snapshot: Mutex::new(snap),
            nodes,
            workload_type: Mutex::new("Normal".to_string()),
        });

        // Verify values read from the shared Mutex
        let current_snap = api_state.snapshot.lock().unwrap().clone();
        assert_eq!(current_snap.throughput_req_sec, 15000.0);
        assert_eq!(current_snap.p99_latency_ms, 12);
        assert_eq!(current_snap.nodes[0].state, "Active");

        // Mutate state (e.g. simulation workload shift)
        {
            let mut guard = api_state.snapshot.lock().unwrap();
            guard.throughput_req_sec = 50000.0;
            guard.p99_latency_ms = 180;
            guard.nodes[0].state = "Degraded".to_string();
        }

        // Verify SSE data projection has picked up changes
        let updated_snap = api_state.snapshot.lock().unwrap().clone();
        assert_eq!(updated_snap.throughput_req_sec, 50000.0);
        assert_eq!(updated_snap.p99_latency_ms, 180);
        assert_eq!(updated_snap.nodes[0].state, "Degraded");
    }

    use phantom_cache_demo::api::server::create_app;
    use phantom_cache_demo::dist::node::NodeState;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn make_test_api_state() -> (Arc<ApiState>, Vec<Arc<VirtualNode>>) {
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
                NodeSnapshot {
                    node_id: 3,
                    state: "Active".to_string(),
                    cpu_utilization: 25.0,
                    memory_used_bytes: 1024,
                },
            ],
        };
        let api_state = Arc::new(ApiState {
            snapshot: Mutex::new(snap),
            nodes: nodes.clone(),
            workload_type: Mutex::new("Normal".to_string()),
        });
        (api_state, nodes)
    }

    async fn spawn_test_server(api_state: Arc<ApiState>) -> std::net::SocketAddr {
        let app = create_app(api_state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        addr
    }

    async fn send_http_request(addr: std::net::SocketAddr, req: &str) -> String {
        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut resp = String::new();
        stream.read_to_string(&mut resp).await.unwrap();
        resp
    }

    #[tokio::test]
    async fn test_api_health_check_live_endpoint() {
        let (api_state, _) = make_test_api_state();
        let addr = spawn_test_server(api_state).await;

        let req = "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        let resp = send_http_request(addr, req).await;

        assert!(
            resp.starts_with("HTTP/1.1 200 OK"),
            "Unexpected status: {}",
            resp
        );
        assert!(resp.contains("OK"), "Response body missing 'OK'");
    }

    #[tokio::test]
    async fn test_api_control_flash_crowd_mutation() {
        let (api_state, _) = make_test_api_state();
        let addr = spawn_test_server(api_state.clone()).await;

        let body = r#"{"event":"FLASH_CROWD"}"#;
        let req = format!(
            "POST /api/control HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let resp = send_http_request(addr, &req).await;

        assert!(
            resp.starts_with("HTTP/1.1 200 OK"),
            "Unexpected status: {}",
            resp
        );
        assert!(resp.contains(r#""status":"success""#));
        assert!(resp.contains(r#""triggered":"FLASH_CROWD""#));

        let current_workload = api_state.workload_type.lock().unwrap().clone();
        assert_eq!(current_workload, "FlashCrowd");
    }

    #[tokio::test]
    async fn test_api_control_node_failure_mutation() {
        let (api_state, nodes) = make_test_api_state();
        let addr = spawn_test_server(api_state).await;

        let body = r#"{"event":"NODE_FAILURE"}"#;
        let req = format!(
            "POST /api/control HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let resp = send_http_request(addr, &req).await;

        assert!(
            resp.starts_with("HTTP/1.1 200 OK"),
            "Unexpected status: {}",
            resp
        );
        assert!(resp.contains(r#""status":"success""#));
        assert!(resp.contains(r#""triggered":"NODE_FAILURE""#));

        let node2 = nodes.iter().find(|n| n.node_id == 2).unwrap();
        assert_eq!(node2.get_state(), NodeState::Failed);
    }

    #[tokio::test]
    async fn test_api_404_not_found() {
        let (api_state, _) = make_test_api_state();
        let addr = spawn_test_server(api_state).await;

        let req =
            "GET /api/nonexistent_route HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
        let resp = send_http_request(addr, req).await;

        assert!(
            resp.starts_with("HTTP/1.1 404 Not Found"),
            "Expected 404, got: {}",
            resp
        );
    }
}

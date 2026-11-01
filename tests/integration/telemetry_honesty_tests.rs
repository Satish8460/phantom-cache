//! D-0 backend honesty: BD-01, BD-02, BD-03, S-7 (schema-1 events), computed `truthful`.
//!
//! Rules under test (docs/research/telemetry_schema_v2.md):
//!  - an absent source is `{src:"unavailable", reason}` with no `v` (never a measured 0 / 1.0)
//!  - in live mode no `src` or `*_src` key anywhere may be `simulated`
//!  - schema-1 `/api/events` keeps its field names; synthetic fields are null/[] in live mode and
//!    described by the additive `v1_provenance` object
//!
//! PHANTOM_SIMULATION is process-global, so every test takes ENV_LOCK.

use phantom_cache_demo::api::server::{build_schema_v1_payload, build_schema_v2_payload, create_app, ApiState, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::dist::router::RendezvousRouter;
use phantom_cache_demo::intel::diagnostics::{CacheCounters, DiagnosticBuffer};
use phantom_cache_demo::intel::telemetry::TelemetryCollector;
use serde_json::Value;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn snapshot() -> SimulationSnapshot {
    SimulationSnapshot {
        version: "1.0".to_string(),
        timestamp_ms: 1,
        throughput_req_sec: 12000.0,
        p50_latency_ms: 10,
        p95_latency_ms: 12,
        p99_latency_ms: 15,
        db_load_ratio: 0.2,
        avoided_compute_dollars: 0.0,
        nodes: vec![],
    }
}

fn placeholder_nodes() -> Vec<Arc<VirtualNode>> {
    let nodes = vec![Arc::new(VirtualNode::new(1)), Arc::new(VirtualNode::new(2)), Arc::new(VirtualNode::new(3))];
    for n in &nodes {
        n.set_state(NodeState::Active);
    }
    nodes
}

fn make_state(with_cache: bool, with_telemetry: bool, router: bool) -> (Arc<ApiState>, Option<Arc<PartitionedCache>>) {
    let cache = if with_cache {
        let counters = Arc::new(CacheCounters::new());
        let diagnostics = Arc::new(DiagnosticBuffer::new(64, counters.clone()));
        Some(Arc::new(PartitionedCache::new_with_diagnostics(2, counters, diagnostics)))
    } else {
        None
    };
    let nodes = placeholder_nodes();
    let router = if router { Some(Arc::new(RendezvousRouter::new(nodes.clone()))) } else { None };
    let state = Arc::new(ApiState {
        snapshot: Mutex::new(snapshot()),
        nodes,
        workload_type: Mutex::new("static".to_string()),
        cache: cache.clone(),
        origin_url: None,
        telemetry: if with_telemetry { Some(Arc::new(TelemetryCollector::new(64))) } else { None },
        pie: None,
        security: None,
        upstream_policy: None,
        router,
    });
    (state, cache)
}

fn with_sim<T>(on: bool, f: impl FnOnce() -> T) -> T {
    let prev = std::env::var("PHANTOM_SIMULATION").ok();
    if on {
        std::env::set_var("PHANTOM_SIMULATION", "true");
    } else {
        std::env::remove_var("PHANTOM_SIMULATION");
    }
    let out = f();
    match prev {
        Some(v) => std::env::set_var("PHANTOM_SIMULATION", v),
        None => std::env::remove_var("PHANTOM_SIMULATION"),
    }
    out
}

/// Collects every object member whose key is `src` or ends in `_src` and whose value is "simulated".
fn simulated_paths(v: &Value, path: &str, out: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            for (k, val) in map {
                let p = format!("{}.{}", path, k);
                if (k == "src" || k.ends_with("_src")) && val == "simulated" {
                    out.push(p.clone());
                }
                simulated_paths(val, &p, out);
            }
        }
        Value::Array(a) => {
            for (i, val) in a.iter().enumerate() {
                simulated_paths(val, &format!("{}[{}]", path, i), out);
            }
        }
        _ => {}
    }
}

fn assert_unavailable(v: &Value, reason: &str, what: &str) {
    assert_eq!(v["src"], "unavailable", "{}: {}", what, v);
    assert_eq!(v["reason"], reason, "{}: {}", what, v);
    assert!(v.get("v").is_none(), "{}: unavailable must not carry v: {}", what, v);
}

#[test]
fn walker_catches_state_src_and_nested_simulated_keys() {
    let sample = serde_json::json!({
        "nodes": [{"state_src": "simulated"}],
        "a": {"src": "simulated"},
        "b": {"deep": {"x_src": "simulated"}},
        "c": {"src": "measured", "note": "simulated"}
    });
    let mut found = Vec::new();
    simulated_paths(&sample, "root", &mut found);
    assert_eq!(found.len(), 3, "{:?}", found);
    assert!(found.iter().any(|p| p.ends_with("state_src")));
}

// BD-01
#[test]
fn bd01_absent_cache_counters_are_unavailable_not_measured_zero() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (state, _) = make_state(false, false, false);
        let p = build_schema_v2_payload(&state);
        let m = &p["metrics"];
        for (name, v) in [
            ("stale_served_total", &m["stale_served_total"]),
            ("stale_cap_violations", &m["stale_cap_violations"]),
            ("revalidations.started", &m["revalidations"]["started"]),
            ("revalidations.ok", &m["revalidations"]["ok"]),
            ("revalidations.not_modified", &m["revalidations"]["not_modified"]),
            ("revalidations.error", &m["revalidations"]["error"]),
            ("revalidations.in_flight", &m["revalidations"]["in_flight"]),
            ("requests_total", &m["requests_total"]),
            ("cache_hits", &m["cache_hits"]),
            ("cache_misses", &m["cache_misses"]),
            ("coalesced_waiters", &m["coalesced_waiters"]),
            ("evictions_total", &m["evictions_total"]),
            ("capacity_rejections", &m["capacity_rejections"]),
        ] {
            assert_unavailable(v, "cache_unavailable", name);
            assert_eq!(v["unit"], "count", "{}", name);
        }
    });
}

#[test]
fn bd01_present_cache_counters_stay_measured() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (state, cache) = make_state(true, true, false);
        cache.unwrap().counters.stale_served_total.fetch_add(3, Ordering::Relaxed);
        let p = build_schema_v2_payload(&state);
        assert_eq!(p["metrics"]["stale_served_total"]["src"], "measured");
        assert_eq!(p["metrics"]["stale_served_total"]["v"], 3);
        assert_eq!(p["metrics"]["stale_cap_violations"]["v"], 0);
        assert_eq!(p["metrics"]["revalidations"]["started"]["src"], "measured");
    });
}

// BD-02
#[test]
fn bd02_hit_ratio_cumulative_unavailable_until_traffic() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (state, cache) = make_state(true, true, false);
        let cache = cache.unwrap();
        let p = build_schema_v2_payload(&state);
        assert_unavailable(&p["metrics"]["hit_ratio_cumulative"], "no_requests", "no traffic");

        cache.counters.requests_total.fetch_add(4, Ordering::Relaxed);
        cache.counters.cache_hits.fetch_add(3, Ordering::Relaxed);
        cache.counters.cache_misses.fetch_add(1, Ordering::Relaxed);
        let p = build_schema_v2_payload(&state);
        let hr = &p["metrics"]["hit_ratio_cumulative"];
        assert_eq!(hr["src"], "derived");
        assert_eq!(hr["v"], 0.75);

        let (no_cache, _) = make_state(false, true, false);
        let p = build_schema_v2_payload(&no_cache);
        assert_unavailable(&p["metrics"]["hit_ratio_cumulative"], "cache_unavailable", "no cache");
    });
}

#[test]
fn bd02_bypass_only_traffic_is_not_a_hit_ratio() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (state, cache) = make_state(true, true, false);
        cache.unwrap().counters.requests_total.fetch_add(2, Ordering::Relaxed);
        let p = build_schema_v2_payload(&state);
        assert_unavailable(&p["metrics"]["hit_ratio_cumulative"], "no_cache_lookups", "requests but no lookups");
    });
}

#[test]
fn bd02_uptime_unavailable_without_telemetry_numeric_with() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (no_tel, _) = make_state(true, false, false);
        let p = build_schema_v2_payload(&no_tel);
        assert_unavailable(&p["uptime_ms"], "telemetry_disabled", "uptime without telemetry");

        let (with_tel, _) = make_state(true, true, false);
        let p = build_schema_v2_payload(&with_tel);
        assert!(p["uptime_ms"].is_number(), "{}", p["uptime_ms"]);
    });
}

// BD-03
#[test]
fn bd03_live_standalone_gateway_reports_no_nodes_and_no_simulated_keys() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        // main.rs builds three placeholder VirtualNodes even for a standalone gateway.
        let (state, _) = make_state(true, true, false);
        assert_eq!(state.nodes.len(), 3);
        let p = build_schema_v2_payload(&state);
        assert_eq!(p["mode"], "live");
        assert_eq!(p["nodes"], serde_json::json!([]));
        let mut found = Vec::new();
        simulated_paths(&p, "root", &mut found);
        assert!(found.is_empty(), "live mode must not contain simulated src/*_src keys: {:?}", found);
    });
}

#[test]
fn bd03_live_cluster_nodes_are_inferred_never_simulated() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (state, _) = make_state(true, true, true);
        let p = build_schema_v2_payload(&state);
        let nodes = p["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.iter().all(|n| n["state_src"] == "inferred"));
        let mut found = Vec::new();
        simulated_paths(&p, "root", &mut found);
        assert!(found.is_empty(), "{:?}", found);
    });
}

#[test]
fn bd03_simulation_mode_may_list_nodes_labelled_simulated() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(true, || {
        let (state, _) = make_state(true, true, false);
        let p = build_schema_v2_payload(&state);
        assert_eq!(p["mode"], "simulation");
        let nodes = p["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.iter().all(|n| n["state_src"] == "simulated"));
    });
}

// S-7: schema-1 events
#[test]
fn s7_v1_live_payload_has_no_synthetic_values_and_declares_provenance() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (state, _) = make_state(true, true, false);
        let p = build_schema_v1_payload(&state);

        // Field names/shape preserved.
        for k in ["version", "timestamp_ms", "nodes", "metrics", "hot_keys"] {
            assert!(p.get(k).is_some(), "v1 key {} missing", k);
        }
        for k in [
            "throughput_req_sec", "p50_latency_ms", "p90_latency_ms", "p95_latency_ms", "p99_latency_ms",
            "backend_load_ratio", "avoided_compute_dollars", "cache_hits", "cache_misses", "hit_ratio",
            "coalesced_waiters", "evictions_total", "capacity_rejections",
        ] {
            assert!(p["metrics"].get(k).is_some(), "v1 metrics.{} missing", k);
        }

        // Synthetic placeholders are not emitted as live.
        assert_eq!(p["hot_keys"], serde_json::json!([]));
        assert_eq!(p["nodes"], serde_json::json!([]));
        assert!(p["metrics"]["backend_load_ratio"].is_null());
        assert!(p["metrics"]["avoided_compute_dollars"].is_null());
        assert!(p["metrics"]["hit_ratio"].is_null(), "no lookups yet: not 1.0");
        assert!(p["metrics"]["p95_latency_ms"].is_null(), "no samples: not the 12 ms placeholder");
        assert_ne!(p["metrics"]["throughput_req_sec"], serde_json::json!(12000.0));
        assert_ne!(p["timestamp_ms"], serde_json::json!(1), "snapshot timestamp is a startup placeholder");

        let prov = &p["v1_provenance"];
        assert_eq!(prov["mode"], "live");
        assert_eq!(prov["fields"]["hot_keys"], "unavailable");
        assert_eq!(prov["fields"]["nodes"], "unavailable");
        assert_eq!(prov["fields"]["metrics.backend_load_ratio"], "unavailable");
        assert_eq!(prov["fields"]["metrics.hit_ratio"], "unavailable");
        assert_eq!(prov["fields"]["metrics.cache_hits"], "measured");
        assert_eq!(prov["fields"]["metrics.throughput_req_sec"], "derived");
        let mut found = Vec::new();
        simulated_paths(&p, "root", &mut found);
        assert!(found.is_empty(), "{:?}", found);
        assert!(!p.to_string().contains("simulated"), "live v1 payload must not mention simulated: {}", p);
    });
}

#[test]
fn s7_v1_live_values_after_traffic_are_real() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (state, cache) = make_state(true, true, false);
        let cache = cache.unwrap();
        cache.counters.cache_hits.fetch_add(2, Ordering::Relaxed);
        cache.counters.cache_misses.fetch_add(1, Ordering::Relaxed);
        let p = build_schema_v1_payload(&state);
        assert_eq!(p["metrics"]["cache_hits"], 2);
        assert_eq!(p["metrics"]["cache_misses"], 1);
        assert!((p["metrics"]["hit_ratio"].as_f64().unwrap() - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(p["v1_provenance"]["fields"]["metrics.hit_ratio"], "derived");
    });
}

#[test]
fn s7_v1_without_cache_marks_counters_unavailable() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(false, || {
        let (state, _) = make_state(false, false, false);
        let p = build_schema_v1_payload(&state);
        assert_eq!(p["v1_provenance"]["fields"]["metrics.cache_hits"], "unavailable");
        assert!(p["metrics"]["hit_ratio"].is_null());
        assert!(p["metrics"]["throughput_req_sec"].is_null());
    });
}

#[test]
fn s7_v1_simulation_mode_keeps_legacy_values_but_labels_them_simulated() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    with_sim(true, || {
        let (state, _) = make_state(true, true, false);
        let p = build_schema_v1_payload(&state);
        assert_eq!(p["v1_provenance"]["mode"], "simulation");
        assert_eq!(p["metrics"]["throughput_req_sec"], 12000.0);
        assert!((p["metrics"]["backend_load_ratio"].as_f64().unwrap() - 0.2).abs() < 1e-6);
        assert_eq!(p["hot_keys"][0]["key"], "/availability/hotel/123");
        assert_eq!(p["v1_provenance"]["fields"]["hot_keys"], "simulated");
        assert_eq!(p["v1_provenance"]["fields"]["metrics.backend_load_ratio"], "simulated");
    });
}

// Sync test driving its own runtime: the std ENV_LOCK guard is held OUTSIDE the async block, never
// across an `.await` inside it (clippy::await_holding_lock), while still serialising env access.
#[test]
fn s7_v1_sse_over_http_carries_v1_provenance_in_live_mode() {
    let guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let prev = std::env::var("PHANTOM_SIMULATION").ok();
    std::env::remove_var("PHANTOM_SIMULATION");
    let rt = tokio::runtime::Runtime::new().unwrap();
    let payload: Option<Value> = rt.block_on(async {
        let (state, _) = make_state(true, true, false);
        let app = create_app(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
        s.write_all(b"GET /api/events HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut r = BufReader::new(s);
        let mut line = String::new();
        let mut payload: Option<Value> = None;
        for _ in 0..40 {
            line.clear();
            if r.read_line(&mut line).await.unwrap() == 0 {
                break;
            }
            if let Some(d) = line.strip_prefix("data:") {
                payload = Some(serde_json::from_str(d.trim()).unwrap());
                break;
            }
        }
        payload
    });
    match prev {
        Some(v) => std::env::set_var("PHANTOM_SIMULATION", v),
        None => std::env::remove_var("PHANTOM_SIMULATION"),
    }
    drop(guard);
    let p = payload.expect("no SSE data event");
    assert_eq!(p["v1_provenance"]["mode"], "live");
    assert_eq!(p["hot_keys"], serde_json::json!([]));
    assert!(!p.to_string().contains("/availability/hotel/123"));
}

// computed `truthful`
#[test]
fn truthful_flag_is_computed_from_simulation_mode() {
    let guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
    let prev = std::env::var("PHANTOM_SIMULATION").ok();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (live, sim): (Value, Value) = rt.block_on(async {
        let (state, _) = make_state(true, true, false);
        let app = create_app(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let client = reqwest::Client::new();
        let url = format!("http://{}/api/v1/intelligence/status", addr);

        std::env::remove_var("PHANTOM_SIMULATION");
        let live: Value = client.get(&url).send().await.unwrap().json().await.unwrap();
        std::env::set_var("PHANTOM_SIMULATION", "true");
        let sim: Value = client.get(&url).send().await.unwrap().json().await.unwrap();
        (live, sim)
    });
    match prev {
        Some(v) => std::env::set_var("PHANTOM_SIMULATION", v),
        None => std::env::remove_var("PHANTOM_SIMULATION"),
    }
    drop(guard);
    assert_eq!(live["truthful"], true);
    assert_eq!(sim["truthful"], false, "must not claim truthful while simulation is enabled");
    assert!(live["truthful_basis"].is_string());
}

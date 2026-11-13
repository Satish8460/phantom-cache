//! G2: Lab HTTP API tests. Real axum router over TCP, real JWTs, real library runners; Scenario B
//! tests spawn the real (debug) gateway binary and are therefore labelled EXPLORATORY by the API.
//! Included from `lab_live_scenario_b_tests.rs` (an existing registered test target).

use phantom_cache_demo::api::server::lab_api::{LabConfig, LabService};
use phantom_cache_demo::api::server::{create_app, create_app_with_lab, ApiState, CorsConfig, NodeSnapshot, SimulationSnapshot};
use phantom_cache_demo::core::cache::PartitionedCache;
use phantom_cache_demo::dist::node::{NodeState, VirtualNode};
use phantom_cache_demo::lab::eval::sha256_file;
use phantom_cache_demo::lab::runner::{self, RunSpec};
use phantom_cache_demo::security::{JwtClaims, JwtValidator, SecurityConfig};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const ISS: &str = "phantom-cache-auth";
const AUD: &str = "phantom-cache-api";
const SECRET: &[u8] = b"lab-api-test-secret-0123456789abcdefghij";

fn validator() -> JwtValidator {
    JwtValidator::new(ISS, AUD, SECRET)
}
fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}
fn tok(roles: Vec<&str>) -> String {
    validator().create_token(&JwtClaims::new("u", ISS, AUD, now() + 3600, None, roles)).unwrap()
}
fn expired_tok() -> String {
    validator().create_token(&JwtClaims::new("u", ISS, AUD, now() - 3600, None, vec!["admin"])).unwrap()
}
fn observer() -> String {
    tok(vec!["telemetry:read"])
}
fn operator() -> String {
    tok(vec!["scenario:operate"])
}
fn gateway_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_phantom-cache-demo"))
}
fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn tmp_root(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "g2_{}_{}_{}_{}",
        name,
        std::process::id(),
        SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn api_state(secure: bool) -> Arc<ApiState> {
    let nodes = vec![Arc::new(VirtualNode::new(1))];
    for n in &nodes {
        n.set_state(NodeState::Active);
    }
    let mut sec = SecurityConfig::new_with_jwt(validator());
    sec.enforce_rfc9111_auth_bypass = false;
    Arc::new(ApiState {
        snapshot: Mutex::new(SimulationSnapshot {
            version: "1.0".into(),
            timestamp_ms: 0,
            throughput_req_sec: 0.0,
            p50_latency_ms: 0,
            p95_latency_ms: 0,
            p99_latency_ms: 0,
            db_load_ratio: 0.0,
            avoided_compute_dollars: 0.0,
            nodes: vec![NodeSnapshot { node_id: 1, state: "Active".into(), cpu_utilization: 0.0, memory_used_bytes: 0 }],
        }),
        nodes,
        workload_type: Mutex::new("Normal".into()),
        cache: Some(Arc::new(PartitionedCache::new(4))),
        origin_url: None,
        telemetry: None,
        pie: None,
        security: if secure { Some(Arc::new(sec)) } else { None },
        upstream_policy: None,
        router: None,
    })
}

struct Lab {
    base: String,
    addr: std::net::SocketAddr,
    c: reqwest::Client,
    root: PathBuf,
}

async fn serve(app: axum::Router, root: PathBuf) -> Lab {
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    Lab { base: format!("http://{}", addr), addr, c: reqwest::Client::builder().pool_max_idle_per_host(0).build().unwrap(), root }
}

async fn lab_with(secure: bool, gateway: Option<PathBuf>, evidence: Option<PathBuf>) -> Lab {
    let root = tmp_root("lab");
    let mut cfg = LabConfig::new(root.clone(), gateway);
    if let Some(e) = evidence {
        cfg = cfg.with_evidence_dir(e);
    }
    let svc = LabService::new(cfg).unwrap();
    serve(create_app_with_lab(api_state(secure), &CorsConfig::same_origin(), Some(svc)), root).await
}

async fn lab(secure: bool) -> Lab {
    lab_with(secure, Some(gateway_exe()), None).await
}

impl Lab {
    fn url(&self, p: &str) -> String {
        format!("{}{}", self.base, p)
    }
    async fn get(&self, p: &str, t: Option<&str>) -> (u16, Value) {
        let mut r = self.c.get(self.url(p));
        if let Some(t) = t {
            r = r.bearer_auth(t);
        }
        let resp = r.send().await.unwrap();
        let s = resp.status().as_u16();
        let text = resp.text().await.unwrap();
        (s, serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }
    async fn post(&self, p: &str, t: Option<&str>, body: Value) -> (u16, Value) {
        self.post_raw(p, t, Some("application/json"), body.to_string()).await
    }
    async fn post_raw(&self, p: &str, t: Option<&str>, ctype: Option<&str>, body: String) -> (u16, Value) {
        let mut r = self.c.post(self.url(p)).body(body);
        if let Some(c) = ctype {
            r = r.header("content-type", c);
        }
        if let Some(t) = t {
            r = r.bearer_auth(t);
        }
        let resp = r.send().await.unwrap();
        let s = resp.status().as_u16();
        let text = resp.text().await.unwrap();
        (s, serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }
    async fn wait_state(&self, id: &str, t: Option<&str>, want: &[&str], secs: u64) -> Value {
        let end = Instant::now() + Duration::from_secs(secs);
        loop {
            let (s, v) = self.get(&format!("/api/v1/lab/runs/{id}"), t).await;
            assert_eq!(s, 200, "{v}");
            let st = v["state"].as_str().unwrap().to_string();
            if want.contains(&st.as_str()) {
                return v;
            }
            assert!(Instant::now() < end, "timeout waiting for {want:?}, state {st}: {v}");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
    /// Collects the SSE stream until the server closes it. Returns (id, event, data).
    async fn sse(&self, id: &str, t: Option<&str>, last: Option<u64>) -> Vec<(u64, String, Value)> {
        let mut r = self.c.get(self.url(&format!("/api/v1/lab/runs/{id}/events")));
        if let Some(t) = t {
            r = r.bearer_auth(t);
        }
        if let Some(l) = last {
            r = r.header("last-event-id", l.to_string());
        }
        let mut resp = r.send().await.unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        assert!(resp.headers().get("content-type").unwrap().to_str().unwrap().starts_with("text/event-stream"));
        let mut buf = String::new();
        let end = Instant::now() + Duration::from_secs(180);
        while let Some(chunk) = tokio::time::timeout(Duration::from_secs(60), resp.chunk()).await.expect("sse stalled").unwrap() {
            buf.push_str(&String::from_utf8_lossy(&chunk));
            assert!(Instant::now() < end, "sse never ended");
        }
        let mut out = Vec::new();
        for block in buf.replace("\r\n", "\n").split("\n\n") {
            let (mut id, mut ev, mut data) = (None, None, None);
            for line in block.lines() {
                if let Some(v) = line.strip_prefix("id:") {
                    id = v.trim().parse::<u64>().ok();
                } else if let Some(v) = line.strip_prefix("event:") {
                    ev = Some(v.trim().to_string());
                } else if let Some(v) = line.strip_prefix("data:") {
                    data = serde_json::from_str::<Value>(v.trim()).ok();
                }
            }
            if let (Some(i), Some(e), Some(d)) = (id, ev, data) {
                out.push((i, e, d));
            }
        }
        out
    }
}

/// Raw request line (no client-side URL normalisation). Returns (status, body).
async fn raw_get(addr: std::net::SocketAddr, target: &str, auth: Option<&str>) -> (u16, String) {
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    let a = auth.map(|t| format!("Authorization: Bearer {t}\r\n")).unwrap_or_default();
    let req = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n{a}\r\n", addr.port());
    s.write_all(req.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(10), s.read_to_end(&mut out)).await;
    let text = String::from_utf8_lossy(&out).to_string();
    let status = text.split_whitespace().nth(1).and_then(|x| x.parse().ok()).unwrap_or(0);
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn t1_spec(n_seeds: usize, n_arms: usize) -> Value {
    json!({
        "name": "lab-api-run",
        "scenarios": [runner::list_scenarios()[0].clone()],
        "families": [runner::list_families()[0].clone()],
        "arms": runner::list_arms()[..n_arms].to_vec(),
        "seed_set": "D", "n_seeds": n_seeds, "profile": "smoke", "arm_params": "untuned", "record_raw": false
    })
}
fn t1_req(n_seeds: usize, n_arms: usize) -> Value {
    json!({"kind": "t1_matrix", "spec": t1_spec(n_seeds, n_arms)})
}
fn sb_req(arms: Value, phase_s: u64, reps: usize) -> Value {
    json!({"kind": "scenario_b", "spec": {"repetitions": reps, "arms": arms,
        "warmup_s": phase_s, "healthy_s": phase_s, "degraded_s": phase_s, "recovery_s": phase_s}})
}

const RUNS: &str = "/api/v1/lab/runs";

#[path = "review_tests.rs"]
mod review_tests;
#[path = "review_tests_b.rs"]
mod review_tests_b;
#[path = "review_tests_c.rs"]
mod review_tests_c;

// ======================================================================= disabled by default

#[tokio::test]
async fn lab_api_is_off_by_default_every_route_is_404() {
    let l = serve(create_app(api_state(false)), tmp_root("off")).await;
    for p in ["/api/v1/lab/capabilities", "/api/v1/lab/evidence", "/api/v1/lab/runs", "/api/v1/lab/runs/x", "/api/v1/lab/evidence/x"] {
        assert_eq!(l.get(p, None).await.0, 404, "{p}");
    }
    assert_eq!(l.post(RUNS, None, t1_req(1, 1)).await.0, 404);
    // the gateway's own routes still work
    assert_eq!(l.get("/health", None).await.0, 200);
}

// ======================================================================= capabilities

#[tokio::test]
async fn capabilities_report_kinds_limits_and_security_mode() {
    let l = lab(false).await;
    let (s, v) = l.get("/api/v1/lab/capabilities", None).await;
    assert_eq!(s, 200);
    assert_eq!(v["schema"], "lab-1");
    assert_eq!(v["security_mode"], "anonymous_dev_loopback_only");
    assert_eq!(v["max_concurrent_runs"], 1);
    assert_eq!(v["kinds"]["t1_matrix"]["available"], true);
    assert_eq!(v["kinds"]["t1_matrix"]["tier"], "SIMULATED");
    let names: Vec<&str> = v["kinds"]["t1_matrix"]["seed_sets"].as_array().unwrap().iter().map(|x| x["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["T", "D"], "the sealed set must not be offered");
    assert_eq!(v["kinds"]["scenario_b"]["available"], true);
    assert_eq!(v["kinds"]["scenario_b"]["tier"], "MEASURED_LOOPBACK");
    if cfg!(debug_assertions) {
        assert_eq!(v["kinds"]["scenario_b"]["label_for_default_experiment"], "EXPLORATORY", "debug builds are never VALID");
    }
    assert_eq!(v["limits"]["body_bytes"], 16384);
    assert!(!v.to_string().contains("sealed-m6"), "no sealed path in any response");

    let s2 = lab_with(true, Some(gateway_exe()), None).await;
    let (_, v2) = s2.get("/api/v1/lab/capabilities", Some(&observer())).await;
    assert_eq!(v2["security_mode"], "jwt");
}

#[tokio::test]
async fn capabilities_scenario_b_unavailable_when_gateway_missing_and_start_is_refused() {
    for gw in [None, Some(PathBuf::from("Z:/definitely/not/here/phantom-cache-demo.exe"))] {
        let l = lab_with(false, gw, None).await;
        let (_, v) = l.get("/api/v1/lab/capabilities", None).await;
        assert_eq!(v["kinds"]["scenario_b"]["available"], false);
        assert!(v["kinds"]["scenario_b"]["reason"].as_str().unwrap().len() > 5);
        assert!(!v.to_string().contains("not/here"), "generic reason, no path");
        assert_eq!(l.post(RUNS, None, sb_req(json!(["BASE"]), 1, 1)).await.0, 422);
        assert_eq!(l.get(RUNS, None).await.1["returned"], 0, "no run created");
    }
}

// ======================================================================= auth matrix

#[tokio::test]
async fn auth_matrix_every_route() {
    let l = lab(true).await;
    let rid = "20261030T000000Z-00000000";
    // (method, path, needs operator)
    let routes: Vec<(&str, String, bool)> = vec![
        ("GET", "/api/v1/lab/capabilities".into(), false),
        ("GET", "/api/v1/lab/evidence".into(), false),
        ("GET", "/api/v1/lab/evidence/nope".into(), false),
        ("GET", RUNS.into(), false),
        ("GET", format!("{RUNS}/{rid}"), false),
        ("GET", format!("{RUNS}/{rid}/events"), false),
        ("GET", format!("{RUNS}/{rid}/artifacts"), false),
        ("GET", format!("{RUNS}/{rid}/artifacts/cells.csv"), false),
        ("POST", RUNS.into(), true),
        ("POST", format!("{RUNS}/{rid}/cancel"), true),
    ];
    let call = |m: &'static str, p: String, t: Option<String>| {
        let l = &l;
        async move {
            let mut r = if m == "GET" { l.c.get(l.url(&p)) } else { l.c.post(l.url(&p)).header("content-type", "application/json").body("{}") };
            if let Some(t) = t {
                r = r.bearer_auth(t);
            }
            r.send().await.unwrap().status().as_u16()
        }
    };
    for (m, p, op) in routes {
        let m: &'static str = if m == "GET" { "GET" } else { "POST" };
        assert_eq!(call(m, p.clone(), None).await, 401, "anonymous {m} {p}");
        assert_eq!(call(m, p.clone(), Some(expired_tok())).await, 401, "expired {m} {p}");
        assert_eq!(call(m, p.clone(), Some("garbage.token.value".into())).await, 401, "garbage {m} {p}");
        assert_eq!(call(m, p.clone(), Some(tok(vec!["cache:read"]))).await, 403, "insufficient role {m} {p}");
        let o = call(m, p.clone(), Some(observer())).await;
        let a = call(m, p.clone(), Some(operator())).await;
        let ad = call(m, p.clone(), Some(tok(vec!["admin"]))).await;
        if op {
            assert_eq!(o, 403, "observer must not {m} {p}");
            assert!(a != 401 && a != 403, "operator {m} {p} -> {a}");
            assert!(ad != 401 && ad != 403, "admin {m} {p} -> {ad}");
        } else {
            assert!(o != 401 && o != 403, "observer {m} {p} -> {o}");
            assert_eq!(a, 403, "an Operator-only token cannot read (Observer is a separate role; admin satisfies both): {m} {p}");
            assert!(ad != 401 && ad != 403, "admin {m} {p} -> {ad}");
        }
    }
}

#[tokio::test]
async fn auth_is_checked_before_body_size_or_parsing_and_nothing_is_created_for_denied_requests() {
    let l = lab(true).await;
    let big = "x".repeat(40 * 1024);
    // anonymous / observer with an oversized or garbage body: auth error, never 413/400/422
    assert_eq!(l.post_raw(RUNS, None, Some("application/json"), big.clone()).await.0, 401);
    assert_eq!(l.post_raw(RUNS, Some(&observer()), Some("application/json"), big.clone()).await.0, 403);
    assert_eq!(l.post_raw(RUNS, Some(&observer()), Some("text/plain"), "not json".into()).await.0, 403);
    // operator: now the size cap applies
    assert_eq!(l.post_raw(RUNS, Some(&operator()), Some("application/json"), big).await.0, 413);
    let (_, v) = l.get(RUNS, Some(&observer())).await;
    assert_eq!(v["returned"], 0, "denied requests have no side effect: {v}");
}

// ======================================================================= validation and limits

#[tokio::test]
async fn start_validation_limits_and_sealed_refusals() {
    let l = lab(false).await;
    let arms = runner::list_arms();
    let sc = runner::list_scenarios();
    let fam = runner::list_families();
    let base = |f: &dyn Fn(&mut Value)| {
        let mut v = t1_req(1, 1);
        f(&mut v["spec"]);
        v
    };
    // sealed seed set in every spelling, and the sealed family
    for s in ["S", "s", " S ", "sealed", "SEALED", "Sealed", "S\t"] {
        let (c, b) = l.post(RUNS, None, base(&|sp| sp["seed_set"] = json!(s))).await;
        assert_eq!(c, 422, "seed_set {s:?}: {b}");
        assert!(!b.to_string().to_lowercase().contains("seed_values"));
    }
    for f in ["W-SF", "W-SF(1)", "w-sf"] {
        let (c, _) = l.post(RUNS, None, base(&|sp| sp["families"] = json!([f]))).await;
        assert_eq!(c, 422, "family {f}");
    }
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["seed_set"] = json!("X"))).await.0, 422);
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["scenarios"] = json!(["NOPE"]))).await.0, 422);
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["arms"] = json!(["NOPE"]))).await.0, 422);
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["n_seeds"] = json!(31))).await.0, 422, ">30 seeds");
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["n_seeds"] = json!(0))).await.0, 422);
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["profile"] = json!("huge"))).await.0, 422);
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["name"] = json!("../../x"))).await.0, 422);
    // the client may not choose the output location or overwrite
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["out_dir"] = json!("C:/x"))).await.0, 422);
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["overwrite"] = json!(true))).await.0, 422);
    // unknown fields
    assert_eq!(l.post(RUNS, None, base(&|sp| sp["bogus"] = json!(1))).await.0, 422);
    let mut top = t1_req(1, 1);
    top["extra"] = json!(1);
    assert_eq!(l.post(RUNS, None, top).await.0, 422);
    assert_eq!(l.post(RUNS, None, json!({"kind": "nope", "spec": {}})).await.0, 422);
    // cell cap (duplicates are accepted ids, so the product exceeds 20,000 without being slow to send)
    let (c, b) = l
        .post(RUNS, None, base(&|sp| {
            sp["scenarios"] = json!(vec![sc[0].clone(); 30]);
            sp["families"] = json!(vec![fam[0].clone(); 30]);
            sp["arms"] = json!(vec![arms[0].clone(); 30]);
            sp["n_seeds"] = json!(30);
        }))
        .await;
    assert_eq!(c, 422, "{b}");
    assert!(b.to_string().contains("20000"));
    // body / syntax / media type
    assert_eq!(l.post_raw(RUNS, None, Some("application/json"), "x".repeat(20 * 1024)).await.0, 413);
    assert_eq!(l.post_raw(RUNS, None, Some("application/json"), "{not json".into()).await.0, 400);
    assert_eq!(l.post_raw(RUNS, None, Some("text/plain"), t1_req(1, 1).to_string()).await.0, 415);
    assert_eq!(l.post_raw(RUNS, None, None, t1_req(1, 1).to_string()).await.0, 415);
    // scenario_b bounds
    for (k, val) in [("repetitions", json!(0)), ("repetitions", json!(31)), ("warmup_s", json!(0)), ("recovery_s", json!(61)), ("rate_rps", json!(0.0)),
        ("rate_rps", json!(5000.0)), ("n_keys", json!(1)), ("static_cap", json!(0)), ("arms", json!(["STATIC"])), ("arms", json!(["BASE", "BASE"])), ("arms", json!([])), ("seed", json!(5))]
    {
        let mut v = sb_req(json!(["BASE"]), 1, 1);
        v["spec"][k] = val.clone();
        let (c, b) = l.post(RUNS, None, v).await;
        assert_eq!(c, 422, "scenario_b {k}={val}: {b}");
    }
    let mut v = sb_req(json!(["BASE"]), 1, 1);
    v["spec"]["aimd_min"] = json!(50);
    v["spec"]["aimd_max"] = json!(40);
    assert_eq!(l.post(RUNS, None, v).await.0, 422);
    // nothing above created a run
    assert_eq!(l.get(RUNS, None).await.1["returned"], 0);
    // error bodies are generic: no filesystem paths
    let (_, b) = l.post(RUNS, None, base(&|sp| sp["seed_set"] = json!("S"))).await;
    assert!(!b.to_string().contains(":\\") && !b.to_string().contains("Temp"), "{b}");
}

#[tokio::test]
async fn index_paging_is_validated_like_the_audit_endpoint() {
    let l = lab(false).await;
    for q in ["limit=0", "limit=201", "limit=abc", "limit=-1", "offset=-1", "offset=x", "limit", "limit=1.5"] {
        assert_eq!(l.get(&format!("{RUNS}?{q}"), None).await.0, 400, "{q}");
        assert_eq!(l.get(&format!("/api/v1/lab/evidence?{q}"), None).await.0, 400, "{q}");
    }
    assert_eq!(l.get(&format!("{RUNS}?limit=200&offset=3"), None).await.0, 200);
}

// ======================================================================= T1 run: events, resume, determinism, artifacts

#[tokio::test]
async fn t1_run_progress_sse_resume_determinism_and_artifacts() {
    let l = lab(true).await;
    let (op, ob) = (operator(), observer());
    let (s, v) = l.post(RUNS, Some(&op), t1_req(2, 2)).await;
    assert_eq!(s, 202, "{v}");
    let id = v["run_id"].as_str().unwrap().to_string();
    assert!(id.len() == 25 && id.as_bytes()[8] == b'T', "id = UTC timestamp + random suffix: {id}");
    let done = l.wait_state(&id, Some(&ob), &["completed", "failed", "cancelled"], 300).await;
    assert_eq!(done["state"], "completed", "{done}");
    assert_eq!(done["tier"], "SIMULATED");
    assert_eq!(done["exit_code"], 0);
    assert!(done["spec"].to_string().contains("\"seed_set\":\"D\""));

    // full replay
    let ev = l.sse(&id, Some(&ob), None).await;
    let ids: Vec<u64> = ev.iter().map(|e| e.0).collect();
    assert_eq!(ids, (1..=ids.len() as u64).collect::<Vec<_>>(), "monotonic, gap free");
    let names: Vec<&str> = ev.iter().map(|e| e.1.as_str()).collect();
    assert_eq!(names.first(), Some(&"state"));
    assert_eq!(names.last(), Some(&"done"));
    assert!(names.contains(&"progress") && names.contains(&"result"));
    let prog: Vec<&Value> = ev.iter().filter(|e| e.1 == "progress").map(|e| &e.2).collect();
    assert!(prog.iter().all(|p| p["tier"] == "SIMULATED"));
    assert_eq!(prog.last().unwrap()["done"], prog.last().unwrap()["total"]);
    let result = ev.iter().find(|e| e.1 == "result").unwrap();
    assert_eq!(result.2["tier"], "SIMULATED");
    assert_eq!(result.2["cells"], 4);
    // only events the runner produced: nothing named completed/cancelled that is not in the log
    let log = std::fs::read_to_string(l.root.join("runs").join(&id).join("events.jsonl")).unwrap();
    assert_eq!(log.lines().count(), ev.len(), "stream == persisted log, no synthetic events");

    // resume
    let mid = ids[ids.len() / 2];
    let resumed = l.sse(&id, Some(&ob), Some(mid)).await;
    assert_eq!(resumed.iter().map(|e| e.0).collect::<Vec<_>>(), ids.iter().copied().filter(|i| *i > mid).collect::<Vec<_>>());
    assert_eq!(resumed.last().unwrap().1, "done");
    assert!(l.sse(&id, Some(&ob), Some(*ids.last().unwrap())).await.is_empty(), "nothing after the last id");
    assert_eq!(l.get(&format!("{RUNS}/{id}/events"), Some(&ob)).await.0, 200);
    let c = l.c.get(l.url(&format!("{RUNS}/{id}/events"))).bearer_auth(&ob).header("last-event-id", "abc").send().await.unwrap();
    assert_eq!(c.status().as_u16(), 400);

    // determinism: same RunSpec through the library gives the same cells.csv hash
    let (_, arts) = l.get(&format!("{RUNS}/{id}/artifacts"), Some(&ob)).await;
    let names: Vec<&str> = arts["artifacts"].as_array().unwrap().iter().map(|a| a["name"].as_str().unwrap()).collect();
    for n in ["cells.csv", "manifest.json", "run.json", "events.jsonl"] {
        assert!(names.contains(&n), "{names:?}");
    }
    assert!(!names.contains(&"requests.csv"), "raw files only when record_raw");
    let api_sha = arts["artifacts"].as_array().unwrap().iter().find(|a| a["name"] == "cells.csv").unwrap()["sha256"].as_str().unwrap().to_string();
    let lib_dir = tmp_root("libdet");
    let spec = RunSpec {
        name: "lab-api-run".into(),
        scenarios: vec![runner::list_scenarios()[0].clone()],
        families: vec![runner::list_families()[0].clone()],
        arms: runner::list_arms()[..2].to_vec(),
        seed_set: "D".into(),
        n_seeds: 2,
        profile: "smoke".into(),
        arm_params: "untuned".into(),
        record_raw: false,
        out_dir: lib_dir.to_string_lossy().to_string(),
        overwrite: true,
    };
    runner::execute(&spec, &mut |_, _| {}).unwrap();
    assert_eq!(api_sha, sha256_file(&lib_dir.join("cells.csv")).unwrap(), "API run must equal the library output");
    // download equals the listing hash
    let r = l.c.get(l.url(&format!("{RUNS}/{id}/artifacts/cells.csv"))).bearer_auth(&ob).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert_eq!(r.headers().get("x-content-type-options").unwrap(), "nosniff");
    let bytes = r.bytes().await.unwrap();
    assert_eq!(api_sha, phantom_cache_demo::lab::rng::sha256_hex(&bytes));
    // sealed seeds: nothing in the persisted artifacts mentions the sealed set
    let manifest = std::fs::read_to_string(l.root.join("runs").join(&id).join("t1").join("manifest.json")).unwrap();
    assert!(!manifest.contains("\"seed_set\": \"S\""));
    // run list
    let (_, list) = l.get(RUNS, Some(&ob)).await;
    assert_eq!(list["items"][0]["run_id"], id.as_str());
    assert_eq!(list["items"][0]["state"], "completed");
    let _ = std::fs::remove_dir_all(lib_dir);
}

// ======================================================================= concurrency, cancel

#[tokio::test]
async fn second_start_is_409_and_cancel_is_cooperative_idempotent_and_frees_the_slot() {
    let l = lab(false).await;
    // a deliberately large run (~ many cells) so that it is still running when we act
    let (s, v) = l.post(RUNS, None, json!({"kind": "t1_matrix", "spec": {
        "scenarios": runner::list_scenarios()[..3].to_vec(), "families": runner::list_families()[..2].to_vec(),
        "arms": runner::list_arms(), "seed_set": "T", "n_seeds": 30, "profile": "smoke"}}))
        .await;
    assert_eq!(s, 202, "{v}");
    let id = v["run_id"].as_str().unwrap().to_string();
    let (s2, b2) = l.post(RUNS, None, t1_req(1, 1)).await;
    assert_eq!(s2, 409, "{b2}");
    assert_eq!(l.get("/api/v1/lab/capabilities", None).await.1["active_run"], id.as_str());
    // wait for the first progress event, then cancel
    let end = Instant::now() + Duration::from_secs(120);
    loop {
        let log = std::fs::read_to_string(l.root.join("runs").join(&id).join("events.jsonl")).unwrap_or_default();
        if log.contains("\"progress\"") {
            break;
        }
        assert!(Instant::now() < end);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let (c, cv) = l.post(&format!("{RUNS}/{id}/cancel"), None, json!({})).await;
    assert!(c == 202 || c == 200, "{c} {cv}");
    let fin = l.wait_state(&id, None, &["cancelled", "completed", "failed"], 120).await;
    assert_eq!(fin["state"], "cancelled", "{fin}");
    assert_eq!(fin["exit_code"], 130);
    let ev = l.sse(&id, None, None).await;
    assert_eq!(ev.last().unwrap().1, "done");
    assert_eq!(ev.last().unwrap().2["state"], "cancelled");
    assert!(ev.iter().any(|e| e.1 == "state" && e.2["state"] == "cancelling"));
    assert!(!ev.iter().any(|e| e.2["state"] == "completed"), "never a synthetic completed");
    // idempotent
    let (c2, cv2) = l.post(&format!("{RUNS}/{id}/cancel"), None, json!({})).await;
    assert_eq!(c2, 200);
    assert_eq!(cv2["changed"], false);
    assert_eq!(l.post(&format!("{RUNS}/20261030T000000Z-00000000/cancel"), None, json!({})).await.0, 404);
    // slot is free again
    assert_eq!(l.get("/api/v1/lab/capabilities", None).await.1["active_run"], Value::Null);
    let (s3, _) = l.post(RUNS, None, t1_req(1, 1)).await;
    assert_eq!(s3, 202);
}

#[tokio::test]
async fn cancel_without_json_content_type_is_refused() {
    let l = lab(false).await;
    let (c, _) = l.post_raw(&format!("{RUNS}/20261030T000000Z-00000000/cancel"), None, None, String::new()).await;
    assert_eq!(c, 415, "a body-less cross-site POST must not be able to cancel");
}

// ======================================================================= restart semantics

#[tokio::test]
async fn restart_rewrites_active_states_to_interrupted_and_never_completes() {
    let root = tmp_root("restart");
    let runs = root.join("runs");
    let mk = |id: &str, state: &str, with_done: bool| {
        let d = runs.join(id);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(
            d.join("run.json"),
            json!({"run_id": id, "kind": "t1_matrix", "state": state, "tier": "SIMULATED", "spec": {}, "created_utc": "2026-10-30T00:00:00Z",
                   "started_utc": "2026-10-30T00:00:01Z", "ended_utc": null, "exit_code": null, "progress": {"done": 3, "total": 10}})
            .to_string(),
        )
        .unwrap();
        let mut ev = json!({"id": 1, "event": "state", "ts": "x", "data": {"state": "running"}}).to_string() + "\n";
        ev += &(json!({"id": 2, "event": "progress", "ts": "x", "data": {"done": 3, "total": 10}}).to_string() + "\n");
        if with_done {
            ev += &(json!({"id": 3, "event": "done", "ts": "x", "data": {"state": state}}).to_string() + "\n");
        }
        std::fs::write(d.join("events.jsonl"), ev).unwrap();
    };
    mk("20261030T000001Z-aaaaaaaa", "running", false);
    mk("20261030T000002Z-bbbbbbbb", "queued", false);
    mk("20261030T000003Z-cccccccc", "cancelling", false);
    mk("20261030T000004Z-dddddddd", "completed", true);
    mk("20261030T000005Z-eeeeeeee", "failed", true);
    let svc = LabService::new(LabConfig::new(root.clone(), None)).unwrap();
    let l = serve(create_app_with_lab(api_state(false), &CorsConfig::same_origin(), Some(svc)), root).await;
    for id in ["20261030T000001Z-aaaaaaaa", "20261030T000002Z-bbbbbbbb", "20261030T000003Z-cccccccc"] {
        let (_, v) = l.get(&format!("{RUNS}/{id}"), None).await;
        assert_eq!(v["state"], "interrupted", "{id}: {v}");
        assert!(v["ended_utc"].is_string());
        assert_eq!(v["exit_code"], Value::Null);
        assert_eq!(v["active"], false);
        let ev = l.sse(id, None, None).await;
        assert_eq!(ev.last().unwrap().1, "done");
        assert_eq!(ev.last().unwrap().2["state"], "interrupted");
        assert!(!ev.iter().any(|e| e.2["state"] == "completed"));
        assert_eq!(ev[1].1, "progress", "the original events are preserved");
    }
    assert_eq!(l.get(&format!("{RUNS}/20261030T000004Z-dddddddd"), None).await.1["state"], "completed", "terminal runs untouched");
    assert_eq!(l.get(&format!("{RUNS}/20261030T000005Z-eeeeeeee"), None).await.1["state"], "failed");
    // no run is active after a restart: a new one can start
    assert_eq!(l.post(RUNS, None, t1_req(1, 1)).await.0, 202);
}

// ======================================================================= artifacts: allowlist, traversal, size cap

#[tokio::test]
async fn artifact_allowlist_traversal_symlink_and_size_cap() {
    let l = lab(true).await;
    let ob = observer();
    let (_, v) = l.post(RUNS, Some(&operator()), t1_req(1, 1)).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    l.wait_state(&id, Some(&ob), &["completed"], 300).await;
    let base = format!("{RUNS}/{id}/artifacts");
    assert_eq!(l.get(&format!("{base}/cells.csv"), Some(&ob)).await.0, 200);
    let attempts = [
        "..", "../run.json", "..%2Frun.json", "%2e%2e", "%2e%2e%2frun.json", "..%5Crun.json", "..\\run.json", "%2e%2e%5c%2e%2e%5cCargo.toml",
        "C:%5CWindows%5Cwin.ini", "C:\\Windows\\win.ini", "%2Fetc%2Fpasswd", "/etc/passwd", "t1%2Fcells.csv", "t1\\cells.csv", "t1", "out", "cells.csv.", "cells.csv%20",
        "CELLS.CSV", "cells.csv::$DATA", "cells.csv%00.json", "requests.csv", "fetches.csv", "summary.json", "gateway.log", ".", "%2e", "run.json.tmp",
        "events.jsonl%00", "~1", "cells.csv/", "cells.csv/..", "....//....//Cargo.toml", "%252e%252e%252frun.json",
    ];
    for a in attempts {
        let (s, body) = raw_get(l.addr, &format!("{base}/{a}"), Some(&ob)).await;
        assert!(s == 404 || s == 400, "{a} -> {s}");
        assert!(!body.contains("[package]") && !body.contains("[fonts]"), "{a} leaked a file");
    }
    // run ids must match the grammar: no traversal through the id either
    for bad in ["..", "..%2f..%2fCargo.toml", "20261030T000000Z-00000000%2f..", "%2e%2e", "x"] {
        let (s, body) = raw_get(l.addr, &format!("{RUNS}/{bad}/artifacts/run.json"), Some(&ob)).await;
        assert!(s == 404 || s == 400, "{bad} -> {s}");
        assert!(!body.contains("[package]"));
    }
    // symlink planted in place of an allowlisted name is neither listed nor served
    let t1 = l.root.join("runs").join(&id).join("t1");
    let link = t1.join("requests.csv");
    let target = repo().join("Cargo.toml");
    #[cfg(windows)]
    let made = std::os::windows::fs::symlink_file(&target, &link).is_ok();
    #[cfg(unix)]
    let made = std::os::unix::fs::symlink(&target, &link).is_ok();
    if made {
        let (s, body) = raw_get(l.addr, &format!("{base}/requests.csv"), Some(&ob)).await;
        assert_eq!(s, 404, "symlinked artifact served: {body}");
        let (_, list) = l.get(&base, Some(&ob)).await;
        assert!(!list.to_string().contains("requests.csv"));
    } else {
        eprintln!("note: symlink creation not permitted on this host; symlink case not exercised");
    }
    // size cap: a >64 MiB cells.csv is refused and its listing says so
    let big = t1.join("cells.csv");
    {
        let f = std::fs::OpenOptions::new().write(true).open(&big).unwrap();
        f.set_len(64 * 1024 * 1024 + 1).unwrap();
    }
    let (s, b) = l.get(&format!("{base}/cells.csv"), Some(&ob)).await;
    assert_eq!(s, 413, "{b}");
    let (_, list) = l.get(&base, Some(&ob)).await;
    let e = list["artifacts"].as_array().unwrap().iter().find(|a| a["name"] == "cells.csv").unwrap();
    assert_eq!(e["downloadable"], false);
    assert_eq!(e["sha256"]["unavailable"], true);
}

// ======================================================================= evidence

#[tokio::test]
async fn evidence_index_shows_sealed_negative_verdict_and_scenario_b_tradeoff_exactly_as_recorded_read_only() {
    let ev_dir = repo().join("lab-results");
    let has_sealed = std::fs::read_dir(&ev_dir)
        .map(|r| r.flatten().any(|e| e.file_name().to_string_lossy().starts_with("m6_sealed_")))
        .unwrap_or(false);
    if !has_sealed || !ev_dir.join("scenario_b/20261030T003932Z/summary.json").is_file() {
        eprintln!("SKIPPED: the retained experiments under lab-results/ are absent on this checkout");
        return;
    }
    let l = lab_with(false, None, Some(ev_dir.clone())).await;
    let before: Vec<(PathBuf, String)> = ["verdict.json", "manifest.json"]
        .iter()
        .flat_map(|f| std::fs::read_dir(&ev_dir).unwrap().flatten().map(move |e| e.path().join(f)).collect::<Vec<_>>())
        .filter(|p| p.is_file())
        .map(|p| {
            let h = sha256_file(&p).unwrap();
            (p, h)
        })
        .collect();
    let (s, idx) = l.get("/api/v1/lab/evidence?limit=200", None).await;
    assert_eq!(s, 200, "{idx}");
    let items = idx["items"].as_array().unwrap();
    let sealed: Vec<&Value> = items.iter().filter(|i| i["kind"] == "m6_sealed_evaluation").collect();
    assert_eq!(sealed.len(), 1, "exactly one retained sealed evaluation");
    let sealed = sealed[0];
    assert_eq!(sealed["tier"], "RETAINED_SIMULATED");
    assert_eq!(sealed["read_only"], true);
    assert_eq!(sealed["verdict_snapshot"]["H1"], "NOT_SUPPORTED", "negative verdict shown as recorded");
    // equals the file
    let recorded: Value = serde_json::from_slice(&std::fs::read(ev_dir.join(sealed["id"].as_str().unwrap()).join("verdict.json")).unwrap()).unwrap();
    assert_eq!(sealed["verdict_snapshot"]["H1"], recorded["H1"]["verdict"]);
    assert_eq!(sealed["verdict_snapshot"]["H2"], recorded["H2"]["status"]);
    assert!(items.iter().any(|i| i["kind"] == "m6_dry_run"), "dry runs are listed");
    assert!(items.iter().any(|i| i["kind"] == "tuning"), "tuning artifacts are listed");
    let sb: Vec<&Value> = items.iter().filter(|i| i["kind"] == "scenario_b").collect();
    assert!(!sb.is_empty());
    let sbm = sb.iter().find(|i| i["id"] == "scenario_b.20261030T003932Z").expect("retained Scenario B experiment");
    assert_eq!(sbm["tier"], "RETAINED_MEASURED_LOOPBACK");
    let rec_sum: Value = serde_json::from_slice(&std::fs::read(ev_dir.join("scenario_b/20261030T003932Z/summary.json")).unwrap()).unwrap();
    for arm in ["STATIC-CAP", "AIMD-CAP"] {
        assert_eq!(sbm["verdict_snapshot"]["verdicts"][arm]["outcome"], "TRADE_OFF_FAILURE", "{arm}");
        assert_eq!(sbm["verdict_snapshot"]["verdicts"][arm]["outcome"], rec_sum["verdicts"][arm]["outcome"]);
    }
    // detail: sealed
    let (s, d) = l.get(&format!("/api/v1/lab/evidence/{}", sealed["id"].as_str().unwrap()), None).await;
    assert_eq!(s, 200);
    assert_eq!(d["recorded_result"]["H1"]["verdict"], "NOT_SUPPORTED");
    assert!(d["sealed"]["seed_values"].is_null());
    assert_eq!(d["provenance"]["seed_set"], "S");
    let text = d.to_string();
    assert!(!text.contains("PHANTOM_SEALED_SALT") && !text.contains(".phantom-sealed"));
    // detail: Scenario B carries the binding notice and limitations
    let (s, d) = l.get("/api/v1/lab/evidence/scenario_b.20261030T003932Z", None).await;
    assert_eq!(s, 200);
    assert_eq!(d["interpretation"]["binding"], true);
    let notice = d["interpretation"]["notice_text"].as_str().unwrap();
    assert!(notice.contains("Permitted conclusion") && notice.contains("Not established") && notice.contains("TRADE_OFF_FAILURE"));
    assert!(d["interpretation"]["limitations"].as_array().unwrap().len() >= 8);
    assert_eq!(d["recorded_result"]["verdicts"]["STATIC-CAP"]["outcome"], "TRADE_OFF_FAILURE");
    // unknown / traversal ids
    for bad in ["nope", "..", "%2e%2e", "m6_sealed_2026-10-29T234845Z%2f..%2fCargo.toml", "runs", "scenario_b"] {
        let (s, _) = raw_get(l.addr, &format!("/api/v1/lab/evidence/{bad}"), None).await;
        assert_eq!(s, 404, "{bad}");
    }
    // read-only: no mutating verb exists for evidence
    for m in [reqwest::Method::POST, reqwest::Method::PUT, reqwest::Method::DELETE, reqwest::Method::PATCH] {
        let r = l.c.request(m, l.url("/api/v1/lab/evidence/scenario_b.20261030T003932Z")).send().await.unwrap();
        assert_eq!(r.status().as_u16(), 405);
    }
    for (p, h) in before {
        assert_eq!(sha256_file(&p).unwrap(), h, "{} changed", p.display());
    }
}

#[tokio::test]
async fn evidence_detail_cancelled_scenario_b_preserves_null_verdicts_in_snapshot() {
    let root = tmp_root("cancelled_ev");
    let exp_dir = root.join("scenario_b").join("20261030T999999Z");
    std::fs::create_dir_all(&exp_dir).unwrap();
    let summary = serde_json::json!({
        "summary_version": 2,
        "tier": "MEASURED_LOOPBACK",
        "label": "EXPLORATORY",
        "label_planned": "VALID",
        "label_reasons": ["run incomplete (cancelled): the planned label VALID does not apply to a partial run"],
        "run_completeness": {
            "complete": false,
            "incomplete_reason": "cancelled",
            "planned_runs": 20,
            "completed_runs": 2
        },
        "verdicts_status": "not_computed",
        "verdicts_reason": "the run did not finish all planned repetitions of every arm; no mechanism verdict or arm verdict is computed from a partial design",
        "status": "CANCELLED",
        "mechanism_h_b1_all_pass": null,
        "verdicts": null
    });
    std::fs::write(exp_dir.join("summary.json"), serde_json::to_string_pretty(&summary).unwrap()).unwrap();
    let manifest = serde_json::json!({
        "status": "CANCELLED",
        "exit_code": 130,
        "extra": {}
    });
    std::fs::write(exp_dir.join("manifest.json"), serde_json::to_string_pretty(&manifest).unwrap()).unwrap();

    let l = lab_with(false, None, Some(root.clone())).await;
    let (s, idx) = l.get("/api/v1/lab/evidence?limit=200", None).await;
    assert_eq!(s, 200, "{idx}");
    let items = idx["items"].as_array().unwrap();
    let found = items.iter().find(|i| i["id"] == "scenario_b.20261030T999999Z").expect("cancelled experiment item found");
    assert_eq!(found["verdict_snapshot"]["status"], "CANCELLED");
    assert_eq!(found["verdict_snapshot"]["label"], "EXPLORATORY");
    assert_eq!(found["verdict_snapshot"]["label_planned"], "VALID");
    assert_eq!(found["verdict_snapshot"]["verdicts_status"], "not_computed");
    assert_eq!(found["verdict_snapshot"]["run_completeness"]["complete"], false);
    // Crucial U-01 assertion: verdicts must be null, NOT empty object {}
    assert!(found["verdict_snapshot"]["verdicts"].is_null(), "verdicts must be null, got {:?}", found["verdict_snapshot"]["verdicts"]);
    assert_ne!(found["verdict_snapshot"]["verdicts"], serde_json::json!({}), "verdicts must never be converted to empty object");
    assert!(found["verdict_snapshot"]["mechanism_h_b1_all_pass"].is_null());
}

// ======================================================================= D1: wording pinned

// ======================================================================= config

#[test]
fn config_lab_keys_default_off_and_fail_fast_without_jwt_on_a_network_bind() {
    use phantom_cache_demo::config::{resolve, Inputs};
    let env = |kv: &[(&str, &str)]| Inputs { env: kv.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(), ..Default::default() };
    let c = resolve(&env(&[])).unwrap();
    assert!(!c.get_bool("PHANTOM_LAB_ENABLED"), "off by default");
    assert_eq!(c.get("PHANTOM_LAB_DIR"), Some("lab-results"));
    assert!(resolve(&env(&[("PHANTOM_LAB_ENABLED", "true")])).is_ok(), "loopback dev bind is fine");
    assert!(resolve(&env(&[("PHANTOM_LAB_ENABLED", "maybe")])).is_err());
    let errs = resolve(&env(&[("PHANTOM_LAB_ENABLED", "true"), ("PHANTOM_BIND_ADDR", "0.0.0.0:8085")])).unwrap_err();
    assert!(errs.iter().any(|e| e.key.as_deref() == Some("PHANTOM_LAB_ENABLED")), "{errs:?}");
    // a LAB_DIR set while disabled only warns
    let c = resolve(&env(&[("PHANTOM_LAB_DIR", "x")])).unwrap();
    assert!(c.warnings.iter().any(|w| w.contains("PHANTOM_LAB_DIR")));
}

// ======================================================================= Scenario B through the API (real gateway, debug = EXPLORATORY)

fn collect_pids(events: &[(u64, String, Value)]) -> Vec<u32> {
    let mut v: Vec<u32> = events.iter().filter_map(|e| e.2["gateway_pid"].as_u64()).filter(|p| *p != 0).map(|p| p as u32).collect();
    v.sort();
    v.dedup();
    v
}

fn wait_dead(pid: u32) -> bool {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(15) {
        if !phantom_cache_demo::lab_live::gateway::pid_alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_b_via_api_short_phases_is_exploratory_in_debug_and_leaves_no_orphans() {
    let _g = super::E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let l = lab(true).await;
    let (op, ob) = (operator(), observer());
    let (s, v) = l.post(RUNS, Some(&op), sb_req(json!(["BASE", "STATIC-CAP"]), 1, 1)).await;
    assert_eq!(s, 202, "{v}");
    let id = v["run_id"].as_str().unwrap().to_string();
    let run = l.wait_state(&id, Some(&ob), &["completed", "failed", "cancelled"], 240).await;
    assert_eq!(run["state"], "completed", "{run}");
    assert_eq!(run["tier"], "MEASURED_LOOPBACK");
    assert!(run["interpretation"]["limitations"].as_array().unwrap().len() >= 8);
    assert_eq!(run["interpretation"]["label_meaning"], phantom_cache_demo::lab_live::scenario_b::LABEL_VALID_MEANING);
    let ev = l.sse(&id, Some(&ob), None).await;
    let result = ev.iter().find(|e| e.1 == "result").unwrap();
    if cfg!(debug_assertions) {
        assert_eq!(result.2["label"], "EXPLORATORY");
    }
    assert!(result.2["verdicts"]["STATIC-CAP"]["outcome"].is_string());
    let pids = collect_pids(&ev);
    assert!(!pids.is_empty(), "gateway pids are reported in progress events");
    for p in pids {
        assert!(wait_dead(p), "gateway {p} still running after the run");
    }
    let (_, arts) = l.get(&format!("{RUNS}/{id}/artifacts"), Some(&ob)).await;
    let names: Vec<&str> = arts["artifacts"].as_array().unwrap().iter().map(|a| a["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"summary.json") && names.contains(&"manifest.json"));
    let r = l.c.get(l.url(&format!("{RUNS}/{id}/artifacts/summary.json"))).bearer_auth(&ob).send().await.unwrap();
    let summary: Value = r.json().await.unwrap();
    assert_eq!(summary["tier"], "MEASURED_LOOPBACK");
    assert_eq!(summary["label"], "EXPLORATORY", "1 repetition can never be VALID");
    assert_eq!(summary["label_meaning"], phantom_cache_demo::lab_live::scenario_b::LABEL_VALID_MEANING);
    assert!(summary["interpretation_notes"].as_array().unwrap().len() >= 5);
}

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scenario_b_cancel_terminates_the_gateway_child() {
    let _g = super::E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let l = lab(false).await;
    let (s, v) = l.post(RUNS, None, sb_req(json!(["BASE", "STATIC-CAP", "AIMD-CAP"]), 4, 2)).await;
    assert_eq!(s, 202, "{v}");
    let id = v["run_id"].as_str().unwrap().to_string();
    // wait until a gateway pid has been reported
    let end = Instant::now() + Duration::from_secs(120);
    let mut pid = 0u32;
    while pid == 0 {
        let log = std::fs::read_to_string(l.root.join("runs").join(&id).join("events.jsonl")).unwrap_or_default();
        for line in log.lines() {
            if let Ok(j) = serde_json::from_str::<Value>(line) {
                if let Some(p) = j["data"]["gateway_pid"].as_u64() {
                    if p != 0 && j["data"]["phase"] == "healthy" {
                        pid = p as u32;
                    }
                }
            }
        }
        assert!(Instant::now() < end, "no gateway pid reported");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let t = Instant::now();
    let (c, _) = l.post(&format!("{RUNS}/{id}/cancel"), None, json!({})).await;
    assert!(c == 202 || c == 200);
    let fin = l.wait_state(&id, None, &["cancelled", "completed", "failed"], 60).await;
    assert_eq!(fin["state"], "cancelled", "{fin}");
    assert!(t.elapsed() < Duration::from_secs(30), "cancellation must be prompt");
    assert!(wait_dead(pid), "gateway {pid} survived cancellation");
    assert_eq!(l.get("/api/v1/lab/capabilities", None).await.1["active_run"], Value::Null);
    let m = std::fs::read_to_string(l.root.join("runs").join(&id).join("out").join("manifest.json")).unwrap();
    assert!(m.contains("CANCELLED"));
}

/// Slow variant: 10 repetitions x 3 arms with 1 s phases in the current (debug) profile. Mechanics only.
#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore]
async fn heavy_scenario_b_ten_reps_via_api_is_exploratory_in_debug() {
    let _g = super::E2E_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let l = lab(false).await;
    let (s, v) = l.post(RUNS, None, sb_req(json!(["BASE", "STATIC-CAP", "AIMD-CAP"]), 1, 10)).await;
    assert_eq!(s, 202, "{v}");
    let id = v["run_id"].as_str().unwrap().to_string();
    let run = l.wait_state(&id, None, &["completed", "failed", "cancelled"], 1800).await;
    assert_eq!(run["state"], "completed", "{run}");
    let ev = l.sse(&id, None, None).await;
    assert_eq!(ev.iter().find(|e| e.1 == "result").unwrap().2["label"], "EXPLORATORY");
}

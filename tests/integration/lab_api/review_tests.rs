//! G2b: tests for the independent security review findings (M1-M7, L1-L8).

use super::*;
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub async fn raw_req(addr: std::net::SocketAddr, method: &str, target: &str, headers: &[(&str, &str)], body: &str) -> (u16, String) {
    let mut s = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut req = format!("{method} {target} HTTP/1.1\r\nConnection: close\r\n");
    for (k, v) in headers {
        req.push_str(&format!("{k}: {v}\r\n"));
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n{}", body.len(), body));
    s.write_all(req.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(15), s.read_to_end(&mut out)).await;
    let text = String::from_utf8_lossy(&out).to_string();
    let status = text.split_whitespace().nth(1).and_then(|x| x.parse().ok()).unwrap_or(0);
    (status, text.split("\r\n\r\n").nth(1).unwrap_or("").to_string())
}

fn run_dirs(root: &Path) -> usize {
    std::fs::read_dir(root.join("runs")).map(|r| r.flatten().count()).unwrap_or(0)
}

async fn lab_cors(secure: bool, origins: &[&str]) -> Lab {
    let root = tmp_root("labc");
    let svc = LabService::new(LabConfig::new(root.clone(), Some(gateway_exe()))).unwrap();
    let cors = CorsConfig::with_origins(origins.iter().copied()).unwrap();
    serve(create_app_with_lab(api_state(secure), &cors, Some(svc)), root).await
}

fn long_t1() -> Value {
    json!({"kind": "t1_matrix", "spec": {
        "scenarios": runner::list_scenarios()[..3].to_vec(), "families": runner::list_families()[..2].to_vec(),
        "arms": runner::list_arms(), "seed_set": "T", "n_seeds": 30, "profile": "smoke"}})
}

async fn wait_first_progress(l: &Lab, id: &str) {
    let end = Instant::now() + Duration::from_secs(120);
    loop {
        let log = std::fs::read_to_string(l.root.join("runs").join(id).join("events.jsonl")).unwrap_or_default();
        if log.contains("\"progress\"") {
            return;
        }
        assert!(Instant::now() < end, "no progress");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// ---- M1: Host / Origin validation in anonymous loopback mode (DNS rebinding)
#[tokio::test]
async fn m1_anonymous_mode_rejects_foreign_host_and_origin_but_jwt_mode_is_unaffected() {
    let l = lab(false).await;
    let port = l.addr.port();
    let good = [format!("127.0.0.1:{port}"), format!("localhost:{port}"), format!("[::1]:{port}"), format!("LOCALHOST:{port}")];
    for h in &good {
        let (s, _) = raw_req(l.addr, "GET", "/api/v1/lab/capabilities", &[("Host", h)], "").await;
        assert_eq!(s, 200, "Host {h}");
    }
    for h in ["evil.example:8085", &format!("evil.example:{port}"), "127.0.0.1.evil.example", &format!("127.0.0.1@evil.example:{port}"), "", "0.0.0.0:80"] {
        let (s, _) = raw_req(l.addr, "GET", "/api/v1/lab/capabilities", &[("Host", h)], "").await;
        assert!(s == 421 || s == 403, "Host {h:?} -> {s}");
        let (s, _) = raw_req(l.addr, "POST", "/api/v1/lab/runs", &[("Host", h), ("Content-Type", "application/json")], &t1_req(1, 1).to_string()).await;
        assert!(s == 421 || s == 403, "POST Host {h:?} -> {s}");
    }
    assert_eq!(run_dirs(&l.root), 0, "a rejected Host must not create a run");
    let host = format!("127.0.0.1:{port}");
    let bad_spec = json!({"kind": "t1_matrix", "spec": {"scenarios": ["x"], "families": ["x"], "arms": ["x"], "n_seeds": 1, "seed_set": "S"}}).to_string();
    for o in ["http://evil.example", &format!("http://evil.example:{port}"), "null", "http://127.0.0.1:1"] {
        let (s, _) = raw_req(l.addr, "POST", "/api/v1/lab/runs", &[("Host", &host), ("Origin", o), ("Content-Type", "application/json")], &bad_spec).await;
        assert_eq!(s, 403, "Origin {o}");
        let (s, _) = raw_req(l.addr, "GET", "/api/v1/lab/evidence", &[("Host", &host), ("Origin", o)], "").await;
        assert_eq!(s, 403, "GET Origin {o}");
    }
    let same = format!("http://127.0.0.1:{port}");
    let (s, _) = raw_req(l.addr, "POST", "/api/v1/lab/runs", &[("Host", &host), ("Origin", &same), ("Content-Type", "application/json")], &bad_spec).await;
    assert_eq!(s, 422, "same-origin passes the layer and reaches validation");
    // an origin in the CORS allowlist (the dev profile lists localhost:3000) keeps full anonymous control: documented
    let d = lab_cors(false, &["http://localhost:3000"]).await;
    let dh = format!("127.0.0.1:{}", d.addr.port());
    let (s, _) = raw_req(d.addr, "POST", "/api/v1/lab/runs", &[("Host", &dh), ("Origin", "http://localhost:3000"), ("Content-Type", "application/json")], &bad_spec).await;
    assert_eq!(s, 422, "allowlisted CORS origin passes");
    // JWT mode: the Host check is not applied (and nothing breaks)
    let j = lab(true).await;
    let (s, _) = raw_req(j.addr, "GET", "/api/v1/lab/capabilities", &[("Host", "evil.example"), ("Authorization", &format!("Bearer {}", observer()))], "").await;
    assert_eq!(s, 200);
}

// ---- M2: one shared event log, unique strictly increasing ids, whole-line writes
#[tokio::test]
async fn m2_cancel_racing_progress_keeps_event_ids_unique_and_increasing() {
    let l = lab(false).await;
    let (s, v) = l.post(RUNS, None, long_t1()).await;
    assert_eq!(s, 202);
    let id = v["run_id"].as_str().unwrap().to_string();
    wait_first_progress(&l, &id).await;
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let (c, u) = (l.c.clone(), l.url(&format!("{RUNS}/{id}/cancel")));
        tasks.push(tokio::spawn(async move {
            for _ in 0..6 {
                let _ = c.post(&u).header("content-type", "application/json").body("{}").send().await;
            }
        }));
    }
    for t in tasks {
        t.await.unwrap();
    }
    l.wait_state(&id, None, &["cancelled", "completed", "failed"], 120).await;
    let log = std::fs::read_to_string(l.root.join("runs").join(&id).join("events.jsonl")).unwrap();
    let ids: Vec<u64> = log.lines().map(|ln| serde_json::from_str::<Value>(ln).expect("every line is whole JSON")["id"].as_u64().unwrap()).collect();
    assert_eq!(ids, (1..=ids.len() as u64).collect::<Vec<_>>(), "ids unique, strictly increasing, gap free");
    let ev = l.sse(&id, None, None).await;
    assert_eq!(ev.len(), ids.len(), "SSE line count == log line count");
    assert_eq!(ev.iter().filter(|e| e.1 == "done").count(), 1);
}

// ---- M3: run.json single writer, state machine without regression, no symlink-following tmp
#[tokio::test]
async fn m3_state_never_regresses_after_cancel_and_second_cancel_is_a_noop() {
    let l = lab(false).await;
    let (_, v) = l.post(RUNS, None, long_t1()).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    wait_first_progress(&l, &id).await;
    let (c, _) = l.post(&format!("{RUNS}/{id}/cancel"), None, json!({})).await;
    assert!(c == 200 || c == 202);
    let rank = |s: &str| match s {
        "queued" => 0,
        "running" => 1,
        "cancelling" => 2,
        _ => 3,
    };
    let mut max = 2;
    let final_state: String;
    let end = Instant::now() + Duration::from_secs(120);
    loop {
        let (_, v) = l.get(&format!("{RUNS}/{id}"), None).await;
        let st = v["state"].as_str().unwrap().to_string();
        assert!(rank(&st) >= max, "state regressed to {st}");
        max = rank(&st);
        if max == 3 {
            // `completed` is the documented outcome when the cancel arrives after the last cell
            assert!(st == "cancelled" || st == "completed", "terminal state {st}");
            final_state = st;
            break;
        }
        assert!(Instant::now() < end);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    for _ in 0..3 {
        let (c, v) = l.post(&format!("{RUNS}/{id}/cancel"), None, json!({})).await;
        assert_eq!(c, 200);
        assert_eq!(v["changed"], false);
        assert_eq!(l.get(&format!("{RUNS}/{id}"), None).await.1["state"], final_state.as_str());
    }
    let on_disk: Value = serde_json::from_slice(&std::fs::read(l.root.join("runs").join(&id).join("run.json")).unwrap()).unwrap();
    assert_eq!(on_disk["state"], final_state.as_str());
    assert!(std::fs::read_dir(l.root.join("runs").join(&id)).unwrap().flatten().all(|e| !e.file_name().to_string_lossy().ends_with(".tmp")), "no temp files left");
}

// ---- M6: stored hashes, SSE cap, body-read timeout
#[tokio::test]
async fn m6_artifact_hashes_are_computed_once_and_stored_in_run_json() {
    let l = lab(false).await;
    let (_, v) = l.post(RUNS, None, t1_req(1, 1)).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    l.wait_state(&id, None, &["completed"], 120).await;
    let on_disk: Value = serde_json::from_slice(&std::fs::read(l.root.join("runs").join(&id).join("run.json")).unwrap()).unwrap();
    let idx = &on_disk["artifact_index"];
    assert!(idx["cells.csv"]["sha256"].is_string(), "{on_disk}");
    let (_, list) = l.get(&format!("{RUNS}/{id}/artifacts"), None).await;
    let e = list["artifacts"].as_array().unwrap().iter().find(|a| a["name"] == "cells.csv").unwrap();
    assert_eq!(e["sha256"], idx["cells.csv"]["sha256"]);
    let r = l.c.get(l.url(&format!("{RUNS}/{id}/artifacts/cells.csv"))).send().await.unwrap();
    assert!(r.headers().get("content-length").is_some());
    assert_eq!(phantom_cache_demo::lab::rng::sha256_hex(&r.bytes().await.unwrap()), e["sha256"].as_str().unwrap());
}

#[tokio::test]
async fn m6_sse_streams_are_capped_at_16() {
    let l = lab(false).await;
    let (_, v) = l.post(RUNS, None, long_t1()).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    wait_first_progress(&l, &id).await;
    let mut held = Vec::new();
    for i in 0..16 {
        let r = l.c.get(l.url(&format!("{RUNS}/{id}/events"))).send().await.unwrap();
        assert_eq!(r.status().as_u16(), 200, "stream {i}");
        held.push(r);
    }
    let r = l.c.get(l.url(&format!("{RUNS}/{id}/events"))).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 429, "17th concurrent stream");
    drop(held);
    let _ = l.post(&format!("{RUNS}/{id}/cancel"), None, json!({})).await;
    l.wait_state(&id, None, &["cancelled", "completed"], 120).await;
}

#[tokio::test]
async fn m6_stalled_request_body_times_out_with_408() {
    let l = lab(false).await;
    let host = format!("127.0.0.1:{}", l.addr.port());
    let mut s = tokio::net::TcpStream::connect(l.addr).await.unwrap();
    let head = format!("POST /api/v1/lab/runs HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: 500\r\nConnection: close\r\n\r\n{{\"kind\"");
    s.write_all(head.as_bytes()).await.unwrap();
    let mut out = Vec::new();
    let t = Instant::now();
    let _ = tokio::time::timeout(Duration::from_secs(12), s.read_to_end(&mut out)).await;
    let text = String::from_utf8_lossy(&out).to_string();
    assert!(text.starts_with("HTTP/1.1 408"), "got {text:?}");
    assert!(t.elapsed() < Duration::from_secs(10));
    assert_eq!(run_dirs(&l.root), 0);
}

// ---- L1: W-Z(alpha) validated API-side
#[tokio::test]
async fn l1_w_z_alpha_must_be_finite_and_in_range() {
    let l = lab(false).await;
    for a in ["nan", "NaN", "inf", "-inf", "-1", "0", "0.0", "10.5", "1e9", "x", ""] {
        let mut v = t1_req(1, 1);
        v["spec"]["families"] = json!([format!("W-Z({a})")]);
        let (c, b) = l.post(RUNS, None, v).await;
        assert_eq!(c, 422, "alpha {a:?}: {b}");
    }
    assert_eq!(run_dirs(&l.root), 0);
    let mut ok = t1_req(1, 1);
    ok["spec"]["families"] = json!(["W-Z(0.9)"]);
    assert_eq!(l.post(RUNS, None, ok).await.0, 202);
}

// ---- L2: server filesystem paths are not disclosed
#[tokio::test]
async fn l2_served_manifests_and_evidence_do_not_disclose_server_paths() {
    let l = lab(false).await;
    let (_, v) = l.post(RUNS, None, t1_req(1, 1)).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    l.wait_state(&id, None, &["completed"], 120).await;
    let root_s = l.root.to_string_lossy().to_string();
    for name in ["manifest.json", "run.json"] {
        let r = l.c.get(l.url(&format!("{RUNS}/{id}/artifacts/{name}"))).send().await.unwrap();
        let body = r.text().await.unwrap();
        for needle in [root_s.clone(), root_s.replace('\\', "/"), root_s.replace('\\', "\\\\"), repo().to_string_lossy().replace('\\', "\\\\")] {
            assert!(!body.contains(&needle), "{name} discloses {needle}");
        }
        let j: Value = serde_json::from_str(&body).expect("scrubbed artifact is still JSON");
        assert!(j.is_object());
    }
    let (_, rd) = l.get(&format!("{RUNS}/{id}"), None).await;
    assert!(!rd.to_string().contains(&root_s.replace('\\', "\\\\")));
    let ev_dir = repo().join("lab-results");
    if ev_dir.join("scenario_b/20261030T003932Z/summary.json").is_file() {
        let e = lab_with(false, None, Some(ev_dir)).await;
        let (_, d) = e.get("/api/v1/lab/evidence/scenario_b.20261030T003932Z", None).await;
        let t = d.to_string();
        assert!(!t.contains("sb_pilot1"), "pilot path leaked: {}", &t[t.find("sb_pilot1").unwrap_or(0).saturating_sub(120)..(t.find("sb_pilot1").unwrap_or(0) + 40).min(t.len())]);
        assert!(!t.contains(":\\\\"), "a Windows absolute path leaked");
        assert!(t.contains("Not established"), "notice text still delivered");
    } else {
        eprintln!("SKIPPED evidence part: lab-results/ fixtures absent");
    }
}

// ---- L4: recovery is idempotent
#[tokio::test]
async fn l4_recovery_does_not_append_a_second_done_event() {
    let root = tmp_root("l4");
    let d = root.join("runs").join("20261030T000009Z-12345678");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("run.json"),
        json!({"run_id": "20261030T000009Z-12345678", "kind": "t1_matrix", "state": "running", "tier": "SIMULATED", "spec": {},
               "created_utc": "2026-10-30T00:00:00Z", "started_utc": "2026-10-30T00:00:01Z", "ended_utc": null, "exit_code": null, "progress": {}})
        .to_string(),
    )
    .unwrap();
    let ev = |i: u64, e: &str, st: &str| json!({"id": i, "event": e, "ts": "x", "data": {"state": st}}).to_string() + "\n";
    std::fs::write(d.join("events.jsonl"), ev(1, "state", "running") + &ev(2, "state", "completed") + &ev(3, "done", "completed")).unwrap();
    let _ = LabService::new(LabConfig::new(root.clone(), None)).unwrap();
    let log = std::fs::read_to_string(d.join("events.jsonl")).unwrap();
    assert_eq!(log.lines().filter(|l| l.contains("\"done\"")).count(), 1, "no second done: {log}");
    let rj: Value = serde_json::from_slice(&std::fs::read(d.join("run.json")).unwrap()).unwrap();
    assert_eq!(rj["state"], "interrupted");
    let before = std::fs::read(d.join("events.jsonl")).unwrap();
    let _ = LabService::new(LabConfig::new(root, None)).unwrap();
    assert_eq!(before, std::fs::read(d.join("events.jsonl")).unwrap(), "a second restart changes nothing");
}

// ---- L5: SSE closes when the token expires
#[tokio::test]
async fn l5_sse_stream_ends_when_the_token_expires() {
    let l = lab(true).await;
    let (_, v) = l.post(RUNS, Some(&operator()), long_t1()).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    wait_first_progress(&l, &id).await;
    let short = validator().create_token(&JwtClaims::new("u", ISS, AUD, now() + 2, None, vec!["telemetry:read"])).unwrap();
    let mut r = l.c.get(l.url(&format!("{RUNS}/{id}/events"))).bearer_auth(&short).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let t = Instant::now();
    let mut ended = false;
    while t.elapsed() < Duration::from_secs(20) {
        match tokio::time::timeout(Duration::from_secs(10), r.chunk()).await {
            Ok(Ok(Some(_))) => continue,
            Ok(Ok(None)) | Ok(Err(_)) => {
                ended = true;
                break;
            }
            Err(_) => break,
        }
    }
    assert!(ended, "stream outlived its token");
    assert!(t.elapsed() < Duration::from_secs(15));
    let _ = l.post(&format!("{RUNS}/{id}/cancel"), Some(&operator()), json!({})).await;
    l.wait_state(&id, Some(&observer()), &["cancelled", "completed"], 120).await;
}

// ---- wiring: the real gateway binary (main.rs) honours PHANTOM_LAB_*; the salt in the environment is refused
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

/// Child process that is killed when dropped (also when an assertion fails), so no gateway is orphaned.
pub struct Proc(pub std::process::Child);
impl std::ops::Deref for Proc {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Proc {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_gateway_proc(env: &[(&str, String)]) -> Proc {
    let mut c = std::process::Command::new(gateway_exe());
    c.env_clear();
    for k in ["SystemRoot", "PATH", "TEMP", "TMP"] {
        if let Ok(v) = std::env::var(k) {
            c.env(k, v);
        }
    }
    for (k, v) in env {
        c.env(k, v);
    }
    Proc(c.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().unwrap())
}

async fn wait_listening(port: u16, child: &mut Proc) -> bool {
    let end = Instant::now() + Duration::from_secs(30);
    while Instant::now() < end {
        if child.try_wait().unwrap().is_some() {
            return false;
        }
        if tokio::net::TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[tokio::test]
async fn wiring_real_gateway_binary_enables_the_lab_only_when_configured() {
    let port = free_port();
    let dir = tmp_root("wire");
    let mut off = spawn_gateway_proc(&[("PHANTOM_BIND_ADDR", format!("127.0.0.1:{port}"))]);
    assert!(wait_listening(port, &mut off).await);
    let c = reqwest::Client::new();
    assert_eq!(c.get(format!("http://127.0.0.1:{port}/api/v1/lab/capabilities")).send().await.unwrap().status().as_u16(), 404);
    let _ = off.kill();
    let _ = off.wait();
    let port2 = free_port();
    let mut on = spawn_gateway_proc(&[
        ("PHANTOM_BIND_ADDR", format!("127.0.0.1:{port2}")),
        ("PHANTOM_LAB_ENABLED", "true".into()),
        ("PHANTOM_LAB_DIR", dir.to_string_lossy().to_string()),
        ("PHANTOM_LAB_GATEWAY_EXE", gateway_exe().to_string_lossy().to_string()),
    ]);
    assert!(wait_listening(port2, &mut on).await);
    let r = c.get(format!("http://127.0.0.1:{port2}/api/v1/lab/capabilities")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let v: Value = r.json().await.unwrap();
    assert_eq!(v["kinds"]["scenario_b"]["available"], true);
    assert!(dir.join("runs").is_dir());
    let (s, _) = raw_req(format!("127.0.0.1:{port2}").parse().unwrap(), "GET", "/api/v1/lab/capabilities", &[("Host", "evil.example")], "").await;
    assert!(s == 421 || s == 403, "the real binary refuses a foreign Host: {s}");
    let _ = on.kill();
    let _ = on.wait();
}

#[tokio::test]
async fn wiring_anonymous_lab_on_a_network_bind_is_a_config_error_and_the_sealed_salt_is_refused() {
    let mut bad = spawn_gateway_proc(&[("PHANTOM_BIND_ADDR", "0.0.0.0:18085".into()), ("PHANTOM_LAB_ENABLED", "true".into())]);
    let end = Instant::now() + Duration::from_secs(30);
    let code = loop {
        if let Some(st) = bad.try_wait().unwrap() {
            break st.code();
        }
        assert!(Instant::now() < end, "gateway kept running");
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(code, Some(2), "configuration error exit code");
    let mut salt = spawn_gateway_proc(&[
        ("PHANTOM_BIND_ADDR", format!("127.0.0.1:{}", free_port())),
        ("PHANTOM_LAB_ENABLED", "true".into()),
        ("PHANTOM_SEALED_SALT_HEX", "00112233445566778899aabbccddeeff".into()),
    ]);
    let end = Instant::now() + Duration::from_secs(30);
    let code = loop {
        if let Some(st) = salt.try_wait().unwrap() {
            break st.code();
        }
        if Instant::now() > end {
            let _ = salt.kill();
            panic!("gateway with the sealed salt in its environment kept running");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };
    assert_eq!(code, Some(2), "refused as a configuration error");
}

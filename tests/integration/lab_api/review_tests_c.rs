//! G2c: tests for the second review round (M-1/M-2 live in review_tests_b.rs).

use super::review_tests::raw_req;
use super::*;
fn long_t1() -> Value {
    json!({"kind": "t1_matrix", "spec": {
        "scenarios": runner::list_scenarios()[..3].to_vec(), "families": runner::list_families()[..2].to_vec(),
        "arms": runner::list_arms(), "seed_set": "T", "n_seeds": 30, "profile": "smoke"}})
}

async fn first_progress(l: &Lab, id: &str) {
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

async fn done_run(l: &Lab) -> String {
    let (_, v) = l.post(RUNS, None, t1_req(1, 1)).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    l.wait_state(&id, None, &["completed"], 120).await;
    id
}

// ---- (a) SSE permits are released when the streams are dropped
#[tokio::test]
async fn sse_permits_are_released_after_the_held_streams_drop() {
    let l = lab(false).await;
    let (_, v) = l.post(RUNS, None, long_t1()).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    first_progress(&l, &id).await;
    let url = l.url(&format!("{RUNS}/{id}/events"));
    let mut held = Vec::new();
    for _ in 0..16 {
        let r = l.c.get(&url).send().await.unwrap();
        assert_eq!(r.status().as_u16(), 200);
        held.push(r);
    }
    assert_eq!(l.c.get(&url).send().await.unwrap().status().as_u16(), 429);
    drop(held);
    let end = Instant::now() + Duration::from_secs(20);
    loop {
        let r = l.c.get(&url).send().await.unwrap();
        if r.status().as_u16() == 200 {
            break;
        }
        assert_eq!(r.status().as_u16(), 429);
        assert!(Instant::now() < end, "permits were not released after the streams were dropped");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    // and a full set of 16 can be opened again
    let mut again = Vec::new();
    let end = Instant::now() + Duration::from_secs(20);
    while again.len() < 16 {
        let r = l.c.get(&url).send().await.unwrap();
        if r.status().as_u16() == 200 {
            again.push(r);
        } else {
            assert!(Instant::now() < end, "only {} streams could be re-opened", again.len());
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
    drop(again);
    let _ = l.post(&format!("{RUNS}/{id}/cancel"), None, json!({})).await;
    l.wait_state(&id, None, &["cancelled", "completed"], 120).await;
}

// ---- (d) Host / Origin guard on every route family
#[tokio::test]
async fn host_guard_covers_sse_and_artifacts_trailing_dot_duplicate_host_and_https_origin() {
    let l = lab_with_bind().await;
    let port = l.addr.port();
    let id = done_run(&l).await;
    let good = format!("127.0.0.1:{port}");
    for target in [format!("{RUNS}/{id}/events"), format!("{RUNS}/{id}/artifacts"), format!("{RUNS}/{id}/artifacts/cells.csv"), format!("{RUNS}/{id}"), "/api/v1/lab/evidence".to_string()] {
        let (s, _) = raw_req(l.addr, "GET", &target, &[("Host", &good)], "").await;
        assert_eq!(s, 200, "good host {target}");
        for bad in ["evil.example", &format!("evil.example:{port}"), &format!("localhost.:{port}"), &format!("127.0.0.1.:{port}"), &format!("127.0.0.1:{}", port.wrapping_add(1))] {
            let (s, _) = raw_req(l.addr, "GET", &target, &[("Host", bad)], "").await;
            assert_eq!(s, 421, "Host {bad} on {target}");
        }
        // duplicate Host headers are refused even when both values are allowed
        let (s, _) = raw_req(l.addr, "GET", &target, &[("Host", &good), ("Host", &format!("localhost:{port}"))], "").await;
        assert_eq!(s, 421, "duplicate Host on {target}");
        // https Origin is refused on a plain-http server, as is any foreign origin
        for o in [format!("https://127.0.0.1:{port}"), "https://evil.example".to_string(), format!("http://127.0.0.1.evil.example:{port}")] {
            let (s, _) = raw_req(l.addr, "GET", &target, &[("Host", &good), ("Origin", &o)], "").await;
            assert_eq!(s, 403, "Origin {o} on {target}");
        }
        let (s, _) = raw_req(l.addr, "GET", &target, &[("Host", &good), ("Origin", &format!("http://127.0.0.1:{port}"))], "").await;
        assert_eq!(s, 200, "same origin on {target}");
    }
}

async fn lab_with_bind() -> Lab {
    let root = tmp_root("hb");
    let svc = LabService::new(LabConfig::new(root.clone(), None)).unwrap();
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    svc.set_bind_addr(addr);
    let app = create_app_with_lab(api_state(false), &CorsConfig::same_origin(), Some(svc));
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    Lab { base: format!("http://{addr}"), addr, c: reqwest::Client::new(), root }
}

// ---- (b) replacement of the vacuous symlink test: no temporary files are left behind
#[tokio::test]
async fn no_temporary_files_are_left_in_the_run_directory() {
    // Justification for removing the old `run.json.tmp` symlink test: temp names are now
    // `run.json.<random 32 bit>.tmp` opened with create_new, so a name cannot be pre-planted in practice;
    // what can be tested is that every write cleans up after itself.
    let l = lab(false).await;
    let id = done_run(&l).await;
    let names: Vec<String> = std::fs::read_dir(l.root.join("runs").join(&id)).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    assert!(names.iter().all(|n| !n.ends_with(".tmp")), "{names:?}");
}

// ---- L-1: scrubbing with spaces, UNC paths and object keys
#[test]
fn l1_scrub_removes_prefixes_with_spaces_unc_paths_and_keys() {
    use phantom_cache_demo::api::server::lab_api::scrub_path_text;
    let prefixes = vec!["C:\\Lab Root With Space\\runs".to_string()];
    let t = scrub_path_text("C:\\Lab Root With Space\\runs\\20261030T000000Z-aaaaaaaa\\t1", &prefixes);
    assert!(!t.contains("Lab Root") && !t.contains("Space"), "{t}");
    assert!(!scrub_path_text("failed at \\\\fileserver\\share\\lab\\out.csv now", &prefixes).contains("fileserver"));
    assert!(!scrub_path_text("\\\\?\\C:\\Users\\Some One\\x.exe", &prefixes).contains("Some One"), "whole-string path");
    assert!(!scrub_path_text("D:\\Other Dir\\thing.exe", &prefixes).contains("Other"), "whole-string path with a space");
    assert_eq!(scrub_path_text("cargo run --release", &prefixes), "cargo run --release");
    assert!(!scrub_path_text("see (`D:\\Temp\\sb_pilot1`, same seed)", &prefixes).contains("sb_pilot1"));
}

#[tokio::test]
async fn l1_served_json_scrubs_a_root_with_a_space_unc_paths_and_object_keys() {
    let root = std::env::temp_dir().join(format!("g2 space dir {}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let svc = LabService::new(LabConfig::new(root.clone(), None)).unwrap();
    let l = serve(create_app_with_lab(api_state(false), &CorsConfig::same_origin(), Some(svc)), root.clone()).await;
    let id = done_run(&l).await;
    // plant path-bearing content into the manifest that is served
    let manifest = root.join("runs").join(&id).join("t1").join("manifest.json");
    let planted = json!({"out_dir": format!("{}\\runs\\x", root.display()), "unc": "\\\\fileserver\\share\\secret dir\\f.csv", "note": "C:\\Users\\Some One\\a b\\c.txt end",
                         format!("{}\\key", root.display()): 1});
    std::fs::write(&manifest, planted.to_string()).unwrap();
    let r = l.c.get(l.url(&format!("{RUNS}/{id}/artifacts/manifest.json"))).send().await.unwrap();
    let body = r.text().await.unwrap();
    for needle in ["g2 space dir", "fileserver", "secret dir", "Some One", "a b"] {
        assert!(!body.contains(needle), "{needle} leaked: {body}");
    }
}

// ---- L-2: no event carries a server path
#[tokio::test]
async fn l2_events_never_contain_the_lab_root() {
    let l = lab(false).await;
    let id = done_run(&l).await;
    let log = std::fs::read_to_string(l.root.join("runs").join(&id).join("events.jsonl")).unwrap();
    let root_s = l.root.to_string_lossy().to_string();
    for needle in [root_s.clone(), root_s.replace('\\', "\\\\"), root_s.replace('\\', "/")] {
        assert!(!log.contains(&needle), "events.jsonl contains {needle}");
    }
    // the write-time scrubber is applied to event data: a path in a result would be redacted
    use phantom_cache_demo::api::server::lab_api::scrub_path_text;
    assert_eq!(scrub_path_text(&root_s, std::slice::from_ref(&root_s)), "<server-path>");
}

// ---- L-3: reparse points at directory level, handle-based checks
#[cfg(windows)]
#[tokio::test]
async fn l3_a_junction_in_place_of_the_t1_directory_is_not_served() {
    let l = lab(false).await;
    let id = done_run(&l).await;
    let dir = l.root.join("runs").join(&id);
    let real = dir.join("t1");
    let moved = l.root.join("moved_t1");
    std::fs::rename(&real, &moved).unwrap();
    let out = std::process::Command::new("cmd").args(["/C", "mklink", "/J", &real.to_string_lossy(), &moved.to_string_lossy()]).output().unwrap();
    assert!(out.status.success(), "mklink /J: {}", String::from_utf8_lossy(&out.stdout));
    let (s, _) = l.get(&format!("{RUNS}/{id}/artifacts/cells.csv"), None).await;
    assert_eq!(s, 404, "an artifact behind a junctioned directory must not be served");
    let (_, list) = l.get(&format!("{RUNS}/{id}/artifacts"), None).await;
    assert!(!list.to_string().contains("cells.csv"), "{list}");
    // removing the junction restores normal behaviour (the check is about the reparse point)
    std::fs::remove_dir(&real).unwrap();
    std::fs::rename(&moved, &real).unwrap();
    assert_eq!(l.get(&format!("{RUNS}/{id}/artifacts/cells.csv"), None).await.0, 200);
}

// ---- L-4: handler latency while a run is persisting progress
#[tokio::test]
async fn l4_requests_stay_responsive_while_a_run_persists_progress() {
    let l = lab(false).await;
    let (_, v) = l.post(RUNS, None, long_t1()).await;
    let id = v["run_id"].as_str().unwrap().to_string();
    first_progress(&l, &id).await;
    let mut worst = Duration::ZERO;
    for _ in 0..60 {
        let t = Instant::now();
        let (s, _) = l.get("/api/v1/lab/capabilities", None).await;
        assert_eq!(s, 200);
        let (s, _) = l.get(&format!("{RUNS}/{id}"), None).await;
        assert_eq!(s, 200);
        worst = worst.max(t.elapsed());
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(worst < Duration::from_millis(1500), "worst round trip {worst:?} (locks are only taken on blocking threads)");
    let _ = l.post(&format!("{RUNS}/{id}/cancel"), None, json!({})).await;
    l.wait_state(&id, None, &["cancelled", "completed"], 120).await;
}

// ---- L-5: torn tail
#[tokio::test]
async fn l5_recovery_starts_a_new_line_after_a_torn_tail_and_reads_only_the_tail() {
    let root = tmp_root("torn");
    let d = root.join("runs").join("20261030T000010Z-abcdef01");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("run.json"),
        json!({"run_id": "20261030T000010Z-abcdef01", "kind": "t1_matrix", "state": "running", "tier": "SIMULATED", "spec": {}, "created_utc": "2026-10-30T00:00:00Z",
               "started_utc": "2026-10-30T00:00:01Z", "ended_utc": null, "exit_code": null, "progress": {}}).to_string(),
    )
    .unwrap();
    // a large log (> 64 KiB) whose last line is torn (crash mid-write)
    let mut log = String::new();
    for i in 1..=1500u64 {
        log.push_str(&json!({"id": i, "event": "progress", "ts": "x", "data": {"done": i, "total": 9999, "pad": "p".repeat(40)}}).to_string());
        log.push('\n');
    }
    log.push_str("{\"id\": 1501, \"event\": \"progre");
    std::fs::write(d.join("events.jsonl"), &log).unwrap();
    let _ = LabService::new(LabConfig::new(root.clone(), None)).unwrap();
    let text = std::fs::read_to_string(d.join("events.jsonl")).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let n = lines.len();
    assert!(lines[n - 2].contains("\"state\"") && lines[n - 1].contains("\"done\""), "{:?}", &lines[n - 3..]);
    assert!(lines[n - 3].starts_with("{\"id\": 1501"), "the torn line stays a separate (invalid) line");
    assert_eq!(serde_json::from_str::<Value>(lines[n - 2]).unwrap()["id"], 1501, "ids continue after the last VALID event (1500)");
    assert_eq!(serde_json::from_str::<Value>(lines[n - 1]).unwrap()["id"], 1502);
}

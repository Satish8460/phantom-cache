//! G2b phase B: tests of the new APIs (labels, startup rules, child-process lifetime).

use super::review_tests::Proc;
use super::*;
use phantom_cache_demo::api::server::lab_api::{check_lab_bind, refuse_if_sealed_salt_present};
use phantom_cache_demo::lab_live::scenario_b as sb;
use std::path::Path;

// ---- M5: label rules
#[test]
fn m5_gateway_profile_is_the_directory_directly_above_the_executable() {
    let cases = [
        ("C:/x/target/release/phantom-cache-demo.exe", "release"),
        ("C:/x/target/debug/phantom-cache-demo.exe", "debug"),
        ("C:/x/release/y/target/debug/phantom-cache-demo.exe", "debug"),
        ("C:/x/debug/y/target/release/phantom-cache-demo.exe", "release"),
        ("C:/x/target/x86_64-pc-windows-msvc/release/phantom-cache-demo.exe", "release"),
        ("C:/x/target/release/deps/phantom-cache-demo.exe", "unknown"),
        ("C:/x/target/release/examples/phantom-cache-demo.exe", "unknown"),
        ("C:/release/phantom-cache-demo-copy/phantom-cache-demo.exe", "unknown"),
        ("/usr/local/bin/phantom-cache-demo", "unknown"),
        ("phantom-cache-demo.exe", "unknown"),
        ("C:/x/target/RELEASE/phantom-cache-demo.exe", "release"),
    ];
    for (p, want) in cases {
        assert_eq!(sb::gateway_profile(Path::new(p)), want, "{p}");
    }
}

fn valid_cfg() -> sb::ScenarioBConfig {
    sb::ScenarioBConfig::default_experiment(PathBuf::from("C:/x/target/release/phantom-cache-demo.exe"), PathBuf::from("out"))
}

#[test]
fn m5_valid_label_is_reachable_only_for_release_binaries_enough_reps_and_documented_workload() {
    let (l, r) = sb::label_for_inputs(&valid_cfg(), false);
    assert_eq!((l.as_str(), r.len()), ("VALID", 0), "{r:?}");
    let exploratory = |c: &sb::ScenarioBConfig, driver_debug: bool, needle: &str| {
        let (l, r) = sb::label_for_inputs(c, driver_debug);
        assert_eq!(l, "EXPLORATORY", "{needle}");
        assert!(r.iter().any(|x| x.contains(needle)), "{needle}: {r:?}");
    };
    exploratory(&valid_cfg(), true, "DEBUG");
    let mut c = valid_cfg();
    c.gateway_exe = PathBuf::from("C:/x/release/y/target/debug/phantom-cache-demo.exe");
    exploratory(&c, false, "not release");
    let mut c = valid_cfg();
    c.gateway_exe = PathBuf::from("C:/x/target/release/deps/phantom-cache-demo.exe");
    exploratory(&c, false, "unknown");
    let mut c = valid_cfg();
    c.reps = 9;
    exploratory(&c, false, "repetitions");
    let mut c = valid_cfg();
    c.arms = vec![sb::Arm::StaticCap];
    exploratory(&c, false, "BASE arm missing");
    // the "10 reps, 1 s phases, 1 rps, 10 keys" case from the review
    let mut c = valid_cfg();
    c.phases = sb::Phases { warmup_ms: 1000, healthy_ms: 1000, degraded_ms: 1000, recovery_ms: 1000 };
    c.workload.rate_rps = 1.0;
    c.workload.n_keys = 10;
    sb::finalize_config(&mut c);
    exploratory(&c, false, sb::DEVIATES_REASON);
    // M-2: caps / arm completeness / origin parameters
    let mut c = valid_cfg();
    c.static_cap = 1000;
    exploratory(&c, false, "static_cap 1000 differs from the registered default 24");
    let mut c = valid_cfg();
    c.aimd_min = 1;
    exploratory(&c, false, "aimd_min");
    let mut c = valid_cfg();
    c.aimd_max = 1000;
    exploratory(&c, false, "aimd_max");
    let mut c = valid_cfg();
    c.aimd_p99_ceiling_ms = 61;
    exploratory(&c, false, "aimd p99 ceiling");
    let mut c = valid_cfg();
    c.arms = vec![sb::Arm::Base];
    exploratory(&c, false, "no capped arm");
    let mut c = valid_cfg();
    c.arms = vec![sb::Arm::StaticCap, sb::Arm::AimdCap];
    exploratory(&c, false, "BASE arm missing");
    let mut c = valid_cfg();
    c.arms = vec![sb::Arm::Base, sb::Arm::StaticCap];
    assert_eq!(sb::label_for_inputs(&c, false).0, "VALID", "BASE + one capped arm is enough");
    let mut c = valid_cfg();
    c.origin.base_service_ms = 21;
    exploratory(&c, false, "origin base service");
    let mut c = valid_cfg();
    c.origin.stale_while_revalidate_s = Some(5);
    exploratory(&c, false, "stale-while-revalidate");
    // tolerance boundaries (inclusive)
    let label = |f: &dyn Fn(&mut sb::ScenarioBConfig)| {
        let mut c = valid_cfg();
        f(&mut c);
        sb::finalize_config(&mut c);
        sb::label_for_inputs(&c, false).0
    };
    assert_eq!(label(&|c| c.phases = sb::Phases { warmup_ms: 4000, healthy_ms: 2500, degraded_ms: 5000, recovery_ms: 4000 }), "VALID");
    assert_eq!(label(&|c| c.phases.warmup_ms = 3999), "EXPLORATORY");
    assert_eq!(label(&|c| c.workload.rate_rps = 150.0), "VALID");
    assert_eq!(label(&|c| c.workload.rate_rps = 149.0), "EXPLORATORY");
    assert_eq!(label(&|c| c.workload.rate_rps = 600.0), "VALID");
    assert_eq!(label(&|c| c.workload.rate_rps = 601.0), "EXPLORATORY");
    assert_eq!(label(&|c| c.workload.n_keys = 2500), "VALID");
    assert_eq!(label(&|c| c.workload.n_keys = 10_001), "EXPLORATORY");
    assert_eq!(label(&|c| c.reps = 30), "VALID");
}

// ---- L8: D1 strings pinned through the produced summary
#[test]
fn d1_summary_carries_the_exact_notes_and_wording() {
    let cfg = valid_cfg();
    let meta = sb::SummaryMeta { label: "EXPLORATORY".into(), label_reasons: vec!["x".into()], cancelled: false };
    let summary = sb::summarize(&cfg, &[], &std::collections::BTreeMap::new(), &meta);
    assert_eq!(summary["label_meaning"], sb::LABEL_VALID_MEANING);
    let notes = summary["interpretation_notes"].as_array().unwrap();
    assert_eq!(notes.len(), 7);
    assert_eq!(summary["interpretation_notes"], json!(sb::interpretation_notes()));
    let text = notes.iter().map(|n| n.as_str().unwrap()).collect::<Vec<_>>().join("\n");
    for needle in [
        "may behave as a static cap near aimd_min",
        "shed about 2% of requests even in the healthy phase",
        "Fairness of canary versus off is excluded by code reading only",
        "are NOT evidence of improved latency for served clients",
        "origin-p99 clause is untestable",
        "did not exercise it (the origin sends only max-age=3",
        "stale_served_total = 0",
        "not pre-registered",
        "single-repetition pilot",
    ] {
        assert!(text.contains(needle), "missing: {needle}");
    }
    let lim = summary["limitations"].as_array().unwrap();
    for n in sb::interpretation_notes().iter().take(6) {
        assert!(lim.iter().any(|l| l == n), "limitations must carry: {n}");
    }
    assert!(sb::LABEL_VALID_MEANING.contains("directory directly above") && sb::LABEL_VALID_MEANING.contains("0.5x..2x"));
}

// ---- M7 / bind rules
#[test]
fn m7_sealed_salt_in_the_environment_refuses_the_lab_api() {
    let e = refuse_if_sealed_salt_present(|k| k == "PHANTOM_SEALED_SALT_HEX").unwrap_err();
    assert!(e.contains("PHANTOM_SEALED_SALT_HEX") && e.contains("refusing"), "{e}");
    assert!(refuse_if_sealed_salt_present(|_| false).is_ok());
}

#[test]
fn bind_rule_anonymous_lab_is_loopback_only() {
    let lo: std::net::SocketAddr = "127.0.0.1:8085".parse().unwrap();
    let lo6: std::net::SocketAddr = "[::1]:8085".parse().unwrap();
    let net: std::net::SocketAddr = "0.0.0.0:8085".parse().unwrap();
    assert!(check_lab_bind(false, &lo, true).is_ok());
    assert!(check_lab_bind(false, &lo6, true).is_ok());
    assert!(check_lab_bind(false, &net, true).is_err());
    assert!(check_lab_bind(true, &net, true).is_ok(), "JWT mode may bind a network address");
    assert!(check_lab_bind(false, &net, false).is_ok(), "lab disabled: not this rule's business");
}

// ---- M1 (port part): the Host port must be the bind port once the bind address is known
#[tokio::test]
async fn m1_host_port_must_match_the_recorded_bind_address() {
    let l = lab_bound().await;
    let p = l.addr.port();
    let (s, _) = super::review_tests::raw_req(l.addr, "GET", "/api/v1/lab/capabilities", &[("Host", &format!("127.0.0.1:{p}"))], "").await;
    assert_eq!(s, 200);
    for h in [format!("127.0.0.1:{}", p.wrapping_add(1)), "127.0.0.1".to_string(), format!("localhost:{}", p.wrapping_add(1))] {
        let (s, _) = super::review_tests::raw_req(l.addr, "GET", "/api/v1/lab/capabilities", &[("Host", &h)], "").await;
        assert_eq!(s, 421, "Host {h}");
    }
}

async fn lab_bound() -> Lab {
    let root = tmp_root("bound");
    let svc = LabService::new(LabConfig::new(root.clone(), None)).unwrap();
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    svc.set_bind_addr(addr);
    let app = create_app_with_lab(api_state(false), &CorsConfig::same_origin(), Some(svc));
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    Lab { base: format!("http://{addr}"), addr, c: reqwest::Client::new(), root }
}

// ---- M4: reaping leftover children and the job object (Windows)
#[cfg(windows)]
mod windows_children {
    use super::*;
    use phantom_cache_demo::api::server::lab_api::{bind_children_to_process_lifetime, process_creation_filetime, reap_leftover_gateway, ReapOutcome};
    use phantom_cache_demo::lab_live::gateway::pid_alive;

    fn ping_exe() -> PathBuf {
        PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into())).join("System32").join("PING.EXE")
    }
    fn sleeper(exe: &Path, secs: u32) -> Proc {
        Proc(
            std::process::Command::new(exe)
                .args(["-n", &secs.to_string(), "127.0.0.1"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        )
    }
    fn plant(root: &Path, id: &str, pid: u32, started: &str, ft: Option<u64>) {
        let d = root.join("runs").join(id);
        std::fs::create_dir_all(&d).unwrap();
        let mut v = json!({"run_id": id, "kind": "scenario_b", "state": "running", "tier": "MEASURED_LOOPBACK", "spec": {}, "created_utc": started,
                   "started_utc": started, "ended_utc": null, "exit_code": null, "progress": {}, "gateway_pid": pid});
        if let Some(ft) = ft {
            v["gateway_created_ft"] = json!(ft.to_string());
        }
        std::fs::write(d.join("run.json"), v.to_string()).unwrap();
        std::fs::write(d.join("events.jsonl"), json!({"id": 1, "event": "state", "ts": "x", "data": {"state": "running"}}).to_string() + "\n").unwrap();
    }
    fn ft(p: &Proc) -> u64 {
        process_creation_filetime(p.id()).expect("creation time of a live child")
    }
    fn gone(p: &mut Proc) -> bool {
        let end = Instant::now() + Duration::from_secs(10);
        while Instant::now() < end {
            if p.try_wait().unwrap().is_some() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        false
    }
    fn alive(p: &mut Proc) -> bool {
        std::thread::sleep(Duration::from_millis(300));
        p.try_wait().unwrap().is_none()
    }
    fn rec(root: &Path, id: &str) -> Value {
        serde_json::from_slice(&std::fs::read(root.join("runs").join(id).join("run.json")).unwrap()).unwrap()
    }

    #[test]
    fn recovery_reaps_only_the_exact_recorded_process_with_the_configured_image() {
        let root = tmp_root("reap");
        let fake = root.join("fake-gateway.exe");
        std::fs::copy(ping_exe(), &fake).unwrap();
        let mut leftover = sleeper(&fake, 120); // recorded child: image == configured exe, ft recorded
        let mut sibling = sleeper(&fake, 120); // same image, a DIFFERENT process (e.g. a sibling gateway)
        let mut other_image = sleeper(&ping_exe(), 120); // different image path
        let mut recycled = sleeper(&fake, 120); // same pid recorded, but with another process's creation time
        let mut no_ft = sleeper(&fake, 120); // no creation time recorded
        let mut early = sleeper(&fake, 120); // exact ft but the run claims to have started in 2099
        std::thread::sleep(Duration::from_millis(500));
        let (a, b, c, d, e, f) = ("20261030T000001Z-aaaaaaaa", "20261030T000002Z-bbbbbbbb", "20261030T000003Z-cccccccc", "20261030T000004Z-dddddddd", "20261030T000005Z-eeeeeeee", "20261030T000006Z-ffffffff");
        plant(&root, a, leftover.id(), "2020-01-01T00:00:00Z", Some(ft(&leftover)));
        plant(&root, b, other_image.id(), "2020-01-01T00:00:00Z", Some(ft(&other_image)));
        plant(&root, c, recycled.id(), "2020-01-01T00:00:00Z", Some(ft(&leftover))); // recycled-pid simulation
        plant(&root, d, no_ft.id(), "2020-01-01T00:00:00Z", None);
        plant(&root, e, early.id(), "2099-01-01T00:00:00Z", Some(ft(&early)));
        let _ = &mut sibling; // `sibling` is deliberately not recorded in any run
        let _ = LabService::new(LabConfig::new(root.clone(), Some(fake.clone()))).unwrap();
        assert!(gone(&mut leftover), "the recorded leftover gateway child must be terminated");
        assert!(alive(&mut sibling), "a same-image process that was not recorded must not be touched");
        assert!(alive(&mut other_image), "a different image must not be touched");
        assert!(alive(&mut recycled), "same pid with another creation time (recycled pid) must not be touched");
        assert!(alive(&mut no_ft), "without a recorded creation time nothing is terminated");
        assert!(alive(&mut early), "a start time that does not fit the run must not be touched");
        assert_eq!(rec(&root, a)["recovery"]["gateway_reap"], "terminated");
        assert_eq!(rec(&root, b)["recovery"]["gateway_reap"], "image_mismatch_not_touched");
        assert_eq!(rec(&root, c)["recovery"]["gateway_reap"], "start_time_mismatch_not_touched");
        assert_eq!(rec(&root, d)["recovery"]["gateway_reap"], "no_creation_time_recorded_not_touched");
        assert_eq!(rec(&root, e)["recovery"]["gateway_reap"], "start_time_mismatch_not_touched");
        for id in [a, b, c, d, e] {
            assert_eq!(rec(&root, id)["state"], "interrupted");
        }
        let _ = f;
        assert_eq!(reap_leftover_gateway(u32::MAX - 1, &fake, 0, Some(1)), ReapOutcome::NotRunning);
    }

    #[test]
    fn never_terminates_itself_or_an_ancestor_and_reports_access_denied_distinctly() {
        let me = std::process::id();
        let exe = std::env::current_exe().unwrap();
        let own_ft = process_creation_filetime(me).unwrap();
        assert_eq!(reap_leftover_gateway(me, &exe, 0, Some(own_ft)), ReapOutcome::SelfOrAncestor, "own pid, correct image and creation time");
        let chain = phantom_cache_demo::api::server::lab_api::process_ancestors();
        assert_eq!(chain[0], me);
        assert!(chain.len() >= 2, "a test process has a parent: {chain:?}");
        // wrong creation time: even a buggy implementation could not kill the parent here
        assert_eq!(reap_leftover_gateway(chain[1], &exe, 0, Some(0)), ReapOutcome::SelfOrAncestor);
        // the System process (pid 4) cannot be opened for termination: access denied, not "not running"
        assert_eq!(reap_leftover_gateway(4, &exe, 0, Some(1)), ReapOutcome::AccessDenied);
        // planted run whose gateway_pid is this very process: recovery must not touch it
        let root = tmp_root("selfreap");
        plant(&root, "20261030T000007Z-99999999", me, "2020-01-01T00:00:00Z", Some(own_ft));
        let _ = LabService::new(LabConfig::new(root.clone(), Some(exe))).unwrap();
        assert_eq!(rec(&root, "20261030T000007Z-99999999")["recovery"]["gateway_reap"], "self_or_ancestor_not_touched");
    }

    fn short_path(p: &Path) -> String {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("cmd").raw_arg(format!("/C for %I in (\"{}\") do @echo %~sI", p.display())).output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    #[test]
    fn image_path_forms_upper_case_extended_prefix_and_8_3_are_all_recognised() {
        let root = tmp_root("reapforms");
        let dir = root.join("A Long Directory Name");
        std::fs::create_dir_all(&dir).unwrap();
        let fake = dir.join("fake-gateway.exe");
        std::fs::copy(ping_exe(), &fake).unwrap();
        let forms: Vec<(&str, String)> = vec![
            ("upper", fake.to_string_lossy().to_ascii_uppercase()),
            ("extended", format!("\\\\?\\{}", fake.display())),
            ("forward slashes", fake.to_string_lossy().replace('\\', "/")),
            ("8.3", short_path(&fake)),
        ];
        for (name, form) in forms {
            let mut p = sleeper(&fake, 120);
            std::thread::sleep(Duration::from_millis(300));
            let want = ft(&p);
            let out = reap_leftover_gateway(p.id(), Path::new(&form), 0, Some(want));
            assert_eq!(out, ReapOutcome::Terminated, "form {name}: {form}");
            assert!(gone(&mut p), "form {name}");
        }
    }

    /// Helper executed in a CHILD copy of this test binary (see `job_object_kills_children...`).
    #[test]
    fn job_helper_child() {
        let mode = std::env::var("G2_JOB_HELPER").unwrap_or_default();
        if mode.is_empty() {
            return;
        }
        if mode == "bind" {
            bind_children_to_process_lifetime().expect("job object");
        }
        let child = std::process::Command::new(ping_exe())
            .args(["-n", "9", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        println!("CHILD_PID={}", child.id());
        use std::io::Write;
        let _ = std::io::stdout().flush();
        std::process::exit(0);
    }

    fn run_helper(mode: &str) -> u32 {
        use std::io::BufRead;
        let mut p = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["lab_api_tests::review_tests_b::windows_children::job_helper_child", "--exact", "--nocapture", "--test-threads", "1"])
            .env("G2_JOB_HELPER", mode)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let out = p.stdout.take().unwrap();
        let mut pid = 0;
        for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(i) = line.find("CHILD_PID=") {
                pid = line[i + 10..].trim().parse().unwrap();
                break;
            }
        }
        let _ = p.wait();
        std::thread::sleep(Duration::from_millis(1500));
        pid
    }

    #[test]
    fn job_object_kills_children_when_the_parent_process_exits() {
        let control = run_helper("nobind");
        assert!(control != 0);
        assert!(pid_alive(control), "control: without the job object the child outlives its parent");
        let bound = run_helper("bind");
        assert!(bound != 0);
        assert!(!pid_alive(bound), "with the job object the child dies with its parent");
    }
}

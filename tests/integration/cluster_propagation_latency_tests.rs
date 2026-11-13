//! Cross-gateway invalidation propagation latency MEASUREMENT (not a pass/fail on 50 ms).
//!
//! Purpose: claim C6 states P(stale)=0 after 50 ms, but the existing test only sleeps 50 ms.
//! This file measures, per trial, the time from the DELETE response on gateway 1 until each of
//! gateway 2 and gateway 3 stops answering `X-Cache-Status: HIT` for that key.
//!
//! Method (per trial, distinct key per trial):
//!   1. warm gateways 2 and 3 (MISS then HIT, version 1);
//!   2. origin commits version 2;
//!   3. DELETE on gateway 1; t0 = instant the DELETE response is received;
//!   4. poll GET on gateways 2 and 3 concurrently (1 ms pause between polls, 2 s hard timeout);
//!      the first non-HIT observation ends the trial for that destination. latency = response
//!      receive time - t0 (upper bound; includes at most one GET round trip).
//!      The poll GET that observes the first MISS refills that gateway's cache for that key only;
//!      keys are unique per trial so later trials are not distorted.
//!
//! Only sanity invariants are asserted (every trial observes invalidation within 2 s; version
//! served after invalidation is 2; no orphan gateway processes). The 50 ms figure is REPORTED,
//! not asserted. Run with `--nocapture` to see the distribution. Per-trial CSV is written to
//! `%TEMP%/cpl_trials_<tag>.csv`.
//!
//! Evidence label: loopback, single host, mTLS cluster with fault proxies in Normal mode,
//! profile = whatever `cargo test` uses (debug by default; see printed profile).

#[path = "../support/fault_proxy.rs"]
mod fault_proxy;

#[path = "../support/cluster_harness.rs"]
mod cluster_harness;

use cluster_harness::ClusterHarness;
use reqwest::StatusCode;
use std::time::{Duration, Instant};

const POLL_TIMEOUT: Duration = Duration::from_secs(2);
const POLL_PAUSE: Duration = Duration::from_millis(1);

#[derive(Clone, Debug)]
struct Observation {
    trial: usize,
    dest: u8,
    /// Upper bound: first non-HIT response receive time - t0. None if never observed in timeout.
    /// NOTE: a MISS response includes the gateway's origin fetch, so this over-states propagation.
    latency_us: Option<u64>,
    /// Send time (relative to t0) of the request that first observed a non-HIT. The invalidation
    /// had been applied before the gateway served this request: tightest practical estimate.
    miss_send_us: Option<u64>,
    /// Send time of the last request that still answered HIT (None if no stale HIT was seen).
    last_hit_send_us: Option<u64>,
    polls: u32,
    /// version in the first non-HIT response
    version_after: Option<u64>,
    /// cache status of the first non-HIT response
    status_after: Option<String>,
    stale_hit_polls: u32,
}

async fn poll_until_not_hit(
    harness: &ClusterHarness,
    dest: u8,
    key: &str,
    t0: Instant,
    trial: usize,
) -> Observation {
    let mut polls = 0u32;
    let mut stale_hits = 0u32;
    let mut last_hit_send_us: Option<u64> = None;
    loop {
        if t0.elapsed() > POLL_TIMEOUT {
            return Observation {
                trial,
                dest,
                latency_us: None,
                miss_send_us: None,
                last_hit_send_us,
                polls,
                version_after: None,
                status_after: None,
                stale_hit_polls: stale_hits,
            };
        }
        polls += 1;
        let send_us = t0.elapsed().as_micros() as u64;
        match harness.http_get(dest, key).await {
            Ok((s, _b, cache, ver)) => {
                let lat = t0.elapsed();
                if s == StatusCode::OK && cache.as_deref() == Some("HIT") {
                    stale_hits += 1;
                    last_hit_send_us = Some(send_us);
                    tokio::time::sleep(POLL_PAUSE).await;
                    continue;
                }
                return Observation {
                    trial,
                    dest,
                    latency_us: Some(lat.as_micros() as u64),
                    miss_send_us: Some(send_us),
                    last_hit_send_us,
                    polls,
                    version_after: ver,
                    status_after: cache,
                    stale_hit_polls: stale_hits,
                };
            }
            Err(_) => {
                tokio::time::sleep(POLL_PAUSE).await;
            }
        }
    }
}

fn pct(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

fn summarize(label: &str, vals_us: &[u64]) {
    let mut v = vals_us.to_vec();
    v.sort_unstable();
    let over50 = v.iter().filter(|&&x| x > 50_000).count();
    let over25 = v.iter().filter(|&&x| x > 25_000).count();
    let over10 = v.iter().filter(|&&x| x > 10_000).count();
    println!(
        "[CPL] {label:<22} N={:<5} P50={:>8.2}ms P90={:>8.2}ms P99={:>8.2}ms max={:>8.2}ms  >10ms={} >25ms={} >50ms={}",
        v.len(),
        pct(&v, 50.0) as f64 / 1000.0,
        pct(&v, 90.0) as f64 / 1000.0,
        pct(&v, 99.0) as f64 / 1000.0,
        v.last().copied().unwrap_or(0) as f64 / 1000.0,
        over10,
        over25,
        over50
    );
}

fn machine_context() {
    let profile = if cfg!(debug_assertions) { "debug (cfg!(debug_assertions)=true)" } else { "release-like (debug_assertions off)" };
    println!("[CPL] profile={profile}");
    println!(
        "[CPL] NUMBER_OF_PROCESSORS={}",
        std::env::var("NUMBER_OF_PROCESSORS").unwrap_or_else(|_| "?".into())
    );
    #[cfg(windows)]
    {
        if let Ok(out) = std::process::Command::new("tasklist").args(["/FO", "CSV", "/NH"]).output() {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            let count = |needle: &str| text.lines().filter(|l| l.contains(needle)).count();
            println!(
                "[CPL] load at start: cargo={} rustc={} phantom-cache-demo={} link/cl={} total_processes={}",
                count("\"cargo.exe\""),
                count("\"rustc.exe\""),
                count("\"phantom-cache-demo.exe\""),
                count("\"link.exe\"") + count("\"cl.exe\""),
                text.lines().count()
            );
        }
    }
}

async fn run_trials(n: usize, tag: &str) {
    machine_context();
    let mut harness = ClusterHarness::start(&[1, 2, 3], true)
        .await
        .expect("Failed to start cluster harness");

    let mut pids = Vec::new();
    for gw in harness.gateways.values() {
        if let Some(pid) = gw.child.id() {
            pids.push(pid);
        }
    }

    let mut obs: Vec<Observation> = Vec::with_capacity(n * 2);
    let mut delete_ms: Vec<u64> = Vec::with_capacity(n);
    let run_start = Instant::now();

    for t in 0..n {
        let key = format!("/cpl_{}_{}", tag, t);

        // Warm peers (MISS then HIT, v1)
        for dest in [2u8, 3u8] {
            let (s, _, c, v) = harness.http_get(dest, &key).await.unwrap();
            assert_eq!(s, StatusCode::OK);
            assert_eq!(c.as_deref(), Some("MISS"), "warmup trial {t} dest {dest}");
            assert_eq!(v, Some(1));
            let (_, _, c, v) = harness.http_get(dest, &key).await.unwrap();
            assert_eq!(c.as_deref(), Some("HIT"), "warm HIT trial {t} dest {dest}");
            assert_eq!(v, Some(1));
        }

        assert_eq!(harness.origin().mutate_key(&key, &format!("v2_{t}")), 2);

        let d_start = Instant::now();
        let st = harness.http_delete(1, &key).await.unwrap();
        let t0 = Instant::now();
        delete_ms.push(t0.duration_since(d_start).as_micros() as u64);
        assert_eq!(st, StatusCode::NO_CONTENT);

        let (o2, o3) = tokio::join!(
            poll_until_not_hit(&harness, 2, &key, t0, t),
            poll_until_not_hit(&harness, 3, &key, t0, t)
        );
        obs.push(o2);
        obs.push(o3);
    }
    let total = run_start.elapsed();

    // CSV for offline analysis
    let csv_path = std::env::temp_dir().join(format!("cpl_trials_{tag}.csv"));
    {
        let mut s = String::from("trial,dest,latency_us,miss_send_us,last_hit_send_us,polls,stale_hit_polls,status_after,version_after\n");
        for o in &obs {
            s.push_str(&format!(
                "{},{},{},{},{},{},{},{},{}\n",
                o.trial,
                o.dest,
                o.latency_us.map(|x| x.to_string()).unwrap_or_else(|| "TIMEOUT".into()),
                o.miss_send_us.map(|x| x.to_string()).unwrap_or_default(),
                o.last_hit_send_us.map(|x| x.to_string()).unwrap_or_default(),
                o.polls,
                o.stale_hit_polls,
                o.status_after.clone().unwrap_or_default(),
                o.version_after.map(|x| x.to_string()).unwrap_or_default()
            ));
        }
        let _ = std::fs::write(&csv_path, s);
    }

    // ---- Report ----
    println!("[CPL] ===== propagation latency report tag={tag} trials={n} wall={:.1}s =====", total.as_secs_f64());
    println!("[CPL] csv: {}", csv_path.display());
    let timeouts = obs.iter().filter(|o| o.latency_us.is_none()).count();
    let wrong_version = obs
        .iter()
        .filter(|o| o.latency_us.is_some() && o.version_after != Some(2))
        .count();
    let stale_obs = obs.iter().filter(|o| o.stale_hit_polls > 0).count();
    println!(
        "[CPL] timeouts(>2s)={timeouts} wrong_version_after_invalidation={wrong_version} observations_with_>=1_stale_HIT_poll={stale_obs} of {}",
        obs.len()
    );

    let all: Vec<u64> = obs.iter().filter_map(|o| o.latency_us).collect();
    println!("[CPL] --- (A) UPPER BOUND: first non-HIT response received - t0 (includes MISS origin fetch) ---");
    summarize("A: ALL dests", &all);
    let sends: Vec<u64> = obs.iter().filter_map(|o| o.miss_send_us).collect();
    println!("[CPL] --- (B) TIGHT ESTIMATE: send time of the request that first saw non-HIT - t0 ---");
    summarize("B: ALL dests", &sends);
    for dest in [2u8, 3u8] {
        let v: Vec<u64> = obs.iter().filter(|o| o.dest == dest).filter_map(|o| o.miss_send_us).collect();
        summarize(&format!("B: dest {dest}"), &v);
    }
    let fetch_cost: Vec<u64> = obs
        .iter()
        .filter_map(|o| Some(o.latency_us?.saturating_sub(o.miss_send_us?)))
        .collect();
    summarize("MISS req duration", &fetch_cost);
    let lh: Vec<u64> = obs.iter().filter_map(|o| o.last_hit_send_us).collect();
    summarize("last stale-HIT send", &lh);
    println!("[CPL] --- (A) per destination / cold-vs-warm ---");
    for dest in [2u8, 3u8] {
        let v: Vec<u64> = obs.iter().filter(|o| o.dest == dest).filter_map(|o| o.latency_us).collect();
        summarize(&format!("dest {dest}"), &v);
    }
    // Cold (trial 0: first invalidation to each destination, cold pooled connection) vs warm
    let cold: Vec<&Observation> = obs.iter().filter(|o| o.trial == 0).collect();
    for o in &cold {
        println!(
            "[CPL] COLD trial0 dest {}: latency={:?} us polls={} stale_hit_polls={}",
            o.dest, o.latency_us, o.polls, o.stale_hit_polls
        );
    }
    let first5: Vec<u64> = obs.iter().filter(|o| o.trial < 5).filter_map(|o| o.latency_us).collect();
    let warm: Vec<u64> = obs.iter().filter(|o| o.trial >= 5).filter_map(|o| o.latency_us).collect();
    summarize("A: trials 0..5", &first5);
    summarize("A: trials 5.. (warm)", &warm);
    let first5b: Vec<u64> = obs.iter().filter(|o| o.trial < 5).filter_map(|o| o.miss_send_us).collect();
    let warmb: Vec<u64> = obs.iter().filter(|o| o.trial >= 5).filter_map(|o| o.miss_send_us).collect();
    summarize("B: trials 0..5", &first5b);
    summarize("B: trials 5.. (warm)", &warmb);
    // Observations whose very first poll already saw non-HIT (invalidation applied before DELETE returned)
    let imm = obs.iter().filter(|o| o.stale_hit_polls == 0 && o.latency_us.is_some()).count();
    println!("[CPL] observations with zero stale-HIT polls (already invalidated when DELETE returned): {imm}/{}", obs.len());
    let slow_b = obs.iter().filter(|o| o.miss_send_us.is_none_or(|x| x > 50_000)).count();
    println!("[CPL] (B) observations with propagation estimate > 50ms: {slow_b}");
    let slow: Vec<&Observation> = obs.iter().filter(|o| o.latency_us.is_none_or(|x| x > 50_000)).collect();
    for o in &slow {
        println!(
            "[CPL] >50ms (A): trial={} dest={} latency_us={:?} miss_send_us={:?} polls={} stale_hit_polls={}",
            o.trial, o.dest, o.latency_us, o.miss_send_us, o.polls, o.stale_hit_polls
        );
    }
    let mut d = delete_ms.clone();
    d.sort_unstable();
    println!(
        "[CPL] DELETE response time (gateway 1 incl. sync primary round trip): P50={:.2}ms P99={:.2}ms max={:.2}ms",
        pct(&d, 50.0) as f64 / 1000.0,
        pct(&d, 99.0) as f64 / 1000.0,
        d.last().copied().unwrap_or(0) as f64 / 1000.0
    );

    // ---- Teardown + orphan check ----
    harness.shutdown();
    tokio::time::sleep(Duration::from_millis(400)).await;
    #[cfg(windows)]
    for &pid in &pids {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/FO", "CSV", "/NH"])
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            !text.contains(&format!("\"{}\"", pid)),
            "orphan gateway PID {pid} still alive after shutdown"
        );
    }
    let _ = &pids;

    // ---- Sanity invariants only ----
    assert_eq!(timeouts, 0, "every trial must observe invalidation within {POLL_TIMEOUT:?}");
    assert_eq!(wrong_version, 0, "post-invalidation response must carry origin version 2");
}

/// Default measurement run: N = 200 trials x 2 destinations.
#[tokio::test]
async fn measure_invalidation_propagation_latency_n200() {
    run_trials(200, "n200").await;
}

/// Larger run: N = 1000 trials. `cargo test --test cluster_propagation_latency_tests -j 1 -- --ignored --nocapture`
#[tokio::test]
#[ignore = "long-running measurement (N=1000)"]
async fn measure_invalidation_propagation_latency_n1000() {
    run_trials(1000, "n1000").await;
}

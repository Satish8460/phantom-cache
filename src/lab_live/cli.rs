//! `phantom-lab scenario-b` command line (kept in the library so it is testable).

use super::loadgen::ArrivalProcess;
use super::scenario_b as sb;
use crate::lab::eval::utc_now_iso;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const HELP: &str = "phantom-lab scenario-b: Scenario B (origin slowdown), tier MEASURED_LOOPBACK (real gateway process, loopback, real time)

USAGE  phantom-lab scenario-b [options]
  --gateway PATH       phantom-cache-demo binary (default: next to this executable; use the RELEASE build for reported numbers)
  --out DIR            experiment directory (default lab-results/scenario_b/<utc>)
  --reps N             repetitions per arm (default 10; < 10 is labelled EXPLORATORY)
  --arms LIST          BASE,STATIC-CAP,AIMD-CAP (default all three)
  --seed N             base seed (repetition r uses N+r)
  --smoke MS           1 repetition, all four phases MS long (mechanics check, EXPLORATORY)
  --warmup-ms / --healthy-ms / --degraded-ms / --recovery-ms N   phase lengths (default 8000/5000/10000/8000)
  --rate RPS           mean arrival rate (default 300)   --keys N (5000)   --alpha A (Zipf, 0.9)   --regular (fixed gaps instead of Poisson)
  --base-service-ms N  origin healthy service time (20)  --mult K (6)   --error-rate P (0)
  --max-age S          origin Cache-Control max-age (3)  --swr S / --sie S  add stale-while-revalidate / stale-if-error
  --body-bytes N (1024)   --cache-bytes N gateway PHANTOM_MAX_CACHE_BYTES (262144)
  --static-cap C (24)   --aimd-min N (12)  --aimd-max N (32)  --aimd-ceiling-ms N (60)  --aimd-window N (30)
  --snapshot-ms N      /api/v1/metrics polling period (250)
Exit codes: 0 completed | 1 run failure (e.g. cap not enforcing) | 2 usage | 130 cancelled (Ctrl-C)
";

const VALUE_FLAGS: &[&str] = &[
    "--gateway", "--out", "--smoke", "--reps", "--arms", "--seed", "--warmup-ms", "--healthy-ms", "--degraded-ms",
    "--recovery-ms", "--rate", "--keys", "--alpha", "--base-service-ms", "--mult", "--error-rate", "--max-age", "--swr",
    "--sie", "--body-bytes", "--cache-bytes", "--static-cap", "--aimd-min", "--aimd-max", "--aimd-ceiling-ms",
    "--aimd-window", "--snapshot-ms",
];

fn num<T: std::str::FromStr>(kv: &BTreeMap<String, String>, k: &str) -> Result<Option<T>, String> {
    match kv.get(k) {
        None => Ok(None),
        Some(v) => v.parse::<T>().map(Some).map_err(|_| format!("{k} expects a number, got '{v}'")),
    }
}

/// Parses the options into a config (no I/O). `Err` = usage error.
pub fn parse_args(args: &[String]) -> Result<sb::ScenarioBConfig, String> {
    let mut kv: BTreeMap<String, String> = BTreeMap::new();
    let mut regular = false;
    let mut i = 0;
    while i < args.len() {
        let f = args[i].as_str();
        if f == "--regular" {
            regular = true;
        } else if VALUE_FLAGS.contains(&f) {
            i += 1;
            let v = args.get(i).ok_or_else(|| format!("{f} needs a value"))?;
            kv.insert(f.to_string(), v.clone());
        } else {
            return Err(format!("unknown option '{f}'"));
        }
        i += 1;
    }
    let gateway: PathBuf = kv.get("--gateway").map(PathBuf::from).unwrap_or_else(sb::default_gateway_exe);
    let out: PathBuf = kv
        .get("--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("lab-results/scenario_b").join(utc_now_iso().replace([':', '-'], "")));
    let mut cfg = match num::<u64>(&kv, "--smoke")? {
        Some(ms) => sb::smoke_config(gateway, out, ms),
        None => sb::ScenarioBConfig::default_experiment(gateway, out),
    };
    if let Some(v) = num::<usize>(&kv, "--reps")? { cfg.reps = v; }
    if let Some(v) = num::<u64>(&kv, "--seed")? { cfg.seed = v; cfg.workload.seed = v; cfg.origin.seed = v; }
    if let Some(v) = num::<u64>(&kv, "--warmup-ms")? { cfg.phases.warmup_ms = v; }
    if let Some(v) = num::<u64>(&kv, "--healthy-ms")? { cfg.phases.healthy_ms = v; }
    if let Some(v) = num::<u64>(&kv, "--degraded-ms")? { cfg.phases.degraded_ms = v; }
    if let Some(v) = num::<u64>(&kv, "--recovery-ms")? { cfg.phases.recovery_ms = v; }
    if let Some(v) = num::<f64>(&kv, "--rate")? { cfg.workload.rate_rps = v; }
    if let Some(v) = num::<usize>(&kv, "--keys")? { cfg.workload.n_keys = v; }
    if let Some(v) = num::<f64>(&kv, "--alpha")? { cfg.workload.zipf_alpha = v; }
    if regular { cfg.workload.arrival = ArrivalProcess::Regular; }
    if let Some(v) = num::<u64>(&kv, "--base-service-ms")? { cfg.origin.base_service_ms = v; }
    if let Some(v) = num::<f64>(&kv, "--mult")? { cfg.origin.degraded_mult = v; }
    if let Some(v) = num::<f64>(&kv, "--error-rate")? { cfg.origin.error_rate = v; }
    if let Some(v) = num::<u32>(&kv, "--max-age")? { cfg.origin.max_age_s = v; }
    if let Some(v) = num::<u32>(&kv, "--swr")? { cfg.origin.stale_while_revalidate_s = Some(v); }
    if let Some(v) = num::<u32>(&kv, "--sie")? { cfg.origin.stale_if_error_s = Some(v); }
    if let Some(v) = num::<usize>(&kv, "--body-bytes")? { cfg.origin.body_bytes = v; }
    if let Some(v) = num::<u64>(&kv, "--cache-bytes")? { cfg.max_cache_bytes = v; }
    if let Some(v) = num::<usize>(&kv, "--static-cap")? { cfg.static_cap = v; }
    if let Some(v) = num::<usize>(&kv, "--aimd-min")? { cfg.aimd_min = v; }
    if let Some(v) = num::<usize>(&kv, "--aimd-max")? { cfg.aimd_max = v; }
    if let Some(v) = num::<u64>(&kv, "--aimd-ceiling-ms")? { cfg.aimd_p99_ceiling_ms = v; }
    if let Some(v) = num::<usize>(&kv, "--aimd-window")? { cfg.aimd_window = v; }
    if let Some(v) = num::<u64>(&kv, "--snapshot-ms")? { cfg.snapshot_interval_ms = v; }
    if let Some(a) = kv.get("--arms") {
        let mut arms = Vec::new();
        for n in a.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()) {
            arms.push(sb::Arm::parse(n).ok_or_else(|| format!("unknown arm '{n}' (BASE, STATIC-CAP, AIMD-CAP)"))?);
        }
        cfg.arms = arms;
    }
    sb::finalize_config(&mut cfg);
    cfg.command_line = std::env::args().collect::<Vec<_>>().join(" ");
    Ok(cfg)
}

/// Entry point for `phantom-lab scenario-b ...`; returns the process exit code.
pub fn run_cli(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{HELP}");
        return 0;
    }
    let cfg = match parse_args(args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}\n(run `phantom-lab scenario-b --help`)");
            return 2;
        }
    };
    let rt = match tokio::runtime::Builder::new_multi_thread().worker_threads(4).enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: tokio runtime: {e}");
            return 1;
        }
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let progress: sb::ProgressFn = Arc::new(|p: sb::Progress| {
        eprintln!("[{}/{}] {} rep {} : {}", p.done, p.total, p.arm, p.rep + 1, p.phase);
    });
    eprintln!(
        "phantom-lab scenario-b: tier MEASURED_LOOPBACK, {} reps x {} arms, gateway {}, out {}",
        cfg.reps,
        cfg.arms.len(),
        cfg.gateway_exe.display(),
        cfg.out_dir.display()
    );
    let res = rt.block_on(async {
        let c = cancel.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                c.store(true, Ordering::SeqCst);
            }
        });
        sb::run_experiment(&cfg, cancel.clone(), progress).await
    });
    match res {
        Ok(o) => {
            println!("label: {}", o.label);
            println!("summary: {}", o.out_dir.join("summary.json").display());
            if let Some(v) = o.summary.get("verdicts").and_then(|v| v.as_object()) {
                for (arm, vd) in v {
                    println!("verdict {arm}: {}", vd["outcome"]);
                }
            }
            if o.cancelled {
                eprintln!("cancelled");
                return 130;
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

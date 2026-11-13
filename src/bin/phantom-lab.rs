//! `phantom-lab`: deterministic Phantom Lab experiment runner (D-3).
//!
//! Runs the T1 (SIMULATED) matrix on the real cache / policy executor via `phantom_cache_demo::lab`.
//! No `cargo test` needed. Refuses the sealed seed set S and the sealed family W-SF.
//!
//! Usage: `phantom-lab help`

use phantom_cache_demo::lab::runner::{
    cell_count, execute, list_arms, list_families, list_scenarios, list_seed_sets, paired_summary, validate, LabError,
    RunSpec,
};
use std::collections::BTreeMap;
use std::process::ExitCode;

const HELP: &str = "phantom-lab: deterministic Phantom Lab runner (tier SIMULATED; virtual time, real cache/executor)

USAGE
  phantom-lab list [scenarios|arms|families|seeds|all]
  phantom-lab run --scenarios A,B --families X,Y --arms P,Q --n-seeds N --out DIR [options]
  phantom-lab scenario-b [options]   (T2 MEASURED_LOOPBACK, real gateway process; `phantom-lab scenario-b --help`)
  phantom-lab help

RUN OPTIONS
  --scenarios LIST     comma separated scenario ids (see `list scenarios`)
  --families LIST      comma separated workload families (see `list families`)
  --arms LIST          comma separated arms/baselines (see `list arms`)
  --seed-set T|D       tuning or development seeds (default D). The sealed set S is refused.
  --n-seeds N          first N seeds of the set (default 5)
  --profile NAME       smoke (default, short horizon) | standard (registered horizon)
  --arm-params NAME    untuned (default) | frozen (lab-results/tuning_v2_2c/frozen_arm_params.json, the registered v2.2c artifact)
  --raw                also write requests.csv and fetches.csv
  --name NAME          run name recorded in the manifest (default phantom-lab-run)
  --out DIR            output directory (cells.csv, manifest.json, ...); required
  --overwrite          allow writing into a directory that already holds cells.csv
  --control ARM        control arm for the paired summary (default: first arm)
  --treatment ARM      treatment arm for the paired summary (default: second arm)

EXIT CODES  0 ok | 1 run failure | 2 usage or refused request (e.g. sealed seed set)
";

fn usage_err(msg: impl AsRef<str>) -> ExitCode {
    eprintln!("error: {}\n(run `phantom-lab help`)", msg.as_ref());
    ExitCode::from(2)
}

fn csv(s: &str) -> Vec<String> {
    s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

fn cmd_list(what: &str) -> ExitCode {
    let all = what == "all";
    let mut ok = false;
    if all || what == "scenarios" {
        println!("scenarios: {}", list_scenarios().join(" "));
        ok = true;
    }
    if all || what == "arms" {
        println!("arms (baselines/policies): {}", list_arms().join(" "));
        ok = true;
    }
    if all || what == "families" {
        println!("workload families: {} (W-Z accepts W-Z(alpha); W-SF is sealed and refused)", list_families().join(" "));
        ok = true;
    }
    if all || what == "seeds" {
        let v: Vec<String> = list_seed_sets().into_iter().map(|(n, c)| format!("{n} ({c} seeds)")).collect();
        println!("seed sets: {} (sealed set S is never listed or runnable here)", v.join(", "));
        ok = true;
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        usage_err(format!("unknown list topic '{what}'"))
    }
}

fn cmd_run(args: &[String]) -> ExitCode {
    let mut spec = RunSpec {
        name: "phantom-lab-run".into(),
        scenarios: vec![],
        families: vec![],
        arms: vec![],
        seed_set: "D".into(),
        n_seeds: 5,
        profile: "smoke".into(),
        arm_params: "untuned".into(),
        record_raw: false,
        out_dir: String::new(),
        overwrite: false,
    };
    let (mut control, mut treatment): (Option<String>, Option<String>) = (None, None);
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        let val = |i: &mut usize| -> Result<String, String> {
            *i += 1;
            args.get(*i).cloned().ok_or_else(|| format!("{flag} needs a value"))
        };
        let r: Result<(), String> = (|| {
            match flag {
                "--scenarios" => spec.scenarios = csv(&val(&mut i)?),
                "--families" => spec.families = csv(&val(&mut i)?),
                "--arms" => spec.arms = csv(&val(&mut i)?),
                "--seed-set" => spec.seed_set = val(&mut i)?,
                "--n-seeds" => spec.n_seeds = val(&mut i)?.parse().map_err(|_| "--n-seeds must be an integer".to_string())?,
                "--profile" => spec.profile = val(&mut i)?,
                "--arm-params" => spec.arm_params = val(&mut i)?,
                "--name" => spec.name = val(&mut i)?,
                "--out" => spec.out_dir = val(&mut i)?,
                "--control" => control = Some(val(&mut i)?),
                "--treatment" => treatment = Some(val(&mut i)?),
                "--raw" => spec.record_raw = true,
                "--overwrite" => spec.overwrite = true,
                other => return Err(format!("unknown option '{other}'")),
            }
            Ok(())
        })();
        if let Err(e) = r {
            return usage_err(e);
        }
        i += 1;
    }
    if spec.out_dir.is_empty() {
        return usage_err("--out DIR is required");
    }
    match validate(&spec) {
        Ok(_) => {}
        Err(e) => return refuse(e),
    }
    let total = cell_count(&spec);
    eprintln!("phantom-lab: {} cells, tier SIMULATED, seed set {} (n={}), profile {}", total, spec.seed_set, spec.n_seeds, spec.profile);
    let mut last_pct = usize::MAX;
    let mut progress = |done: usize, total: usize| {
        let pct = done * 10 / total.max(1);
        if pct != last_pct {
            last_pct = pct;
            eprintln!("  progress {done}/{total}");
        }
    };
    let result = match execute(&spec, &mut progress) {
        Ok(r) => r,
        Err(e) => return refuse(e),
    };

    println!("run '{}' complete: {} cells -> {}", spec.name, result.cells.len(), result.out_dir.display());
    println!("tier: SIMULATED (virtual-time simulator driving the real cache/executor; latencies are virtual microseconds)");
    println!("files: cells.csv, manifest.json{}", if spec.record_raw { ", requests.csv, fetches.csv" } else { "" });

    // Per-arm means over all cells.
    let mut acc: BTreeMap<&str, (f64, f64, f64, usize)> = BTreeMap::new();
    for c in &result.cells {
        let e = acc.entry(c.arm.as_str()).or_insert((0.0, 0.0, 0.0, 0));
        e.0 += c.peak_outstanding as f64;
        e.1 += c.p99_us as f64;
        e.2 += c.fresh_hit_ratio;
        e.3 += 1;
    }
    println!("\n{:<20} {:>6} {:>18} {:>16} {:>14}", "arm", "cells", "mean peak origin", "mean p99 (us)", "mean fresh-hit");
    for (arm, (p, l, h, n)) in &acc {
        let n = *n as f64;
        println!("{:<20} {:>6} {:>18.2} {:>16.0} {:>14.4}", arm, n as usize, p / n, l / n, h / n);
    }

    let (ctrl, treat) = match (control, treatment) {
        (Some(c), Some(t)) => (Some(c), Some(t)),
        (c, t) if spec.arms.len() >= 2 => (c.or_else(|| Some(spec.arms[0].clone())), t.or_else(|| Some(spec.arms[1].clone()))),
        _ => (None, None),
    };
    if let (Some(c), Some(t)) = (ctrl, treat) {
        match paired_summary(&result, &t, &c) {
            Ok(rows) => {
                println!("\npaired-by-seed relative reduction of {t} vs control {c} (positive = {t} lower; descriptive, 95% BCa CI, n_seeds={})", spec.n_seeds);
                println!("{:<8} {:<26} {:>12} {:>12} {:>22} {:>10}", "scenario", "endpoint", "mean ctrl", "mean treat", "rel. reduction [CI]", "p (perm)");
                for r in rows {
                    println!(
                        "{:<8} {:<26} {:>12.1} {:>12.1} {:>8.3} [{:>6.3},{:>6.3}] {:>10.4}",
                        r.scenario, r.endpoint, r.mean_ctrl, r.mean_treat, r.mean_rel_reduction, r.ci_lo, r.ci_hi, r.p_value
                    );
                }
            }
            Err(e) => return refuse(e),
        }
    }
    ExitCode::SUCCESS
}

fn refuse(e: LabError) -> ExitCode {
    match e {
        LabError::Io(_) | LabError::FrozenArmParamsMissing => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
        other => usage_err(other.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("help") | Some("--help") | Some("-h") => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        Some("list") => cmd_list(args.get(1).map(String::as_str).unwrap_or("all")),
        Some("run") => cmd_run(&args[1..]),
        Some("scenario-b") => ExitCode::from(phantom_cache_demo::lab_live::cli::run_cli(&args[1..]) as u8),
        Some(other) => usage_err(format!("unknown command '{other}'")),
    }
}

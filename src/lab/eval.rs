//! M6 evaluation protocol v2 driver (R-02 + protocol repair).
//!
//! - Runs matrices of T1 cells (`t1_sim`) and writes RAW per-cell (and optionally per-request /
//!   per-fetch) data plus a manifest (source hashes, seeds or seed-set commitment, configuration,
//!   git HEAD, real wall-clock timestamps).
//! - Analysis (paired-by-seed relative reductions, BCa bootstrap CI, sign-flip permutation test,
//!   Holm-Bonferroni) consumes ONLY the raw per-cell CSV (re-read from disk by callers).
//! - Freeze verification and sealed-seed leak scanning (preregistration_m6_v2.md sections 6 and 9).
//! - Tuning on tuning seeds T and tuning families only.
//!
//! Provenance: every number produced here is tier SIMULATED.

use super::analysis::{HolmBonferroni, StatisticalEvaluator};
use super::rng::{sha256_hex, SEALED_SALT_COMMITMENT_SHA256};
use super::t1_sim::*;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Real wall-clock UTC timestamp (ISO-8601, second resolution) from the system clock.
pub fn utc_now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0) as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, rem / 3600, (rem % 3600) / 60, rem % 60)
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn git_output(root: &Path, args: &[&str]) -> String {
    std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

pub fn git_head(root: &Path) -> String {
    git_output(root, &["rev-parse", "HEAD"])
}

pub fn git_dirty_entries(root: &Path) -> usize {
    git_output(root, &["status", "--short"]).lines().count()
}

/// Files whose hashes define "the code under test" (recorded in manifests and the freeze record).
pub fn tracked_source_files(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    for dir in ["src/lab", "tests/support/lab", "src/core", "src/intelligence"] {
        if let Ok(rd) = fs::read_dir(root.join(dir)) {
            for e in rd.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "rs").unwrap_or(false) {
                    v.push(p);
                }
            }
        }
    }
    v.push(root.join("tests/support/mod.rs"));
    // Protocol v2.2 (finding M4): build inputs that determine behaviour are part of the frozen inventory.
    v.push(root.join("Cargo.lock"));
    v.push(root.join("Cargo.toml"));
    v.push(root.join("src/lib.rs"));
    v.push(root.join("tests/integration/m6_research_evaluation_tests.rs"));
    v.push(root.join("docs/research/preregistration_m6_v2.md"));
    v.sort();
    v
}

pub fn source_hashes(root: &Path) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    for p in tracked_source_files(root) {
        if let Ok(h) = sha256_file(&p) {
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            m.insert(rel, h);
        }
    }
    m
}

// ---------------------------------------------------------------------------
// Matrix execution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct SeedSpec {
    pub seed: u64,
    pub label: String,
}

pub fn plain_seeds(seeds: &[u64]) -> Vec<SeedSpec> {
    seeds.iter().map(|&s| SeedSpec { seed: s, label: s.to_string() }).collect()
}

/// Sealed seeds are labelled by index only; their numeric values never reach any output file.
pub fn sealed_seed_specs(seeds: &[u64]) -> Vec<SeedSpec> {
    seeds.iter().enumerate().map(|(i, &s)| SeedSpec { seed: s, label: format!("S{:02}", i) }).collect()
}

#[derive(Debug, Clone)]
pub struct MatrixSpec {
    pub name: String,
    pub seed_set_name: String, // "T" | "D" | "S"
    pub scenarios: Vec<String>,
    pub families: Vec<String>,
    pub arms: Vec<String>,
    pub seeds: Vec<SeedSpec>,
    pub params: SimParams,
    pub arm_params: ArmParams,
    pub record_raw: bool,
}

pub struct MatrixRun {
    /// The manifest exactly as written to `manifest.json`.
    pub manifest: serde_json::Value,
    pub cells: Vec<CellSummary>,
    pub out_dir: PathBuf,
}

/// Executes the matrix and writes RAW data and a manifest. Interleaving: seed -> scenario ->
/// family -> arms in a seed-dependent rotation (order is immaterial in deterministic virtual
/// time; recorded for protocol fidelity).
pub fn run_matrix(spec: &MatrixSpec, out_dir: &Path) -> Result<MatrixRun, String> {
    run_matrix_with_progress(spec, out_dir, &mut |_done, _total| {})
}

/// Same as [`run_matrix`], calling `progress(cells_done, cells_total)` after every executed cell.
pub fn run_matrix_with_progress(spec: &MatrixSpec, out_dir: &Path, progress: &mut dyn FnMut(usize, usize)) -> Result<MatrixRun, String> {
    let total_cells = spec.seeds.len() * spec.scenarios.len() * spec.families.len() * spec.arms.len();
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let root = repo_root();
    let started = utc_now_iso();
    let mut cells_f = fs::File::create(out_dir.join("cells.csv")).map_err(|e| e.to_string())?;
    writeln!(cells_f, "{}", CELL_CSV_HEADER).map_err(|e| e.to_string())?;
    let mut req_f = None;
    let mut fetch_f = None;
    if spec.record_raw {
        let mut rf = fs::File::create(out_dir.join("requests.csv")).map_err(|e| e.to_string())?;
        writeln!(rf, "{}", REQ_CSV_HEADER).map_err(|e| e.to_string())?;
        let mut ff = fs::File::create(out_dir.join("fetches.csv")).map_err(|e| e.to_string())?;
        writeln!(ff, "{}", FETCH_CSV_HEADER).map_err(|e| e.to_string())?;
        req_f = Some(rf);
        fetch_f = Some(ff);
    }
    let mut cells = Vec::new();
    let mut arm_cfgs: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    for (si, seed) in spec.seeds.iter().enumerate() {
        for (ci, scenario) in spec.scenarios.iter().enumerate() {
            for family in &spec.families {
                let n = spec.arms.len();
                for k in 0..n {
                    let arm_id = &spec.arms[(k + si + ci) % n];
                    let arm = arm_config(arm_id, &spec.arm_params, scenario);
                    arm_cfgs.insert(format!("{}@{}", arm_id, scenario), serde_json::to_value(&arm).unwrap());
                    let cell = CellSpec {
                        scenario: scenario.clone(),
                        family: family.clone(),
                        arm,
                        seed: seed.seed,
                        seed_label: seed.label.clone(),
                        params: spec.params.clone(),
                        record_raw: spec.record_raw,
                    };
                    // A panic inside a cell is recorded as a defect cell (finding C1b) instead of aborting the run.
                    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_cell(&cell))).unwrap_or_else(|_| defect_cell_output(&cell));
                    writeln!(cells_f, "{}", out.summary.csv_row()).map_err(|e| e.to_string())?;
                    if let (Some(rf), Some(ff)) = (req_f.as_mut(), fetch_f.as_mut()) {
                        let id = &out.summary.cell_id;
                        for r in &out.requests {
                            writeln!(rf, "{},{},{},{},{},{},{},{}", id, r.req_id, r.arrival_us, r.key, r.outcome, r.latency_us, r.attempts, r.stale_age_ms)
                                .map_err(|e| e.to_string())?;
                        }
                        for f in &out.fetches {
                            writeln!(ff, "{},{},{},{},{},{}", id, f.dispatch_us, f.finish_us, f.status, f.outstanding_after_dispatch, f.background)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    cells.push(out.summary);
                    progress(cells.len(), total_cells);
                }
            }
        }
    }
    drop(cells_f);
    drop(req_f);
    drop(fetch_f);
    let ended = utc_now_iso();

    let seed_values_hash = {
        let mut s = String::new();
        for x in &spec.seeds {
            s.push_str(&format!("{},", x.seed));
        }
        sha256_hex(s.as_bytes())
    };
    let mut files = BTreeMap::new();
    for f in ["cells.csv", "requests.csv", "fetches.csv"] {
        let p = out_dir.join(f);
        if p.exists() {
            files.insert(f.to_string(), sha256_file(&p)?);
        }
    }
    let manifest = json!({
        "manifest_version": "m6-v2-1",
        "name": spec.name,
        "tier": "SIMULATED",
        "provenance_note": "All metrics are computed from executed discrete events of the T1 virtual-time simulator driving the real product cache/executor; no measurement of a real network or host.",
        "seed_set": spec.seed_set_name,
        "seed_labels": spec.seeds.iter().map(|s| s.label.clone()).collect::<Vec<_>>(),
        // Numeric seeds are listed only for tuning/dev sets; sealed seed values are never written.
        "seed_values": if spec.seed_set_name == "S" { serde_json::Value::Null } else { json!(spec.seeds.iter().map(|s| s.seed).collect::<Vec<_>>()) },
        "seed_list_sha256": seed_values_hash,
        "sealed_salt_commitment_sha256": SEALED_SALT_COMMITMENT_SHA256,
        "scenarios": spec.scenarios,
        "families": spec.families,
        "arms": spec.arms,
        "arm_configs": arm_cfgs,
        "arm_params": spec.arm_params,
        "sim_params": spec.params,
        "cell_count": cells.len(),
        "git_head": git_head(&root),
        "git_status_entries": git_dirty_entries(&root),
        "source_sha256": source_hashes(&root),
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "host_os": std::env::consts::OS,
        "host_arch": std::env::consts::ARCH,
        "started_utc": started,
        "ended_utc": ended,
        "output_files_sha256": files,
        "interleaving": "seed -> scenario -> family -> arms rotated by (seed_index+scenario_index) mod n_arms",
    });
    fs::write(out_dir.join("manifest.json"), serde_json::to_string_pretty(&manifest).unwrap()).map_err(|e| e.to_string())?;
    Ok(MatrixRun { manifest, cells, out_dir: out_dir.to_path_buf() })
}

pub fn read_cells_csv(path: &Path) -> Result<Vec<CellSummary>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut lines = text.lines();
    let header = lines.next().ok_or("empty csv")?;
    if header.trim() != CELL_CSV_HEADER {
        return Err("unexpected cells.csv header".to_string());
    }
    lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| CellSummary::from_csv_row(l).ok_or_else(|| format!("bad row: {l}")))
        .collect()
}

// ---------------------------------------------------------------------------
// Analysis (consumes raw CellSummary rows only)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Metric {
    pub name: &'static str,
    pub f: fn(&CellSummary) -> f64,
}

pub const M_PEAK: Metric = Metric { name: "peak_origin_concurrency", f: |c| c.peak_outstanding as f64 };
/// Primary P99 (v2/v2.2): nearest-rank P99 over ALL logical requests of the window. Failed requests enter at
/// their failure latency (timeout = 1 s, shed/reject/origin error = time until the failure was known, which can
/// be ~0), so a policy that sheds quickly can look better here; see the v2.2 amendment, section H1.
pub const M_P99: Metric = Metric { name: "client_p99_latency_us", f: |c| c.p99_us as f64 };
/// SECONDARY (v2.2): nearest-rank P99 over successfully served requests only.
pub const M_P99_SUCCESS: Metric = Metric { name: "client_p99_success_only_us", f: |c| c.p99_success_us as f64 };

/// Registered error-rate non-inferiority margin (v2.2): treatment error rate may exceed BL-12b by at most 0.5 pp.
pub const ERROR_RATE_MARGIN: f64 = 0.005;
/// A per-seed relative reduction with |r_i| above this value is flagged (v2.2, M1; exploratory).
pub const EXTREME_RATIO_FLAG: f64 = 10.0;

#[derive(Debug, Clone)]
pub struct EndpointResult {
    pub scenario: String,
    pub endpoint: &'static str,
    pub treat: String,
    pub ctrl: String,
    pub n_seeds: usize,
    pub mean_ctrl: f64,
    pub mean_treat: f64,
    pub mean_rel_reduction: f64,
    pub ci_lo: f64,
    pub ci_hi: f64,
    pub p_value: f64,
    pub p_holm: f64,
    pub holm_significant: bool,
    pub supported: bool,
    pub practically_null: bool,
    /// BCa upper bound < 0 (the treatment is credibly worse). Registered rule input (v2.2 verdict).
    pub adverse: bool,
    /// EXPLORATORY (not pre-registered): 1 - exp(mean ln(treat/ctrl)) over seeds with both values > 0.
    pub expl_log_ratio_rel_reduction: f64,
    /// EXPLORATORY: median of the per-seed relative reductions r_i.
    pub expl_median_rel_reduction: f64,
    /// Number of seeds with |r_i| > EXTREME_RATIO_FLAG (flagged; they are NOT removed from any statistic).
    pub n_extreme_ratio: usize,
}

/// Per-seed value: mean of `metric` over `families` for (scenario, arm).
fn per_seed(rows: &[CellSummary], scenario: &str, arm: &str, families: &[String], metric: &Metric) -> BTreeMap<String, f64> {
    let mut acc: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.scenario == scenario && r.arm == arm && families.contains(&r.family)) {
        let e = acc.entry(r.seed_label.clone()).or_insert((0.0, 0));
        e.0 += (metric.f)(r);
        e.1 += 1;
    }
    acc.into_iter().map(|(k, (s, n))| (k, s / n as f64)).collect()
}

/// Paired-by-seed analysis of `treat` vs `ctrl`: per-seed relative reduction (ctrl-treat)/ctrl,
/// BCa 95% CI (B=10,000), sign-flip permutation p-value, Holm adjustment across ALL returned hypotheses.
// The positional parameter list mirrors the pre-registered analysis (preregistration_m6_v2.md) and the frozen
// call sites in the M6 tests; a parameter struct would change those sites without changing behavior.
#[allow(clippy::too_many_arguments)]
pub fn analyze_endpoints(
    rows: &[CellSummary],
    treat: &str,
    ctrl: &str,
    scenarios: &[String],
    families: &[String],
    metrics: &[Metric],
    delta_min: f64,
    boot_seed: u64,
) -> Vec<EndpointResult> {
    let mut out = Vec::new();
    for sc in scenarios {
        for m in metrics {
            let t = per_seed(rows, sc, treat, families, m);
            let c = per_seed(rows, sc, ctrl, families, m);
            let labels: Vec<&String> = t.keys().filter(|k| c.contains_key(*k)).collect();
            let rel: Vec<f64> = labels
                .iter()
                .map(|k| {
                    let cv = c[*k];
                    if cv.abs() < 1e-12 { 0.0 } else { (cv - t[*k]) / cv }
                })
                .collect();
            let n = rel.len();
            let mean_rel = if n == 0 { 0.0 } else { rel.iter().sum::<f64>() / n as f64 };
            let logs: Vec<f64> = labels.iter().filter(|k| c[**k] > 0.0 && t[**k] > 0.0).map(|k| (t[*k] / c[*k]).ln()).collect();
            let expl_log = if logs.is_empty() { 0.0 } else { 1.0 - (logs.iter().sum::<f64>() / logs.len() as f64).exp() };
            let expl_median = {
                let mut v = rel.clone();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                match v.len() {
                    0 => 0.0,
                    l if l % 2 == 1 => v[l / 2],
                    l => 0.5 * (v[l / 2 - 1] + v[l / 2]),
                }
            };
            let n_extreme = rel.iter().filter(|r| r.abs() > EXTREME_RATIO_FLAG).count();
            let (lo, hi) = StatisticalEvaluator::bca_ci_mean(&rel, 10_000, boot_seed);
            let p = StatisticalEvaluator::sign_flip_p_value(&rel, boot_seed ^ 0x5eed);
            out.push(EndpointResult {
                scenario: sc.clone(),
                endpoint: m.name,
                treat: treat.to_string(),
                ctrl: ctrl.to_string(),
                n_seeds: n,
                mean_ctrl: labels.iter().map(|k| c[*k]).sum::<f64>() / n.max(1) as f64,
                mean_treat: labels.iter().map(|k| t[*k]).sum::<f64>() / n.max(1) as f64,
                mean_rel_reduction: mean_rel,
                ci_lo: lo,
                ci_hi: hi,
                p_value: p,
                p_holm: 1.0,
                holm_significant: false,
                supported: false,
                practically_null: hi < delta_min,
                adverse: hi < 0.0,
                expl_log_ratio_rel_reduction: expl_log,
                expl_median_rel_reduction: expl_median,
                n_extreme_ratio: n_extreme,
            });
        }
    }
    let ps: Vec<f64> = out.iter().map(|r| r.p_value).collect();
    let adj = HolmBonferroni::adjust(&ps);
    for (r, a) in out.iter_mut().zip(adj) {
        r.p_holm = a;
        r.holm_significant = a < 0.05;
        // Practical-significance rule (preregistration_m6_v2.md section 4): Holm-significant AND
        // mean relative reduction >= delta_min AND BCa lower bound > 0.
        r.supported = r.holm_significant && r.mean_rel_reduction >= delta_min && r.ci_lo > 0.0;
    }
    out
}

#[derive(Debug, Clone)]
pub struct ConstraintReport {
    pub scenario: String,
    pub mean_fresh_hit_ratio_diff: f64,
    pub mean_error_rate_diff: f64,
    pub treat_cap_violations: u64,
    pub treat_stale_cap_violations: u64,
    pub hit_ratio_ok: bool,
    /// v2.2: mean per-seed error-rate difference (treat - ctrl) <= ERROR_RATE_MARGIN (0.5 pp).
    pub error_rate_ok: bool,
}

/// Constraint endpoints: hit-ratio loss <= 1 pp, error-rate increase <= 0.5 pp (v2.2), zero cap/stale-cap violations.
pub fn check_constraints(rows: &[CellSummary], treat: &str, ctrl: &str, scenarios: &[String], families: &[String]) -> Vec<ConstraintReport> {
    scenarios
        .iter()
        .map(|sc| {
            let hit = Metric { name: "fresh_hit_ratio", f: |c| c.fresh_hit_ratio };
            let err = Metric { name: "error_rate", f: |c| c.error_rate };
            let diff = |m: &Metric| {
                let t = per_seed(rows, sc, treat, families, m);
                let c = per_seed(rows, sc, ctrl, families, m);
                let d: Vec<f64> = t.iter().filter_map(|(k, v)| c.get(k).map(|cv| v - cv)).collect();
                if d.is_empty() { 0.0 } else { d.iter().sum::<f64>() / d.len() as f64 }
            };
            let hd = diff(&hit);
            let treat_rows = rows.iter().filter(|r| r.scenario == *sc && r.arm == treat);
            let (mut cv, mut sv) = (0u64, 0u64);
            for r in treat_rows {
                cv += r.cap_violations;
                sv += r.stale_cap_violations_counter + r.stale_cap_violations_indep;
            }
            let ed = diff(&err);
            ConstraintReport {
                scenario: sc.clone(),
                mean_fresh_hit_ratio_diff: hd,
                mean_error_rate_diff: ed,
                error_rate_ok: ed <= ERROR_RATE_MARGIN,
                treat_cap_violations: cv,
                treat_stale_cap_violations: sv,
                hit_ratio_ok: hd >= -0.01,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Freeze record verification (protocol v2.2) and sealed-seed leak scan
// ---------------------------------------------------------------------------

/// The ORIGINAL v2 freeze record. Historical, immutable evidence (its frozen-file hashes no longer match
/// the tree, by design); later records pin its exact bytes.
pub fn original_freeze_record_path(root: &Path) -> PathBuf {
    root.join("docs/research/preregistration_m6_v2_freeze.json")
}

/// The v2.1 freeze record. Historical (superseded by v2.2); pinned by SHA-256 in the v2.2 record.
pub fn v2_1_freeze_record_path(root: &Path) -> PathBuf {
    root.join("docs/research/preregistration_m6_v2_1_freeze.json")
}

/// The v2.1 amendment document (prose). Unchanged; pinned by SHA-256 in the v2.2 record.
pub fn v2_1_amendment_doc_path(root: &Path) -> PathBuf {
    root.join("docs/research/preregistration_m6_v2_1_amendment.md")
}

/// The dated v2.2 amendment document (prose).
pub fn amendment_doc_path(root: &Path) -> PathBuf {
    root.join("docs/research/preregistration_m6_v2_2_amendment.md")
}

/// The CURRENT freeze record (protocol v2.2). Written by the research lead, never by lab code.
pub fn freeze_record_path(root: &Path) -> PathBuf {
    root.join("docs/research/preregistration_m6_v2_2_freeze.json")
}

/// Paths (relative to the repository root, forward slashes) that MUST appear in `files_sha256` of the v2.2
/// freeze record. Together with "every file returned by `tracked_source_files` that exists" this fixes the
/// frozen scope (finding M4): lockfile, manifest, `src/lib.rs`, and the re-tuned arm parameters.
pub const MANDATORY_FROZEN_FILES: [&str; 7] = [
    "Cargo.lock",
    "Cargo.toml",
    "src/lib.rs",
    "lab-results/tuning_v2_2/frozen_arm_params.json",
    "lab-results/tuning_v2_2/tuning_trials.csv",
    "lab-results/tuning_v2_2c/frozen_arm_params.json",
    "lab-results/tuning_v2_2c/tuning_trials.csv",
];

/// `rustc --version` of the toolchain found on this machine (the compiler used to build the tests is the one
/// first on PATH or `$RUSTC`; recorded in the freeze record and compared by `verify_freeze`).
pub fn current_rustc_version() -> Result<String, String> {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let out = std::process::Command::new(rustc).arg("--version").output().map_err(|e| format!("cannot run rustc --version: {e}"))?;
    if !out.status.success() {
        return Err("rustc --version failed".to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// The sealed evaluation and the dry run are release-only (the registered build profile).
pub fn require_release_build() -> Result<(), String> {
    if cfg!(debug_assertions) {
        Err("this evaluation requires a release build (cargo test --release); debug assertions are enabled".to_string())
    } else {
        Ok(())
    }
}

/// Verifies (fail closed) that the v2.2 freeze record exists and
/// - carries the protocol version compiled into the code and `required_build_profile == "release"`,
/// - records a `rustc_version` equal to the toolchain found now,
/// - pins the unchanged v2 preregistration, the unchanged v2.1 amendment, the v2.2 amendment, the original v2
///   freeze record and the (superseded) v2.1 freeze record by SHA-256,
/// - lists the sealed commitment compiled into the code and has a timestamp,
/// - lists every [`MANDATORY_FROZEN_FILES`] entry and every existing file of [`tracked_source_files`], and
/// - every listed file still has the recorded SHA-256.
pub fn verify_freeze(root: &Path) -> Result<serde_json::Value, String> {
    verify_freeze_core(root, &BTreeMap::new()).map(|(v, _)| v)
}

/// Shared implementation of [`verify_freeze`] and [`verify_freeze_with_addendum`]. `tolerated` maps a frozen
/// path to the ONE current SHA-256 that may differ from the freeze record; it is empty for the pure v2.2
/// record, so `verify_freeze` behaves exactly as before. Returns the record and the set of paths whose
/// current hash differed from the record (all of which were in `tolerated` with exactly that hash).
fn verify_freeze_core(root: &Path, tolerated: &BTreeMap<String, String>) -> Result<(serde_json::Value, BTreeSet<String>), String> {
    let text = fs::read_to_string(freeze_record_path(root)).map_err(|e| format!("freeze record missing: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    if v["protocol_version"].as_str() != Some(super::rng::PROTOCOL_VERSION) {
        return Err(format!(
            "freeze: protocol_version {:?} is not the compiled protocol version {}",
            v["protocol_version"],
            super::rng::PROTOCOL_VERSION
        ));
    }
    if v["required_build_profile"].as_str() != Some("release") {
        return Err("freeze: required_build_profile must be \"release\"".to_string());
    }
    let recorded_rustc = v["rustc_version"].as_str().filter(|s| !s.is_empty()).ok_or("freeze: rustc_version missing")?;
    let now_rustc = current_rustc_version()?;
    if recorded_rustc != now_rustc {
        return Err(format!("freeze: rustc version differs (recorded '{recorded_rustc}', now '{now_rustc}')"));
    }
    let pin = |field: &str, path: PathBuf, what: &str| -> Result<(), String> {
        let rec = v[field].as_str().ok_or_else(|| format!("freeze: {field} missing"))?;
        let now = sha256_file(&path)?;
        if rec != now {
            return Err(format!("{what} changed after freeze (recorded {rec}, now {now})"));
        }
        Ok(())
    };
    pin("prereg_sha256", root.join("docs/research/preregistration_m6_v2.md"), "preregistration_m6_v2.md")?;
    pin("amendment_v2_1_sha256", v2_1_amendment_doc_path(root), "v2.1 amendment document")?;
    pin("amendment_sha256", amendment_doc_path(root), "v2.2 amendment document")?;
    pin("supersedes_freeze_sha256", v2_1_freeze_record_path(root), "v2.1 freeze record")?;
    pin("supersedes_original_freeze_sha256", original_freeze_record_path(root), "original v2 freeze record")?;
    if v["sealed_salt_commitment_sha256"].as_str() != Some(SEALED_SALT_COMMITMENT_SHA256) {
        return Err("freeze: sealed commitment differs from the one compiled into the code".to_string());
    }
    let files = v["files_sha256"].as_object().ok_or("freeze: files_sha256 missing")?;
    if files.is_empty() {
        return Err("freeze: no files recorded".to_string());
    }
    for m in MANDATORY_FROZEN_FILES {
        if !files.contains_key(m) {
            return Err(format!("freeze: mandatory file {m} is not listed in files_sha256"));
        }
    }
    for p in tracked_source_files(root) {
        if p.exists() {
            let rel = p.strip_prefix(root).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            if !files.contains_key(&rel) {
                return Err(format!("freeze: {rel} exists in the tracked inventory but is not listed in files_sha256"));
            }
        }
    }
    let mut differing = BTreeSet::new();
    for (rel, h) in files {
        let now = sha256_file(&root.join(rel))?;
        if Some(now.as_str()) != h.as_str() {
            if tolerated.get(rel.as_str()).map(String::as_str) != Some(now.as_str()) {
                return Err(format!("{rel} changed after freeze"));
            }
            differing.insert(rel.clone());
        }
    }
    if v["frozen_at_utc"].as_str().map(|s| s.is_empty()).unwrap_or(true) {
        return Err("freeze: timestamp missing".to_string());
    }
    Ok((v, differing))
}

/// Addendum 1 to the v2.2 freeze (R-01): dated, independently inspectable record of legitimate additive changes
/// made to frozen files AFTER the sealed run. Machine record.
pub fn addendum_1_path(root: &Path) -> PathBuf {
    root.join("docs/research/preregistration_m6_v2_2_addendum_1.json")
}

/// Addendum 1 prose.
pub fn addendum_1_doc_path(root: &Path) -> PathBuf {
    root.join("docs/research/preregistration_m6_v2_2_addendum_1.md")
}

/// Pin file for addendum 1: one line, the SHA-256 of [`addendum_1_path`]. It lives outside the frozen scope
/// on purpose: the addendum must list the post-edit hashes of `eval.rs` and the m6 test file, so the
/// addendum's own hash cannot be compiled into either of them (that would be circular).
pub fn addendum_1_pin_path(root: &Path) -> PathBuf {
    root.join("docs/research/preregistration_m6_v2_2_addendum_1.pin")
}

/// Paths an addendum may never excuse: the pinned protocol text and every recorded tuning/evidence artifact.
fn addendum_forbidden_path(rel: &str) -> bool {
    rel == "docs/research/preregistration_m6_v2.md" || rel.starts_with("lab-results/") || rel.starts_with("docs/research/preregistration_m6_v2_")
}

/// Fail-closed freeze verification that tolerates ONLY the post-freeze changes listed in addendum 1.
///
/// - No addendum and no pin file: identical to [`verify_freeze`] (any changed frozen file fails).
/// - Exactly one of addendum/pin present, or the pin differs from the addendum's SHA-256: error.
/// - The addendum must carry the compiled protocol version, the SHA-256 of the unchanged freeze record and
///   v2.2 amendment, and the SHA-256 of its own prose document (so neither can be swapped without
///   re-pinning), and list unique `changed_files` entries `{path, frozen_sha256, current_sha256}`.
/// - Each entry's path must be in the freeze record, must not be a protocol document or `lab-results/` file,
///   its `frozen_sha256` must equal the freeze record's hash, and its `current_sha256` must equal the file's
///   actual hash AND differ from the frozen hash.
/// - All other freeze checks are unchanged (rustc, pinned docs, inventory, every other file's hash).
/// - The entries must be EXACTLY the set of files that differ: an entry for a file that did not change is
///   rejected (a stale or padded addendum is not silently accepted).
///
/// A later edit of an excused file changes its current hash and fails until a further addendum is pinned.
pub fn verify_freeze_with_addendum(root: &Path) -> Result<serde_json::Value, String> {
    let (ap, pp) = (addendum_1_path(root), addendum_1_pin_path(root));
    if !ap.exists() && !pp.exists() {
        return verify_freeze(root);
    }
    if !ap.exists() || !pp.exists() {
        return Err("addendum: the addendum JSON and its pin file must both exist".to_string());
    }
    let ah = sha256_file(&ap)?;
    let pinned = fs::read_to_string(&pp).map_err(|e| format!("addendum pin unreadable: {e}"))?;
    if pinned.trim() != ah {
        return Err(format!("addendum: pin mismatch (pin {}, addendum {ah})", pinned.trim()));
    }
    let a: serde_json::Value = serde_json::from_slice(&fs::read(&ap).map_err(|e| e.to_string())?).map_err(|e| format!("addendum malformed: {e}"))?;
    if a["protocol_version"].as_str() != Some(super::rng::PROTOCOL_VERSION) || a["addendum_version"].as_u64() != Some(1) {
        return Err("addendum: protocol_version/addendum_version mismatch".to_string());
    }
    let bind = |field: &str, path: PathBuf, what: &str| -> Result<(), String> {
        let rec = a[field].as_str().ok_or_else(|| format!("addendum: {field} missing"))?;
        let now = sha256_file(&path)?;
        if rec != now {
            return Err(format!("addendum: {what} differs from the one the addendum was written against"));
        }
        Ok(())
    };
    bind("freeze_record_sha256", freeze_record_path(root), "v2.2 freeze record")?;
    bind("amendment_sha256", amendment_doc_path(root), "v2.2 amendment")?;
    bind("addendum_md_sha256", addendum_1_doc_path(root), "addendum prose document")?;
    let freeze: serde_json::Value = serde_json::from_str(&fs::read_to_string(freeze_record_path(root)).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let frozen = freeze["files_sha256"].as_object().ok_or("freeze: files_sha256 missing")?;
    let entries = a["changed_files"].as_array().ok_or("addendum: changed_files missing")?;
    let mut tolerated: BTreeMap<String, String> = BTreeMap::new();
    for e in entries {
        let path = e["path"].as_str().ok_or("addendum: entry without path")?;
        if addendum_forbidden_path(path) {
            return Err(format!("addendum: {path} may not be excused by an addendum"));
        }
        let fh = frozen.get(path).and_then(|x| x.as_str()).ok_or_else(|| format!("addendum: {path} is not in the freeze record"))?;
        if e["frozen_sha256"].as_str() != Some(fh) {
            return Err(format!("addendum: frozen_sha256 of {path} does not match the freeze record"));
        }
        let ch = e["current_sha256"].as_str().ok_or_else(|| format!("addendum: current_sha256 of {path} missing"))?;
        if ch == fh {
            return Err(format!("addendum: {path} lists an identical frozen and current hash"));
        }
        if tolerated.insert(path.to_string(), ch.to_string()).is_some() {
            return Err(format!("addendum: duplicate entry for {path}"));
        }
    }
    let (v, differing) = verify_freeze_core(root, &tolerated)?;
    if let Some(extra) = tolerated.keys().find(|k| !differing.contains(*k)) {
        return Err(format!("addendum: lists {extra}, which did not change after the freeze"));
    }
    Ok(v)
}

/// Scans every file under `dirs` (files or directories) for the decimal representation of any of `seeds`.
/// Files are searched as bytes decoded lossily as UTF-8 and, when they start with a UTF-16LE BOM, as UTF-16.
/// Directories named `target`, `.git` and `node_modules` are skipped. Returns the number of files scanned.
pub fn scan_for_seed_leak(dirs: &[PathBuf], seeds: &[u64]) -> Result<usize, String> {
    let needles: Vec<String> = seeds.iter().map(|s| s.to_string()).collect();
    let mut scanned = 0usize;
    fn walk(p: &Path, needles: &[String], scanned: &mut usize) -> Result<(), String> {
        if p.is_dir() {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if matches!(name.as_str(), "target" | ".git" | "node_modules") {
                return Ok(());
            }
            if let Ok(rd) = fs::read_dir(p) {
                for e in rd.flatten() {
                    walk(&e.path(), needles, scanned)?;
                }
            }
        } else if let Ok(bytes) = fs::read(p) {
            *scanned += 1;
            let text = if bytes.starts_with(&[0xFF, 0xFE]) {
                let u16s: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
                String::from_utf16_lossy(&u16s)
            } else {
                String::from_utf8_lossy(&bytes).into_owned()
            };
            for n in needles {
                if text.contains(n.as_str()) {
                    return Err(format!("sealed seed value found in {}", p.display()));
                }
            }
        }
        Ok(())
    }
    for d in dirs {
        walk(d, &needles, &mut scanned)?;
    }
    Ok(scanned)
}

/// The registered leak-scan scope (v2.2, finding M3): every top-level directory that can hold text, the
/// transcripts under `docs/handoff` (inside `docs`, scanned recursively), `ui`, `packaging`, benches and bench results, plus ALL
/// regular files in the repository root (CLAUDE.md, CORRECTIONS.md, MILESTONE_1_DESIGN_PLAN.md,
/// transcript_copy.jsonl, Cargo files, ...). Nothing is excluded except build output and VCS metadata.
pub fn leak_scan_paths(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = [
        "lab-results", "docs", "tests", "src", "scripts", "bench-results", "benches", "ui", "packaging",
    ]
    .iter()
    .map(|d| root.join(d))
    .filter(|p| p.exists())
    .collect();
    if let Ok(rd) = fs::read_dir(root) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_file() && !v.contains(&p) {
                v.push(p);
            }
        }
    }
    v
}

// ---------------------------------------------------------------------------
// Frozen tuning artifact
// ---------------------------------------------------------------------------

/// Directory of the protocol-v2.2 UNCONSTRAINED reference tuning artifact (`frozen_arm_params.json` + `tuning_trials.csv`).
/// The v2 / v2.1 artifacts in `lab-results/tuning_v2_original/` and `lab-results/tuning/` are historical.
pub fn frozen_tuning_dir_v2_2(root: &Path) -> PathBuf {
    root.join("lab-results/tuning_v2_2")
}

/// Directory of the REGISTERED tuning artifact (v2.2c: PIE-Retained constrained to a genuinely adaptive AIMD range,
/// max >= 4*min and max >= 8). `tuning_v2_2` is kept as the unconstrained reference record (its PIE optimum was static).
pub fn frozen_tuning_dir_v2_2c(root: &Path) -> PathBuf {
    root.join("lab-results/tuning_v2_2c")
}

/// Loads the frozen arm parameters of protocol v2.2 if the artifact exists; otherwise None (callers must not
/// treat untuned defaults as the registered configuration).
pub fn load_frozen_arm_params(root: &Path) -> Option<ArmParams> {
    load_frozen_arm_params_from(&frozen_tuning_dir_v2_2c(root).join("frozen_arm_params.json"))
}

/// Loads `arm_params` from an explicit `frozen_arm_params.json`.
pub fn load_frozen_arm_params_from(path: &Path) -> Option<ArmParams> {
    let text = fs::read_to_string(path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    serde_json::from_value(v["arm_params"].clone()).ok()
}

/// BL-6 must have an entry for every scenario it is run on (v2.2, finding H3): otherwise it silently equals BL-12b.
pub fn bl6_missing_scenarios(p: &ArmParams, scenarios: &[String]) -> Vec<String> {
    scenarios.iter().filter(|s| !p.bl6_by_scenario.contains_key(*s)).cloned().collect()
}

// ---------------------------------------------------------------------------
// H2 attribution and H4 stability (descriptive statistics from raw cells)
// ---------------------------------------------------------------------------

/// H2: share of the full adaptive gain on `metric` (lower is better) NOT explained by the
/// eviction/admission-only ablation: share = 1 - mean(ctrl - evict_only) / mean(ctrl - full), pooled over
/// scenarios and families per seed; percentile bootstrap (B = 10,000) over seeds for the interval.
/// Returns None when the full arm shows no gain (nothing to attribute).
// Same justification as nalyze_endpoints: positional parameters match the pre-registered H2 analysis call sites.
#[allow(clippy::too_many_arguments)]
pub fn h2_origin_mechanism_share(
    rows: &[CellSummary],
    ctrl: &str,
    full: &str,
    evict_only: &str,
    scenarios: &[String],
    families: &[String],
    metric: &Metric,
    seed: u64,
) -> Option<(f64, f64, f64)> {
    let mut g: BTreeMap<String, f64> = BTreeMap::new();
    let mut e: BTreeMap<String, f64> = BTreeMap::new();
    for sc in scenarios {
        let c = per_seed(rows, sc, ctrl, families, metric);
        let f = per_seed(rows, sc, full, families, metric);
        let v = per_seed(rows, sc, evict_only, families, metric);
        for (k, cv) in &c {
            if let (Some(fv), Some(vv)) = (f.get(k), v.get(k)) {
                *g.entry(k.clone()).or_insert(0.0) += cv - fv;
                *e.entry(k.clone()).or_insert(0.0) += cv - vv;
            }
        }
    }
    let keys: Vec<&String> = g.keys().collect();
    let n = keys.len();
    if n == 0 {
        return None;
    }
    let share = |idx: &[usize]| -> Option<f64> {
        let gm: f64 = idx.iter().map(|&i| g[keys[i]]).sum::<f64>() / idx.len() as f64;
        let em: f64 = idx.iter().map(|&i| e[keys[i]]).sum::<f64>() / idx.len() as f64;
        if gm <= 0.0 { None } else { Some(1.0 - em / gm) }
    };
    let all: Vec<usize> = (0..n).collect();
    let point = share(&all)?;
    let mut rng = super::rng::StreamRng::new(seed);
    let mut boots = Vec::new();
    for _ in 0..10_000 {
        let idx: Vec<usize> = (0..n).map(|_| rng.next_range_u64(0, (n - 1) as u64) as usize).collect();
        if let Some(s) = share(&idx) {
            boots.push(s);
        }
    }
    if boots.len() < 100 {
        return Some((point, f64::NAN, f64::NAN));
    }
    boots.sort_by(|a, b| a.partial_cmp(b).unwrap());
    Some((point, boots[(boots.len() as f64 * 0.025) as usize], boots[((boots.len() as f64 * 0.975) as usize).min(boots.len() - 1)]))
}

/// H4 proxy: 95th percentile (nearest rank) over cells of the number of dynamic-stale-window changes
/// per run for `arm`. Registered bound: <= 4.
pub fn h4_stale_window_changes_p95(rows: &[CellSummary], arm: &str) -> u64 {
    let mut v: Vec<u64> = rows.iter().filter(|r| r.arm == arm).map(|r| r.stale_window_changes).collect();
    if v.is_empty() {
        return 0;
    }
    v.sort_unstable();
    v[((v.len() as f64 * 0.95).ceil() as usize).clamp(1, v.len()) - 1]
}

/// Behaviour fingerprint of a cell (everything an experiment outcome could depend on, EXCLUDING the id, the arm
/// label and the two fields that merely record the controller's own state: `stale_window_changes` and
/// `max_stale_ms_final`).
fn behaviour_fingerprint(c: &CellSummary) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:x}|{:x}|{}|{}|{}|{}|{}|{}",
        c.requests, c.fresh_hits, c.miss_served, c.stale_served, c.failures, c.timeouts, c.shed, c.origin_dispatches,
        c.origin_errors, c.peak_outstanding, c.peak_servers_busy, c.cap_violations, c.p50_us, c.p95_us, c.p99_us,
        c.p99_success_us, c.mean_stale_age_ms.to_bits(), c.served_from_cache_ratio.to_bits(), c.final_cap,
        c.events_processed, c.attempts_total, c.cap_changes, c.stale_cap_violations_counter, c.stale_cap_violations_indep
    )
}

/// Compares arm `a` against arm `b` cell by cell on the (scenario, family, seed) key and returns
/// `(identical_cells, compared_cells)`. Used to detect an INERT dynamic stale window: if PIE-no-stale executes
/// identically to PIE-Retained in every cell, the stale-window mechanism had no observable effect (H2/H4).
pub fn arms_cell_for_cell_identical(rows: &[CellSummary], a: &str, b: &str) -> (usize, usize) {
    let mut map_a: BTreeMap<(String, String, String), String> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.arm == a) {
        map_a.insert((r.scenario.clone(), r.family.clone(), r.seed_label.clone()), behaviour_fingerprint(r));
    }
    let (mut same, mut total) = (0usize, 0usize);
    for r in rows.iter().filter(|r| r.arm == b) {
        if let Some(fa) = map_a.get(&(r.scenario.clone(), r.family.clone(), r.seed_label.clone())) {
            total += 1;
            if *fa == behaviour_fingerprint(r) {
                same += 1;
            }
        }
    }
    (same, total)
}

/// Run-once guard for the sealed evaluation (v2.2, finding C1c). Atomically creates `marker` (fails if it already
/// exists) containing the UTC timestamp, the freeze record hash and the protocol version - never any seed.
pub fn create_run_once_marker(marker: &Path, freeze_record_sha256: &str) -> Result<(), String> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker)
        .map_err(|e| format!("cannot create run-once marker {} (already started?): {e}", marker.display()))?;
    writeln!(
        f,
        "sealed run started\nutc={}\nfreeze_record_sha256={}\nprotocol_version={}",
        utc_now_iso(),
        freeze_record_sha256,
        super::rng::PROTOCOL_VERSION
    )
    .map_err(|e| e.to_string())?;
    f.sync_all().map_err(|e| e.to_string())
}

//! Library API of the Phantom Lab runner (D-3, owner decision O4-1).
//!
//! `RunSpec` (serde-serializable) -> [`execute`] -> `RunResult`. The same experiment
//! implementation ([`super::eval::run_matrix_with_progress`], which drives the real product cache,
//! policy executor and coalescing registry on a virtual clock) is used by the test suite, the
//! `phantom-lab` binary and any future API layer.
//!
//! Provenance: every result is tier SIMULATED. There is no real-process measurement in this module.
//!
//! Guards: the sealed seed set `S` and the sealed workload family `W-SF` are refused here. The
//! gated sealed evaluation is a separate path (see docs/research/preregistration_m6_v2.md).

use super::eval::{
    analyze_endpoints, load_frozen_arm_params, plain_seeds, read_cells_csv, repo_root, run_matrix_with_progress,
    EndpointResult, MatrixSpec, M_P99, M_PEAK,
};
use super::rng::SeedSets;
use super::t1_sim::{
    arm_ids, ArmParams, CellSummary, SimParams, HELD_OUT_FAMILIES, M6_SCENARIOS, SEALED_FAMILY, TUNING_FAMILIES,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// Legacy (M4) scenario ids that the simulator still accepts.
pub const LEGACY_SCENARIOS: [&str; 4] = ["O-NONE", "O-ST", "O-BR", "O-OUT"];

/// Seed used for the bootstrap/permutation resampling of the summary statistics.
pub const SUMMARY_BOOT_SEED: u64 = 20_261_029;

fn default_seed_set() -> String {
    "D".to_string()
}
fn default_profile() -> String {
    "smoke".to_string()
}
fn default_arm_params() -> String {
    "untuned".to_string()
}

/// Reusable experiment configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunSpec {
    pub name: String,
    pub scenarios: Vec<String>,
    pub families: Vec<String>,
    pub arms: Vec<String>,
    /// "T" (tuning) or "D" (development). "S" (sealed) is always refused.
    #[serde(default = "default_seed_set")]
    pub seed_set: String,
    /// Number of seeds taken from the start of the chosen set.
    pub n_seeds: usize,
    /// "smoke" (short horizon) or "standard" (registered horizon).
    #[serde(default = "default_profile")]
    pub profile: String,
    /// "untuned" (built-in defaults, no repository files needed) or "frozen" (lab-results/tuning artifact).
    #[serde(default = "default_arm_params")]
    pub arm_params: String,
    /// Also write per-request and per-fetch raw CSVs.
    #[serde(default)]
    pub record_raw: bool,
    /// Output directory (created if missing).
    pub out_dir: String,
    /// Allow writing into a directory that already holds a cells.csv.
    #[serde(default)]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LabError {
    /// The sealed seed set (or a name that could denote it) was requested.
    SealedSeedSetRefused,
    /// The sealed workload family W-SF was requested.
    SealedFamilyRefused,
    UnknownSeedSet(String),
    UnknownScenario(String),
    UnknownFamily(String),
    UnknownArm(String),
    UnknownProfile(String),
    UnknownArmParams(String),
    FrozenArmParamsMissing,
    EmptySelection(&'static str),
    InvalidSeedCount { requested: usize, available: usize },
    OutputExists(String),
    Io(String),
}

impl fmt::Display for LabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SealedSeedSetRefused => write!(f, "the sealed seed set S is refused by the lab runner (gated evaluation path only)"),
            Self::SealedFamilyRefused => write!(f, "workload family {SEALED_FAMILY} is reserved for the sealed evaluation and is refused"),
            Self::UnknownSeedSet(s) => write!(f, "unknown seed set '{s}' (allowed: T, D)"),
            Self::UnknownScenario(s) => write!(f, "unknown scenario '{s}'"),
            Self::UnknownFamily(s) => write!(f, "unknown workload family '{s}'"),
            Self::UnknownArm(s) => write!(f, "unknown arm '{s}'"),
            Self::UnknownProfile(s) => write!(f, "unknown profile '{s}' (allowed: smoke, standard)"),
            Self::UnknownArmParams(s) => write!(f, "unknown arm_params '{s}' (allowed: untuned, frozen)"),
            Self::FrozenArmParamsMissing => write!(f, "arm_params=frozen requested but lab-results/tuning_v2_2c/frozen_arm_params.json is missing or unreadable"),
            Self::EmptySelection(what) => write!(f, "empty selection: at least one {what} is required"),
            Self::InvalidSeedCount { requested, available } => write!(f, "n_seeds must be in 1..={available}, got {requested}"),
            Self::OutputExists(p) => write!(f, "output directory {p} already contains cells.csv (set overwrite to replace it)"),
            Self::Io(s) => write!(f, "i/o error: {s}"),
        }
    }
}

impl std::error::Error for LabError {}

/// Scenario ids the runner accepts.
pub fn list_scenarios() -> Vec<String> {
    M6_SCENARIOS.iter().chain(LEGACY_SCENARIOS.iter()).map(|s| s.to_string()).collect()
}
/// Arm (baseline / policy configuration) ids the runner accepts.
pub fn list_arms() -> Vec<String> {
    arm_ids().into_iter().map(|s| s.to_string()).collect()
}
/// Workload families runnable here (the sealed family W-SF is excluded).
pub fn list_families() -> Vec<String> {
    TUNING_FAMILIES.iter().chain(HELD_OUT_FAMILIES.iter()).map(|s| s.to_string()).collect()
}
/// Seed sets runnable here, with sizes. Seed values of the sealed set are never exposed.
pub fn list_seed_sets() -> Vec<(String, usize)> {
    vec![("T".to_string(), SeedSets::tuning_seeds().len()), ("D".to_string(), SeedSets::dev_eval_seeds().len())]
}

fn family_base(f: &str) -> &str {
    f.split('(').next().unwrap_or(f)
}

/// Validates a spec without running anything. Returns the resolved numeric seeds.
pub fn validate(spec: &RunSpec) -> Result<Vec<u64>, LabError> {
    let set = spec.seed_set.trim();
    let all = match set.to_ascii_uppercase().as_str() {
        "T" => SeedSets::tuning_seeds(),
        "D" => SeedSets::dev_eval_seeds(),
        "S" | "SEALED" => return Err(LabError::SealedSeedSetRefused),
        _ => return Err(LabError::UnknownSeedSet(spec.seed_set.clone())),
    };
    if spec.scenarios.is_empty() {
        return Err(LabError::EmptySelection("scenario"));
    }
    if spec.families.is_empty() {
        return Err(LabError::EmptySelection("workload family"));
    }
    if spec.arms.is_empty() {
        return Err(LabError::EmptySelection("arm"));
    }
    let scen = list_scenarios();
    for s in &spec.scenarios {
        if !scen.contains(s) {
            return Err(LabError::UnknownScenario(s.clone()));
        }
    }
    let fams = list_families();
    for f in &spec.families {
        let b = family_base(f);
        if b == SEALED_FAMILY {
            return Err(LabError::SealedFamilyRefused);
        }
        if !fams.iter().any(|x| x == b) {
            return Err(LabError::UnknownFamily(f.clone()));
        }
        // Parameterised form is only defined for W-Z(alpha).
        if f.contains('(') && (b != "W-Z" || !f.ends_with(')') || f[b.len() + 1..f.len() - 1].parse::<f64>().is_err()) {
            return Err(LabError::UnknownFamily(f.clone()));
        }
    }
    let arms = list_arms();
    for a in &spec.arms {
        if !arms.contains(a) {
            return Err(LabError::UnknownArm(a.clone()));
        }
    }
    if !matches!(spec.profile.as_str(), "smoke" | "standard") {
        return Err(LabError::UnknownProfile(spec.profile.clone()));
    }
    if !matches!(spec.arm_params.as_str(), "untuned" | "frozen") {
        return Err(LabError::UnknownArmParams(spec.arm_params.clone()));
    }
    if spec.n_seeds == 0 || spec.n_seeds > all.len() {
        return Err(LabError::InvalidSeedCount { requested: spec.n_seeds, available: all.len() });
    }
    let seeds: Vec<u64> = all[..spec.n_seeds].to_vec();
    // Defence in depth: the chosen seeds must be disjoint from the sealed set.
    SeedSets::verify_no_sealed_leak(&seeds).map_err(|_| LabError::SealedSeedSetRefused)?;
    Ok(seeds)
}

/// Total number of cells the spec will execute.
pub fn cell_count(spec: &RunSpec) -> usize {
    spec.n_seeds * spec.scenarios.len() * spec.families.len() * spec.arms.len()
}

#[derive(Debug, Clone)]
pub struct RunResult {
    pub spec: RunSpec,
    pub cells: Vec<CellSummary>,
    /// The manifest as written to `<out_dir>/manifest.json`.
    pub manifest: serde_json::Value,
    pub out_dir: PathBuf,
}

/// Executes the run. `progress(cells_done, cells_total)` is called after every cell.
/// Deterministic: the same `RunSpec` (and source) yields byte-identical `cells.csv` (and raw CSVs).
pub fn execute(spec: &RunSpec, progress: &mut dyn FnMut(usize, usize)) -> Result<RunResult, LabError> {
    let seeds = validate(spec)?;
    let out_dir = PathBuf::from(&spec.out_dir);
    if !spec.overwrite && out_dir.join("cells.csv").exists() {
        return Err(LabError::OutputExists(spec.out_dir.clone()));
    }
    let params = if spec.profile == "standard" { SimParams::standard() } else { SimParams::smoke() };
    let arm_params = if spec.arm_params == "frozen" {
        load_frozen_arm_params(&repo_root()).ok_or(LabError::FrozenArmParamsMissing)?
    } else {
        ArmParams::untuned_defaults()
    };
    let ms = MatrixSpec {
        name: spec.name.clone(),
        seed_set_name: spec.seed_set.trim().to_ascii_uppercase(),
        scenarios: spec.scenarios.clone(),
        families: spec.families.clone(),
        arms: spec.arms.clone(),
        seeds: plain_seeds(&seeds),
        params,
        arm_params,
        record_raw: spec.record_raw,
    };
    let run = run_matrix_with_progress(&ms, &out_dir, progress).map_err(LabError::Io)?;
    let mut manifest = run.manifest;
    if let Some(obj) = manifest.as_object_mut() {
        obj.insert("tool".into(), serde_json::json!("phantom-lab (src/lab/runner.rs)"));
        obj.insert("run_spec".into(), serde_json::to_value(spec).unwrap_or_default());
        obj.insert("profile".into(), serde_json::json!(spec.profile));
        obj.insert("arm_params_source".into(), serde_json::json!(spec.arm_params));
        obj.insert(
            "provenance".into(),
            serde_json::json!({
                "tier": "SIMULATED",
                "simulated_events": "request arrivals, origin service times, failures and degradation (seeded virtual-time model)",
                "real_components": "product cache, coalescing registry, policy executor and candidates executed in-process",
                "real_process_measurements": "none",
                "latency_unit": "virtual microseconds; not wall-clock measurements",
            }),
        );
    }
    let text = serde_json::to_string_pretty(&manifest).map_err(|e| LabError::Io(e.to_string()))?;
    std::fs::write(out_dir.join("manifest.json"), text).map_err(|e| LabError::Io(e.to_string()))?;
    Ok(RunResult { spec: spec.clone(), cells: run.cells, manifest, out_dir })
}

/// Reads `cells.csv` from an output directory (analysis consumes the raw file, as in the protocol).
pub fn read_cells(dir: &Path) -> Result<Vec<CellSummary>, LabError> {
    read_cells_csv(&dir.join("cells.csv")).map_err(LabError::Io)
}

/// Paired-by-seed comparison of `treat` against `ctrl` over the spec's scenarios and families for the
/// peak-origin-concurrency and client-P99 endpoints. Descriptive (not a preregistered test): the
/// lab runner never evaluates hypotheses on the sealed set.
pub fn paired_summary(result: &RunResult, treat: &str, ctrl: &str) -> Result<Vec<EndpointResult>, LabError> {
    for a in [treat, ctrl] {
        if !result.spec.arms.iter().any(|x| x == a) {
            return Err(LabError::UnknownArm(a.to_string()));
        }
    }
    Ok(analyze_endpoints(
        &result.cells,
        treat,
        ctrl,
        &result.spec.scenarios,
        &result.spec.families,
        &[M_PEAK, M_P99],
        0.05,
        SUMMARY_BOOT_SEED,
    ))
}

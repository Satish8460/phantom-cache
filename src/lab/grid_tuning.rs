//! Protocol v2.2 tuning (finding H4): symmetric, wider grids and an equal evaluation budget for every tuned arm.
//!
//! Replaces the v2/v2.1 tuning (18 configurations per arm; BL-12b cap=4 sat at the minimum of its grid, PIE-Retained
//! winners sat at grid corners). The old artifacts are kept unchanged in `lab-results/tuning_v2_original/` and
//! `lab-results/tuning/`; this module writes `lab-results/tuning_v2_2/` (`frozen_arm_params.json`, `tuning_trials.csv`).
//!
//! Wording: the result is "the best configuration found in the documented grid on tuning seeds T" for each arm. It is
//! NOT claimed to be the best deployable static policy.
//!
//! Budget: every tuned arm (BL-12b, BL-4, PIE-Retained) evaluates exactly `grid.budget` distinct configurations on the
//! same tuning seeds T, tuning families {W-Z, W-FC, W-SC} and all eight M6 scenarios.
//!   - BL-12b: exhaustive grid over (cap, stale window): 8 x 3 = 24.
//!   - BL-4: exhaustive grid over (p99 ceiling, max cap, min cap): 4 x 3 x 2 = 24.
//!   - PIE-Retained: six parameters (min cap, max cap, p99 ceiling, error threshold, AIMD window, base stale window;
//!     5 x 5 x 3 x 4 x 3 x 3 combinations on its value lists): a deterministic coordinate search from the fixed start
//!     point, 24 evaluations. It has MORE tuned parameters than BL-12b but exactly as many evaluated configurations.
//!
//! Objective (unchanged): mean over scenarios of 0.5 * (peak / peak_BL1) + 0.5 * (p99 / p99_BL1), subject to zero cap
//! and stale-cap violations, fresh-hit-ratio loss <= 1 pp and error-rate increase <= 0.5 pp versus BL-1 (all means).
//! BL-6 (per-scenario oracle) takes, for EACH of the 8 scenarios, the best member of the BL-12b grid for that scenario
//! (feasibility judged per scenario; if none is feasible the best violation-free score is used and the fallback is
//! recorded in the artifact).

use super::eval::{git_head, repo_root, utc_now_iso, SeedSpec};
use super::rng::SeedSets;
use super::t1_sim::*;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Axis indices of a PIE-Retained search point: [min, max, p99 ceiling, error threshold, AIMD window, base stale window].
pub type PiePoint = [usize; 6];

#[derive(Debug, Clone)]
pub struct TuningGrid {
    pub bl12b_caps: Vec<usize>,
    pub bl12b_stales: Vec<u64>,
    pub bl4_ceilings: Vec<u32>,
    pub bl4_maxes: Vec<usize>,
    pub bl4_mins: Vec<usize>,
    pub pie_mins: Vec<usize>,
    pub pie_maxes: Vec<usize>,
    pub pie_ceilings: Vec<u32>,
    pub pie_errs: Vec<f64>,
    pub pie_windows: Vec<usize>,
    pub pie_stales: Vec<u64>,
    pub pie_start: PiePoint,
    pub budget: usize,
    /// Adaptivity requirement for PIE-Retained (v2.2c): `Some((ratio, abs))` admits only points with max >= ratio*min and max >= abs.
    /// A controller with max == min is a static policy (already covered by BL-12b/BL-4).
    pub pie_adaptivity: Option<(usize, usize)>,
}

impl TuningGrid {
    /// The registered v2.2 grids.
    pub fn v2_2() -> Self {
        Self {
            bl12b_caps: vec![1, 2, 3, 4, 6, 8, 16, 32],
            bl12b_stales: vec![2_000, 10_000, 60_000],
            bl4_ceilings: vec![50, 100, 200, 400],
            bl4_maxes: vec![8, 16, 32],
            bl4_mins: vec![1, 2],
            pie_mins: vec![1, 2, 4],
            pie_maxes: vec![4, 8, 16, 32, 64],
            pie_ceilings: vec![50, 100, 200, 400, 800],
            pie_errs: vec![0.05, 0.1, 0.2, 0.4],
            pie_windows: vec![8, 16, 32],
            pie_stales: vec![2_000, 10_000, 60_000],
            // min=2, max=16, ceiling=200 ms, err=0.2, window=16, stale=10 s (the middle of every list).
            pie_start: [1, 2, 2, 2, 1, 1],
            budget: 24,
            pie_adaptivity: None,
        }
    }

    /// Round 2 of the v2.2 tuning (the single permitted extension): used because round-1 winners sat at grid boundaries
    /// (BL-12b stale window at its maximum; BL-4 max cap at its minimum; PIE-Retained max cap at its minimum, AIMD window and
    /// base stale window at their maxima). Each boundary that can be moved is moved by one value: stale windows gain 300 s,
    /// BL-4 and PIE max caps gain a smaller value (4 / 2), the PIE AIMD window gains 64. Budget rises to 32 for every arm.
    pub fn v2_2_extended() -> Self {
        Self {
            bl12b_caps: vec![1, 2, 3, 4, 6, 8, 16, 32],
            bl12b_stales: vec![2_000, 10_000, 60_000, 300_000],
            bl4_ceilings: vec![50, 100, 200, 400],
            bl4_maxes: vec![4, 8, 16, 32],
            bl4_mins: vec![1, 2],
            pie_mins: vec![1, 2, 4],
            pie_maxes: vec![2, 4, 8, 16, 32, 64],
            pie_ceilings: vec![50, 100, 200, 400, 800],
            pie_errs: vec![0.05, 0.1, 0.2, 0.4],
            pie_windows: vec![8, 16, 32, 64],
            pie_stales: vec![2_000, 10_000, 60_000, 300_000],
            // min=2, max=16, ceiling=200 ms, err=0.2, window=16, stale=10 s (interior on every axis).
            pie_start: [1, 3, 2, 2, 1, 1],
            budget: 32,
            pie_adaptivity: None,
        }
    }

    /// v2.2c round 1: the v2.2 round-1 value lists with the adaptivity requirement max >= 4*min and max >= 8, budget 24.
    pub fn v2_2c() -> Self {
        Self { pie_adaptivity: Some((4, 8)), ..Self::v2_2() }
    }

    /// v2.2c round 2 (the single permitted extension of the constrained tuning): wider max-cap and AIMD-window lists, budget 32.
    pub fn v2_2c_extended() -> Self {
        Self {
            pie_maxes: vec![4, 8, 16, 32, 64, 128],
            pie_adaptivity: Some((4, 8)),
            pie_start: [1, 2, 2, 2, 1, 1],
            ..Self::v2_2_extended()
        }
    }

    fn pie_point_valid(&self, pt: &PiePoint) -> bool {
        match self.pie_adaptivity {
            Some((ratio, abs)) => {
                let (mn, mx) = (self.pie_mins[pt[0]], self.pie_maxes[pt[1]]);
                mx >= ratio * mn && mx >= abs
            }
            None => true,
        }
    }

    pub fn describe(&self) -> serde_json::Value {
        json!({
            "budget_per_tuned_arm": self.budget,
            "BL-12b": {"cap": self.bl12b_caps, "stale_ms": self.bl12b_stales, "search": "exhaustive", "configs": self.bl12b_caps.len() * self.bl12b_stales.len()},
            "BL-4": {"p99_ceiling_ms": self.bl4_ceilings, "max": self.bl4_maxes, "min": self.bl4_mins, "search": "exhaustive", "configs": self.bl4_ceilings.len() * self.bl4_maxes.len() * self.bl4_mins.len()},
            "PIE-Retained": {
                "min": self.pie_mins, "max": self.pie_maxes, "p99_ceiling_ms": self.pie_ceilings, "error_threshold": self.pie_errs,
                "aimd_window": self.pie_windows, "base_stale_ms": self.pie_stales,
                "search": "deterministic coordinate search (axis order max, p99 ceiling, min, error threshold, window, base stale; pass 1 sweeps each axis around the incumbent, later passes probe +-1 neighbours, then diagonal neighbours) from the start point, exactly `budget` distinct evaluations",
                "start_point_indices": self.pie_start,
                "adaptivity_requirement": self.pie_adaptivity.map(|(r, a)| format!("max >= {r}*min and max >= {a}")),
                "full_combinations": self.pie_mins.len() * self.pie_maxes.len() * self.pie_ceilings.len() * self.pie_errs.len() * self.pie_windows.len() * self.pie_stales.len(),
            },
        })
    }

    fn pie_params(&self, base: &ArmParams, pt: &PiePoint) -> (ArmParams, String) {
        let mut p = base.clone();
        p.pie_min = self.pie_mins[pt[0]];
        p.pie_max = self.pie_maxes[pt[1]].max(p.pie_min);
        p.pie_p99_ceiling_ms = self.pie_ceilings[pt[2]];
        p.pie_error_threshold = self.pie_errs[pt[3]];
        p.pie_window = self.pie_windows[pt[4]];
        p.pie_base_stale_ms = self.pie_stales[pt[5]];
        let d = format!(
            "min={};max={};p99_ceiling_ms={};err_thr={};window={};base_stale_ms={}",
            p.pie_min, p.pie_max, p.pie_p99_ceiling_ms, p.pie_error_threshold, p.pie_window, p.pie_base_stale_ms
        );
        (p, d)
    }

    fn axis_len(&self, axis: usize) -> usize {
        [self.pie_mins.len(), self.pie_maxes.len(), self.pie_ceilings.len(), self.pie_errs.len(), self.pie_windows.len(), self.pie_stales.len()][axis]
    }
}

struct Evaluator<'a> {
    seeds: &'a [SeedSpec],
    scenarios: &'a [String],
    families: Vec<String>,
    params: &'a SimParams,
    refs: BTreeMap<(String, String, String), CellSummary>,
    csv: fs::File,
    base: ArmParams,
    /// per scenario: (score, cap, stale) of the best feasible BL-12b grid member.
    bl6_feasible: BTreeMap<String, (f64, usize, u64)>,
    /// per scenario: violation-free member closest to feasibility (smallest constraint excess, then best score): (excess, score, cap, stale).
    bl6_fallback: BTreeMap<String, (f64, f64, usize, u64)>,
    evaluations: BTreeMap<&'static str, usize>,
}

impl Evaluator<'_> {
    /// Evaluates one configuration on all (seed, scenario, family) cells and writes its rows. Returns (global score, feasible).
    fn eval(&mut self, arm: &'static str, p: &ArmParams, desc: &str) -> Result<(f64, bool), String> {
        let mut score: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        let mut hit: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        let mut err: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        let mut viol_sc: BTreeMap<String, u64> = BTreeMap::new();
        let mut viol = 0u64;
        for sd in self.seeds {
            for sc in self.scenarios {
                for fam in &self.families {
                    let spec = CellSpec {
                        scenario: sc.clone(),
                        family: fam.clone(),
                        arm: arm_config(arm, p, sc),
                        seed: sd.seed,
                        seed_label: sd.label.clone(),
                        params: self.params.clone(),
                        record_raw: false,
                    };
                    let c = run_cell(&spec).summary;
                    let r = &self.refs[&(sd.label.clone(), sc.clone(), fam.clone())];
                    let v = c.cap_violations + c.stale_cap_violations_counter + c.stale_cap_violations_indep + c.invariant_violations;
                    viol += v;
                    *viol_sc.entry(sc.clone()).or_default() += v;
                    let s = 0.5 * (c.peak_outstanding as f64 / r.peak_outstanding.max(1) as f64) + 0.5 * (c.p99_us as f64 / (r.p99_us.max(1)) as f64);
                    score.entry(sc.clone()).or_default().push(s);
                    hit.entry(sc.clone()).or_default().push(c.fresh_hit_ratio - r.fresh_hit_ratio);
                    err.entry(sc.clone()).or_default().push(c.error_rate - r.error_rate);
                }
            }
        }
        let mean = |v: &Vec<f64>| v.iter().sum::<f64>() / v.len().max(1) as f64;
        let (mut all_s, mut all_h, mut all_e) = (Vec::new(), Vec::new(), Vec::new());
        for sc in self.scenarios {
            let (ms, mh, me) = (mean(&score[sc]), mean(&hit[sc]), mean(&err[sc]));
            let sv = viol_sc[sc];
            writeln!(self.csv, "{},{},{},{:.6},{:.6},{:.6},{},{}", arm, desc, sc, ms, mh, me, sv, score[sc].len()).map_err(|e| e.to_string())?;
            all_s.push(ms);
            all_h.push(mh);
            all_e.push(me);
            if arm == "BL-12b" && sv == 0 {
                let entry = (ms, p.bl12b_cap, p.bl12b_stale_ms);
                let excess = (-0.01 - mh).max(0.0) + (me - 0.005).max(0.0);
                let fb = self.bl6_fallback.entry(sc.clone()).or_insert((f64::INFINITY, f64::INFINITY, 0, 0));
                if excess < fb.0 || (excess == fb.0 && ms < fb.1) {
                    *fb = (excess, ms, entry.1, entry.2);
                }
                if mh >= -0.01 && me <= 0.005 {
                    let f = self.bl6_feasible.entry(sc.clone()).or_insert((f64::INFINITY, 0, 0));
                    if ms < f.0 {
                        *f = entry;
                    }
                }
            }
        }
        let n = all_s.len() as f64;
        let (gs, gh, ge) = (all_s.iter().sum::<f64>() / n, all_h.iter().sum::<f64>() / n, all_e.iter().sum::<f64>() / n);
        let feasible = viol == 0 && gh >= -0.01 && ge <= 0.005;
        writeln!(self.csv, "{},{},ALL,{:.6},{:.6},{:.6},{},feasible={}", arm, desc, gs, gh, ge, viol, feasible).map_err(|e| e.to_string())?;
        *self.evaluations.entry(arm).or_default() += 1;
        Ok((gs, feasible))
    }
}

fn boundary_entry(arm: &str, param: &str, idx: usize, len: usize, value: String, natural_floor: bool) -> serde_json::Value {
    json!({
        "arm": arm, "param": param, "value": value,
        "at_grid_minimum": idx == 0, "at_grid_maximum": idx + 1 == len,
        "minimum_is_natural_floor": idx == 0 && natural_floor,
    })
}

fn index_of<T: PartialEq>(list: &[T], v: &T) -> usize {
    list.iter().position(|x| x == v).unwrap_or(0)
}

/// Runs the v2.2 tuning and writes `tuning_trials.csv` and `frozen_arm_params.json` into `out_dir`.
/// Tuning seeds must be a subset of T; the sealed set and the burned range are rejected.
pub fn run_tuning_v2_2(out_dir: &Path, seeds: &[SeedSpec], params: &SimParams, scenarios: &[String], grid: &TuningGrid) -> Result<ArmParams, String> {
    SeedSets::verify_no_sealed_leak(&seeds.iter().map(|s| s.seed).collect::<Vec<_>>()).map_err(|s| format!("sealed leak {s}"))?;
    let tset: std::collections::HashSet<u64> = SeedSets::tuning_seeds().into_iter().collect();
    if seeds.iter().any(|s| !tset.contains(&s.seed)) {
        return Err("tuning may only use tuning seeds T".to_string());
    }
    let b12 = grid.bl12b_caps.len() * grid.bl12b_stales.len();
    let b4 = grid.bl4_ceilings.len() * grid.bl4_maxes.len() * grid.bl4_mins.len();
    if b12 != grid.budget || b4 != grid.budget {
        return Err(format!("unequal budgets: BL-12b {b12}, BL-4 {b4}, declared {}", grid.budget));
    }
    let families: Vec<String> = TUNING_FAMILIES.iter().map(|s| s.to_string()).collect();
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let base = ArmParams::untuned_defaults();

    let mut refs: BTreeMap<(String, String, String), CellSummary> = BTreeMap::new();
    for sd in seeds {
        for sc in scenarios {
            for fam in &families {
                let spec = CellSpec { scenario: sc.clone(), family: fam.clone(), arm: arm_config("BL-1", &base, sc), seed: sd.seed, seed_label: sd.label.clone(), params: params.clone(), record_raw: false };
                refs.insert((sd.label.clone(), sc.clone(), fam.clone()), run_cell(&spec).summary);
            }
        }
    }
    let mut csv = fs::File::create(out_dir.join("tuning_trials.csv")).map_err(|e| e.to_string())?;
    writeln!(csv, "# tier=SIMULATED; protocol v2.2 tuning on tuning seeds T; not a measurement").map_err(|e| e.to_string())?;
    writeln!(csv, "arm,config,scenario,score,mean_fresh_hit_ratio_diff,mean_error_rate_diff,violations,cells").map_err(|e| e.to_string())?;
    let mut ev = Evaluator {
        seeds,
        scenarios,
        families: families.clone(),
        params,
        refs,
        csv,
        base: base.clone(),
        bl6_feasible: BTreeMap::new(),
        bl6_fallback: BTreeMap::new(),
        evaluations: BTreeMap::new(),
    };

    let mut frozen = base.clone();
    let mut notes: Vec<String> = Vec::new();
    let mut boundary: Vec<serde_json::Value> = Vec::new();

    // ---- BL-12b (exhaustive) ----
    let mut best12: Option<(f64, ArmParams)> = None;
    for &cap in &grid.bl12b_caps {
        for &stale in &grid.bl12b_stales {
            let mut p = ev.base.clone();
            p.bl12b_cap = cap;
            p.bl12b_stale_ms = stale;
            let (s, feas) = ev.eval("BL-12b", &p, &format!("cap={cap};stale_ms={stale}"))?;
            if feas && best12.as_ref().map(|b| s < b.0).unwrap_or(true) {
                best12 = Some((s, p));
            }
        }
    }
    match &best12 {
        Some((s, p)) => {
            notes.push(format!("BL-12b: best feasible score {s:.4} at cap={} stale_ms={}", p.bl12b_cap, p.bl12b_stale_ms));
            frozen.bl12b_cap = p.bl12b_cap;
            frozen.bl12b_stale_ms = p.bl12b_stale_ms;
            boundary.push(boundary_entry("BL-12b", "cap", index_of(&grid.bl12b_caps, &p.bl12b_cap), grid.bl12b_caps.len(), p.bl12b_cap.to_string(), true));
            boundary.push(boundary_entry("BL-12b", "stale_ms", index_of(&grid.bl12b_stales, &p.bl12b_stale_ms), grid.bl12b_stales.len(), p.bl12b_stale_ms.to_string(), false));
        }
        None => notes.push("BL-12b: NO feasible configuration; untuned default kept".to_string()),
    }

    // ---- BL-4 (exhaustive) ----
    let mut best4: Option<(f64, ArmParams)> = None;
    for &ceil in &grid.bl4_ceilings {
        for &max in &grid.bl4_maxes {
            for &min in &grid.bl4_mins {
                let mut p = ev.base.clone();
                p.bl4_p99_ceiling_ms = ceil;
                p.bl4_max = max;
                p.bl4_min = min;
                let (s, feas) = ev.eval("BL-4", &p, &format!("p99_ceiling_ms={ceil};max={max};min={min}"))?;
                if feas && best4.as_ref().map(|b| s < b.0).unwrap_or(true) {
                    best4 = Some((s, p));
                }
            }
        }
    }
    match &best4 {
        Some((s, p)) => {
            notes.push(format!("BL-4: best feasible score {s:.4}"));
            frozen.bl4_p99_ceiling_ms = p.bl4_p99_ceiling_ms;
            frozen.bl4_max = p.bl4_max;
            frozen.bl4_min = p.bl4_min;
            boundary.push(boundary_entry("BL-4", "p99_ceiling_ms", index_of(&grid.bl4_ceilings, &p.bl4_p99_ceiling_ms), grid.bl4_ceilings.len(), p.bl4_p99_ceiling_ms.to_string(), false));
            boundary.push(boundary_entry("BL-4", "max", index_of(&grid.bl4_maxes, &p.bl4_max), grid.bl4_maxes.len(), p.bl4_max.to_string(), false));
            boundary.push(boundary_entry("BL-4", "min", index_of(&grid.bl4_mins, &p.bl4_min), grid.bl4_mins.len(), p.bl4_min.to_string(), true));
        }
        None => notes.push("BL-4: NO feasible configuration; untuned default kept".to_string()),
    }

    // ---- PIE-Retained (coordinate search with the same budget) ----
    let mut memo: BTreeMap<PiePoint, (f64, bool)> = BTreeMap::new();
    let mut best_pt: Option<(PiePoint, f64)> = None;
    let mut cur: PiePoint = grid.pie_start;
    let axis_order = [1usize, 2, 0, 3, 4, 5];
    probe(grid, &mut ev, cur, &mut memo, &mut best_pt)?;
    if let Some((pt, _)) = best_pt {
        cur = pt;
    }
    // Pass 1: full sweep of each axis around the incumbent.
    for &axis in &axis_order {
        for j in 0..grid.axis_len(axis) {
            if j != cur[axis] {
                let mut pt = cur;
                pt[axis] = j;
                probe(grid, &mut ev, pt, &mut memo, &mut best_pt)?;
            }
        }
        if let Some((pt, _)) = best_pt {
            cur = pt;
        }
    }
    // Later passes: +-1 neighbours of the incumbent, axis by axis, until the budget is spent.
    let mut guard = 0;
    while memo.len() < grid.budget && guard < 6 {
        guard += 1;
        let before = memo.len();
        for &axis in &axis_order {
            for d in [-1i64, 1] {
                let j = cur[axis] as i64 + d;
                if j >= 0 && (j as usize) < grid.axis_len(axis) {
                    let mut pt = cur;
                    pt[axis] = j as usize;
                    probe(grid, &mut ev, pt, &mut memo, &mut best_pt)?;
                }
            }
            if let Some((pt, _)) = best_pt {
                cur = pt;
            }
        }
        if memo.len() == before {
            // Diagonal neighbours (two axes at once) when all axis neighbours have been seen.
            for &a in &axis_order {
                for &b in &axis_order {
                    if a < b {
                        for da in [-1i64, 1] {
                            for db in [-1i64, 1] {
                                let (ja, jb) = (cur[a] as i64 + da, cur[b] as i64 + db);
                                if ja >= 0 && jb >= 0 && (ja as usize) < grid.axis_len(a) && (jb as usize) < grid.axis_len(b) {
                                    let mut pt = cur;
                                    pt[a] = ja as usize;
                                    pt[b] = jb as usize;
                                    probe(grid, &mut ev, pt, &mut memo, &mut best_pt)?;
                                }
                            }
                        }
                    }
                }
            }
            if let Some((pt, _)) = best_pt {
                cur = pt;
            }
            if memo.len() == before {
                break;
            }
        }
    }
    match best_pt {
        Some((pt, s)) => {
            let (p, _) = grid.pie_params(&base, &pt);
            notes.push(format!("PIE-Retained: best feasible score {s:.4} after {} evaluations", memo.len()));
            frozen.pie_min = p.pie_min;
            frozen.pie_max = p.pie_max;
            frozen.pie_p99_ceiling_ms = p.pie_p99_ceiling_ms;
            frozen.pie_error_threshold = p.pie_error_threshold;
            frozen.pie_window = p.pie_window;
            frozen.pie_base_stale_ms = p.pie_base_stale_ms;
            let names = ["min", "max", "p99_ceiling_ms", "error_threshold", "aimd_window", "base_stale_ms"];
            let vals = [p.pie_min.to_string(), p.pie_max.to_string(), p.pie_p99_ceiling_ms.to_string(), p.pie_error_threshold.to_string(), p.pie_window.to_string(), p.pie_base_stale_ms.to_string()];
            for a in 0..6 {
                boundary.push(boundary_entry("PIE-Retained", names[a], pt[a], grid.axis_len(a), vals[a].clone(), a == 0));
            }
        }
        None => notes.push("PIE-Retained: NO feasible configuration; untuned default kept".to_string()),
    }
    let n_pie = memo.len();

    // ---- BL-6: per-scenario oracle over the BL-12b grid, ALL scenarios ----
    let mut bl6_notes = Vec::new();
    let mut bl6: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    for sc in scenarios {
        match ev.bl6_feasible.get(sc).filter(|e| e.0.is_finite()) {
            Some(e) => {
                bl6.insert(sc.clone(), (e.1, e.2));
            }
            None => match ev.bl6_fallback.get(sc).filter(|e| e.0.is_finite()) {
                Some(e) => {
                    bl6.insert(sc.clone(), (e.2, e.3));
                    bl6_notes.push(format!("{sc}: no grid member met the hit/error constraints; the violation-free member closest to feasibility (excess {:.4}) was used", e.0));
                }
                None => {
                    bl6.insert(sc.clone(), (frozen.bl12b_cap, frozen.bl12b_stale_ms));
                    bl6_notes.push(format!("{sc}: every grid member had violations; BL-12b parameters used"));
                }
            },
        }
    }
    frozen.bl6_by_scenario = bl6;

    let evals = ev.evaluations.clone();
    for (arm, n) in &evals {
        if *n != grid.budget {
            notes.push(format!("WARNING: {arm} evaluated {n} configurations, budget {}", grid.budget));
        }
    }
    let body = serde_json::to_string_pretty(&json!({
        "tier": "SIMULATED",
        "banner": "SIMULATION RESULT (T1 virtual-time simulation on tuning seeds T), NOT A MEASUREMENT",
        "protocol_version": super::rng::PROTOCOL_VERSION,
        "wording": "each arm's parameters are the best configuration found in the documented grid on tuning seeds T; not claimed to be the best deployable static policy",
        "tuning_seeds": seeds.iter().map(|s| s.seed).collect::<Vec<_>>(),
        "tuning_families": families,
        "tuning_scenarios": scenarios,
        "grid": grid.describe(),
        "evaluations_per_arm": evals,
        "pie_distinct_configs_evaluated": n_pie,
        "untuned_arms": ["BL-0", "BL-1", "BL-5 (fixed defaults)"],
        "notes": notes,
        "bl6_notes": bl6_notes,
        "grid_boundary": boundary,
        "tuned_at_utc": utc_now_iso(),
        "git_head": git_head(&repo_root()),
        "arm_params": frozen,
    }))
    .map_err(|e| e.to_string())?;
    fs::write(out_dir.join("frozen_arm_params.json"), body).map_err(|e| e.to_string())?;
    Ok(frozen)
}

/// Evaluates PIE point `pt` unless it was already evaluated or the budget is exhausted; updates the incumbent.
fn probe(
    grid: &TuningGrid,
    ev: &mut Evaluator,
    pt: PiePoint,
    memo: &mut BTreeMap<PiePoint, (f64, bool)>,
    best_pt: &mut Option<(PiePoint, f64)>,
) -> Result<bool, String> {
    if !grid.pie_point_valid(&pt) || memo.contains_key(&pt) || memo.len() >= grid.budget {
        return Ok(false);
    }
    let (p, desc) = grid.pie_params(&ev.base, &pt);
    let (s, feas) = ev.eval("PIE-Retained", &p, &desc)?;
    memo.insert(pt, (s, feas));
    if feas && best_pt.as_ref().map(|b| s < b.1).unwrap_or(true) {
        *best_pt = Some((pt, s));
    }
    Ok(true)
}

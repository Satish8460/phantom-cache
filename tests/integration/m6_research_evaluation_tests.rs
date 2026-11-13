//! Phase 6 (M6) research evaluation harness tests (R-02 + protocol repair v2).
//!
//! The previous version of this file computed the "adaptive arm" as a fixed multiple of the static arm and
//! a constant offset of the static hit ratio, used hard-coded constants for the ablation and fabricated `CellResult`
//! structs for the T1<->T2 check. All of that is removed. Here:
//!
//! - Cells run the REAL product components through `lab::t1_sim` (tier: SIMULATED).
//! - The normal suite runs a small deterministic SMOKE configuration on development seeds D only
//!   (not evidence about H1; used to prove that the harness works end to end and can fail).
//! - The full evaluation on the sealed seed set S is an `#[ignore]`d test (R-03) that needs the
//!   secret salt in `PHANTOM_SEALED_SALT_HEX` and a valid freeze record. It is NOT run by the
//!   normal suite and was not run by the author of this file.
//! - Analysis (paired-by-seed relative reductions, BCa CI, permutation p, Holm) reads the RAW
//!   per-cell CSV back from disk.
//!
//! Protocol: docs/research/preregistration_m6_v2.md (supersedes preregistration_m6.md).

#[path = "../support/lab/mod.rs"]
mod lab;

use lab::analysis::{HolmBonferroni, StatisticalEvaluator, SyntheticAnalysisValidator};
use lab::eval::*;
use lab::rng::{sha256_hex, SeedSets, SEALED_SALT_COMMITMENT_SHA256};
use lab::t1_sim::*;
use phantom_cache_demo::lab::grid_tuning::{run_tuning_v2_2, TuningGrid};
use std::path::PathBuf;

fn dev_seeds(n: usize) -> Vec<SeedSpec> {
    plain_seeds(&SeedSets::dev_eval_seeds()[..n])
}

fn smoke_spec(name: &str, arms: &[&str], scenarios: &[&str], families: &[&str], n_seeds: usize, raw: bool) -> MatrixSpec {
    MatrixSpec {
        name: name.to_string(),
        seed_set_name: "D".to_string(),
        scenarios: scenarios.iter().map(|s| s.to_string()).collect(),
        families: families.iter().map(|s| s.to_string()).collect(),
        arms: arms.iter().map(|s| s.to_string()).collect(),
        seeds: dev_seeds(n_seeds),
        params: SimParams::smoke(),
        // UNTUNED defaults: smoke runs prove mechanics only; the registered configuration is the
        // frozen tuning artifact (see preregistration_m6_v2.md section 7).
        arm_params: load_frozen_arm_params(&repo_root()).unwrap_or_else(ArmParams::untuned_defaults),
        record_raw: raw,
    }
}

fn out_dir(name: &str) -> PathBuf {
    repo_root().join("lab-results").join(name)
}

fn one_cell(scenario: &str, family: &str, arm: &str, seed: u64) -> CellSummary {
    let p = ArmParams::untuned_defaults();
    run_cell(&CellSpec {
        scenario: scenario.to_string(),
        family: family.to_string(),
        arm: arm_config(arm, &p, scenario),
        seed,
        seed_label: seed.to_string(),
        params: SimParams::smoke(),
        record_raw: false,
    })
    .summary
}

// ---------------------------------------------------------------------------
// Analysis machinery
// ---------------------------------------------------------------------------

#[test]
fn test_synthetic_analysis_pipeline_validation() {
    let result = SyntheticAnalysisValidator::verify_analysis_pipeline();
    assert!(result.is_ok(), "Synthetic analysis validation failed: {:?}", result.err());
}

#[test]
fn test_bca_permutation_and_holm_on_synthetic_data() {
    // Planted +8% effect (relative reductions around 0.08 with noise): detected by BCa and permutation.
    let (t, c) = SyntheticAnalysisValidator::generate_planted(30, 100.0, -0.08, 1.0, 7);
    let rel: Vec<f64> = t.iter().zip(&c).map(|(t, c)| (c - t) / c).collect();
    let (lo, hi) = StatisticalEvaluator::bca_ci_mean(&rel, 10_000, 11);
    assert!(lo > 0.0 && hi > lo, "planted effect not detected by BCa: [{lo}, {hi}]");
    assert!(StatisticalEvaluator::sign_flip_p_value(&rel, 3) < 0.001);
    // Null: CI spans zero, p large.
    let (t0, c0) = SyntheticAnalysisValidator::generate_planted(30, 100.0, 0.0, 1.0, 9);
    let rel0: Vec<f64> = t0.iter().zip(&c0).map(|(t, c)| (c - t) / c).collect();
    let (lo0, hi0) = StatisticalEvaluator::bca_ci_mean(&rel0, 10_000, 11);
    assert!(lo0 <= 0.0 && hi0 >= 0.0, "null dataset CI excludes zero: [{lo0}, {hi0}]");
    // Holm adjustment is monotone and >= raw p.
    let adj = HolmBonferroni::adjust(&[0.001, 0.01, 0.04, 0.06]);
    assert!(adj[0] >= 0.001 && adj[1] >= adj[0] && adj[2] >= adj[1] && adj[3] >= adj[2]);
    assert!((adj[0] - 0.004).abs() < 1e-12);
}

// ---------------------------------------------------------------------------
// Runner: real scenarios, real distinct arms, determinism
// ---------------------------------------------------------------------------

#[test]
fn test_every_m6_scenario_is_a_real_origin_degradation() {
    let seed = SeedSets::dev_eval_seeds()[0];
    let none = one_cell("O-NONE", "W-Z", "BL-1", seed);
    assert_eq!(none.tier, "SIMULATED");
    for sc in M6_SCENARIOS {
        let c = one_cell(sc, "W-Z", "BL-1", seed);
        // Each scenario must change what the executed events look like relative to a healthy origin.
        let differs = c.csv_row().replace(sc, "X") != none.csv_row().replace("O-NONE", "X");
        assert!(differs, "scenario {sc} produced the same executed result as the healthy origin");
        assert!(c.requests > 0);
    }
}

#[test]
fn test_runner_is_deterministic() {
    let a = one_cell("O-BRN", "W-HK", "PIE-Retained", 20_001);
    let b = one_cell("O-BRN", "W-HK", "PIE-Retained", 20_001);
    assert_eq!(a.csv_row(), b.csv_row());
}

#[test]
fn test_arms_are_distinct_real_configurations() {
    let p = ArmParams::untuned_defaults();
    let ids = ["BL-0", "BL-1", "BL-4", "BL-5", "BL-12b", "PIE-Retained"];
    let cfgs: Vec<ArmConfig> = ids.iter().map(|i| arm_config(i, &p, "O-DEG")).collect();
    for i in 0..cfgs.len() {
        for j in i + 1..cfgs.len() {
            assert_ne!(cfgs[i], cfgs[j], "{} and {} are the same configuration", ids[i], ids[j]);
        }
    }
    // Different configurations produce different executions under degradation (no shared result path).
    let seed = 20_002;
    let rows: Vec<String> = ids
        .iter()
        .map(|i| one_cell("O-DEG", "W-DI", i, seed).csv_row().split(',').skip(4).collect::<Vec<_>>().join(","))
        .collect();
    let distinct: std::collections::HashSet<&String> = rows.iter().collect();
    assert!(distinct.len() >= 4, "arms collapse to identical executions: {} distinct of {}", distinct.len(), rows.len());
}

#[test]
fn test_hard_cap_invariants_hold_in_every_smoke_cell() {
    // H3-style hard caps checked on executed cells (zero stale-cap and origin-cap violations;
    // byte accounting audit equality). Overhead (CPU / hit-path latency / RSS) is NOT measured by T1
    // and remains UNVERIFIED (needs a release-mode benchmark; see preregistration_m6_v2.md section 8).
    for arm in ["BL-12b", "PIE-Retained", "PLANT-CAP4"] {
        for sc in ["O-DEG", "O-BRN", "O-CO"] {
            let c = one_cell(sc, "W-HK", arm, 20_003);
            assert_eq!(c.stale_cap_violations_counter, 0, "{arm}/{sc}");
            assert_eq!(c.stale_cap_violations_indep, 0, "{arm}/{sc}");
            assert_eq!(c.cap_violations, 0, "{arm}/{sc}: dispatch accepted at/over the cap in force");
            assert!(c.bytes_audit_equal, "{arm}/{sc}: byte accounting audit mismatch");
        }
    }
}

// ---------------------------------------------------------------------------
// Smoke pipeline: raw data written, then analysed FROM DISK
// ---------------------------------------------------------------------------

#[test]
fn test_smoke_pipeline_writes_raw_and_analysis_consumes_it() {
    let spec = smoke_spec(
        "m6_smoke_dev_seeds",
        &["BL-1", "BL-12b", "PIE-Retained", "PIE-evict-only"],
        &["O-DEG", "O-BRN"],
        &["W-DI", "W-HK"],
        6,
        false,
    );
    let dir = out_dir("smoke_m6_dev");
    let run = run_matrix(&spec, &dir).expect("matrix run");
    assert_eq!(run.cells.len(), 6 * 2 * 2 * 4);

    // Raw data and manifest exist with required provenance fields.
    for f in ["cells.csv", "manifest.json"] {
        assert!(dir.join(f).exists(), "missing {f}");
    }
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["tier"], "SIMULATED");
    assert_eq!(manifest["seed_set"], "D");
    assert!(manifest["git_head"].as_str().unwrap().len() >= 40);
    assert!(manifest["started_utc"].as_str().unwrap().ends_with('Z'));
    assert!(manifest["source_sha256"].as_object().unwrap().len() > 10);
    assert!(manifest["output_files_sha256"]["cells.csv"].as_str().unwrap().len() == 64);

    // Seeds used are development seeds only; none in T or burned/sealed sets.
    let used: Vec<u64> = spec.seeds.iter().map(|s| s.seed).collect();
    assert!(SeedSets::verify_no_sealed_leak(&used).is_ok());
    assert!(used.iter().all(|s| SeedSets::dev_eval_seeds().contains(s)));

    // Analysis reads the CSV from disk (not the in-memory results).
    let rows = read_cells_csv(&dir.join("cells.csv")).expect("read cells.csv");
    assert_eq!(rows.len(), run.cells.len());
    assert!(rows.iter().all(|r| r.tier == "SIMULATED"));
    assert!(rows.iter().all(|r| r.bytes_audit_equal));
    assert!(rows.iter().all(|r| r.cap_violations == 0 && r.stale_cap_violations_counter == 0 && r.stale_cap_violations_indep == 0));

    // Per-request raw records are consistent with the per-cell summary.
    // Per-request / per-fetch raw records on a tiny matrix (keeps lab-results compact).
    let raw_spec = smoke_spec("m6_smoke_raw", &["BL-1", "PIE-Retained"], &["O-DEG"], &["W-DI"], 1, true);
    let raw_dir = out_dir("smoke_m6_raw");
    let raw_run = run_matrix(&raw_spec, &raw_dir).expect("raw matrix");
    for f in ["cells.csv", "requests.csv", "fetches.csv", "manifest.json"] {
        assert!(raw_dir.join(f).exists(), "missing {f}");
    }
    let req_text = std::fs::read_to_string(raw_dir.join("requests.csv")).unwrap();
    let first = &raw_run.cells[0];
    let n_req_rows = req_text.lines().filter(|l| l.starts_with(&format!("{},", first.cell_id))).count() as u64;
    assert_eq!(n_req_rows, first.requests, "requests.csv rows must equal summary request count");

    let scenarios: Vec<String> = spec.scenarios.clone();
    let families: Vec<String> = spec.families.clone();
    let res = analyze_endpoints(&rows, "PIE-Retained", "BL-12b", &scenarios, &families, &[M_PEAK, M_P99], 0.05, 4242);
    assert_eq!(res.len(), 4);
    for r in &res {
        assert_eq!(r.n_seeds, 6);
        assert!(r.ci_lo <= r.mean_rel_reduction + 1e-9 && r.mean_rel_reduction <= r.ci_hi + 1e-9 || r.ci_lo == r.ci_hi);
    }
    let cons = check_constraints(&rows, "PIE-Retained", "BL-12b", &scenarios, &families);
    // Report only (SMOKE on development seeds D; NOT evidence about H1).
    for r in &res {
        println!(
            "SMOKE(D,SIMULATED) {} {} PIE-Retained vs BL-12b: mean_rel_reduction={:.4} BCa95=[{:.4},{:.4}] p={:.4} holm_p={:.4} supported={}",
            r.scenario, r.endpoint, r.mean_rel_reduction, r.ci_lo, r.ci_hi, r.p_value, r.p_holm, r.supported
        );
    }
    match h2_origin_mechanism_share(&rows, "BL-12b", "PIE-Retained", "PIE-evict-only", &scenarios, &families, &M_PEAK, 99) {
        Some((s, lo, hi)) => println!("SMOKE(D,SIMULATED) H2 origin-mechanism share of peak-concurrency gain = {s:.3} [{lo:.3},{hi:.3}]"),
        None => println!("SMOKE(D,SIMULATED) H2 not applicable (no gain to attribute)"),
    }
    println!("SMOKE(D,SIMULATED) H4 proxy: PIE-Retained stale-window changes p95 = {}", h4_stale_window_changes_p95(&rows, "PIE-Retained"));
    for c in &cons {
        println!(
            "SMOKE(D,SIMULATED) {} d_fresh_hit_ratio={:.4} d_error_rate={:.4} cap_viol={} stale_viol={}",
            c.scenario, c.mean_fresh_hit_ratio_diff, c.mean_error_rate_diff, c.treat_cap_violations, c.treat_stale_cap_violations
        );
        assert_eq!(c.treat_cap_violations, 0);
        assert_eq!(c.treat_stale_cap_violations, 0);
    }
}

// ---------------------------------------------------------------------------
// Sensitivity checks: the harness must be able to FAIL
// ---------------------------------------------------------------------------

#[test]
fn test_sensitivity_a_disabling_the_adaptive_mechanism_yields_zero_effect() {
    // PIE-off = PIE-Retained's configuration with the executor installed but IntelligenceMode::Off.
    // CTRL-nonadaptive = same substrate without any adaptive component. Per-seed results must be
    // identical, so the analysis must report exactly zero effect and no support.
    let spec = smoke_spec("sens_a", &["PIE-off", "CTRL-nonadaptive"], &["O-DEG", "O-BRN", "O-SLW"], &["W-DI"], 6, false);
    let dir = out_dir("smoke_m6_sensitivity_a");
    run_matrix(&spec, &dir).unwrap();
    let rows = read_cells_csv(&dir.join("cells.csv")).unwrap();
    let res = analyze_endpoints(&rows, "PIE-off", "CTRL-nonadaptive", &spec.scenarios, &spec.families, &[M_PEAK, M_P99], 0.05, 77);
    for r in &res {
        assert_eq!(r.mean_rel_reduction, 0.0, "{} {}: disabled mechanism shows an effect", r.scenario, r.endpoint);
        assert!(!r.supported);
        assert!(r.ci_lo <= 0.0 && r.ci_hi >= 0.0);
    }
    // Positive control: with the mechanism ENABLED the executed result differs from the control
    // (otherwise the test above would be vacuous).
    let on = one_cell("O-DEG", "W-DI", "PIE-Retained", 20_001);
    let off = one_cell("O-DEG", "W-DI", "CTRL-nonadaptive", 20_001);
    assert_ne!(
        on.csv_row().split(',').skip(4).collect::<Vec<_>>(),
        off.csv_row().split(',').skip(4).collect::<Vec<_>>(),
        "PIE-Retained executes identically to the non-adaptive control: mechanism has no observable effect in this cell"
    );
}

#[test]
fn test_sensitivity_b_planted_real_effect_is_detected() {
    // PLANT-CAP4 is a deliberately better configuration for peak origin concurrency: BL-1 plus a
    // hard origin cap of 4, enforced by the real executor gate. The harness must detect it.
    let spec = smoke_spec("sens_b", &["BL-1", "PLANT-CAP4"], &["O-DEG", "O-GRD"], &["W-DI"], 10, false);
    let dir = out_dir("smoke_m6_sensitivity_b");
    run_matrix(&spec, &dir).unwrap();
    let rows = read_cells_csv(&dir.join("cells.csv")).unwrap();
    let res = analyze_endpoints(&rows, "PLANT-CAP4", "BL-1", &spec.scenarios, &spec.families, &[M_PEAK], 0.05, 5);
    for r in &res {
        assert!(r.mean_ctrl > 4.0 * 1.05, "{}: control peak {} too low for the planted cap to matter", r.scenario, r.mean_ctrl);
        assert!(r.supported, "planted effect NOT detected in {}: {:?}", r.scenario, r);
        assert!(r.mean_treat <= 4.0 + 1e-9, "cap of 4 not enforced: mean peak {}", r.mean_treat);
        assert!(r.ci_lo > 0.0);
    }
    // The cell with the cap removed (BL-1) differs from the capped one: deleting the cap changes the output.
    let capped = one_cell("O-DEG", "W-DI", "PLANT-CAP4", 20_001);
    let uncapped = one_cell("O-DEG", "W-DI", "BL-1", 20_001);
    assert!(capped.peak_outstanding < uncapped.peak_outstanding);
}

#[test]
fn test_sensitivity_c_old_shortcuts_are_gone() {
    let root = repo_root();
    let mut files = vec![
        root.join("tests/integration/m6_research_evaluation_tests.rs"),
        root.join("src/lab/matrix_runner.rs"),
        root.join("src/lab/t1_sim.rs"),
        root.join("src/lab/eval.rs"),
    ];
    files.sort();
    // Fragments are assembled at run time so this test does not match itself.
    let forbidden: Vec<String> = vec![
        format!("{}{}", "* 0", ".72"),
        format!("{}{}", "+ 0", ".02"),
        format!("{}{}", "* 0", ".85) as u64"),
        format!("{}{}", "peak_in_flight() *", " 2"),
        format!("{}{}", "let base_static", "_peak"),
        format!("{}{}", "full_adaptive", "_peak ="),
        format!("{}{}", "resident_keys", " = std::collections::HashSet"),
        format!("{}{}", "added_hit_latency", "_ns ="),
    ];
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        for frag in &forbidden {
            // Only code occurrences count: strip this test's own assembled strings by requiring the literal.
            assert!(!text.contains(frag.as_str()), "{} still contains shortcut fragment {:?}", f.display(), frag);
        }
    }
    // The runner module must drive the real components.
    let sim = std::fs::read_to_string(root.join("src/lab/t1_sim.rs")).unwrap();
    for needed in ["begin_fetch_atomic", "OriginFetchGuard", "try_acquire_origin_permit", "check_stale_serving", "SimulatedOrigin"] {
        assert!(sim.contains(needed), "t1_sim.rs no longer references {needed}");
    }
    // Mutation check on behaviour: removing the origin cap (PLANT-CAP4 -> BL-1) changes executed output.
    let a = one_cell("O-DEG", "W-DI", "PLANT-CAP4", 20_004);
    let b = one_cell("O-DEG", "W-DI", "BL-1", 20_004);
    assert_ne!(a.peak_outstanding, b.peak_outstanding);
}

#[test]
fn test_parameter_sensitivity_invariants_hold_for_pm50_percent() {
    // +/-50% variation of PIE hyperparameters: invariants must hold in every executed cell.
    // (Effect stability/flapping H4 is evaluated on the sealed run, not here.)
    for scale in [0.5f64, 1.0, 1.5] {
        let mut p = ArmParams::untuned_defaults();
        p.pie_p99_ceiling_ms = ((p.pie_p99_ceiling_ms as f64) * scale) as u32;
        p.pie_base_stale_ms = ((p.pie_base_stale_ms as f64) * scale) as u64;
        p.pie_max = ((p.pie_max as f64) * scale).max(2.0) as usize;
        let c = run_cell(&CellSpec {
            scenario: "O-FL".to_string(),
            family: "W-DI".to_string(),
            arm: arm_config("PIE-Retained", &p, "O-FL"),
            seed: 20_005,
            seed_label: "20005".to_string(),
            params: SimParams::smoke(),
            record_raw: false,
        })
        .summary;
        assert_eq!(c.cap_violations + c.stale_cap_violations_counter + c.stale_cap_violations_indep, 0, "scale {scale}");
        assert!(c.bytes_audit_equal);
        assert!(c.max_stale_ms_final <= 86_400_000);
    }
}

// ---------------------------------------------------------------------------
// Seed protocol: tuning / dev / sealed disjointness and hash-only sealed set
// ---------------------------------------------------------------------------

#[test]
fn test_seed_sets_disjoint_and_sealed_set_is_hash_only() {
    let t: std::collections::HashSet<u64> = SeedSets::tuning_seeds().into_iter().collect();
    let d: std::collections::HashSet<u64> = SeedSets::dev_eval_seeds().into_iter().collect();
    assert!(t.is_disjoint(&d));
    // Synthetic test salt exercises the derivation (the real salt is not in the repository).
    let s: std::collections::HashSet<u64> = SeedSets::derive_with_test_salt(b"m6-protocol-test-salt").into_iter().collect();
    assert_eq!(s.len(), 30);
    assert!(s.is_disjoint(&t) && s.is_disjoint(&d));
    // The burned legacy range is rejected everywhere.
    assert!(SeedSets::verify_no_sealed_leak(&[30_001]).is_err());
    assert!(SeedSets::verify_no_sealed_leak(&[30_030]).is_err());

    // Commitment: wrong salt must not derive anything.
    assert_eq!(SEALED_SALT_COMMITMENT_SHA256.len(), 64);
    assert!(SeedSets::derive_sealed_seeds(b"definitely-not-the-salt", SEALED_SALT_COMMITMENT_SHA256).is_err());
    assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");

    // No plaintext sealed seed list in code or docs: the old accessor is gone.
    let rng_src = std::fs::read_to_string(repo_root().join("src/lab/rng.rs")).unwrap();
    let old_accessor = format!("{}{}", "pub fn sealed_seeds", "() -> Vec<u64>");
    assert!(!rng_src.contains(&old_accessor));
    let old_hashes = format!("{}{}", "fn sealed_seed", "_hashes");
    assert!(!rng_src.contains(&old_hashes));
}

#[test]
fn test_sealed_seed_leak_scanner_flags_planted_seed_and_passes_clean_dir() {
    let seeds = SeedSets::derive_with_test_salt(b"scanner-test-salt");
    let tmp = std::env::temp_dir().join(format!("phantom_leak_scan_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(tmp.join("clean.csv"), "a,b,c\n1,2,3\n").unwrap();
    assert!(scan_for_seed_leak(std::slice::from_ref(&tmp), &seeds).is_ok());
    std::fs::write(tmp.join("leak.txt"), format!("run seed={}\n", seeds[7])).unwrap();
    assert!(scan_for_seed_leak(std::slice::from_ref(&tmp), &seeds).is_err());
    let _ = std::fs::remove_dir_all(&tmp);

    // If (and only if) the real salt is present in the environment, prove the real sealed seeds
    // do not appear anywhere in the repository's text files. Nothing is printed.
    if let Ok(real) = SeedSets::sealed_seeds_from_env() {
        let root = repo_root();
        let scanned = scan_for_seed_leak(&leak_scan_paths(&root), &real).expect("REAL sealed seed value found in the repository");
        assert!(scanned > 100);
    }
}

#[test]
fn test_leak_scan_scope_covers_the_registered_directories_and_root_files() {
    // v2.2 finding M3: the scan must reach docs (incl. docs/handoff transcripts), benches, bench-results, ui, packaging,
    // and every root file, and must detect UTF-16 text and invalid-UTF-8 files.
    let root = repo_root();
    let paths = leak_scan_paths(&root);
    for d in ["docs", "src", "tests", "scripts", "lab-results", "benches", "ui", "packaging"] {
        if root.join(d).exists() {
            assert!(paths.contains(&root.join(d)), "leak scan does not cover {d}");
        }
    }
    for f in ["CLAUDE.md", "CORRECTIONS.md", "MILESTONE_1_DESIGN_PLAN.md", "Cargo.toml", "README.md"] {
        if root.join(f).exists() {
            assert!(paths.contains(&root.join(f)), "leak scan does not cover root file {f}");
        }
    }
    if root.join("docs/handoff").exists() {
        assert!(paths.contains(&root.join("docs")), "docs/handoff must be reached through docs");
    }
    let seeds = SeedSets::derive_with_test_salt(b"scope-test-salt");
    let tmp = std::env::temp_dir().join(format!("phantom_leak_scope_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    // UTF-16LE with BOM (PowerShell redirection default).
    let text = format!("seed {}", seeds[3]);
    let mut bytes = vec![0xFF, 0xFE];
    for u in text.encode_utf16() {
        bytes.extend_from_slice(&u.to_le_bytes());
    }
    std::fs::write(tmp.join("u16.txt"), &bytes).unwrap();
    assert!(scan_for_seed_leak(std::slice::from_ref(&tmp), &seeds).is_err(), "UTF-16 leak not detected");
    std::fs::remove_file(tmp.join("u16.txt")).unwrap();
    // Invalid UTF-8 elsewhere in the file must not hide a seed.
    let mut b2 = vec![0xC3, 0x28, 0xFF];
    b2.extend_from_slice(format!(" {} ", seeds[9]).as_bytes());
    std::fs::write(tmp.join("bin.dat"), &b2).unwrap();
    assert!(scan_for_seed_leak(std::slice::from_ref(&tmp), &seeds).is_err(), "seed hidden by invalid UTF-8 not detected");
    // A single root-level file is a valid scan path.
    assert!(scan_for_seed_leak(&[tmp.join("bin.dat")], &seeds).is_err());
    let _ = std::fs::remove_dir_all(&tmp);
}

// ---------------------------------------------------------------------------
// Freeze verification (protocol v2.2)
// ---------------------------------------------------------------------------

/// Builds a synthetic repository root with a VALID v2.2 freeze, to exercise the valid path and every
/// fail-closed path of `verify_freeze` without touching the real tree.
fn synthetic_frozen_root(tag: &str) -> PathBuf {
    let tmp = std::env::temp_dir().join(format!("phantom_freeze_{}_{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    for d in ["docs/research", "src/lab", "lab-results/tuning_v2_2", "lab-results/tuning_v2_2c"] {
        std::fs::create_dir_all(tmp.join(d)).unwrap();
    }
    let w = |rel: &str, body: &str| std::fs::write(tmp.join(rel), body).unwrap();
    w("docs/research/preregistration_m6_v2.md", "prereg text");
    w("docs/research/preregistration_m6_v2_freeze.json", "original v2 freeze record");
    w("docs/research/preregistration_m6_v2_1_freeze.json", "v2.1 freeze record");
    w("docs/research/preregistration_m6_v2_1_amendment.md", "amendment v2.1 text");
    w("docs/research/preregistration_m6_v2_2_amendment.md", "amendment v2.2 text");
    w("Cargo.lock", "lock");
    w("Cargo.toml", "manifest");
    w("src/lib.rs", "lib");
    w("lab-results/tuning_v2_2/frozen_arm_params.json", "{}");
    w("lab-results/tuning_v2_2/tuning_trials.csv", "x");
    w("lab-results/tuning_v2_2c/frozen_arm_params.json", "{}");
    w("lab-results/tuning_v2_2c/tuning_trials.csv", "y");
    w("other.rs", "code");
    write_synthetic_record(&tmp, &serde_json::json!({}));
    tmp
}

/// Writes a v2.2 freeze record valid for the synthetic root, with `overrides` merged on top.
fn write_synthetic_record(root: &std::path::Path, overrides: &serde_json::Value) {
    let mut files = serde_json::Map::new();
    for (k, v) in source_hashes(root) {
        files.insert(k, serde_json::Value::String(v));
    }
    for m in MANDATORY_FROZEN_FILES {
        files.insert(m.to_string(), serde_json::Value::String(sha256_file(&root.join(m)).unwrap()));
    }
    files.insert("other.rs".to_string(), serde_json::Value::String(sha256_hex(b"code")));
    let mut rec = serde_json::json!({
        "protocol_version": phantom_cache_demo::lab::rng::PROTOCOL_VERSION,
        "required_build_profile": "release",
        "rustc_version": current_rustc_version().expect("rustc must be on PATH for the freeze tests"),
        "prereg_sha256": sha256_hex(b"prereg text"),
        "amendment_v2_1_sha256": sha256_hex(b"amendment v2.1 text"),
        "amendment_sha256": sha256_hex(b"amendment v2.2 text"),
        "supersedes_freeze_sha256": sha256_hex(b"v2.1 freeze record"),
        "supersedes_original_freeze_sha256": sha256_hex(b"original v2 freeze record"),
        "sealed_salt_commitment_sha256": SEALED_SALT_COMMITMENT_SHA256,
        "files_sha256": files,
        "frozen_at_utc": "2026-01-01T00:00:00Z"
    });
    if let (Some(r), Some(o)) = (rec.as_object_mut(), overrides.as_object()) {
        for (k, v) in o {
            r.insert(k.clone(), v.clone());
        }
    }
    std::fs::write(freeze_record_path(root), rec.to_string()).unwrap();
}

/// Edits the record's `files_sha256` map in place.
fn edit_record_files(root: &std::path::Path, f: impl Fn(&mut serde_json::Map<String, serde_json::Value>)) {
    let mut rec: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(freeze_record_path(root)).unwrap()).unwrap();
    f(rec["files_sha256"].as_object_mut().unwrap());
    std::fs::write(freeze_record_path(root), rec.to_string()).unwrap();
}

#[test]
fn test_freeze_verification_valid_path_accepts_a_consistent_v2_2_freeze() {
    let tmp = synthetic_frozen_root("valid");
    let v = verify_freeze(&tmp).expect("a consistent v2.2 freeze must verify");
    assert_eq!(v["protocol_version"], phantom_cache_demo::lab::rng::PROTOCOL_VERSION);
    assert_eq!(v["protocol_version"], "2.2");
    assert_eq!(v["required_build_profile"], "release");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_freeze_verification_fails_closed_on_every_invalid_state() {
    // No record at all.
    let none = std::env::temp_dir().join(format!("phantom_freeze_none_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&none);
    std::fs::create_dir_all(none.join("docs/research")).unwrap();
    assert!(verify_freeze(&none).is_err(), "missing record must fail closed");
    let _ = std::fs::remove_dir_all(&none);

    // Each case starts from a valid root, breaks exactly one thing and must be rejected.
    type Breaker = Box<dyn Fn(&PathBuf)>;
    let ov = |v: serde_json::Value| -> Breaker { Box::new(move |r| write_synthetic_record(r, &v)) };
    let cases: Vec<(&str, Breaker)> = vec![
        ("frozen file modified", Box::new(|r| std::fs::write(r.join("other.rs"), "code changed").unwrap())),
        ("frozen file removed", Box::new(|r| std::fs::remove_file(r.join("other.rs")).unwrap())),
        ("Cargo.lock modified", Box::new(|r| std::fs::write(r.join("Cargo.lock"), "lock changed").unwrap())),
        ("Cargo.toml modified", Box::new(|r| std::fs::write(r.join("Cargo.toml"), "manifest changed").unwrap())),
        ("src/lib.rs modified", Box::new(|r| std::fs::write(r.join("src/lib.rs"), "lib changed").unwrap())),
        ("re-tuned artifact modified", Box::new(|r| std::fs::write(r.join("lab-results/tuning_v2_2/frozen_arm_params.json"), "{\"x\":1}").unwrap())),
        ("mandatory Cargo.lock not listed", Box::new(|r| edit_record_files(r, |m| { m.remove("Cargo.lock"); }))),
        ("mandatory tuning artifact not listed", Box::new(|r| edit_record_files(r, |m| { m.remove("lab-results/tuning_v2_2/tuning_trials.csv"); }))),
        ("new tracked file not listed", Box::new(|r| std::fs::write(r.join("src/lab/sneaky.rs"), "// new").unwrap())),
        ("v2 preregistration edited", Box::new(|r| std::fs::write(r.join("docs/research/preregistration_m6_v2.md"), "prereg text edited").unwrap())),
        ("v2.1 amendment edited", Box::new(|r| std::fs::write(r.join("docs/research/preregistration_m6_v2_1_amendment.md"), "edited").unwrap())),
        ("v2.2 amendment edited", Box::new(|r| std::fs::write(r.join("docs/research/preregistration_m6_v2_2_amendment.md"), "amendment edited").unwrap())),
        ("v2.2 amendment missing", Box::new(|r| std::fs::remove_file(r.join("docs/research/preregistration_m6_v2_2_amendment.md")).unwrap())),
        ("v2.1 freeze record modified", Box::new(|r| std::fs::write(r.join("docs/research/preregistration_m6_v2_1_freeze.json"), "tampered").unwrap())),
        ("original v2 freeze record modified", Box::new(|r| std::fs::write(r.join("docs/research/preregistration_m6_v2_freeze.json"), "tampered").unwrap())),
        ("original v2 freeze record removed", Box::new(|r| std::fs::remove_file(r.join("docs/research/preregistration_m6_v2_freeze.json")).unwrap())),
        ("wrong protocol version", ov(serde_json::json!({"protocol_version": "2.1"}))),
        ("missing protocol version", ov(serde_json::json!({"protocol_version": null}))),
        ("debug profile recorded", ov(serde_json::json!({"required_build_profile": "debug"}))),
        ("no build profile recorded", ov(serde_json::json!({"required_build_profile": null}))),
        ("rustc version differs", ov(serde_json::json!({"rustc_version": "rustc 0.0.1 (fake)"}))),
        ("rustc version missing", ov(serde_json::json!({"rustc_version": null}))),
        ("commitment differs from the code", ov(serde_json::json!({"sealed_salt_commitment_sha256": "bc7c9683fb46b1e4d8f3b2c01612668c9591427402e49c576220bba3724f0e75"}))),
        ("empty timestamp", ov(serde_json::json!({"frozen_at_utc": ""}))),
        ("no files recorded", ov(serde_json::json!({"files_sha256": {}}))),
        ("malformed record", Box::new(|r| std::fs::write(freeze_record_path(r), "{ not json").unwrap())),
    ];
    for (name, breaker) in cases {
        let tmp = synthetic_frozen_root("invalid");
        assert!(verify_freeze(&tmp).is_ok(), "case '{name}': baseline must be valid before it is broken");
        breaker(&tmp);
        assert!(verify_freeze(&tmp).is_err(), "case '{name}' must fail closed");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

// ---------------------------------------------------------------------------
// Freeze addendum 1 (R-01): fail-closed acceptance of listed post-freeze changes only
// ---------------------------------------------------------------------------

/// Writes (or rewrites) a pinned addendum for the synthetic root. `entries` = (path, frozen, current).
/// `repin` controls whether the pin file is refreshed to the new addendum hash.
fn write_synthetic_addendum(root: &std::path::Path, entries: &[(&str, String, String)], repin: bool) {
    let md = root.join("docs/research/preregistration_m6_v2_2_addendum_1.md");
    std::fs::write(&md, "addendum prose").unwrap();
    let list: Vec<serde_json::Value> = entries
        .iter()
        .map(|(p, f, c)| serde_json::json!({"path": p, "frozen_sha256": f, "current_sha256": c, "category": "additive"}))
        .collect();
    let a = serde_json::json!({
        "addendum_version": 1,
        "protocol_version": phantom_cache_demo::lab::rng::PROTOCOL_VERSION,
        "freeze_record_sha256": sha256_file(&freeze_record_path(root)).unwrap(),
        "amendment_sha256": sha256_file(&amendment_doc_path(root)).unwrap(),
        "addendum_md_sha256": sha256_file(&md).unwrap(),
        "changed_files": list
    });
    write_addendum_json(root, &a, repin);
}

fn write_addendum_json(root: &std::path::Path, a: &serde_json::Value, repin: bool) {
    std::fs::write(addendum_1_path(root), a.to_string()).unwrap();
    if repin {
        std::fs::write(addendum_1_pin_path(root), format!("{}\n", sha256_file(&addendum_1_path(root)).unwrap())).unwrap();
    }
}

fn edit_addendum(root: &std::path::Path, repin: bool, f: impl Fn(&mut serde_json::Value)) {
    let mut a: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(addendum_1_path(root)).unwrap()).unwrap();
    f(&mut a);
    write_addendum_json(root, &a, repin);
}

/// Synthetic root where `other.rs` and `src/lib.rs` changed after the freeze and a valid pinned addendum lists both.
fn synthetic_root_with_addendum(tag: &str) -> PathBuf {
    let tmp = synthetic_frozen_root(tag);
    std::fs::write(tmp.join("other.rs"), "code + additive").unwrap();
    std::fs::write(tmp.join("src/lib.rs"), "lib + pub mod lab_live;").unwrap();
    write_synthetic_addendum(
        &tmp,
        &[
            ("other.rs", sha256_hex(b"code"), sha256_hex(b"code + additive")),
            ("src/lib.rs", sha256_hex(b"lib"), sha256_hex(b"lib + pub mod lab_live;")),
        ],
        true,
    );
    tmp
}

#[test]
fn test_addendum_valid_path_accepts_exactly_the_listed_changes() {
    let tmp = synthetic_root_with_addendum("add_valid");
    assert!(verify_freeze(&tmp).is_err(), "the pure v2.2 record must still reject the changed files");
    let v = verify_freeze_with_addendum(&tmp).expect("listed, pinned changes must verify");
    assert_eq!(v["protocol_version"], "2.2");
    let _ = std::fs::remove_dir_all(&tmp);
    // Without any addendum and without changes the function equals verify_freeze.
    let clean = synthetic_frozen_root("add_clean");
    assert!(verify_freeze_with_addendum(&clean).is_ok());
    let _ = std::fs::remove_dir_all(&clean);
}

#[test]
fn test_addendum_fails_closed_on_every_invalid_state() {
    type Breaker = Box<dyn Fn(&PathBuf)>;
    fn h(b: &[u8]) -> String {
        sha256_hex(b)
    }
    let cases: Vec<(&str, Breaker)> = vec![
        ("changed file with no entry (third file edited)", Box::new(|r| std::fs::write(r.join("Cargo.toml"), "manifest changed").unwrap())),
        ("listed file changed AGAIN after the addendum", Box::new(|r| std::fs::write(r.join("other.rs"), "code + additive + more").unwrap())),
        ("entry with wrong current hash", Box::new(|r| edit_addendum(r, true, |a| a["changed_files"][0]["current_sha256"] = serde_json::json!(h(b"nope"))))),
        ("entry with wrong frozen hash", Box::new(|r| edit_addendum(r, true, |a| a["changed_files"][0]["frozen_sha256"] = serde_json::json!(h(b"nope"))))),
        ("entry for a path not in the freeze", Box::new(|r| edit_addendum(r, true, |a| a["changed_files"].as_array_mut().unwrap().push(serde_json::json!({"path": "src/new_unfrozen.rs", "frozen_sha256": h(b"a"), "current_sha256": h(b"b")}))))),
        ("entry for a frozen file that did NOT change (stale/padded addendum)", Box::new(|r| {
            let cur = sha256_file(&r.join("Cargo.toml")).unwrap();
            edit_addendum(r, true, |a| a["changed_files"].as_array_mut().unwrap().push(serde_json::json!({"path": "Cargo.toml", "frozen_sha256": cur, "current_sha256": "0".repeat(64)})));
        })),
        ("entry with identical frozen and current hash", Box::new(|r| {
            let cur = sha256_file(&r.join("Cargo.toml")).unwrap();
            edit_addendum(r, true, |a| a["changed_files"].as_array_mut().unwrap().push(serde_json::json!({"path": "Cargo.toml", "frozen_sha256": cur, "current_sha256": cur})));
        })),
        ("duplicate entry", Box::new(|r| edit_addendum(r, true, |a| { let e = a["changed_files"][0].clone(); a["changed_files"].as_array_mut().unwrap().push(e); }))),
        ("addendum tries to excuse the v2 preregistration", Box::new(|r| {
            std::fs::write(r.join("docs/research/preregistration_m6_v2.md"), "prereg text edited").unwrap();
            let (f, c) = (h(b"prereg text"), h(b"prereg text edited"));
            edit_addendum(r, true, |a| a["changed_files"].as_array_mut().unwrap().push(serde_json::json!({"path": "docs/research/preregistration_m6_v2.md", "frozen_sha256": f, "current_sha256": c})));
        })),
        ("addendum tries to excuse a tuning artifact", Box::new(|r| {
            let p = "lab-results/tuning_v2_2/frozen_arm_params.json";
            let f = sha256_file(&r.join(p)).unwrap();
            std::fs::write(r.join(p), "{\"x\":1}").unwrap();
            let c = sha256_file(&r.join(p)).unwrap();
            edit_addendum(r, true, |a| a["changed_files"].as_array_mut().unwrap().push(serde_json::json!({"path": p, "frozen_sha256": f, "current_sha256": c})));
        })),
        ("pinned doc tampered (v2.2 amendment)", Box::new(|r| std::fs::write(r.join("docs/research/preregistration_m6_v2_2_amendment.md"), "edited").unwrap())),
        ("pinned doc tampered (v2.1 freeze record)", Box::new(|r| std::fs::write(r.join("docs/research/preregistration_m6_v2_1_freeze.json"), "tampered").unwrap())),
        ("addendum tampered without re-pin", Box::new(|r| edit_addendum(r, false, |a| a["changed_files"][0]["current_sha256"] = serde_json::json!(h(b"code + additive + sneaky"))))),
        ("addendum emptied without re-pin", Box::new(|r| edit_addendum(r, false, |a| a["changed_files"] = serde_json::json!([])))),
        ("addendum emptied WITH re-pin (changes now unlisted)", Box::new(|r| edit_addendum(r, true, |a| a["changed_files"] = serde_json::json!([])))),
        ("addendum prose tampered", Box::new(|r| std::fs::write(r.join("docs/research/preregistration_m6_v2_2_addendum_1.md"), "edited prose").unwrap())),
        ("addendum removed (pin left behind)", Box::new(|r| std::fs::remove_file(addendum_1_path(r)).unwrap())),
        ("pin removed (addendum left behind)", Box::new(|r| std::fs::remove_file(addendum_1_pin_path(r)).unwrap())),
        ("both addendum and pin removed while files differ", Box::new(|r| { std::fs::remove_file(addendum_1_path(r)).unwrap(); std::fs::remove_file(addendum_1_pin_path(r)).unwrap(); })),
        ("pin holds a wrong hash", Box::new(|r| std::fs::write(addendum_1_pin_path(r), "0".repeat(64)).unwrap())),
        ("addendum for another protocol version", Box::new(|r| edit_addendum(r, true, |a| a["protocol_version"] = serde_json::json!("2.1")))),
        ("addendum bound to a different freeze record", Box::new(|r| edit_addendum(r, true, |a| a["freeze_record_sha256"] = serde_json::json!("0".repeat(64))))),
        ("freeze record modified after the addendum was written", Box::new(|r| edit_record_files(r, |m| { m.insert("extra.rs".into(), serde_json::json!("0".repeat(64))); }))),
        ("malformed addendum (re-pinned)", Box::new(|r| { std::fs::write(addendum_1_path(r), "{ not json").unwrap(); std::fs::write(addendum_1_pin_path(r), sha256_file(&addendum_1_path(r)).unwrap()).unwrap(); })),
    ];
    for (name, breaker) in cases {
        let tmp = synthetic_root_with_addendum("add_invalid");
        assert!(verify_freeze_with_addendum(&tmp).is_ok(), "case '{name}': baseline must be valid before it is broken");
        breaker(&tmp);
        assert!(verify_freeze_with_addendum(&tmp).is_err(), "case '{name}' must fail closed");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

#[test]
fn test_release_requirement_is_enforced_by_helper() {
    // The sealed test and the dry run call this first; in a debug build it must refuse.
    assert_eq!(require_release_build().is_err(), cfg!(debug_assertions));
}

#[test]
fn test_tracked_inventory_includes_build_inputs() {
    let root = repo_root();
    let rel: Vec<String> = tracked_source_files(&root).iter().map(|p| p.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/")).collect();
    for needed in ["Cargo.lock", "Cargo.toml", "src/lib.rs", "src/lab/m6_pipeline.rs", "src/lab/grid_tuning.rs", "src/lab/eval.rs", "tests/integration/m6_research_evaluation_tests.rs"] {
        assert!(rel.iter().any(|r| r == needed), "{needed} missing from the freeze inventory");
    }
}

#[test]
fn test_historical_freeze_records_are_preserved() {
    // The original v2 and v2.1 records are historical, immutable evidence; v2.2 supersedes them without editing them.
    let root = repo_root();
    let orig = original_freeze_record_path(&root);
    assert!(orig.exists(), "the original v2 freeze record must be preserved");
    assert_ne!(orig, freeze_record_path(&root));
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&orig).unwrap()).unwrap();
    assert_eq!(v["frozen_at_utc"], "2026-10-29T14:09:47Z");
    assert_eq!(v["sealed_salt_commitment_sha256"], "bc7c9683fb46b1e4d8f3b2c01612668c9591427402e49c576220bba3724f0e75");
    let v21 = v2_1_freeze_record_path(&root);
    assert!(v21.exists(), "the v2.1 freeze record must be preserved");
    let r: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&v21).unwrap()).unwrap();
    assert_eq!(r["protocol_version"], "2.1");
    assert_ne!(v21, freeze_record_path(&root));
}

#[test]
fn test_real_v2_2_freeze_record_verifies_when_present() {
    let root = repo_root();
    let p = freeze_record_path(&root);
    if !p.exists() {
        println!("v2.2 freeze record not yet written (protocol not frozen)");
        return;
    }
    // Fail closed: if ANY frozen file changed after the freeze that is not listed in the pinned addendum 1
    // (R-01), the standard suite fails here, which forces a dated re-amendment instead of silently reusing
    // invalidated evidence. The pure v2.2 record alone (`verify_freeze`) is intentionally NOT expected to pass
    // any more: Cargo.toml and src/lib.rs received additive Scenario B edits after the sealed run.
    let v = verify_freeze_with_addendum(&root).expect("v2.2 freeze (+ addendum 1) no longer matches the tree: amend (dated) before using any evidence");
    assert_eq!(v["sealed_salt_commitment_sha256"], SEALED_SALT_COMMITMENT_SHA256);
    assert_eq!(v["prereg_sha256"].as_str().unwrap().len(), 64);
    assert!(v["frozen_at_utc"].as_str().unwrap().ends_with('Z'));
    assert!(v["evaluation_runs_before_freeze"].is_string());
}

// ---------------------------------------------------------------------------
// v2.2: verdict rule, secondary endpoints, exploratory columns, hardening, common random numbers
// ---------------------------------------------------------------------------

/// Builds one synthetic CellSummary (test data for the decision rule only; never used as evidence).
fn syn_row(scenario: &str, arm: &str, seed: usize, peak: u64, p99: u64, fresh: f64, err: f64) -> CellSummary {
    let spec = CellSpec {
        scenario: scenario.to_string(),
        family: "W-DI".to_string(),
        arm: arm_config("BL-1", &ArmParams::untuned_defaults(), scenario),
        seed: 0,
        seed_label: format!("D{seed:02}"),
        params: SimParams::smoke(),
        record_raw: false,
    };
    let mut c = defect_cell_output(&spec).summary;
    c.arm = arm.to_string();
    c.cell_id = format!("{scenario}|W-DI|{arm}|D{seed:02}");
    c.invariant_violations = 0;
    c.bytes_audit_equal = true;
    c.requests = 1000;
    c.peak_outstanding = peak;
    c.p99_us = p99;
    c.p99_success_us = p99;
    c.fresh_hit_ratio = fresh;
    c.error_rate = err;
    c
}

/// One effect per scenario for both endpoints (relative reduction of PIE vs BL-12b); `err_delta` per scenario.
fn syn_matrix(peak_eff: [f64; 8], p99_eff: [f64; 8], err_delta: [f64; 8], bl1_p99_factor: [f64; 8]) -> Vec<CellSummary> {
    let mut rows = Vec::new();
    for (i, sc) in M6_SCENARIOS.iter().enumerate() {
        for j in 0..30usize {
            let bp = 100 + (j % 7) as u64;
            let b99 = 200_000 + 1000 * (j % 11) as u64;
            let jitter = 0.002 * (j % 5) as f64;
            let pie_peak = ((bp as f64) * (1.0 - peak_eff[i] - jitter)).round().max(1.0) as u64;
            let pie_p99 = ((b99 as f64) * (1.0 - p99_eff[i] - jitter)).round().max(1.0) as u64;
            for arm in ["BL-12b", "BL-6"] {
                rows.push(syn_row(sc, arm, j, bp, b99, 0.80, 0.02));
            }
            rows.push(syn_row(sc, "BL-1", j, bp * 2, ((b99 as f64) * 2.0 * bl1_p99_factor[i]) as u64, 0.85, 0.05));
            for arm in ["PIE-Retained", "PIE-no-stale"] {
                rows.push(syn_row(sc, arm, j, pie_peak, pie_p99, 0.80, 0.02 + err_delta[i]));
            }
        }
    }
    rows
}

fn verdict_of(rows: &[CellSummary]) -> serde_json::Value {
    let scenarios: Vec<String> = M6_SCENARIOS.iter().map(|s| s.to_string()).collect();
    phantom_cache_demo::lab::m6_pipeline::compute_verdict(rows, &scenarios, &["W-DI".to_string()], 30)
}

#[test]
fn test_verdict_rule_is_mechanical_and_matches_the_registered_cases() {
    let none = [0.0; 8];
    let one = [1.0; 8];
    // A: strong, uniform win with constraints satisfied -> SUPPORTED.
    let v = verdict_of(&syn_matrix([0.30; 8], [0.30; 8], none, one));
    assert_eq!(v["H1"]["verdict"], "SUPPORTED", "{}", v["H1"]);
    assert_eq!(v["H1"]["supported_scenarios_peak_concurrency"], 8);
    // B: error rate +1 pp in one scenario: constraint fails, SUPPORTED withheld, no falsifier -> MIXED.
    let mut e = none;
    e[3] = 0.01;
    let v = verdict_of(&syn_matrix([0.30; 8], [0.30; 8], e, one));
    assert_eq!(v["H1"]["error_rate_constraint_holds_in_all_scenarios"], false);
    assert_eq!(v["H1"]["verdict"], "MIXED_INCONCLUSIVE");
    // C: an adverse CI on one endpoint in one scenario blocks SUPPORTED but does not by itself falsify.
    let mut pe = [0.30; 8];
    pe[2] = -0.30;
    let v = verdict_of(&syn_matrix(pe, [0.30; 8], none, one));
    assert_eq!(v["H1"]["any_adverse_ci_peak"], true);
    assert_eq!(v["H1"]["verdict"], "MIXED_INCONCLUSIVE");
    // D: negligible effect everywhere -> fewer than 3 supported / F-a / F-e -> NOT_SUPPORTED.
    let v = verdict_of(&syn_matrix([0.02; 8], [0.02; 8], none, one));
    assert_eq!(v["H1"]["verdict"], "NOT_SUPPORTED");
    assert_eq!(v["falsifiers"]["F-a"]["fires"], true);
    // E: 4 supported scenarios, 4 null scenarios: between 3 and 5 -> MIXED.
    let half = [0.30, 0.30, 0.30, 0.30, 0.0, 0.0, 0.0, 0.0];
    let v = verdict_of(&syn_matrix(half, half, none, one));
    assert_eq!(v["H1"]["supported_scenarios_peak_concurrency"], 4);
    assert_eq!(v["H1"]["verdict"], "MIXED_INCONCLUSIVE");
    // F: F-c: PIE's P99 more than 5% above BL-1 in one scenario overrides everything -> NOT_SUPPORTED.
    let mut f = one;
    f[0] = 0.3;
    let v = verdict_of(&syn_matrix([0.30; 8], [0.30; 8], none, f));
    assert_eq!(v["falsifiers"]["F-c"]["fires"], true);
    assert_eq!(v["H1"]["verdict"], "NOT_SUPPORTED");
    // G: F-d: one cap violation in a PIE-Retained cell -> NOT_SUPPORTED.
    let mut rows = syn_matrix([0.30; 8], [0.30; 8], none, one);
    rows.iter_mut().find(|r| r.arm == "PIE-Retained").unwrap().cap_violations = 1;
    let v = verdict_of(&rows);
    assert_eq!(v["falsifiers"]["F-d"]["fires"], true);
    assert_eq!(v["H1"]["verdict"], "NOT_SUPPORTED");
    // Every verdict carries the SIMULATED label and banner.
    assert_eq!(v["tier"], "SIMULATED");
    assert!(v["banner"].as_str().unwrap().contains("NOT A MEASUREMENT"));
}

#[test]
fn test_h2_h4_are_reported_not_testable_when_the_stale_window_is_inert() {
    let rows = syn_matrix([0.30; 8], [0.30; 8], [0.0; 8], [1.0; 8]);
    let (same, total) = arms_cell_for_cell_identical(&rows, "PIE-no-stale", "PIE-Retained");
    assert_eq!((same, total), (240, 240));
    let v = verdict_of(&rows);
    assert_eq!(v["H2"]["status"], "NOT_TESTABLE_AT_REGISTERED_HORIZON");
    assert_eq!(v["H4"]["status"], "NOT_TESTABLE_AT_REGISTERED_HORIZON");
    // One differing cell makes the precondition fail and the numbers are computed instead.
    let mut rows = rows;
    let c = rows.iter_mut().find(|r| r.arm == "PIE-no-stale").unwrap();
    c.p99_us += 1;
    assert_eq!(arms_cell_for_cell_identical(&rows, "PIE-no-stale", "PIE-Retained"), (239, 240));
    let v = verdict_of(&rows);
    assert_eq!(v["H2"]["status"], "COMPUTED_SEE_analysis_h2_h4.csv");
    // The controller's own bookkeeping alone (window changes, final window) does not count as an effect.
    let mut rows = syn_matrix([0.30; 8], [0.30; 8], [0.0; 8], [1.0; 8]);
    for r in rows.iter_mut().filter(|r| r.arm == "PIE-Retained") {
        r.stale_window_changes = 7;
        r.max_stale_ms_final = 60_000;
    }
    assert_eq!(arms_cell_for_cell_identical(&rows, "PIE-no-stale", "PIE-Retained"), (240, 240));
}

#[test]
fn test_exploratory_ratio_columns_and_extreme_ratio_flag() {
    // 29 seeds at -10% (r = 0.10) and one seed where the treatment is 22x the control (r = -21): flagged, not removed.
    let mut rows = Vec::new();
    for j in 0..30usize {
        rows.push(syn_row("O-DEG", "BL-12b", j, 100, 200_000, 0.8, 0.0));
        let t = if j == 7 { 2200 } else { 90 };
        rows.push(syn_row("O-DEG", "PIE-Retained", j, t, 180_000, 0.8, 0.0));
    }
    let res = analyze_endpoints(&rows, "PIE-Retained", "BL-12b", &["O-DEG".to_string()], &["W-DI".to_string()], &[M_PEAK, M_P99_SUCCESS], 0.05, 3);
    let peak = &res[0];
    assert_eq!(peak.n_extreme_ratio, 1);
    assert!((peak.expl_median_rel_reduction - 0.10).abs() < 1e-9);
    // The pre-registered mean is dragged far below the median by the single outlier; the exploratory log-ratio is shown beside it.
    assert!(peak.mean_rel_reduction < 0.0 && peak.expl_median_rel_reduction > 0.0);
    assert!(peak.expl_log_ratio_rel_reduction.is_finite());
    assert!(!peak.supported);
    // Secondary success-only P99 is a separate endpoint name.
    assert_eq!(res[1].endpoint, "client_p99_success_only_us");
    assert_eq!(res[1].n_extreme_ratio, 0);
    // Adverse flag (BCa upper bound < 0): treatment uniformly 2x the control.
    let mut bad = Vec::new();
    for j in 0..30usize {
        bad.push(syn_row("O-DEG", "BL-12b", j, 100 + j as u64 % 3, 200_000, 0.8, 0.0));
        bad.push(syn_row("O-DEG", "PIE-Retained", j, 200 + j as u64 % 3, 200_000, 0.8, 0.0));
    }
    let r = analyze_endpoints(&bad, "PIE-Retained", "BL-12b", &["O-DEG".to_string()], &["W-DI".to_string()], &[M_PEAK], 0.05, 3);
    assert!(r[0].adverse && !r[0].supported);
}

#[test]
fn test_error_rate_constraint_is_part_of_the_constraint_report() {
    let mut rows = syn_matrix([0.30; 8], [0.30; 8], [0.0, 0.0, 0.006, 0.0, 0.0, 0.0, 0.0, 0.0], [1.0; 8]);
    let sc: Vec<String> = M6_SCENARIOS.iter().map(|s| s.to_string()).collect();
    let c = check_constraints(&rows, "PIE-Retained", "BL-12b", &sc, &["W-DI".to_string()]);
    assert!(c[0].error_rate_ok && !c[2].error_rate_ok, "0.6 pp above BL-12b must violate the 0.5 pp margin");
    // Exactly at the margin passes.
    for r in rows.iter_mut().filter(|r| r.arm == "PIE-Retained" && r.scenario == "O-SLW") {
        r.error_rate = 0.02 + 0.005 - 1e-12;
    }
    let c = check_constraints(&rows, "PIE-Retained", "BL-12b", &sc, &["W-DI".to_string()]);
    assert!(c[2].error_rate_ok);
}

#[test]
fn test_defect_cell_is_recorded_round_trips_and_fails_completeness() {
    // A panic or invariant violation becomes a counted defect in the raw data (finding C1b), never an abort.
    let spec = CellSpec {
        scenario: "O-DEG".to_string(),
        family: "W-DI".to_string(),
        arm: arm_config("BL-1", &ArmParams::untuned_defaults(), "O-DEG"),
        seed: 1,
        seed_label: "D00".to_string(),
        params: SimParams::smoke(),
        record_raw: false,
    };
    let d = defect_cell_output(&spec).summary;
    let back = CellSummary::from_csv_row(&d.csv_row()).expect("defect row must round-trip");
    assert_eq!(back.invariant_violations, 1);
    assert!(!back.bytes_audit_equal);
    let problems = phantom_cache_demo::lab::m6_pipeline::completeness_problems(&[back], 1, 1, &ArmParams::untuned_defaults(), &[]);
    assert!(problems.iter().any(|p| p.contains("invariant")), "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("bytes_audit_equal")), "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("BL-6")), "BL-6 without per-scenario entries must be flagged: {problems:?}");
    // A healthy cell has zero invariant violations.
    assert_eq!(one_cell("O-BRN", "W-DI", "PIE-Retained", 20_001).invariant_violations, 0);
}

#[test]
fn test_brownout_faults_use_common_random_numbers_across_dispatch_orders() {
    use phantom_cache_demo::lab::origin_sim::{DegradationScenario, ServiceDistribution, SimulatedOrigin};
    use phantom_cache_demo::lab::rng::StreamRng;
    let scen = DegradationScenario::Brownout { start_us: 0, end_us: 10_000_000, error_rate: 0.5 };
    let mk = || SimulatedOrigin::new(8, ServiceDistribution::Constant(1000)).with_scenario(scen.clone()).with_fault_crn(77, 10_000);
    let (mut a, mut b) = (mk(), mk());
    let (mut ra, mut rb) = (StreamRng::new(1), StreamRng::new(2));
    let probes: Vec<(String, u64)> = (0..200u64).map(|i| (format!("/k{}", i % 17), 1_000 + i * 12_345)).collect();
    // Arm A dispatches in order; arm B dispatches extra unrelated fetches first and then in reverse order.
    let fate_a: Vec<bool> = probes.iter().map(|(k, t)| a.fetch(*t, k, 1, &mut ra).is_error).collect();
    for i in 0..50u64 {
        let _ = b.fetch(5_000_000 + i, "/noise", 1, &mut rb);
    }
    let mut fate_b = vec![false; probes.len()];
    for (idx, (k, t)) in probes.iter().enumerate().rev() {
        fate_b[idx] = b.fetch(*t, k, 1, &mut rb).is_error;
    }
    assert_eq!(fate_a, fate_b, "fault fate must depend only on (seed, key, time bucket), not on dispatch history");
    let errs = fate_a.iter().filter(|e| **e).count();
    assert!(errs > 50 && errs < 150, "50% brownout should fail about half of the probes, got {errs}");
}

#[test]
fn test_v2_2_tuning_grids_are_symmetric_wider_and_budget_equal() {
    let g = TuningGrid::v2_2();
    assert_eq!(g.bl12b_caps, vec![1, 2, 3, 4, 6, 8, 16, 32]);
    assert_eq!(g.bl12b_stales, vec![2_000, 10_000, 60_000]);
    let b12 = g.bl12b_caps.len() * g.bl12b_stales.len();
    let b4 = g.bl4_ceilings.len() * g.bl4_maxes.len() * g.bl4_mins.len();
    assert_eq!((b12, b4, g.budget), (24, 24, 24), "BL-12b, BL-4 and PIE-Retained must be given the same budget");
    // PIE-Retained is tuned over at least as many parameters as BL-12b (six vs two) with the same evaluation budget.
    let d = g.describe();
    assert_eq!(d["PIE-Retained"]["full_combinations"], 5 * 5 * 3 * 4 * 3 * 3);
    for (i, len) in [g.pie_mins.len(), g.pie_maxes.len(), g.pie_ceilings.len(), g.pie_errs.len(), g.pie_windows.len(), g.pie_stales.len()].iter().enumerate() {
        assert!(g.pie_start[i] > 0 && g.pie_start[i] + 1 < *len, "PIE start point must be interior on axis {i}");
    }
    // The registered caps include values below and above the old winner (4) and the old grid minimum.
    assert!(g.bl12b_caps.iter().any(|c| *c < 4) && g.bl12b_caps.iter().any(|c| *c > 4));
    // Round 2 (the single permitted extension) keeps the budgets equal and the start point interior.
    let x = TuningGrid::v2_2_extended();
    assert_eq!(x.bl12b_caps.len() * x.bl12b_stales.len(), 32);
    assert_eq!(x.bl4_ceilings.len() * x.bl4_maxes.len() * x.bl4_mins.len(), 32);
    assert_eq!(x.budget, 32);
    for (i, len) in [x.pie_mins.len(), x.pie_maxes.len(), x.pie_ceilings.len(), x.pie_errs.len(), x.pie_windows.len(), x.pie_stales.len()].iter().enumerate() {
        assert!(x.pie_start[i] > 0 && x.pie_start[i] + 1 < *len, "extended PIE start must be interior on axis {i}");
    }
}

#[test]
fn test_grid_tuning_runs_with_equal_budgets_writes_artifacts_and_tunes_bl6_for_all_scenarios() {
    // Miniature grid (budget 4) on 1 tuning seed and two scenarios, smoke horizon: exercises the code path of
    // the registered tuning (equal budgets, boundary report, BL-6 for every scenario, loader).
    let mut g = TuningGrid::v2_2();
    g.bl12b_caps = vec![2, 4];
    g.bl12b_stales = vec![2_000, 10_000];
    g.bl4_ceilings = vec![100, 200];
    g.bl4_maxes = vec![8];
    g.bl4_mins = vec![1, 2];
    g.budget = 4;
    let seeds = plain_seeds(&SeedSets::tuning_seeds()[..1]);
    let scenarios = vec!["O-BRN".to_string(), "O-FL".to_string()];
    let dir = std::env::temp_dir().join(format!("phantom_v22_tuning_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let frozen = run_tuning_v2_2(&dir, &seeds, &SimParams::smoke(), &scenarios, &g).expect("tuning");
    for sc in &scenarios {
        assert!(frozen.bl6_by_scenario.contains_key(sc), "BL-6 has no entry for {sc}");
    }
    assert!(bl6_missing_scenarios(&frozen, &scenarios).is_empty());
    let art: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("frozen_arm_params.json")).unwrap()).unwrap();
    assert_eq!(art["tier"], "SIMULATED");
    for arm in ["BL-12b", "BL-4", "PIE-Retained"] {
        assert_eq!(art["evaluations_per_arm"][arm], 4, "{arm} must be evaluated on exactly the budget");
    }
    assert!(art["grid_boundary"].is_array());
    let trials = std::fs::read_to_string(dir.join("tuning_trials.csv")).unwrap();
    assert!(trials.starts_with("# tier=SIMULATED"));
    assert_eq!(trials.lines().filter(|l| l.starts_with("PIE-Retained") && l.contains(",ALL,")).count(), 4);
    assert_eq!(load_frozen_arm_params_from(&dir.join("frozen_arm_params.json")).unwrap(), frozen);
    // Tuning may only use tuning seeds T.
    assert!(run_tuning_v2_2(&dir, &plain_seeds(&SeedSets::dev_eval_seeds()[..1]), &SimParams::smoke(), &scenarios, &g).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// Explicit evaluations (not part of the normal suite)
// ---------------------------------------------------------------------------

/// Tuning (protocol v2.2) on tuning seeds T and tuning families only. Writes lab-results/tuning_v2_2/ and refuses to
/// overwrite an existing artifact (move it aside first; evidence is never deleted).
/// Run explicitly: `cargo test --test m6_research_evaluation_tests --release -j 1 -- --ignored m6_tuning --nocapture`
#[test]
#[ignore = "explicit tuning run on tuning seeds T; writes lab-results/tuning_v2_2/"]
fn m6_tuning_on_tuning_seeds() {
    require_release_build().expect("tuning is run in the release profile");
    let n: usize = std::env::var("PHANTOM_TUNING_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(8);
    let seeds = plain_seeds(&SeedSets::tuning_seeds()[..n]);
    let scenarios: Vec<String> = M6_SCENARIOS.iter().map(|s| s.to_string()).collect();
    let constrained = std::env::var("PHANTOM_TUNING_GRID").ok().map(|s| s.starts_with("constrained")).unwrap_or(false);
    let dir = if constrained { frozen_tuning_dir_v2_2c(&repo_root()) } else { frozen_tuning_dir_v2_2(&repo_root()) };
    assert!(!dir.join("frozen_arm_params.json").exists(), "{} already holds a tuning artifact: move it aside first", dir.display());
    // Round 1 uses the registered grid; the single permitted extension (round 2) is selected with PHANTOM_TUNING_GRID=extended.
    let grid = match std::env::var("PHANTOM_TUNING_GRID").ok().as_deref() {
        Some("extended") => TuningGrid::v2_2_extended(),
        Some("constrained") => TuningGrid::v2_2c(),
        Some("constrained_extended") => TuningGrid::v2_2c_extended(),
        _ => TuningGrid::v2_2(),
    };
    let frozen = run_tuning_v2_2(&dir, &seeds, &SimParams::standard(), &scenarios, &grid).expect("tuning");
    println!("tuned BL-12b cap={} stale_ms={}", frozen.bl12b_cap, frozen.bl12b_stale_ms);
    println!("tuned PIE min={} max={} ceil={} err={} window={} stale={}", frozen.pie_min, frozen.pie_max, frozen.pie_p99_ceiling_ms, frozen.pie_error_threshold, frozen.pie_window, frozen.pie_base_stale_ms);
    println!("BL-6 by scenario: {:?}", frozen.bl6_by_scenario);
}

fn dry_run_dir() -> PathBuf {
    repo_root().join("lab-results").join(format!("m6_dryrun_{}", utc_now_iso().replace(':', "")))
}

/// Full-matrix DRY RUN on development seeds D with the IDENTICAL pipeline as the sealed evaluation (same families
/// including W-SF, 8 scenarios, 10 arms, `SimParams::standard()`, same analysis and verdict code), release profile.
/// 8 x 5 x 10 x 30 = 12,000 cells. Output: lab-results/m6_dryrun_<utc>/. This is DEVELOPMENT data (design validation),
/// not evidence about H1. It does not need a freeze record or the sealed salt and never touches the sealed set.
/// `PHANTOM_DRYRUN_N_SEEDS` (default 30) may shorten it for a smoke check; anything but 30 is NOT the registered dry run.
#[test]
#[ignore = "full-matrix dry run on development seeds D (release, ~10 min); writes lab-results/m6_dryrun_<utc>/"]
fn m6_full_matrix_dry_run_on_dev_seeds() {
    use phantom_cache_demo::lab::m6_pipeline::*;
    require_release_build().expect("the dry run mirrors the sealed run and requires a release build");
    let n: usize = std::env::var("PHANTOM_DRYRUN_N_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
    let all = SeedSets::dev_eval_seeds();
    assert_eq!(all.len(), 30, "the development set D must provide 30 seeds");
    assert!(SeedSets::verify_no_sealed_leak(&all).is_ok());
    let tset: std::collections::HashSet<u64> = SeedSets::tuning_seeds().into_iter().collect();
    assert!(all.iter().all(|s| !tset.contains(s)), "D and T must stay disjoint");
    let root = repo_root();
    let arm_params = load_frozen_arm_params(&root).expect("lab-results/tuning_v2_2c/frozen_arm_params.json missing: run m6_tuning with PHANTOM_TUNING_GRID=constrained first");
    let cfg = PipelineConfig {
        name: "m6_dry_run_dev_seeds".to_string(),
        seed_set_name: "D".to_string(),
        seeds: plain_seeds(&all[..n]),
        out_dir: dry_run_dir(),
        arm_params,
        record_raw: false,
        progress_every: 500,
    };
    let out = run_pipeline(&cfg).expect("pipeline");
    println!("DRYRUN(D,SIMULATED) dir={}", out.out_dir.display());
    println!("DRYRUN(D,SIMULATED) cells={} expected={} runtime_s={:.1} registered_full_dry_run={}", out.cells, out.expected_cells, out.runtime_secs, n == 30);
    println!("DRYRUN(D,SIMULATED) problems={:?}", out.problems);
    println!("DRYRUN(D,SIMULATED) H1 verdict={}", out.verdict["H1"]["verdict"]);
    assert_eq!(out.cells, out.expected_cells);
    if n == 30 {
        assert_eq!(out.expected_cells, 12_000);
    }
    assert!(out.problems.is_empty(), "dry run completeness problems: {:?}", out.problems);
}

/// R-03: the full sealed-seed evaluation. NOT run by the author of this harness. Requires:
///   - a RELEASE build (`cargo test --release`),
///   - PHANTOM_SEALED_SALT_HEX (secret salt whose SHA-256 equals the committed value),
///   - a valid docs/research/preregistration_m6_v2_2_freeze.json (all frozen files unchanged, rustc as recorded),
///   - the frozen tuning artifact lab-results/tuning_v2_2/frozen_arm_params.json,
///   - NO run-once marker lab-results/m6_sealed_run_started.marker (it is created, atomically, before cell 1 and
///     never removed: the sealed set can be spent only once, whether or not the run completes).
///
/// Raw data, analysis and `verdict.json` are written to lab-results/m6_sealed_<utc>/ (seed values never written).
/// Per-request raw records: set PHANTOM_M6_RAW_REQUESTS=1 (large).
#[test]
#[ignore = "R-03: sealed-seed evaluation; requires secret salt, valid freeze record and a release build; run-once"]
fn m6_full_sealed_evaluation() {
    use phantom_cache_demo::lab::m6_pipeline::*;
    require_release_build().expect("the sealed evaluation requires a release build");
    let root = repo_root();
    let marker = root.join("lab-results").join("m6_sealed_run_started.marker");
    assert!(!marker.exists(), "REFUSING TO RUN: the sealed evaluation was already started (marker {} exists); the sealed set can be used only once", marker.display());
    verify_freeze(&root).expect("protocol is not frozen or frozen files changed");
    let seeds = SeedSets::sealed_seeds_from_env().expect("sealed salt unavailable");
    assert_eq!(seeds.len(), 30);
    let arm_params = load_frozen_arm_params(&root).expect("frozen tuning artifact missing");
    assert!(bl6_missing_scenarios(&arm_params, &pipeline_scenarios()).is_empty(), "BL-6 must be tuned for all 8 scenarios");
    let scanned_before = scan_for_seed_leak(&leak_scan_paths(&root), &seeds).expect("sealed seed leaked into repository files before the run");
    assert!(scanned_before > 100);

    // Run-once marker: timestamp and freeze record hash, NOT seeds. Creation is atomic and fails if it exists.
    let freeze_hash = sha256_file(&freeze_record_path(&root)).expect("freeze record hash");
    create_run_once_marker(&marker, &freeze_hash).expect("run-once marker");

    let cfg = PipelineConfig {
        name: "m6_sealed_evaluation".to_string(),
        seed_set_name: "S".to_string(),
        seeds: sealed_seed_specs(&seeds),
        out_dir: root.join("lab-results").join(format!("m6_sealed_{}", utc_now_iso().replace(':', ""))),
        arm_params,
        record_raw: std::env::var("PHANTOM_M6_RAW_REQUESTS").ok().as_deref() == Some("1"),
        progress_every: 1000,
    };
    let out = run_pipeline(&cfg).expect("pipeline");

    // Post-run leak scan over the whole registered scope (the run's own output is included).
    let scanned_after = scan_for_seed_leak(&leak_scan_paths(&root), &seeds).expect("sealed seed value found in the repository AFTER the run");
    assert!(scanned_after >= scanned_before);
    println!("sealed evaluation complete: cells={} expected={} runtime_s={:.1}; data in {}", out.cells, out.expected_cells, out.runtime_secs, out.out_dir.display());
    println!("sealed H1 verdict (mechanical): {}", out.verdict["H1"]["verdict"]);
    // Fail loudly AFTER everything has been recorded. No assertion on the sign of the effect.
    assert_eq!(out.cells, out.expected_cells, "cell count");
    assert_eq!(out.expected_cells, 12_000);
    assert!(out.problems.is_empty(), "sealed run completeness problems: {:?}", out.problems);
}

/// Environment calibration on a TUNING seed only (T). Prints per-scenario executed results of three
/// arms so the fixed simulation parameters can be sanity-checked before freezing. Not evidence.
#[test]
#[ignore = "calibration print on tuning seed T"]
fn m6_calibration_report_tuning_seed() {
    let p = SimParams::standard();
    let ap = ArmParams::untuned_defaults();
    for sc in std::iter::once("O-NONE").chain(M6_SCENARIOS.iter().copied()) {
        for arm in ["BL-1", "BL-12b", "PIE-Retained"] {
            let c = run_cell(&CellSpec {
                scenario: sc.to_string(),
                family: "W-Z".to_string(),
                arm: arm_config(arm, &ap, sc),
                seed: SeedSets::tuning_seeds()[0],
                seed_label: "T1".to_string(),
                params: p.clone(),
                record_raw: false,
            })
            .summary;
            println!(
                "CAL(T,SIMULATED) {sc:7} {arm:13} req={} fresh={:.3} stale={} fail={} to={} shed={} disp={} peak={} p50={}us p99={}us stale_chg={} cap_chg={} final_cap={}",
                c.requests, c.fresh_hit_ratio, c.stale_served, c.failures, c.timeouts, c.shed, c.origin_dispatches, c.peak_outstanding, c.p50_us, c.p99_us, c.stale_window_changes, c.cap_changes, c.final_cap
            );
        }
    }
}

#[test]
fn test_run_once_marker_is_atomic_refuses_a_second_start_and_contains_no_seed() {
    let tmp = std::env::temp_dir().join(format!("phantom_marker_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    let marker = tmp.join("m6_sealed_run_started.marker");
    let fh = sha256_hex(b"freeze record");
    create_run_once_marker(&marker, &fh).expect("first start");
    let text = std::fs::read_to_string(&marker).unwrap();
    assert!(text.contains("sealed run started") && text.contains(&fh) && text.contains("utc=") && text.contains("protocol_version=2.2"));
    assert!(create_run_once_marker(&marker, &fh).is_err(), "a second start must be refused");
    // The marker never contains a seed-like (>= 10 digit) number.
    let long_digits = text.split(|c: char| !c.is_ascii_digit()).any(|t| t.len() >= 10);
    assert!(!long_digits, "marker must not contain seed-like numbers: {text}");
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_adaptivity_requirement_constrains_the_pie_search_space() {
    // v2.2c: PIE-Retained's AIMD range must satisfy max >= 4*min and max >= 8 (a min == max controller is a static policy).
    let g = TuningGrid::v2_2c();
    assert_eq!(g.pie_adaptivity, Some((4, 8)));
    assert_eq!(g.budget, 24);
    assert!(TuningGrid::v2_2().pie_adaptivity.is_none(), "the unconstrained reference grid stays unconstrained");
    assert_eq!(g.describe()["PIE-Retained"]["adaptivity_requirement"], "max >= 4*min and max >= 8");
    // Miniature constrained tuning: winner must satisfy the requirement and evaluations stay equal to the budget.
    let mut m = TuningGrid::v2_2c();
    m.bl12b_caps = vec![2, 4];
    m.bl12b_stales = vec![2_000, 10_000];
    m.bl4_ceilings = vec![100, 200];
    m.bl4_maxes = vec![8];
    m.bl4_mins = vec![1, 2];
    m.budget = 4;
    let dir = std::env::temp_dir().join(format!("phantom_v22c_tuning_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let frozen = run_tuning_v2_2(&dir, &plain_seeds(&SeedSets::tuning_seeds()[..1]), &SimParams::smoke(), &["O-DEG".to_string()], &m).expect("tuning");
    assert!(frozen.pie_max >= 4 * frozen.pie_min && frozen.pie_max >= 8, "min {} max {}", frozen.pie_min, frozen.pie_max);
    let art: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("frozen_arm_params.json")).unwrap()).unwrap();
    assert_eq!(art["evaluations_per_arm"]["PIE-Retained"], 4);
    let _ = std::fs::remove_dir_all(&dir);
}

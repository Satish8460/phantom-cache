//! D-3: tests of the reusable lab runner library API (`phantom_cache_demo::lab::runner`).
//!
//! These use the library directly (no test-only harness), i.e. the same code path as the
//! `phantom-lab` binary. Outputs are tier SIMULATED development-seed smoke runs (not evidence
//! about any hypothesis).

use phantom_cache_demo::lab::eval::repo_root;
use phantom_cache_demo::lab::runner::{
    cell_count, execute, list_arms, list_families, list_scenarios, paired_summary, read_cells, validate, LabError, RunSpec,
};

fn spec(name: &str, scenarios: &[&str], families: &[&str], arms: &[&str], n_seeds: usize) -> RunSpec {
    RunSpec {
        name: name.to_string(),
        scenarios: scenarios.iter().map(|s| s.to_string()).collect(),
        families: families.iter().map(|s| s.to_string()).collect(),
        arms: arms.iter().map(|s| s.to_string()).collect(),
        seed_set: "D".to_string(),
        n_seeds,
        profile: "smoke".to_string(),
        arm_params: "untuned".to_string(),
        record_raw: false,
        out_dir: repo_root().join("lab-results").join(format!("smoke_{name}")).to_string_lossy().to_string(),
        overwrite: true,
    }
}

fn no_progress() -> impl FnMut(usize, usize) {
    |_, _| {}
}

#[test]
fn same_runspec_twice_gives_identical_cells_and_bytes() {
    let mut a = spec("lib_det_a", &["O-DEG", "O-BRN"], &["W-DI"], &["BL-1", "BL-12b", "PIE-Retained"], 2);
    a.record_raw = true;
    let mut b = a.clone();
    b.name = "lib_det_b".into();
    b.out_dir = repo_root().join("lab-results").join("smoke_lib_det_b").to_string_lossy().to_string();
    let ra = execute(&a, &mut no_progress()).unwrap();
    let rb = execute(&b, &mut no_progress()).unwrap();
    assert_eq!(ra.cells.len(), cell_count(&a));
    let rows = |r: &phantom_cache_demo::lab::runner::RunResult| r.cells.iter().map(|c| c.csv_row()).collect::<Vec<_>>();
    assert_eq!(rows(&ra), rows(&rb), "same RunSpec must produce identical cells");
    for f in ["cells.csv", "requests.csv", "fetches.csv"] {
        let x = std::fs::read(ra.out_dir.join(f)).unwrap();
        let y = std::fs::read(rb.out_dir.join(f)).unwrap();
        assert!(!x.is_empty());
        assert_eq!(x, y, "{f} differs between identical runs");
    }
    // Analysis consumes the raw file from disk and agrees with the in-memory result.
    let disk = read_cells(&ra.out_dir).unwrap();
    assert_eq!(disk.iter().map(|c| c.csv_row()).collect::<Vec<_>>(), rows(&ra));
}

#[test]
fn sealed_seed_set_and_family_are_refused_before_any_output() {
    for set in ["S", "s", "sealed", "SEALED", " S "] {
        let mut s = spec("lib_sealed_refused", &["O-DEG"], &["W-DI"], &["BL-1"], 1);
        s.seed_set = set.to_string();
        s.out_dir = repo_root().join("lab-results").join("smoke_lib_sealed_must_not_exist").to_string_lossy().to_string();
        assert_eq!(execute(&s, &mut no_progress()).unwrap_err(), LabError::SealedSeedSetRefused, "set {set:?}");
        assert!(!std::path::Path::new(&s.out_dir).exists(), "refused run must not create output");
    }
    let mut s = spec("lib_sealed_refused", &["O-DEG"], &["W-SF"], &["BL-1"], 1);
    assert_eq!(validate(&s).unwrap_err(), LabError::SealedFamilyRefused);
    s.families = vec!["W-Z(0.8)".into()];
    assert!(validate(&s).is_ok());
    s.seed_set = "X".into();
    assert!(matches!(validate(&s), Err(LabError::UnknownSeedSet(_))));
    // Every seed the library can resolve is outside the sealed set.
    for set in ["T", "D"] {
        let mut s = spec("lib_sealed_refused", &["O-DEG"], &["W-DI"], &["BL-1"], 30);
        s.seed_set = set.into();
        let seeds = validate(&s).unwrap();
        assert!(phantom_cache_demo::lab::rng::SeedSets::verify_no_sealed_leak(&seeds).is_ok());
    }
}

#[test]
fn invalid_selections_are_rejected() {
    let ok = spec("lib_invalid", &["O-DEG"], &["W-DI"], &["BL-1"], 1);
    let mut s = ok.clone();
    s.scenarios = vec!["O-NOPE".into()];
    assert!(matches!(validate(&s), Err(LabError::UnknownScenario(_))));
    let mut s = ok.clone();
    s.arms = vec!["BL-999".into()];
    assert!(matches!(validate(&s), Err(LabError::UnknownArm(_))));
    let mut s = ok.clone();
    s.families = vec!["W-ZZ".into()];
    assert!(matches!(validate(&s), Err(LabError::UnknownFamily(_))));
    let mut s = ok.clone();
    s.n_seeds = 0;
    assert!(matches!(validate(&s), Err(LabError::InvalidSeedCount { .. })));
    let mut s = ok.clone();
    s.n_seeds = 31;
    assert!(matches!(validate(&s), Err(LabError::InvalidSeedCount { .. })));
    let mut s = ok.clone();
    s.arms.clear();
    assert!(matches!(validate(&s), Err(LabError::EmptySelection(_))));
    assert!(list_scenarios().contains(&"O-DEG".to_string()));
    assert!(list_arms().contains(&"PIE-off".to_string()));
    assert!(list_families().iter().all(|f| f != "W-SF"));
}

#[test]
fn disabled_mechanism_still_yields_zero_effect_via_library_api() {
    // PIE-off (executor installed, Mode::Off) vs CTRL-nonadaptive: per-seed identical, so effect == 0.
    let s = spec("lib_sens_a", &["O-DEG", "O-BRN", "O-SLW"], &["W-DI"], &["PIE-off", "CTRL-nonadaptive"], 6);
    let r = execute(&s, &mut no_progress()).unwrap();
    let rows = paired_summary(&r, "PIE-off", "CTRL-nonadaptive").unwrap();
    assert_eq!(rows.len(), 6);
    for e in &rows {
        assert_eq!(e.mean_rel_reduction, 0.0, "{} {}: disabled mechanism shows an effect", e.scenario, e.endpoint);
        assert!(!e.supported);
        assert!(e.ci_lo <= 0.0 && e.ci_hi >= 0.0);
    }
    // Positive control: the enabled mechanism must differ from the control, else the check is vacuous.
    let s2 = spec("lib_sens_a_pos", &["O-DEG"], &["W-DI"], &["PIE-Retained", "CTRL-nonadaptive"], 3);
    let r2 = execute(&s2, &mut no_progress()).unwrap();
    let e = paired_summary(&r2, "PIE-Retained", "CTRL-nonadaptive").unwrap();
    assert!(e.iter().any(|x| x.mean_rel_reduction != 0.0), "enabled mechanism has no observable effect");
    assert!(paired_summary(&r2, "BL-1", "CTRL-nonadaptive").is_err());
}

#[test]
fn manifest_has_required_fields_and_labels() {
    let s = spec("lib_manifest", &["O-DEG"], &["W-DI"], &["BL-1", "BL-12b"], 2);
    let mut seen = Vec::new();
    let r = execute(&s, &mut |done, total| seen.push((done, total))).unwrap();
    assert_eq!(seen.len(), 4);
    assert_eq!(seen.first(), Some(&(1, 4)));
    assert_eq!(seen.last(), Some(&(4, 4)));

    let on_disk: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(r.out_dir.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(on_disk, r.manifest);
    let m = &r.manifest;
    assert_eq!(m["tier"], "SIMULATED");
    assert_eq!(m["provenance"]["tier"], "SIMULATED");
    assert_eq!(m["provenance"]["real_process_measurements"], "none");
    assert_eq!(m["seed_set"], "D");
    assert_eq!(m["seed_values"].as_array().unwrap().len(), 2);
    assert_eq!(m["cell_count"], 4);
    for k in ["git_head", "source_sha256", "build_profile", "host_os", "host_arch", "sim_params", "arm_params", "arm_configs", "output_files_sha256", "run_spec", "profile"] {
        assert!(!m[k].is_null(), "manifest field {k} missing");
    }
    assert!(m["source_sha256"].as_object().unwrap().keys().any(|k| k.starts_with("src/lab/")), "src/lab not hashed");
    let started = m["started_utc"].as_str().unwrap();
    let ended = m["ended_utc"].as_str().unwrap();
    assert!(started.ends_with('Z') && ended.ends_with('Z') && started.len() == 20 && started <= ended);
    assert!(!started.starts_with("1970"), "timestamp must be a real wall-clock time");
    assert!(m["output_files_sha256"]["cells.csv"].as_str().unwrap().len() == 64);

    // RunSpec is serde round-trippable (API-ready).
    let j = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<RunSpec>(&j).unwrap(), s);
    let minimal: RunSpec = serde_json::from_str(
        r#"{"name":"x","scenarios":["O-DEG"],"families":["W-DI"],"arms":["BL-1"],"n_seeds":1,"out_dir":"o"}"#,
    )
    .unwrap();
    assert_eq!(minimal.seed_set, "D");
    assert_eq!(minimal.profile, "smoke");
    assert!(!minimal.overwrite);
}

#[test]
fn existing_output_is_not_overwritten_by_default() {
    let mut s = spec("lib_overwrite", &["O-DEG"], &["W-DI"], &["BL-1"], 1);
    execute(&s, &mut no_progress()).unwrap();
    s.overwrite = false;
    assert!(matches!(execute(&s, &mut no_progress()), Err(LabError::OutputExists(_))));
}

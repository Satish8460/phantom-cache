//! In-module test suite for PIE v2 (no Cargo.toml change needed). Every test protects behaviour of this lane's code.
//! Workloads come from the frozen generators `crate::lab::workloads` (read-only); seeds are arbitrary development
//! seeds (NOT the registered tuning/dev/sealed sets), so nothing here is an evaluation result.

use super::api::*;
use super::arbiter::{standard_constraints, Conflict, DecisionRecord, DecisionRing, Verdict};
use super::engine::{Pie, PieConfig};
use super::policies::{hysteresis, Incumbent};
use super::registry::{registry, validate, Lifecycle, Params};
use super::signals::{bucket_of, bucket_upper_us, Event, OriginResult, Outcome};
use super::*;
use crate::lab::rng::MasterRng;
use crate::lab::workloads::*;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

// ------------------------------------------------------------------ helpers

fn cfg(mode: Mode) -> PieConfig {
    PieConfig {
        mode,
        queue_capacity: 65_536,
        decision_every_windows: 1,
        cache_capacity_entries: 512,
        incumbent: Incumbent::Lru,
        ..PieConfig::default()
    }
}

fn ev(key: u64, outcome: Outcome) -> Event {
    Event {
        key_hash: key,
        latency_us: 100,
        origin_latency_us: 0,
        outcome,
        origin: OriginResult::None,
        size_bytes: 0,
    }
}

fn miss_with_origin(key: u64, origin_us: u32, ok: bool) -> Event {
    Event {
        key_hash: key,
        latency_us: origin_us,
        origin_latency_us: origin_us,
        outcome: Outcome::Miss,
        origin: if ok {
            OriginResult::Ok
        } else {
            OriginResult::Err
        },
        size_bytes: 0,
    }
}

/// Turns a frozen workload generator into per-second windows of events (infinite-cache hit/miss labels).
fn from_workload(w: &mut dyn WorkloadGenerator, seed: u64, windows: u64) -> Vec<Vec<Event>> {
    let mut rng = MasterRng::new(seed).stream("pie_v2_test");
    let mut now_us = 0u64;
    let mut seen = HashSet::new();
    let mut out: Vec<Vec<Event>> = vec![vec![]; windows as usize];
    loop {
        let r = w.next_request(now_us, &mut rng);
        now_us += r.arrival_delta_us;
        let win = (now_us / 1_000_000) as usize;
        if win >= windows as usize {
            break;
        }
        let k = hash_key(r.key.as_bytes());
        let e = if seen.insert(k) {
            miss_with_origin(k, 20_000, true)
        } else {
            ev(k, Outcome::Hit)
        };
        out[win].push(e);
    }
    out
}

fn replay(pie: &Pie, windows: &[Vec<Event>]) {
    pie.tick(0);
    replay_from(pie, windows, 0);
}

/// Replays windows starting at window index `from` (the engine was already ticked at t = 0).
fn replay_from(pie: &Pie, windows: &[Vec<Event>], from: u64) {
    let sink = pie.sink();
    for (i, w) in windows.iter().enumerate() {
        for e in w {
            sink.try_record(*e);
        }
        pie.tick((from + i as u64) * 1000 + 1000);
    }
}

fn records(pie: &Pie) -> Vec<Value> {
    decisions_json(pie, 200, 0, None)["records"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn recs_of(pie: &Pie, pid: &str) -> Vec<Value> {
    records(pie)
        .into_iter()
        .filter(|r| r["policy_id"] == pid)
        .collect()
}

fn verdicts(pie: &Pie, pid: &str) -> Vec<String> {
    recs_of(pie, pid)
        .iter()
        .map(|r| r["verdict"].as_str().unwrap().to_string())
        .collect()
}

fn count_recommend(pie: &Pie) -> usize {
    records(pie)
        .iter()
        .filter(|r| r["verdict"] == "Recommend")
        .count()
}

/// 30% reused hot keys (300), 70% never-repeated keys: scan-resistant policies beat LRU on this stream.
fn scan_mix(seed: u64, windows: u64, per_window: u64) -> Vec<Vec<Event>> {
    let mut rng = MasterRng::new(seed).stream("scan_mix");
    let mut next_unique = 1u64 << 40;
    let mut seen = HashSet::new();
    (0..windows)
        .map(|_| {
            (0..per_window)
                .map(|_| {
                    let k = if rng.next_f64() < 0.3 {
                        hash_key(format!("/hot_{}", rng.next_range_u64(0, 299)).as_bytes())
                    } else {
                        next_unique += 1;
                        hash_key(&next_unique.to_le_bytes())
                    };
                    if seen.insert(k) {
                        miss_with_origin(k, 20_000, true)
                    } else {
                        ev(k, Outcome::Hit)
                    }
                })
                .collect()
        })
        .collect()
}

fn shuffled(windows: &[Vec<Event>], seed: u64) -> Vec<Vec<Event>> {
    let mut all: Vec<Event> = windows.iter().flatten().copied().collect();
    let mut rng = MasterRng::new(seed).stream("shuffle");
    for i in (1..all.len()).rev() {
        let j = rng.next_range_u64(0, i as u64) as usize;
        all.swap(i, j);
    }
    let n = windows[0].len().max(1);
    all.chunks(n).map(|c| c.to_vec()).collect()
}

fn fake_record(id: u64) -> DecisionRecord {
    DecisionRecord {
        decision_id: id,
        timestamp_ms: 0,
        registry_version: REGISTRY_VERSION,
        policy_id: "POL-CLASS-1",
        policy_version: "1.0.0",
        mode: "advisory",
        verdict: Verdict::NoAction,
        reason_code: "x".repeat(200),
        inputs: vec![],
        candidates: vec![],
        constraints: standard_constraints(),
        recommendation: None,
        guardrail_events: vec![],
        conflict: Conflict {
            lever: "none",
            role: "none",
            with: vec![],
        },
        observed_effect: super::arbiter::observed_effect_unavailable(),
        simulated: false,
        explanation: String::new(),
    }
}

// ------------------------------------------------------------------ modes and registry

#[test]
fn canary_and_active_cannot_be_parsed_or_represented() {
    assert_eq!(parse_mode("off"), Ok(Mode::Off));
    assert_eq!(parse_mode(" Observe "), Ok(Mode::Observe));
    assert_eq!(parse_mode("ADVISORY"), Ok(Mode::Advisory));
    assert!(matches!(
        parse_mode("canary"),
        Err(ModeError::ForbiddenMode(_))
    ));
    assert!(matches!(
        parse_mode("Active"),
        Err(ModeError::ForbiddenMode(_))
    ));
    assert!(matches!(parse_mode("turbo"), Err(ModeError::Unknown(_))));
    assert_eq!(Mode::from_u8(200), Mode::Off);
    assert!(Mode::Off < Mode::Observe && Mode::Observe < Mode::Advisory);
}

#[test]
fn default_registry_is_valid_and_no_policy_exceeds_advisory() {
    let specs = registry();
    validate(&specs).expect("default registry must validate");
    assert_eq!(specs.iter().filter(|s| s.implemented).count(), 10);
    for s in &specs {
        let m = parse_mode(s.max_runtime_state).unwrap();
        assert!(m <= Mode::Advisory, "{} exceeds advisory", s.id);
        if !s.implemented {
            assert_eq!(m, Mode::Off, "{} is not implemented and must not run", s.id);
            assert_eq!(s.lifecycle, Lifecycle::Proposed);
        }
    }
}

#[test]
fn registry_validation_rejects_bad_entries() {
    let base = registry();
    let mut s = base.clone();
    s[0].max_runtime_state = "active";
    assert!(validate(&s).is_err(), "active must be rejected");
    let mut s = base.clone();
    s[0].max_runtime_state = "canary";
    assert!(validate(&s).is_err(), "canary must be rejected");
    let mut s = base.clone();
    s[0].lifecycle = Lifecycle::Implemented; // advisory before UnitTested
    assert!(validate(&s).is_err());
    let mut s = base.clone();
    s[0].lifecycle = Lifecycle::Rejected;
    assert!(validate(&s).is_err(), "a rejected policy cannot run");
    let mut s = base.clone();
    s[1].id = s[0].id; // duplicate id
    assert!(validate(&s).is_err());
    let mut s = base.clone();
    s[3].max_runtime_state = "advisory"; // POL-TTL-1 is not implemented
    s[3].lifecycle = Lifecycle::UnitTested;
    assert!(validate(&s).is_err());
    let mut s = base;
    s[0].safety_constraints = &[];
    assert!(validate(&s).is_err());
}

#[test]
fn parameters_and_config_are_bounds_checked() {
    let mut o = BTreeMap::new();
    o.insert("p2.delta_min".to_string(), 0.9);
    assert!(matches!(
        Params::from_overrides(&o),
        Err(ConfigError::OutOfBounds { .. })
    ));
    o.insert("p2.delta_min".to_string(), f64::NAN);
    assert!(matches!(
        Params::from_overrides(&o),
        Err(ConfigError::NotFinite(_))
    ));
    let mut o = BTreeMap::new();
    o.insert("p9.nope".to_string(), 1.0);
    assert!(matches!(
        Params::from_overrides(&o),
        Err(ConfigError::UnknownParameter(_))
    ));
    let mut o = BTreeMap::new();
    o.insert("p2.delta_min".to_string(), 0.05);
    assert_eq!(
        Params::from_overrides(&o).unwrap().get("p2.delta_min"),
        0.05
    );

    let bad_mem = PieConfig {
        memory_budget_bytes: 1000,
        ..cfg(Mode::Advisory)
    };
    assert!(matches!(
        Pie::new(bad_mem),
        Err(ConfigError::MemoryBudget { .. })
    ));
    let bad_q = PieConfig {
        queue_capacity: 1,
        ..cfg(Mode::Advisory)
    };
    assert!(Pie::new(bad_q).is_err());
    assert!(Pie::new(cfg(Mode::Advisory)).is_ok());
}

#[test]
fn runtime_mode_can_only_be_downgraded() {
    let p = Pie::new(cfg(Mode::Observe)).unwrap();
    assert!(!p.downgrade(Mode::Advisory), "upgrade must be refused");
    assert_eq!(p.mode(), Mode::Observe);
    assert!(p.downgrade(Mode::Off));
    assert!(!p.downgrade(Mode::Observe));
    assert_eq!(p.mode(), Mode::Off);
}

#[test]
fn off_enqueues_nothing_and_observe_makes_no_decisions() {
    let off = Pie::new(cfg(Mode::Off)).unwrap();
    let s = off.sink();
    assert!(!s.try_record(ev(1, Outcome::Hit)));
    assert_eq!(status_json(&off)["queue"]["enqueued_total"]["value"], 0);

    let obs = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&obs, &scan_mix(3, 5, 500));
    assert_eq!(
        records(&obs).len(),
        0,
        "observe must not write decision records"
    );
    assert!(
        signals_json(&obs)["window"]["events"].as_u64().unwrap() > 0,
        "observe still windows features"
    );
    assert_eq!(
        signals_json(&obs)["classification"]["reason"],
        "advisory_mode_required"
    );
}

// ------------------------------------------------------------------ queue, windows, bounds

#[test]
fn queue_is_bounded_drops_are_counted_and_window_is_degraded_not_extrapolated() {
    let c = PieConfig {
        queue_capacity: 64,
        ..cfg(Mode::Advisory)
    };
    let p = Pie::new(c).unwrap();
    let s = p.sink();
    p.tick(0);
    let mut accepted = 0;
    for i in 0..1000u64 {
        if s.try_record(ev(i, Outcome::Hit)) {
            accepted += 1;
        }
    }
    assert_eq!(
        accepted, 64,
        "try_record must never block and must accept at most the capacity"
    );
    let st = status_json(&p);
    assert_eq!(st["queue"]["dropped_total"]["value"], 1000 - 64);
    assert_eq!(st["queue"]["enqueued_total"]["value"], 64);
    p.tick(1000);
    let sig = signals_json(&p);
    assert_eq!(sig["window"]["degraded"], true);
    assert_eq!(sig["window"]["degraded_reason"], "events_dropped");
    assert_eq!(sig["window"]["dropped_events"], 1000 - 64);
    for (id, f) in sig["features"].as_object().unwrap() {
        assert_eq!(
            f["src"], "unavailable",
            "{id} must not be extrapolated in a degraded window"
        );
        assert!(f["value"].is_null());
    }
    assert_eq!(sig["features"]["F1"]["reason"], "window_degraded");
}

#[test]
fn drain_per_tick_is_bounded_and_marks_window_truncated() {
    let c = PieConfig {
        queue_capacity: 4096,
        max_drain_per_tick: 64,
        ..cfg(Mode::Observe)
    };
    let p = Pie::new(c).unwrap();
    let s = p.sink();
    p.tick(0);
    for i in 0..1000u64 {
        s.try_record(ev(i, Outcome::Hit));
    }
    p.tick(1000); // drains only 64 events and must say the window is not trustworthy
    assert_eq!(
        signals_json(&p)["window"]["degraded_reason"],
        "drain_truncated"
    );
}

#[test]
fn features_below_min_samples_are_unavailable_and_enough_samples_are_derived() {
    let p = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&p, &[(0..50).map(|i| ev(i, Outcome::Hit)).collect()]);
    let f = &signals_json(&p)["features"];
    assert_eq!(f["F2"]["src"], "unavailable");
    assert_eq!(f["F2"]["reason"], "insufficient_samples");
    assert_eq!(f["F15"]["reason"], "needs_multi_node_view");
    assert_eq!(f["F8"]["reason"], "not_implemented");

    let q = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(
        &q,
        &[(0..400)
            .map(|i| {
                ev(
                    i % 40,
                    if i % 4 == 0 {
                        Outcome::Miss
                    } else {
                        Outcome::Hit
                    },
                )
            })
            .collect()],
    );
    let f = &signals_json(&q)["features"];
    assert_eq!(f["F2"]["src"], "derived");
    assert!((f["F2"]["value"].as_f64().unwrap() - 0.75).abs() < 1e-9);
    assert_eq!(f["F1"]["src"], "derived");
}

#[test]
fn memory_is_bounded_in_bytes_and_reported() {
    let p = Pie::new(cfg(Mode::Advisory)).unwrap();
    let st = status_json(&p);
    let bytes = st["memory"]["bytes"]["value"].as_u64().unwrap();
    assert!(bytes <= st["memory"]["budget_bytes"].as_u64().unwrap());
    assert_eq!(st["memory"]["bytes"]["src"], "derived");

    // the decision ring is bounded by count and by serialized bytes
    let one = serde_json::to_vec(&fake_record(1)).unwrap().len();
    let mut ring = DecisionRing::new(8, 6000);
    for i in 0..100 {
        ring.append(fake_record(i));
    }
    assert!(ring.len() <= 8);
    assert!(
        ring.bytes() <= 6000 + one,
        "bytes {} over budget",
        ring.bytes()
    );
    assert_eq!(ring.total(), 100);
    assert!(ring.get(99).is_some() && ring.get(0).is_none());
    let mut small = DecisionRing::new(1000, 3 * one);
    for i in 0..50 {
        small.append(fake_record(i));
    }
    assert!(
        small.len() <= 3 && small.bytes() <= 3 * one,
        "byte budget must bind before the count cap"
    );
}

#[test]
fn bucket_math_is_an_upper_bound_within_25_percent() {
    for us in [0u32, 1, 3, 4, 5, 7, 8, 100, 999, 20_000, 123_456, 5_000_000] {
        let b = bucket_of(us);
        let up = bucket_upper_us(b);
        assert!(up > us as u64, "{us} -> bucket {b} upper {up}");
        if us >= 8 {
            assert!((up as f64) <= us as f64 * 1.26, "{us} -> {up}");
        }
    }
    assert!(bucket_of(u32::MAX) < super::signals::HIST_BUCKETS);
}

// ------------------------------------------------------------------ provenance gate, P5, P6, cooldown

fn brownout_windows(origin_us: u32, n: usize) -> Vec<Vec<Event>> {
    (0..n)
        .map(|_| {
            (0..400u64)
                .map(|i| miss_with_origin(i, origin_us, true))
                .collect()
        })
        .collect()
}

#[test]
fn p6_warns_on_measured_origin_degradation_and_is_quiet_on_healthy_origin() {
    let bad = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&bad, &brownout_windows(800_000, 3));
    let r = recs_of(&bad, "POL-ORIGIN-1");
    assert!(r.iter().any(|x| x["verdict"] == "Recommend"), "{r:?}");
    let rec = r.iter().find(|x| x["verdict"] == "Recommend").unwrap();
    assert_eq!(rec["recommendation"]["applied"], false);
    assert_eq!(rec["recommendation"]["label"], "advisory, not applied");
    assert!(rec["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|i| i["src"] == "derived"));

    let ok = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&ok, &brownout_windows(20_000, 3));
    assert!(!verdicts(&ok, "POL-ORIGIN-1").contains(&"Recommend".to_string()));
}

#[test]
fn provenance_gate_forces_no_action_for_non_measured_inputs() {
    let win = brownout_windows(20_000, 2); // healthy events: only the injected health input differs
                                           // positive control: measured high latency reported by the gateway -> Recommend
    let a = Pie::new(cfg(Mode::Advisory)).unwrap();
    a.tick(0);
    a.submit_origin_health(500, 900.0, 0.0, 500, Some(Src::Measured));
    replay_from(&a, &win, 0);
    assert!(
        verdicts(&a, "POL-ORIGIN-1").contains(&"Recommend".to_string()),
        "{:?}",
        recs_of(&a, "POL-ORIGIN-1")
    );
    // same numbers but simulated -> NoAction with provenance_refused
    let b = Pie::new(cfg(Mode::Advisory)).unwrap();
    b.tick(0);
    b.submit_origin_health(500, 900.0, 0.0, 500, Some(Src::Simulated));
    replay_from(&b, &win, 0);
    let r = recs_of(&b, "POL-ORIGIN-1");
    assert!(!r.is_empty());
    assert!(
        r.iter()
            .all(|x| x["verdict"] == "NoAction" && x["reason_code"] == "provenance_refused"),
        "{r:?}"
    );
    assert!(r[0]["guardrail_events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g.as_str().unwrap().starts_with("provenance_refused")));
    // unavailable provenance is also refused
    let c = Pie::new(cfg(Mode::Advisory)).unwrap();
    c.tick(0);
    c.submit_origin_health(500, 900.0, 0.0, 500, Some(Src::Unavailable));
    replay_from(&c, &win, 0);
    assert!(recs_of(&c, "POL-ORIGIN-1")
        .iter()
        .all(|x| x["verdict"] == "NoAction"));
    // lab mode allows simulated inputs but labels the record simulated
    let d = Pie::new(PieConfig {
        lab_mode: true,
        ..cfg(Mode::Advisory)
    })
    .unwrap();
    d.tick(0);
    d.submit_origin_health(500, 900.0, 0.0, 500, Some(Src::Simulated));
    replay_from(&d, &win, 0);
    let r = recs_of(&d, "POL-ORIGIN-1");
    assert!(
        r.iter()
            .any(|x| x["verdict"] == "Recommend" && x["simulated"] == true),
        "{r:?}"
    );
}

#[test]
fn degraded_input_window_forces_no_action_for_every_policy() {
    let c = PieConfig {
        queue_capacity: 64,
        ..cfg(Mode::Advisory)
    };
    let p = Pie::new(c).unwrap();
    p.tick(0);
    let s = p.sink();
    for e in scan_mix(5, 1, 2000).remove(0) {
        s.try_record(e);
    }
    p.tick(1000);
    assert_eq!(count_recommend(&p), 0);
    for pid in [
        "POL-EVICT-1",
        "POL-ADMIT-1",
        "POL-STAMPEDE-1",
        "POL-ORIGIN-1",
        "POL-CLASS-1",
    ] {
        let r = recs_of(&p, pid);
        assert_eq!(r.len(), 1, "{pid}");
        assert_eq!(r[0]["verdict"], "NoAction", "{pid}");
        assert_eq!(r[0]["reason_code"], "window_degraded", "{pid}");
    }
}

#[test]
fn p5_warns_on_high_waiters_per_fetch_only() {
    let mk = |waiters: usize| -> Vec<Vec<Event>> {
        (0..3)
            .map(|_| {
                let mut v: Vec<Event> = (0..100u64)
                    .map(|i| miss_with_origin(i, 10_000, true))
                    .collect();
                v.extend((0..waiters).map(|i| ev(i as u64 % 5, Outcome::Coalesced)));
                v
            })
            .collect()
    };
    let hot = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&hot, &mk(1500)); // 15 waiters per fetch
    assert!(verdicts(&hot, "POL-STAMPEDE-1").contains(&"Recommend".to_string()));
    let calm = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&calm, &mk(100)); // 1 waiter per fetch
    assert!(!verdicts(&calm, "POL-STAMPEDE-1").contains(&"Recommend".to_string()));
}

#[test]
fn hysteresis_clears_only_below_80_percent_of_threshold() {
    assert!(hysteresis(8.0, 8.0, false));
    assert!(!hysteresis(7.9, 8.0, false));
    assert!(hysteresis(7.0, 8.0, true), "stays active above 80%");
    assert!(!hysteresis(6.3, 8.0, true), "clears below 80%");
}

#[test]
fn cooldown_suppresses_repeated_identical_recommendations() {
    let p = Pie::new(PieConfig {
        cooldown_ms: 60_000,
        ..cfg(Mode::Advisory)
    })
    .unwrap();
    replay(&p, &brownout_windows(800_000, 6));
    let rec: Vec<_> = verdicts(&p, "POL-ORIGIN-1")
        .into_iter()
        .filter(|v| v == "Recommend")
        .collect();
    assert_eq!(
        rec.len(),
        1,
        "one recommendation inside the cooldown, not six"
    );
    let st = status_json(&p);
    assert!(
        st["decisions"]["suppressed_by_cooldown"]["value"]
            .as_u64()
            .unwrap()
            >= 4
    );
    // with no cooldown every window may emit
    let q = Pie::new(PieConfig {
        cooldown_ms: 0,
        ..cfg(Mode::Advisory)
    })
    .unwrap();
    replay(&q, &brownout_windows(800_000, 6));
    assert!(
        verdicts(&q, "POL-ORIGIN-1")
            .iter()
            .filter(|v| *v == "Recommend")
            .count()
            >= 5
    );
}

// ------------------------------------------------------------------ P1 against seeded frozen workloads

fn p1_labels(windows: &[Vec<Event>]) -> Vec<String> {
    let p = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&p, windows);
    recs_of(&p, "POL-CLASS-1")
        .iter()
        .filter_map(|r| {
            r["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["status"] == "selected")
                .map(|c| c["candidate_id"].as_str().unwrap().to_string())
        })
        .collect()
}

#[test]
fn p1_classifies_seeded_workloads() {
    for seed in [101u64, 102, 103] {
        let z = p1_labels(&from_workload(
            &mut ZipfWorkload::new(10_000, 1.0, 2000.0),
            seed,
            8,
        ));
        assert_eq!(
            z.last().map(String::as_str),
            Some("zipf_like"),
            "zipf seed {seed}: {z:?}"
        );
        let h = p1_labels(&from_workload(
            &mut ZipfWorkload::new(10_000, 1.6, 2000.0),
            seed,
            8,
        ));
        assert_eq!(
            h.last().map(String::as_str),
            Some("hot_key_dominated"),
            "hot seed {seed}: {h:?}"
        );
        let u = p1_labels(&from_workload(
            &mut UniformWorkload::new(500, 2000.0),
            seed,
            8,
        ));
        assert_eq!(
            u.last().map(String::as_str),
            Some("uniform_like"),
            "uniform seed {seed}: {u:?}"
        );
        let s = p1_labels(&from_workload(
            &mut ScanWorkload::new(1000, 1.0, 0.7, 2000.0),
            seed,
            8,
        ));
        assert_eq!(
            s.last().map(String::as_str),
            Some("scan_cold"),
            "scan seed {seed}: {s:?}"
        );
    }
}

#[test]
fn p1_detects_phase_change_and_flash_crowd_and_not_on_stationary_or_shuffled_traces() {
    for seed in [201u64, 202, 203] {
        let shift = from_workload(
            &mut HotSetShiftWorkload::new(10_000, 1.0, 10_000_000, 5000, 2000.0),
            seed,
            16,
        );
        let l = p1_labels(&shift);
        assert!(
            l.contains(&"phase_change".to_string()),
            "shift seed {seed}: {l:?}"
        );
        // negative control: the same events in random order have no phase structure
        let l = p1_labels(&shuffled(&shift, seed));
        assert!(
            !l.contains(&"phase_change".to_string()),
            "shuffled seed {seed}: {l:?}"
        );
        // stationary control
        let l = p1_labels(&from_workload(
            &mut ZipfWorkload::new(10_000, 1.0, 2000.0),
            seed,
            16,
        ));
        assert!(
            !l.contains(&"phase_change".to_string())
                && !l.contains(&"bursty_flash_crowd".to_string()),
            "{l:?}"
        );
        // flash crowd: 6x rate on 20 keys from t = 10 s to 13 s
        let fc = from_workload(
            &mut FlashCrowdWorkload::new(10_000, 0.8, 2000.0, 10_000_000, 13_000_000, 6.0, 20),
            seed,
            16,
        );
        let l = p1_labels(&fc);
        assert!(
            l.contains(&"bursty_flash_crowd".to_string()),
            "flash seed {seed}: {l:?}"
        );
    }
}

// ------------------------------------------------------------------ P2/P3 and the arbiter

#[test]
fn p2_and_p3_recommend_on_scan_heavy_stream_and_the_arbiter_marks_the_conflict() {
    let p = Pie::new(PieConfig {
        cooldown_ms: 0,
        ..cfg(Mode::Advisory)
    })
    .unwrap();
    replay(&p, &scan_mix(7, 12, 3000));
    let p2: Vec<_> = recs_of(&p, "POL-EVICT-1")
        .into_iter()
        .filter(|r| r["verdict"] == "Recommend")
        .collect();
    assert!(!p2.is_empty(), "{:?}", recs_of(&p, "POL-EVICT-1"));
    let r = &p2[0];
    assert_eq!(r["recommendation"]["applied"], false);
    let sel = r["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["status"] == "selected")
        .unwrap();
    let eff = &sel["estimated_effect"];
    assert!(
        eff["ci95"][0].as_f64().unwrap() > 0.02,
        "interval lower bound must exceed delta_min: {eff}"
    );
    assert_eq!(eff["src"], "derived");
    assert!(
        r["candidates"].as_array().unwrap().len() >= 2,
        "rejected candidates are listed too"
    );
    assert!(r["explanation"]
        .as_str()
        .unwrap()
        .contains("Nothing was applied"));
    // P3 also fires (TinyLFU admission) and shares the lever with P2 -> exactly one primary
    let p3: Vec<_> = recs_of(&p, "POL-ADMIT-1")
        .into_iter()
        .filter(|r| r["verdict"] == "Recommend")
        .collect();
    assert!(!p3.is_empty(), "{:?}", recs_of(&p, "POL-ADMIT-1"));
    // at a tick where both fire, exactly one is primary and the other an alternative that points at it
    let mut paired = false;
    for a in &p2 {
        for b in p3.iter().filter(|b| b["timestamp_ms"] == a["timestamp_ms"]) {
            let ra = a["conflict"]["role"].as_str().unwrap();
            let rb = b["conflict"]["role"].as_str().unwrap();
            assert!(
                (ra == "primary" && rb == "alternative")
                    || (ra == "alternative" && rb == "primary"),
                "{ra} {rb}"
            );
            assert_eq!(a["conflict"]["with"][0], "POL-ADMIT-1");
            assert_eq!(b["conflict"]["with"][0], "POL-EVICT-1");
            paired = true;
        }
    }
    assert!(paired, "no tick had both P2 and P3 recommending");
    assert_eq!(p2[0]["conflict"]["lever"], "cache_replacement");
}

#[test]
fn negative_controls_constant_uniform_and_shuffled_loop_produce_no_recommendation() {
    // constant workload: one key forever
    let c = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(
        &c,
        &(0..10)
            .map(|_| (0..2000).map(|_| ev(42, Outcome::Hit)).collect())
            .collect::<Vec<_>>(),
    );
    assert_eq!(count_recommend(&c), 0, "{:?}", records(&c));
    assert!(verdicts(&c, "POL-EVICT-1").iter().all(|v| v == "NoAction"));
    // i.i.d. uniform over a catalog much larger than the cache: no policy can win
    for seed in [301u64, 302, 303] {
        let u = Pie::new(cfg(Mode::Advisory)).unwrap();
        replay(
            &u,
            &from_workload(&mut UniformWorkload::new(200_000, 3000.0), seed, 12),
        );
        assert_eq!(
            count_recommend(&u),
            0,
            "uniform seed {seed}: {:?}",
            records(&u)
        );
    }
    // ordering-dependent trace: alternating hot and scan phases. Positive control: the ordered trace makes a
    // scan-resistant policy beat the incumbent LRU with an interval above the margin. Shuffling destroys the
    // phases (same keys, same frequencies) and must then give NoAction.
    for seed in [1u64, 2, 3] {
        let t = burst_scan(seed, 12, 3000);
        let o = Pie::new(cfg(Mode::Advisory)).unwrap();
        replay(&o, &t);
        let n_o = recs_of(&o, "POL-EVICT-1")
            .iter()
            .filter(|r| r["verdict"] == "Recommend")
            .count();
        assert!(
            n_o >= 1,
            "positive control failed, seed {seed}: {:?}",
            recs_of(&o, "POL-EVICT-1")
        );
        let s = Pie::new(cfg(Mode::Advisory)).unwrap();
        replay(&s, &shuffled(&t, seed + 50));
        let n = recs_of(&s, "POL-EVICT-1")
            .iter()
            .filter(|r| r["verdict"] == "Recommend")
            .count();
        assert_eq!(
            n,
            0,
            "shuffled seed {seed}: {:?}",
            recs_of(&s, "POL-EVICT-1")
        );
    }
}

// ------------------------------------------------------------------ fault containment

fn assert_fault(p: &Pie, code: &str) {
    assert_eq!(p.mode(), Mode::Off, "a fault must drop the mode to off");
    let f = p.fault().expect("fault info");
    assert_eq!(f.reason_code, code);
    let faults: Vec<_> = recs_of(p, "PIE-RUNTIME");
    assert_eq!(faults.len(), 1, "exactly one Fault record");
    assert_eq!(faults[0]["verdict"], "Fault");
    assert_eq!(faults[0]["reason_code"], code);
    assert_eq!(status_json(p)["fault"]["reason_code"], code);
    // serving is unaffected: the hot-path handle still works, never blocks, never panics, and enqueues nothing
    assert!(!p.sink().try_record(ev(1, Outcome::Hit)));
    p.tick(10_000_000); // later ticks are harmless
    assert_eq!(recs_of(p, "PIE-RUNTIME").len(), 1);
    assert_eq!(p.mode(), Mode::Off);
}

#[test]
fn fault_panic_in_the_intelligence_task_drops_to_off_and_is_contained() {
    let p = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&p, &scan_mix(9, 3, 500));
    p.inject_panic_next_tick();
    p.tick(10_000); // must not propagate the panic
    assert_fault(&p, "panic");
    assert!(p.fault().unwrap().detail.contains("injected panic"));
}

#[test]
fn fault_stall_is_detected_by_the_watchdog() {
    let p = Pie::new(PieConfig {
        stall_ms: 5000,
        ..cfg(Mode::Advisory)
    })
    .unwrap();
    p.watchdog(0); // arms
    p.tick(1000);
    p.watchdog(3000);
    assert_eq!(p.mode(), Mode::Advisory, "not stalled yet");
    p.watchdog(6001); // last completed tick was at 1000: 5001 ms ago
    assert_fault(&p, "stall");
}

#[test]
fn fault_poisoned_origin_health_nan_huge_negative_and_missing_provenance() {
    let cases: Vec<(f64, f64, Option<Src>, &str)> = vec![
        (f64::NAN, 0.1, Some(Src::Measured), "poisoned_input"),
        (10.0, f64::INFINITY, Some(Src::Measured), "poisoned_input"),
        (1e18, 0.1, Some(Src::Measured), "poisoned_input"),
        (-5.0, 0.1, Some(Src::Measured), "poisoned_input"),
        (10.0, 7.0, Some(Src::Measured), "poisoned_input"),
        (10.0, 0.1, None, "missing_provenance"),
    ];
    for (p99, err, src, code) in cases {
        let p = Pie::new(cfg(Mode::Advisory)).unwrap();
        p.tick(0);
        p.submit_origin_health(5, p99, err, 100, src);
        assert_fault(&p, code);
    }
    // a valid submission is accepted and does not fault
    let p = Pie::new(cfg(Mode::Advisory)).unwrap();
    p.submit_origin_health(5, 12.0, 0.01, 100, Some(Src::Measured));
    assert_eq!(p.mode(), Mode::Advisory);
}

#[test]
fn fault_poisoned_event_and_clock_jumps() {
    let p = Pie::new(cfg(Mode::Advisory)).unwrap();
    p.tick(0);
    let mut bad = ev(1, Outcome::Hit);
    bad.latency_us = u32::MAX;
    p.sink().try_record(bad);
    p.tick(1000);
    assert_fault(&p, "poisoned_input");

    let back = Pie::new(cfg(Mode::Advisory)).unwrap();
    back.tick(5000);
    back.tick(4000);
    assert_fault(&back, "clock_jump");

    let fwd = Pie::new(PieConfig {
        max_clock_jump_ms: 60_000,
        ..cfg(Mode::Advisory)
    })
    .unwrap();
    fwd.tick(0);
    fwd.tick(1000);
    fwd.tick(1_000_000);
    assert_fault(&fwd, "clock_jump");
}

// ------------------------------------------------------------------ determinism and API shapes

#[test]
fn same_seed_and_trace_give_byte_identical_decision_records() {
    let run = |seed: u64| -> (String, String) {
        let p = Pie::new(cfg(Mode::Advisory)).unwrap();
        replay(&p, &scan_mix(seed, 12, 3000));
        let w = from_workload(&mut ScanWorkload::new(1000, 0.9, 0.5, 3000.0), seed, 12);
        let q = Pie::new(cfg(Mode::Advisory)).unwrap();
        replay(&q, &w);
        (
            format!(
                "{}{}",
                decisions_json(&p, 200, 0, None),
                decisions_json(&q, 200, 0, None)
            ),
            format!("{}{}", signals_json(&p), signals_json(&q)),
        )
    };
    let a = run(11);
    let b = run(11);
    assert_eq!(a.0, b.0, "decision records must be byte identical");
    assert_eq!(a.1, b.1, "signals must be byte identical");
    assert!(a.0.len() > 1000, "the trace must actually produce records");
    let c = run(12);
    assert_ne!(
        a.0, c.0,
        "a different seed should change the records (sanity)"
    );
}

#[test]
fn api_shapes_follow_the_contract() {
    let p = Pie::new(cfg(Mode::Advisory)).unwrap();
    // before any window
    let s = signals_json(&p);
    assert!(s["window"].is_null());
    assert_eq!(s["features"]["F2"]["reason"], "no_window_closed_yet");
    assert_eq!(s["classification"]["src"], "unavailable");
    replay(&p, &scan_mix(21, 12, 3000));

    let r = registry_json(&p);
    assert_eq!(r["schema"], "pie-api-v1");
    assert_eq!(r["max_mode"], "advisory");
    let pols = r["policies"].as_array().unwrap();
    assert_eq!(pols.len(), 12);
    let evict = pols
        .iter()
        .find(|x| x["policy_id"] == "POL-EVICT-1")
        .unwrap();
    assert_eq!(evict["effective_runtime_state"]["value"], "advisory");
    assert_eq!(evict["effective_runtime_state"]["src"], "measured");
    assert!(evict["parameters"][0]["current"].is_number());
    let ttl = pols.iter().find(|x| x["policy_id"] == "POL-TTL-1").unwrap();
    assert_eq!(ttl["effective_runtime_state"]["value"], "off");

    let st = status_json(&p);
    for k in [
        "mode",
        "queue",
        "window",
        "memory",
        "decisions",
        "config",
        "claims",
        "fault",
    ] {
        assert!(st.get(k).is_some(), "status.v2 lacks {k}");
    }
    assert_eq!(st["claims"]["benefit_claim"], "none");
    assert!(st["fault"].is_null());

    let sig = signals_json(&p);
    assert_eq!(sig["features"]["F2"]["src"], "derived");
    assert_eq!(sig["classification"]["src"], "derived");
    let tk = &sig["top_keys"]["items"][0]["key_hash"];
    assert_eq!(tk.as_str().unwrap().len(), 16);

    let d = decisions_json(&p, 3, 1, None);
    assert_eq!(d["ordering"], "oldest_first");
    assert!(d["returned"].as_u64().unwrap() <= 3);
    let first = &records(&p)[0];
    for k in [
        "decision_id",
        "timestamp_ms",
        "registry_version",
        "policy_id",
        "policy_version",
        "mode",
        "verdict",
        "reason_code",
        "inputs",
        "candidates",
        "constraints",
        "recommendation",
        "guardrail_events",
        "conflict",
        "observed_effect",
        "simulated",
        "explanation",
    ] {
        assert!(first.get(k).is_some(), "decision lacks {k}");
    }
    assert_eq!(first["observed_effect"]["src"], "unavailable");
    let id = first["decision_id"].as_u64().unwrap();
    assert!(decision_json(&p, id).is_some() && decision_json(&p, 9_999_999).is_none());

    let pj = policy_json(&p, "POL-EVICT-1").unwrap();
    assert!(pj["runtime"]["evaluations"]["value"].as_u64().unwrap() > 0);
    assert!(pj["recent_decisions"].as_array().unwrap().len() <= 20);
    assert!(policy_json(&p, "POL-NOPE").is_none());

    let only = decisions_json(&p, 50, 0, Some("POL-ADMIT-1"));
    assert!(only["records"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["policy_id"] == "POL-ADMIT-1"));

    assert_eq!(
        parse_decisions_query(Some("limit=5&offset=2&policy_id=POL-EVICT-1")),
        Ok((5, 2, Some("POL-EVICT-1".into())))
    );
    assert!(parse_decisions_query(Some("limit=0")).is_err());
    assert!(parse_decisions_query(Some("limit=201")).is_err());
    assert!(parse_decisions_query(Some("policy_id=a b")).is_err());
    assert!(parse_decisions_query(Some("x=1")).is_err());
    assert_eq!(not_installed_json()["installed"], false);
    // no key material or secrets: only hex hashes leave the library
    assert!(!sig.to_string().contains("/hot_"));
}

#[test]
fn hash_key_is_stable() {
    assert_eq!(hash_key(b"/a"), hash_key(b"/a"));
    assert_ne!(hash_key(b"/a"), hash_key(b"/b"));
    assert_eq!(hash_key(b""), mix64(0xcbf29ce484222325));
}

/// Alternating phases: 1000 reused hot keys (160 keys) then 1000 never-repeated keys. Ordering matters.
fn burst_scan(seed: u64, windows: u64, per_window: u64) -> Vec<Vec<Event>> {
    let mut rng = MasterRng::new(seed).stream("burst_scan");
    let mut next_unique = 1u64 << 41;
    let mut n = 0u64;
    (0..windows)
        .map(|_| {
            (0..per_window)
                .map(|_| {
                    let phase_hot = (n / 1000).is_multiple_of(2);
                    n += 1;
                    if phase_hot {
                        ev(
                            hash_key(format!("/h_{}", rng.next_range_u64(0, 159)).as_bytes()),
                            Outcome::Hit,
                        )
                    } else {
                        next_unique += 1;
                        ev(hash_key(&next_unique.to_le_bytes()), Outcome::Miss)
                    }
                })
                .collect()
        })
        .collect()
}

mod insight_tests;

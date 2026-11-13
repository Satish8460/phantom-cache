//! Tests for the insight layer: hot keys, frequency summary, HyperLogLog cardinality, statistical prediction with
//! scorecard, flash-crowd detection, and size / origin-cost weighting of the admission advice.

use super::*;
use crate::intelligence_v2::insight::{Hll, TRACKS_CAP};

fn uniq_events(n: usize, base: u64) -> Vec<Event> {
    (0..n as u64).map(|i| ev(hash_key(&(base + i).to_le_bytes()), Outcome::Miss)).collect()
}

/// `n` events: a fraction `share` goes to `hot` (cycled), the rest are never-repeated keys (from `base`).
fn mixed_window(n: usize, hot: &[u64], share: f64, base: u64) -> Vec<Event> {
    let hot_n = (n as f64 * share) as usize;
    let mut v: Vec<Event> = (0..hot_n).map(|i| ev(hot[i % hot.len()], Outcome::Hit)).collect();
    v.extend(uniq_events(n - hot_n, base));
    let mut out = Vec::with_capacity(n);
    let (mut a, mut b) = (0usize, hot_n);
    while out.len() < n {
        if a < hot_n {
            out.push(v[a]);
            a += 1;
        }
        if b < n {
            out.push(v[b]);
            b += 1;
        }
    }
    out
}

fn feat<'a>(sig: &'a Value, id: &str) -> &'a Value {
    &sig["features"][id]
}

#[test]
fn hyperloglog_estimates_distinct_within_declared_error_and_is_deterministic() {
    for n in [500usize, 5000, 50_000] {
        let mut h = Hll::new();
        for i in 0..n as u64 {
            h.insert(hash_key(&i.to_le_bytes()));
            h.insert(hash_key(&i.to_le_bytes())); // duplicates change nothing
        }
        let e = h.estimate();
        let rel = (e - n as f64).abs() / n as f64;
        assert!(rel < 0.13, "n={n} estimate={e} rel={rel}"); // 4 sigma of the declared 3.25 percent
        let mut h2 = Hll::new();
        for i in 0..n as u64 {
            h2.insert(hash_key(&i.to_le_bytes()));
        }
        assert_eq!(h.estimate(), h2.estimate(), "deterministic");
    }
    assert_eq!(std::mem::size_of::<Hll>(), 1024, "p=10 means 1 KiB of registers");
    assert_eq!(Hll::new().estimate(), 0.0);
}

#[test]
fn cardinality_features_flag_high_cardinality_and_growth() {
    let p = Pie::new(cfg(Mode::Observe)).unwrap();
    p.tick(0);
    let hot: Vec<u64> = (0..50u64).map(|i| hash_key(&(7_000_000 + i).to_le_bytes())).collect();
    replay_from(&p, &[(0..3000).map(|i| ev(hot[i % 50], Outcome::Hit)).collect()], 0);
    let s = signals_json(&p);
    assert_eq!(feat(&s, "F26")["value"], false);
    assert!(feat(&s, "F24")["value"].as_f64().unwrap() < 0.1);
    assert_eq!(feat(&s, "F25")["reason"], "no_previous_window");
    let est1 = feat(&s, "F23")["value"].as_f64().unwrap();
    assert!((est1 - 50.0).abs() < 8.0, "{est1}");
    replay_from(&p, &[uniq_events(3000, 1)], 1);
    let s = signals_json(&p);
    assert_eq!(feat(&s, "F26")["value"], true);
    let est2 = feat(&s, "F23")["value"].as_f64().unwrap();
    assert!((est2 - 3000.0).abs() / 3000.0 < 0.13, "{est2}");
    assert!(feat(&s, "F24")["value"].as_f64().unwrap() > 0.85);
    assert!(feat(&s, "F25")["value"].as_f64().unwrap() > 10.0, "growth is relative to window 1");
    let se = feat(&s, "F23b")["value"].as_f64().unwrap();
    assert!((se / est2 - 0.0325).abs() < 1e-6, "standard error is 3.25 percent of the estimate");
    assert_eq!(feat(&s, "F23")["src"], "derived");
    let q = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&q, &[uniq_events(40, 1)]);
    assert_eq!(feat(&signals_json(&q), "F23")["reason"], "insufficient_samples");
}

#[test]
fn hot_key_tracker_reports_bounded_hash_only_hot_keys() {
    let key = hash_key(b"/the-hot-key");
    let p = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&p, &[mixed_window(3000, &[key], 0.2, 1)]);
    let s = signals_json(&p);
    let items = s["hot_keys"]["items"].as_array().unwrap();
    assert_eq!(items[0]["key_hash"], format!("{key:016x}"));
    assert_eq!(items[0]["hot"], true);
    let share = items[0]["share"].as_f64().unwrap();
    assert!((share - 0.2).abs() < 0.03, "share {share}");
    let (lo, up) = (items[0]["count_lower"].as_u64().unwrap(), items[0]["count_upper"].as_u64().unwrap());
    assert!(lo <= 600 && 600 <= up, "Space-Saving bounds must contain the true count 600: {lo}..{up}");
    assert!(items[0]["confidence"].as_f64().unwrap() > 0.9);
    assert_eq!(items.iter().filter(|i| i["hot"] == true).count(), 1, "only the heavy key is hot");
    assert_eq!(feat(&s, "F30")["value"].as_f64().unwrap(), 1.0);
    assert!(!s.to_string().contains("the-hot-key"), "no key material leaves the library");
    assert!(items.len() <= 10);
    let q = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&q, &[uniq_events(3000, 1)]);
    assert_eq!(feat(&signals_json(&q), "F30")["value"].as_f64().unwrap(), 0.0);
    let r = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&r, &[uniq_events(30, 1)]);
    let s = signals_json(&r);
    assert_eq!(feat(&s, "F30")["src"], "unavailable");
    assert_eq!(s["hot_keys"]["items"].as_array().unwrap().len(), 0);
}

#[test]
fn frequency_summary_counts_once_twice_and_repeated_keys() {
    let mut w = uniq_events(2000, 1);
    for r in 0..20u64 {
        w.extend((0..100u64).map(|i| {
            ev(hash_key(&(9_000_000 + i).to_le_bytes()), if r == 0 { Outcome::Miss } else { Outcome::Hit })
        }));
    }
    let p = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&p, &[w]);
    let s = signals_json(&p);
    let once = feat(&s, "F18")["value"].as_f64().unwrap();
    let three = feat(&s, "F20")["value"].as_f64().unwrap();
    assert!(once > 0.85 && three < 0.1, "once={once} three_plus={three}");
    assert!((once + feat(&s, "F19")["value"].as_f64().unwrap() + three - 1.0).abs() < 1e-9);
    assert!(feat(&s, "F22")["value"].as_f64().unwrap() > 0.35, "hottest 10 percent of keys carry a large share");
    assert_eq!(feat(&s, "F18")["src"], "derived");
    assert!(feat(&s, "F18")["samples"].as_u64().unwrap() >= 30);
    let q = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&q, &[(0..400).map(|i| ev(i % 20, Outcome::Hit)).collect()]);
    assert_eq!(feat(&signals_json(&q), "F18")["reason"], "insufficient_samples");
}

#[test]
fn prediction_scorecard_is_unavailable_until_enough_windows_and_reports_failure_honestly() {
    let key = hash_key(b"/steady");
    let p = Pie::new(cfg(Mode::Observe)).unwrap();
    p.tick(0);
    for w in 0..8u64 {
        replay_from(&p, &[mixed_window(2000, &[key], 0.1, 1 + w * 10_000)], w);
        if w < 4 {
            let sc = &signals_json(&p)["prediction"]["scorecard"];
            assert_eq!(sc["precision"]["src"], "unavailable", "window {w}");
        }
    }
    let sc = signals_json(&p)["prediction"]["scorecard"].clone();
    assert!(sc["windows_scored"].as_u64().unwrap() >= 5);
    assert_eq!(sc["precision"]["value"].as_f64().unwrap(), 1.0, "a steady hot key is predicted");
    assert_eq!(sc["recall"]["value"].as_f64().unwrap(), 1.0);
    let pj = signals_json(&p)["prediction"].clone();
    assert_eq!(pj["method"], "rules_plus_ewma_statistics_not_machine_learning");
    assert_eq!(pj["predicted_hot_next_window"][0]["key_hash"], format!("{key:016x}"));

    // a hot key that moves every window cannot be predicted: the scorecard says so
    let q = Pie::new(cfg(Mode::Observe)).unwrap();
    q.tick(0);
    for w in 0..9u64 {
        let k = hash_key(&(500 + w).to_le_bytes());
        replay_from(&q, &[mixed_window(2000, &[k], 0.2, 1 + w * 10_000)], w);
    }
    let sc = signals_json(&q)["prediction"]["scorecard"].clone();
    assert_eq!(sc["precision"]["value"].as_f64().unwrap(), 0.0, "{sc}");
    assert_eq!(sc["recall"]["value"].as_f64().unwrap(), 0.0);
    assert_eq!(sc["baseline_persistence"]["precision"]["value"].as_f64().unwrap(), 0.0);
}

fn flash_trace() -> Vec<Vec<Event>> {
    let crowd: Vec<u64> = (0..10u64).map(|i| hash_key(&(3_000_000 + i).to_le_bytes())).collect();
    let mut w: Vec<Vec<Event>> = (0..8u64).map(|i| uniq_events(1000, 1 + i * 100_000)).collect();
    w.push(mixed_window(4000, &crowd, 0.9, 5_000_000));
    w.push(mixed_window(4000, &crowd, 0.9, 6_000_000));
    w
}

#[test]
fn flash_crowd_is_detected_with_evidence_and_an_advisory_that_applies_nothing() {
    let p = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&p, &flash_trace());
    let r: Vec<_> = recs_of(&p, "POL-FLASH-1");
    assert_eq!(r.len(), 1, "one record (cooldown suppresses the repeat): {r:?}");
    let rec = &r[0];
    assert_eq!(rec["verdict"], "Recommend");
    assert_eq!(rec["recommendation"]["applied"], false);
    let text = rec["recommendation"]["text"].as_str().unwrap();
    assert!(text.contains("pre-warming") && text.contains("Nothing was applied"), "{text}");
    let st = &rec["recommendation"]["structured"];
    assert_eq!(st["confidence_kind"], "heuristic_rule_score");
    assert!(st["confidence"].as_f64().unwrap() >= 0.6);
    assert!(st["rate_ratio"].as_f64().unwrap() > 2.0);
    assert!(st["top10_share"].as_f64().unwrap() > 0.5);
    assert_eq!(st["hot_keys"].as_array().unwrap().len(), 5);
    assert_eq!(rec["inputs"].as_array().unwrap().len(), 3);
    assert!(rec["constraints"].as_array().unwrap().iter().all(|c| c["holds"] == true));
    // observe mode computes the evidence but writes no record
    let o = Pie::new(cfg(Mode::Observe)).unwrap();
    replay(&o, &flash_trace());
    assert_eq!(records(&o).len(), 0);
    assert_eq!(signals_json(&o)["flash_crowd"]["detected_in_last_window"], true);
    // negative controls
    let calm = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&calm, &(0..12u64).map(|i| uniq_events(1000, 1 + i * 100_000)).collect::<Vec<_>>());
    assert_eq!(recs_of(&calm, "POL-FLASH-1").len(), 0);
    let zipf = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(&zipf, &from_workload(&mut ZipfWorkload::new(10_000, 1.0, 2000.0), 31, 16));
    assert_eq!(recs_of(&zipf, "POL-FLASH-1").len(), 0, "stationary Zipf must not look like a flash crowd");
    // seeded frozen generator positive control
    let fc = Pie::new(cfg(Mode::Advisory)).unwrap();
    replay(
        &fc,
        &from_workload(&mut FlashCrowdWorkload::new(10_000, 0.8, 2000.0, 10_000_000, 13_000_000, 6.0, 20), 32, 16),
    );
    assert!(!recs_of(&fc, "POL-FLASH-1").is_empty(), "flash crowd workload must be detected");
}

fn sized_scan_mix(seed: u64, windows: u64, per_window: u64, hot_size: u32, scan_size: u32) -> Vec<Vec<Event>> {
    let mut rng = MasterRng::new(seed).stream("scan_mix");
    let mut next_unique = 1u64 << 40;
    let mut seen = HashSet::new();
    (0..windows)
        .map(|_| {
            (0..per_window)
                .map(|_| {
                    let (k, size) = if rng.next_f64() < 0.3 {
                        (hash_key(format!("/hot_{}", rng.next_range_u64(0, 299)).as_bytes()), hot_size)
                    } else {
                        next_unique += 1;
                        (hash_key(&next_unique.to_le_bytes()), scan_size)
                    };
                    let mut e = if seen.insert(k) { miss_with_origin(k, 20_000, true) } else { ev(k, Outcome::Hit) };
                    e.size_bytes = size;
                    e
                })
                .collect()
        })
        .collect()
}

#[test]
fn p3_weighs_object_size_and_origin_cost_and_says_unavailable_when_unknown() {
    let p = Pie::new(PieConfig { cooldown_ms: 0, ..cfg(Mode::Advisory) }).unwrap();
    replay(&p, &scan_mix(7, 12, 3000));
    assert_eq!(feat(&signals_json(&p), "F27")["reason"], "size_unknown");
    assert_eq!(feat(&signals_json(&p), "F28")["src"], "unavailable");
    let rec = recs_of(&p, "POL-ADMIT-1").into_iter().find(|r| r["verdict"] == "Recommend").expect("P3 still recommends");
    assert!(rec["recommendation"]["text"].as_str().unwrap().contains("Object sizes are unknown"));
    assert_eq!(rec["recommendation"]["structured"]["weighting"], "size_unknown_count_only");
    assert!(rec["guardrail_events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|g| g.as_str().unwrap().starts_with("weight_unavailable:F27")));

    let q = Pie::new(PieConfig { cooldown_ms: 0, ..cfg(Mode::Advisory) }).unwrap();
    replay(&q, &sized_scan_mix(7, 12, 3000, 2_000, 400_000));
    let s = signals_json(&q);
    assert_eq!(feat(&s, "F27")["src"], "derived");
    assert!(feat(&s, "F28")["value"].as_f64().unwrap() > 0.9, "one-hit keys hold nearly all sampled bytes");
    assert!(feat(&s, "F29")["value"].as_f64().unwrap() > 0.5);
    let rec = recs_of(&q, "POL-ADMIT-1").into_iter().find(|r| r["verdict"] == "Recommend").expect("recommend");
    assert_eq!(rec["recommendation"]["structured"]["weighting"], "size_and_cost_considered");
    assert!(rec["recommendation"]["text"].as_str().unwrap().contains("object bytes"));
    assert_eq!(rec["recommendation"]["applied"], false);

    let t = Pie::new(PieConfig { cooldown_ms: 0, ..cfg(Mode::Advisory) }).unwrap();
    replay(&t, &sized_scan_mix(7, 12, 3000, 1_000_000, 20));
    assert!(feat(&signals_json(&t), "F28")["value"].as_f64().unwrap() < 0.3);
    let v = recs_of(&t, "POL-ADMIT-1");
    assert!(v.iter().all(|r| r["verdict"] == "NoAction"), "{v:?}");
    assert!(v.iter().any(|r| r["reason_code"] == "one_hit_bytes_share_low"));
}

#[test]
fn new_detectors_are_registered_honestly_and_their_state_is_bounded() {
    let r = registry_json(&Pie::new(cfg(Mode::Observe)).unwrap());
    let pols = r["policies"].as_array().unwrap();
    for id in ["POL-HOT-1", "POL-FREQ-1", "POL-CARD-1", "POL-PRED-1", "POL-FLASH-1"] {
        let x = pols.iter().find(|p| p["policy_id"] == id).unwrap_or_else(|| panic!("{id} not registered"));
        assert_eq!(x["implemented"], true);
        assert_eq!(x["lifecycle"], "UnitTested", "no claim of Integrated or Evaluated");
        assert_ne!(x["max_runtime_state"], "off");
    }
    assert_eq!(pols.iter().find(|p| p["policy_id"] == "POL-FLASH-1").unwrap()["max_runtime_state"], "advisory");
    assert_eq!(pols.iter().find(|p| p["policy_id"] == "POL-HOT-1").unwrap()["max_runtime_state"], "observe");
    let ttl = pols.iter().find(|p| p["policy_id"] == "POL-TTL-1").unwrap();
    assert_eq!(ttl["implemented"], false, "adaptive TTL stays a stub");
    assert_eq!(ttl["effective_runtime_state"]["value"], "off");
    assert!(pols.iter().all(|p| p["lifecycle"] != "Integrated" && p["lifecycle"] != "ShadowEvaluated"));
    validate(&registry()).unwrap();

    let p = Pie::new(cfg(Mode::Observe)).unwrap();
    p.tick(0);
    for w in 0..30u64 {
        let hot: Vec<u64> = (0..40u64).map(|i| hash_key(&(w * 1000 + i).to_le_bytes())).collect();
        replay_from(&p, &[mixed_window(2000, &hot, 0.9, w * 100_000)], w);
    }
    let g = p.locked();
    assert!(g.fstate.ins.tracked_keys() <= TRACKS_CAP);
    assert!(g.windows.back().unwrap().hot.len() <= 10);
    drop(g);
    let st = status_json(&p);
    assert!(st["memory"]["bytes"]["value"].as_u64().unwrap() <= st["memory"]["budget_bytes"].as_u64().unwrap());
}

#[test]
fn insight_outputs_are_deterministic_and_degraded_windows_hide_them() {
    let run = || {
        let p = Pie::new(cfg(Mode::Advisory)).unwrap();
        replay(&p, &flash_trace());
        format!("{}{}", signals_json(&p), decisions_json(&p, 200, 0, None))
    };
    assert_eq!(run(), run());
    let c = PieConfig { queue_capacity: 64, ..cfg(Mode::Advisory) };
    let p = Pie::new(c).unwrap();
    p.tick(0);
    for e in uniq_events(2000, 1) {
        p.sink().try_record(e);
    }
    p.tick(1000);
    let s = signals_json(&p);
    for id in ["F18", "F23", "F24", "F26", "F27", "F29", "F30"] {
        assert_eq!(feat(&s, id)["src"], "unavailable", "{id}");
        assert_eq!(feat(&s, id)["reason"], "window_degraded");
    }
    assert_eq!(s["hot_keys"]["items"].as_array().unwrap().len(), 0);
}

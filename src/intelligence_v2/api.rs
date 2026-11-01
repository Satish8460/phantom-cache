//! JSON bodies for the pie-api-v1 endpoints (see `docs/lanes/CONTRACT-pie-api-v1.md`). No HTTP types here.

use super::engine::{Pie, WINDOWS_KEPT};
use super::registry::{registry, PolicySpec};
use super::signals::{close_window, Accum, FeatureState};
use super::{parse_mode, Mode, Src, REGISTRY_VERSION, SCHEMA};
use serde_json::{json, Map, Value};

fn v(value: Value, src: Src, samples: u64) -> Value {
    json!({"value": value, "src": src.as_str(), "samples": samples})
}

fn unavailable(reason: &str, samples: u64) -> Value {
    json!({"value": null, "src": "unavailable", "samples": samples, "reason": reason})
}

pub fn not_installed_json() -> Value {
    json!({"status":"ok","schema":SCHEMA,"installed":false,"src":"unavailable","reason":"intelligence_v2_not_installed"})
}

pub fn unknown_policy_json(id: &str) -> Value {
    json!({"status":"error","error":"unknown_policy","policy_id":id})
}

pub fn unknown_decision_json() -> Value {
    json!({"status":"error","error":"unknown_decision"})
}

pub fn bad_request_json(detail: &str) -> Value {
    json!({"status":"error","error":"bad_request","detail":detail})
}

fn mode_v(p: &Pie) -> Value {
    v(json!(p.mode().as_str()), Src::Measured, 0)
}

fn effective_state(spec: &PolicySpec, process: Mode) -> Mode {
    let max = parse_mode(spec.max_runtime_state).unwrap_or(Mode::Off);
    if !spec.implemented {
        Mode::Off
    } else {
        max.min(process)
    }
}

fn spec_json(p: &Pie, s: &PolicySpec) -> Value {
    let feats: Vec<Value> = s
        .required_inputs
        .iter()
        .map(|f| {
            json!({"feature": f, "min_samples": p.config().min_feature_samples, "allowed_src": ["measured","derived"]})
        })
        .collect();
    let params: Vec<Value> = s
        .parameters
        .iter()
        .map(|x| {
            json!({"name": x.name, "default": x.default, "min": x.min, "max": x.max, "unit": x.unit,
                   "current": p.params().get(x.name)})
        })
        .collect();
    json!({
        "policy_id": s.id, "name": s.name, "kind": s.kind, "implemented": s.implemented,
        "objective": s.objective, "trigger": s.trigger,
        "required_inputs": feats, "parameters": params,
        "safety_constraints": s.safety_constraints,
        "expected_effect": s.expected_effect, "evaluation_method": s.evaluation_method,
        "limitations": s.limitations, "version": s.version,
        "lifecycle": s.lifecycle.as_str(),
        "max_runtime_state": s.max_runtime_state,
        "effective_runtime_state": v(json!(effective_state(s, p.mode()).as_str()), Src::Measured, 0),
    })
}

pub fn registry_json(p: &Pie) -> Value {
    let policies: Vec<Value> = registry().iter().map(|s| spec_json(p, s)).collect();
    json!({
        "status":"ok","schema":SCHEMA,"installed":true,
        "registry_version":REGISTRY_VERSION,"max_mode":"advisory",
        "process_mode": mode_v(p),
        "doc_fields":["objective","trigger","expected_effect","evaluation_method","limitations","lifecycle","safety_constraints"],
        "policies": policies,
    })
}

/// The object to place under key `"v2"` of the existing `/api/v1/intelligence/status` body.
pub fn status_json(p: &Pie) -> Value {
    let cfg = p.config();
    let g = p.locked();
    let sh = p.shared();
    let enq = sh.enqueued.load(std::sync::atomic::Ordering::Relaxed);
    let dropped = sh.dropped.load(std::sync::atomic::Ordering::Relaxed);
    let deq = sh.dequeued.load(std::sync::atomic::Ordering::Relaxed);
    let mem = cfg.memory_estimate();
    let fault = p.fault().map(|f| {
        json!({"decision_id": f.decision_id, "timestamp_ms": f.timestamp_ms, "reason_code": f.reason_code, "detail": f.detail})
    });
    let last_tick = match g.last_tick_ms {
        Some(t) => v(json!(t), Src::Measured, 0),
        None => unavailable("no_tick_yet", 0),
    };
    json!({
        "schema": SCHEMA, "installed": true, "registry_version": REGISTRY_VERSION,
        "mode": mode_v(p), "max_mode": "advisory",
        "fault": fault,
        "queue": {
            "capacity": cfg.queue_capacity,
            "enqueued_total": v(json!(enq), Src::Measured, 0),
            "dropped_total": v(json!(dropped), Src::Measured, 0),
            "depth": v(json!(enq.saturating_sub(deq)), Src::Measured, 0),
        },
        "window": {
            "window_ms": cfg.window_ms, "decision_every_windows": cfg.decision_every_windows,
            "windows_closed": v(json!(g.windows_closed), Src::Measured, 0),
            "last_window_degraded": v(json!(g.last_window_degraded), Src::Measured, 0),
            "last_tick_ms": last_tick,
        },
        "memory": {
            "bytes": v(json!(mem.total()), Src::Derived, 0),
            "budget_bytes": cfg.memory_budget_bytes,
            "note": "upper bound from declared capacities, not an allocator measurement",
            "breakdown": {"queue": mem.queue, "window_state": mem.window_state,
                          "shadow_caches": mem.shadow_caches, "decision_ring": mem.decision_ring},
        },
        "decisions": {
            "recorded_total": v(json!(g.ring.total()), Src::Measured, 0),
            "retained": v(json!(g.ring.len()), Src::Measured, 0),
            "ring_capacity": g.ring.capacity(),
            "ring_bytes": v(json!(g.ring.bytes()), Src::Measured, 0),
            "suppressed_by_cooldown": v(json!(g.suppressed_total), Src::Measured, 0),
            "collapsed_repeat_noaction": v(json!(g.collapsed_total), Src::Measured, 0),
        },
        "config": {
            "lab_mode": cfg.lab_mode, "cache_capacity_entries": cfg.cache_capacity_entries,
            "shadow_sample_modulus": cfg.shadow_sample_modulus, "incumbent_policy": cfg.incumbent.as_str(),
            "declared": true,
        },
        "claims": {"benefit_claim": "none",
                   "note": "M6 H1 remains NOT_SUPPORTED; advisory output is a shadow estimate, never an applied change"},
    })
}

fn feat_json(f: &super::signals::Feat) -> Value {
    let mut m = Map::new();
    m.insert("name".into(), json!(f.name));
    m.insert("unit".into(), json!(f.unit));
    m.insert("src".into(), json!(f.src.as_str()));
    m.insert("samples".into(), json!(f.samples));
    match f.value {
        Some(super::FVal::F(x)) if x.is_finite() => {
            m.insert("value".into(), json!(x));
        }
        Some(super::FVal::B(b)) => {
            m.insert("value".into(), json!(b));
        }
        _ => {
            m.insert("value".into(), Value::Null);
        }
    }
    if let Some(r) = f.reason {
        m.insert("reason".into(), json!(r));
    }
    Value::Object(m)
}

pub fn signals_json(p: &Pie) -> Value {
    let min = p.config().min_feature_samples;
    let g = p.locked();
    let (window, features, top) = match g.windows.back() {
        Some(w) => {
            let mut fm = Map::new();
            for f in &w.features {
                fm.insert(f.id.to_string(), feat_json(f));
            }
            let items: Vec<Value> = w
                .top_keys
                .iter()
                .map(|(k, c)| json!({"key_hash": format!("{k:016x}"), "count": c}))
                .collect();
            (
                json!({"index": w.index, "start_ms": w.start_ms, "end_ms": w.end_ms, "duration_ms": w.duration_ms(),
                       "events": w.events, "dropped_events": w.dropped, "degraded": w.degraded,
                       "degraded_reason": w.degraded_reason}),
                Value::Object(fm),
                items,
            )
        }
        None => {
            let empty = close_window(&Accum::new(0, 0), &mut FeatureState::new(), 0, 1, 0, min);
            let mut fm = Map::new();
            for f in &empty.features {
                let mut fe = f.clone();
                if fe.reason == Some("no_events") {
                    fe.reason = Some("no_window_closed_yet");
                }
                fm.insert(fe.id.to_string(), feat_json(&fe));
            }
            (Value::Null, Value::Object(fm), vec![])
        }
    };
    let classification = match (&g.label, p.mode()) {
        (Some((l, _)), Mode::Advisory) => v(
            json!(l),
            Src::Derived,
            g.windows.back().map(|w| w.requests).unwrap_or(0),
        ),
        (_, Mode::Advisory) => unavailable("not_evaluated_yet", 0),
        _ => unavailable("advisory_mode_required", 0),
    };
    let history: Vec<Value> = g
        .windows
        .iter()
        .rev()
        .take(WINDOWS_KEPT)
        .rev()
        .map(|w| json!({"index": w.index, "end_ms": w.end_ms, "events": w.events, "degraded": w.degraded}))
        .collect();
    let ins = &g.fstate.ins;
    let (hot_items, hot_samples) = match g.windows.back() {
        Some(w) => (
            w.hot
                .iter()
                .map(|h| {
                    json!({"key_hash": format!("{:016x}", h.key_hash), "count_lower": h.lower, "count_upper": h.upper,
                           "error_bound": h.error, "share": h.share, "confidence": h.confidence, "hot": h.hot})
                })
                .collect::<Vec<_>>(),
            w.requests,
        ),
        None => (vec![], 0),
    };
    let hot_keys = json!({
        "src": "derived", "samples": hot_samples, "method": "space_saving_64_counters",
        "hot_share_threshold": ins.cfg.hot_share,
        "note": "count_lower <= true count <= count_upper; confidence = count_lower / count_upper (1.0 exact), not a probability; key hashes only",
        "items": hot_items,
    });
    let prec = |x: Option<f64>, n: u64| match x {
        Some(v) if n >= ins.cfg.pred_min_windows => v_json(json!(v), n),
        _ => unavailable("insufficient_windows_scored", n),
    };
    let prediction = json!({
        "method": "rules_plus_ewma_statistics_not_machine_learning",
        "src": "derived",
        "predicted_hot_next_window": ins.predicted_next.iter()
            .map(|(k, s)| json!({"key_hash": format!("{k:016x}"), "predicted_share": s})).collect::<Vec<_>>(),
        "scorecard": {
            "windows_scored": ins.score.windows,
            "min_windows": ins.cfg.pred_min_windows,
            "precision": prec(ins.score.precision(), ins.score.windows),
            "recall": prec(ins.score.recall(), ins.score.windows),
            "baseline_persistence": {
                "precision": prec(ins.baseline_score.precision(), ins.baseline_score.windows),
                "recall": prec(ins.baseline_score.recall(), ins.baseline_score.windows),
                "note": "predicts that last window's hot keys stay hot; the predictor is only useful if it beats this",
            },
        },
    });
    let flash_crowd = match g.windows.back().and_then(|w| w.flash.as_ref()) {
        Some(e) => json!({
            "detected_in_last_window": true, "src": "derived",
            "confidence": e.confidence, "confidence_kind": "heuristic_rule_score",
            "rate_ratio": e.rate_ratio, "rate_cusum": e.rate_cusum, "rate_fired": e.rate_fired,
            "top10_share": e.top10_share, "concentration_baseline": e.conc_baseline,
            "concentration_cusum": e.conc_cusum, "concentration_fired": e.conc_fired,
            "thresholds": {"rate_h": ins.cfg.rate_h, "rate_slack": ins.cfg.rate_slack,
                           "conc_h": ins.cfg.conc_h, "conc_slack": ins.cfg.conc_slack},
        }),
        None => json!({"detected_in_last_window": false, "src": "derived",
            "thresholds": {"rate_h": ins.cfg.rate_h, "rate_slack": ins.cfg.rate_slack,
                           "conc_h": ins.cfg.conc_h, "conc_slack": ins.cfg.conc_slack}}),
    };
    json!({
        "status":"ok","schema":SCHEMA,"installed":true,"mode":mode_v(p),
        "window": window, "features": features,
        "top_keys": {"src":"derived","items": top},
        "classification": classification,
        "hot_keys": hot_keys, "prediction": prediction, "flash_crowd": flash_crowd,
        "history": history,
    })
}

pub fn decisions_json(p: &Pie, limit: usize, offset: usize, policy_id: Option<&str>) -> Value {
    let g = p.locked();
    let filtered: Vec<&super::arbiter::DecisionRecord> = g
        .ring
        .iter()
        .filter(|r| policy_id.map(|id| r.policy_id == id).unwrap_or(true))
        .collect();
    let page: Vec<Value> = filtered
        .iter()
        .skip(offset)
        .take(limit)
        .map(|r| serde_json::to_value(r).unwrap_or(Value::Null))
        .collect();
    json!({
        "status":"ok","schema":SCHEMA,"installed":true,
        "total_recorded": g.ring.total(), "retained": g.ring.len(),
        "limit": limit, "offset": offset, "returned": page.len(),
        "ordering":"oldest_first", "records": page,
    })
}

pub fn decision_json(p: &Pie, id: u64) -> Option<Value> {
    let g = p.locked();
    g.ring.get(id).map(|r| json!({"status":"ok","schema":SCHEMA,"installed":true,"record": serde_json::to_value(r).unwrap_or(Value::Null)}))
}

pub fn policy_json(p: &Pie, id: &str) -> Option<Value> {
    let spec = registry().into_iter().find(|s| s.id == id)?;
    let g = p.locked();
    let st = g.states.get(spec.id).cloned().unwrap_or_default();
    let recent: Vec<Value> = {
        let all: Vec<&super::arbiter::DecisionRecord> =
            g.ring.iter().filter(|r| r.policy_id == spec.id).collect();
        let start = all.len().saturating_sub(20);
        all[start..]
            .iter()
            .map(|r| serde_json::to_value(r).unwrap_or(Value::Null))
            .collect()
    };
    let opt = |o: Option<Value>, reason: &str| match o {
        Some(x) => v(x, Src::Measured, 0),
        None => unavailable(reason, 0),
    };
    drop(g);
    let spec_v = spec_json(p, &spec);
    Some(json!({
        "status":"ok","schema":SCHEMA,"installed":true,
        "policy": spec_v,
        "runtime": {
            "last_evaluated_ms": opt(st.last_eval_ms.map(|x| json!(x)), "not_evaluated_yet"),
            "last_verdict": opt(st.last_verdict.map(|x| json!(format!("{x:?}"))), "not_evaluated_yet"),
            "last_reason_code": opt(st.last_reason.clone().map(|x| json!(x)), "not_evaluated_yet"),
            "evaluations": v(json!(st.evaluations), Src::Measured, 0),
            "recommendations": v(json!(st.recommendations), Src::Measured, 0),
            "suppressed_by_cooldown": v(json!(st.suppressed_by_cooldown), Src::Measured, 0),
            "warning_active": v(json!(st.warning_active), Src::Measured, 0),
        },
        "recent_decisions": recent,
    }))
}

/// Parses the query string of `/decisions`: `limit` 1..=200 (default 50), `offset` >= 0, `policy_id`.
pub fn parse_decisions_query(q: Option<&str>) -> Result<(usize, usize, Option<String>), String> {
    let (mut limit, mut offset, mut pid) = (50usize, 0usize, None);
    for kv in q.unwrap_or("").split('&').filter(|s| !s.is_empty()) {
        let (k, val) = kv
            .split_once('=')
            .ok_or_else(|| format!("malformed query pair '{kv}'"))?;
        match k {
            "limit" => {
                limit = val
                    .parse::<usize>()
                    .map_err(|_| "limit must be an integer".to_string())?;
                if !(1..=200).contains(&limit) {
                    return Err("limit must be between 1 and 200".into());
                }
            }
            "offset" => {
                offset = val
                    .parse::<usize>()
                    .map_err(|_| "offset must be a non-negative integer".to_string())?
            }
            "policy_id" => {
                if val.is_empty()
                    || val.len() > 64
                    || !val.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                {
                    return Err("policy_id has invalid characters".into());
                }
                pid = Some(val.to_string());
            }
            other => return Err(format!("unknown query parameter '{other}'")),
        }
    }
    Ok((limit, offset, pid))
}

fn v_json(value: Value, samples: u64) -> Value {
    json!({"value": value, "src": "derived", "samples": samples})
}

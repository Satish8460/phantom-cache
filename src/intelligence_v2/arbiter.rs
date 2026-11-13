//! Decision record v2, bounded decision ring, and arbiter helpers (conflict roles, cooldown).

use super::{FVal, Src};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Verdict {
    Recommend,
    NoAction,
    Observation,
    Fault,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputUsed {
    pub feature: &'static str,
    pub value: Option<FVal>,
    pub src: Src,
    pub samples: u64,
    pub window_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Effect {
    pub metric: &'static str,
    pub value: f64,
    pub ci95: [f64; 2],
    pub src: Src,
    pub basis: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateEval {
    pub candidate_id: String,
    pub status: &'static str,
    pub rejection_reason: Option<&'static str>,
    pub estimated_effect: Option<Effect>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Constraint {
    pub name: &'static str,
    pub value: &'static str,
    pub holds: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Recommendation {
    pub text: String,
    pub structured: Value,
    pub applied: bool,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct Conflict {
    pub lever: &'static str,
    pub role: &'static str,
    pub with: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionRecord {
    pub decision_id: u64,
    pub timestamp_ms: u64,
    pub registry_version: &'static str,
    pub policy_id: &'static str,
    pub policy_version: &'static str,
    pub mode: &'static str,
    pub verdict: Verdict,
    pub reason_code: String,
    pub inputs: Vec<InputUsed>,
    pub candidates: Vec<CandidateEval>,
    pub constraints: Vec<Constraint>,
    pub recommendation: Option<Recommendation>,
    pub guardrail_events: Vec<String>,
    pub conflict: Conflict,
    pub observed_effect: Value,
    pub simulated: bool,
    pub explanation: String,
}

pub fn observed_effect_unavailable() -> Value {
    json!({"value": null, "src": "unavailable", "samples": 0, "reason": "not_implemented"})
}

pub fn standard_constraints() -> Vec<Constraint> {
    vec![
        Constraint {
            name: "stale_cap_unchanged",
            value: "not_touched",
            holds: true,
        },
        Constraint {
            name: "origin_cap_unchanged",
            value: "not_touched",
            holds: true,
        },
        Constraint {
            name: "memory_budget_unchanged",
            value: "not_touched",
            holds: true,
        },
        Constraint {
            name: "applied",
            value: "never",
            holds: true,
        },
    ]
}

/// Deterministic template text. A reviewer can reconstruct the decision from the record alone.
pub fn explain(r: &DecisionRecord) -> String {
    let mut s = format!(
        "Policy {} v{} at {} ms in {} mode: verdict {:?} ({}).",
        r.policy_id, r.policy_version, r.timestamp_ms, r.mode, r.verdict, r.reason_code
    );
    if r.inputs.is_empty() {
        s.push_str(" No inputs were used.");
    } else {
        s.push_str(" Inputs:");
        for i in &r.inputs {
            let v = match i.value {
                Some(FVal::F(x)) => format!("{x:.4}"),
                Some(FVal::B(b)) => b.to_string(),
                None => "n/a".to_string(),
            };
            s.push_str(&format!(
                " {}={} [{}, n={}];",
                i.feature,
                v,
                i.src.as_str(),
                i.samples
            ));
        }
    }
    for c in &r.candidates {
        s.push_str(&format!(" Candidate {} {}", c.candidate_id, c.status));
        if let Some(rr) = c.rejection_reason {
            s.push_str(&format!(" ({rr})"));
        }
        if let Some(e) = &c.estimated_effect {
            s.push_str(&format!(
                ": {} {:+.4} (95% interval [{:.4}, {:.4}], {}, {}).",
                e.metric,
                e.value,
                e.ci95[0],
                e.ci95[1],
                e.src.as_str(),
                e.basis
            ));
        } else {
            s.push('.');
        }
    }
    if let Some(rec) = &r.recommendation {
        s.push_str(&format!(" Recommendation ({}): {}", rec.label, rec.text));
    }
    if !r.guardrail_events.is_empty() {
        s.push_str(&format!(
            " Guardrail events: {}.",
            r.guardrail_events.join(", ")
        ));
    }
    s.push_str(" Nothing was applied.");
    s
}

/// Bounded ring of records, bounded both by count and by serialized bytes.
pub struct DecisionRing {
    cap: usize,
    budget_bytes: usize,
    records: VecDeque<(DecisionRecord, usize)>,
    bytes: usize,
    total: u64,
}

impl DecisionRing {
    pub fn new(cap: usize, budget_bytes: usize) -> Self {
        Self {
            cap: cap.max(1),
            budget_bytes,
            records: VecDeque::new(),
            bytes: 0,
            total: 0,
        }
    }
    pub fn append(&mut self, rec: DecisionRecord) {
        let sz = serde_json::to_vec(&rec).map(|v| v.len()).unwrap_or(0);
        self.records.push_back((rec, sz));
        self.bytes += sz;
        self.total += 1;
        while self.records.len() > self.cap
            || (self.bytes > self.budget_bytes && self.records.len() > 1)
        {
            if let Some((_, s)) = self.records.pop_front() {
                self.bytes = self.bytes.saturating_sub(s);
            }
        }
    }
    pub fn len(&self) -> usize {
        self.records.len()
    }
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn total(&self) -> u64 {
        self.total
    }
    pub fn capacity(&self) -> usize {
        self.cap
    }
    pub fn iter(&self) -> impl Iterator<Item = &DecisionRecord> {
        self.records.iter().map(|(r, _)| r)
    }
    pub fn get(&self, id: u64) -> Option<&DecisionRecord> {
        self.iter().find(|r| r.decision_id == id)
    }
}

/// Per-policy runtime state kept by the arbiter.
#[derive(Debug, Clone, Default)]
pub struct PolicyState {
    pub evaluations: u64,
    pub recommendations: u64,
    pub suppressed_by_cooldown: u64,
    pub warning_active: bool,
    pub last_eval_ms: Option<u64>,
    pub last_verdict: Option<Verdict>,
    pub last_reason: Option<String>,
    pub last_rec: Option<(String, u64)>,
    pub last_label: Option<String>,
}

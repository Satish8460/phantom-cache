//! Decision Audit & Explainability Store (WP-35)
//!
//! Authoritative Specification: `docs/research/phase_5_execution_package.md` §4 M5-06
//!
//! Enforces:
//! - Bounded, byte-accounted circular buffer of immutable DecisionRecords
//! - Complete audit trail: inputs with provenance, CI bounds, guardrail states, config hashes
//! - Human-readable explainability reconstruction
//! - Exportable to JSON for lab replay, Tier-1 validation, and M6 evaluation

use super::*;
use std::collections::VecDeque;

pub const DEFAULT_AUDIT_STORE_CAPACITY: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStore {
    capacity: usize,
    records: VecDeque<DecisionRecord>,
    total_decisions_recorded: u64,
}

impl AuditStore {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(10);
        Self {
            capacity: cap,
            records: VecDeque::with_capacity(cap),
            total_decisions_recorded: 0,
        }
    }

    pub fn append(&mut self, record: DecisionRecord) {
        if self.records.len() >= self.capacity {
            self.records.pop_front();
        }
        self.records.push_back(record);
        self.total_decisions_recorded += 1;
    }

    pub fn get_by_id(&self, decision_id: u64) -> Option<&DecisionRecord> {
        self.records.iter().find(|r| r.decision_id == decision_id)
    }

    pub fn recent_decisions(&self, count: usize) -> Vec<DecisionRecord> {
        self.records
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect::<Vec<_>>()
    }

    pub fn all_decisions(&self) -> Vec<DecisionRecord> {
        self.records.iter().cloned().collect()
    }

    pub fn decisions_by_verdict(&self, verdict: Verdict) -> Vec<DecisionRecord> {
        self.records
            .iter()
            .filter(|r| r.verdict == verdict)
            .cloned()
            .collect()
    }

    pub fn total_recorded(&self) -> u64 {
        self.total_decisions_recorded
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Reconstructs a full human-readable explanation from the decision record alone.
    pub fn explain_decision(record: &DecisionRecord) -> String {
        format!(
            "Decision #{id} [{verdict:?}] at {time}ms (Mode: {mode:?})\n\
             Trigger: {trigger}\n\
             Candidate: {cand_id} ({cand_kind:?})\n\
             Estimated Effect: {effect:.4} (95% CI: [{ci_l:.4}, {ci_u:.4}])\n\
             Guardrails Passed: {guardrails}\n\
             Configuration: {cfg_before} -> {cfg_after}\n\
             Rationale: {explanation}",
            id = record.decision_id,
            verdict = record.verdict,
            time = record.timestamp_ms,
            mode = record.mode,
            trigger = record.trigger_reason,
            cand_id = record.candidate.candidate_id,
            cand_kind = record.candidate.kind,
            effect = record.estimated_effect,
            ci_l = record.confidence_interval.0,
            ci_u = record.confidence_interval.1,
            guardrails = record.guardrails_passed,
            cfg_before = record.config_hash_before,
            cfg_after = record.config_hash_after,
            explanation = record.explanation,
        )
    }

    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self)
    }

    pub fn import_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for AuditStore {
    fn default() -> Self {
        Self::new(DEFAULT_AUDIT_STORE_CAPACITY)
    }
}

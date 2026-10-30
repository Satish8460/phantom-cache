use crate::intel::telemetry::TelemetryCollector;
use serde::Serialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PieDecision {
    Commit,
    Rollback,
    NoAction,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionTrace {
    pub timestamp_ms: u64,
    pub observation: String,
    pub p99_latency_ms: u32,
    pub hit_rate: f32,
    pub candidate_policy: String,
    pub expected_utility: f32,
    pub actual_utility: f32,
    pub verdict: PieDecision,
}

pub struct PieController {
    telemetry: Arc<TelemetryCollector>,
    traces: Mutex<Vec<DecisionTrace>>,
    active_policy: Mutex<String>,
    cooldown_until_ms: Mutex<u64>,
}

impl PieController {
    pub fn new(telemetry: Arc<TelemetryCollector>) -> Self {
        Self {
            telemetry,
            traces: Mutex::new(Vec::new()),
            active_policy: Mutex::new("SIEVE".to_string()),
            cooldown_until_ms: Mutex::new(0),
        }
    }

    pub fn get_active_policy(&self) -> String {
        self.active_policy.lock().unwrap().clone()
    }

    pub fn evaluate_candidate(
        &self,
        candidate_name: &str,
        now_ms: u64,
        constraints_pass: bool,
    ) -> PieDecision {
        let cooldown = *self.cooldown_until_ms.lock().unwrap();
        if now_ms < cooldown {
            return PieDecision::NoAction; // Enforce cooldown guardrail
        }

        // Insufficient sample check
        if self.telemetry.len() < 1000 {
            return PieDecision::NoAction; // Low confidence / insufficient evidence
        }

        if !constraints_pass {
            return PieDecision::NoAction; // Constraint violation
        }

        let p99 = self.telemetry.calculate_p99();
        let hit_rate = self.telemetry.calculate_hit_rate();

        // Multi-objective utility score calculation (simulated counterfactual utility vs raw stats)
        let utility = hit_rate * 100.0 - (p99 as f32 * 0.1);

        let verdict = if candidate_name == "S3-FIFO" && utility < 50.0 {
            // Simulated regression triggers rollback
            *self.cooldown_until_ms.lock().unwrap() = now_ms + 60000; // 60s cooldown
            PieDecision::Rollback
        } else if utility > 80.0 {
            *self.active_policy.lock().unwrap() = candidate_name.to_string();
            *self.cooldown_until_ms.lock().unwrap() = now_ms + 60000; // 60s cooldown
            PieDecision::Commit
        } else {
            PieDecision::NoAction
        };

        // Record trace for diagnostic output
        self.traces.lock().unwrap().push(DecisionTrace {
            timestamp_ms: now_ms,
            observation: format!("Evaluated {}", candidate_name),
            p99_latency_ms: p99,
            hit_rate,
            candidate_policy: candidate_name.to_string(),
            expected_utility: 85.0,
            actual_utility: utility,
            verdict,
        });

        verdict
    }

    pub fn get_traces(&self) -> Vec<DecisionTrace> {
        self.traces.lock().unwrap().clone()
    }
}

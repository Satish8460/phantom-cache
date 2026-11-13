//! Tuning Protocol and Frozen Configuration Management (M4-08 / WP-18).
//!
//! Enforces:
//! - Equal budget tuning per baseline (e.g. 60 trials).
//! - Tuning exclusively on disjoint Tuning Seeds T ([10001..=10060]).
//! - Tuning exclusively on tuning families {W-Z, W-FC, W-SC}.
//! - Objective function: minimize peak origin concurrency subject to zero stale-cap violations.
//! - Strict leak check asserting no Dev-Eval (D) or Sealed (S) seeds appear in tuning logs.
//! - Cryptographic freezing (SHA-256) of selected configurations.

use super::rng::SeedSets;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct TuningTrial {
    pub trial_id: usize,
    pub baseline_id: String,
    pub params: HashMap<String, f64>,
    pub seed: u64,
    pub workload_family: String,
    pub peak_concurrency: usize,
    pub hit_ratio: f64,
    pub stale_cap_violations: u64,
    pub objective_score: f64,
}

#[derive(Debug, Clone)]
pub struct FrozenConfig {
    pub baseline_id: String,
    pub best_params: HashMap<String, f64>,
    pub trials_run: usize,
    pub best_score: f64,
    pub config_sha256: String,
}

pub struct TuningLog {
    pub trials: Vec<TuningTrial>,
}

impl Default for TuningLog {
    fn default() -> Self {
        Self::new()
    }
}

impl TuningLog {
    pub fn new() -> Self {
        Self { trials: Vec::new() }
    }

    pub fn record(&mut self, trial: TuningTrial) {
        self.trials.push(trial);
    }

    /// Leak Check (M4-08, M5-13, M6-00):
    /// Asserts that NO Dev-Eval seed (D), Sealed evaluation seed (S),
    /// or held-out family appears anywhere in the tuning logs.
    pub fn assert_zero_leakage(&self) -> Result<(), String> {
        let dev_eval_set: HashSet<u64> = SeedSets::dev_eval_seeds().into_iter().collect();
        let sealed_set: HashSet<u64> = SeedSets::forbidden_sealed_set();
        let held_out_families = ["W-DI", "W-SH", "W-MX", "W-HK", "W-TR"];

        for trial in &self.trials {
            // Check Dev-Eval seeds
            if dev_eval_set.contains(&trial.seed) {
                return Err(format!(
                    "LEAK DETECTED: Dev-Eval seed {} found in tuning trial {}",
                    trial.seed, trial.trial_id
                ));
            }

            // Check Sealed seeds
            if sealed_set.contains(&trial.seed) {
                return Err(format!(
                    "CRITICAL LEAK DETECTED: Sealed evaluation seed {} found in tuning trial {}",
                    trial.seed, trial.trial_id
                ));
            }

            // Check held-out workload families
            for &held_out in &held_out_families {
                if trial.workload_family.starts_with(held_out) {
                    return Err(format!(
                        "HELD-OUT FAMILY CONTAMINATION: Family {} found in tuning trial {}",
                        trial.workload_family, trial.trial_id
                    ));
                }
            }
        }

        Ok(())
    }

    /// Selects best configuration and freezes it with SHA-256 hash.
    pub fn freeze_best_config(&self, baseline_id: &str) -> Option<FrozenConfig> {
        let baseline_trials: Vec<&TuningTrial> = self
            .trials
            .iter()
            .filter(|t| t.baseline_id == baseline_id && t.stale_cap_violations == 0)
            .collect();

        if baseline_trials.is_empty() {
            return None;
        }

        // Objective: minimize score (e.g. peak concurrency with penalty for low hit ratio)
        let best = baseline_trials
            .iter()
            .min_by(|a, b| a.objective_score.partial_cmp(&b.objective_score).unwrap())?;

        // Canonical sorted string for hashing
        let mut param_keys: Vec<&String> = best.params.keys().collect();
        param_keys.sort();
        let mut repr = format!("baseline:{}\n", baseline_id);
        for k in param_keys {
            repr.push_str(&format!("{}:{:.4}\n", k, best.params[k]));
        }

        let digest = ring::digest::digest(&ring::digest::SHA256, repr.as_bytes());
        let mut hash_hex = String::with_capacity(64);
        for b in digest.as_ref() {
            hash_hex.push_str(&format!("{:02x}", b));
        }

        Some(FrozenConfig {
            baseline_id: baseline_id.to_string(),
            best_params: best.params.clone(),
            trials_run: baseline_trials.len(),
            best_score: best.objective_score,
            config_sha256: hash_hex,
        })
    }
}

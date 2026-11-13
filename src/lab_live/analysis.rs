//! Pure analysis for Scenario B: paired differences (BCa CI + sign-flip permutation p from
//! `crate::lab::analysis`, called not modified) and the mechanical success/failure/inconclusive rule
//! of `docs/demo/scenario_B_origin_slowdown.md` section 7.

use crate::lab::analysis::StatisticalEvaluator;
use serde::{Deserialize, Serialize};

pub const MIN_VALID_RUNS: usize = 10;
/// Required relative reduction of peak origin concurrency (contract: 5%).
pub const BENEFIT_THRESHOLD: f64 = 0.05;
/// Allowed client success-rate loss vs BASE, in percentage points (contract: 1 pp).
pub const SUCCESS_TOLERANCE_PP: f64 = 1.0;
pub const BCA_RESAMPLES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paired {
    pub n_pairs: usize,
    pub mean_base: f64,
    pub mean_treat: f64,
    /// treat - base (same unit as the metric).
    pub mean_diff: f64,
    /// Mean of per-pair (base - treat) / base; positive = treatment lower. None if no pair has base != 0.
    pub rel_reduction_mean: Option<f64>,
    pub rel_ci95_lo: Option<f64>,
    pub rel_ci95_hi: Option<f64>,
    /// Two-sided sign-flip permutation p on the per-pair relative reductions.
    pub perm_p: Option<f64>,
    /// 95% BCa CI of the mean of (treat - base).
    pub diff_ci95_lo: Option<f64>,
    pub diff_ci95_hi: Option<f64>,
}

/// Paired comparison; `base[i]` and `treat[i]` belong to the same repetition. Non-finite pairs are dropped
/// (never replaced by zero).
pub fn paired(base: &[f64], treat: &[f64], seed: u64) -> Option<Paired> {
    let pairs: Vec<(f64, f64)> = base
        .iter()
        .zip(treat.iter())
        .filter(|(a, b)| a.is_finite() && b.is_finite())
        .map(|(a, b)| (*a, *b))
        .collect();
    if pairs.is_empty() {
        return None;
    }
    let n = pairs.len() as f64;
    let mean_base = pairs.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_treat = pairs.iter().map(|p| p.1).sum::<f64>() / n;
    let diffs: Vec<f64> = pairs.iter().map(|p| p.1 - p.0).collect();
    let rel: Vec<f64> = pairs.iter().filter(|p| p.0 != 0.0).map(|p| (p.0 - p.1) / p.0).collect();
    let (dlo, dhi) = StatisticalEvaluator::bca_ci_mean(&diffs, BCA_RESAMPLES, seed ^ 0xD1FF);
    let (rel_mean, rlo, rhi, p) = if rel.is_empty() {
        (None, None, None, None)
    } else {
        let m = rel.iter().sum::<f64>() / rel.len() as f64;
        let (lo, hi) = StatisticalEvaluator::bca_ci_mean(&rel, BCA_RESAMPLES, seed ^ 0x2E1);
        (Some(m), Some(lo), Some(hi), Some(StatisticalEvaluator::sign_flip_p_value(&rel, seed ^ 0xA11)))
    };
    Some(Paired {
        n_pairs: pairs.len(),
        mean_base,
        mean_treat,
        mean_diff: mean_base.mul_add(-1.0, mean_treat),
        rel_reduction_mean: rel_mean,
        rel_ci95_lo: rlo,
        rel_ci95_hi: rhi,
        perm_p: p,
        diff_ci95_lo: Some(dlo),
        diff_ci95_hi: Some(dhi),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// H-B1 falsified: an origin concurrency above the cap in force was observed.
    MechanismFailed,
    Inconclusive,
    BenefitNotSupported,
    /// Benefit on origin load but client success rate lower than BASE by more than the tolerance.
    TradeOffFailure,
    BenefitSupported,
}

impl Outcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Outcome::MechanismFailed => "MECHANISM_FAILED",
            Outcome::Inconclusive => "INCONCLUSIVE",
            Outcome::BenefitNotSupported => "BENEFIT_NOT_SUPPORTED",
            Outcome::TradeOffFailure => "TRADE_OFF_FAILURE",
            Outcome::BenefitSupported => "BENEFIT_SUPPORTED",
        }
    }
}

/// Inputs of the mechanical rule for one capped arm versus BASE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArmEvidence {
    pub n_valid_pairs: usize,
    pub contention_flagged: bool,
    /// max origin concurrency <= cap in force in EVERY run of the arm.
    pub mechanism_pass: bool,
    pub rel_reduction_mean: Option<f64>,
    pub rel_ci_lo: Option<f64>,
    pub rel_ci_hi: Option<f64>,
    /// Mean paired (treat - base) of origin p99 service time (ms); positive = worse.
    pub origin_p99_diff_mean: Option<f64>,
    /// Mean paired (treat - base) of client success rate, in percentage points; negative = lower.
    pub success_diff_pp_mean: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub outcome: Outcome,
    pub reasons: Vec<String>,
}

/// The fixed rule (contract section 7), applied mechanically. Order of precedence:
/// 1. mechanism failure (a hard-invariant violation is never masked by "inconclusive");
/// 2. inconclusive: < 10 valid pairs, host contention flagged, missing endpoint, or the CI straddles both
///    zero and the 5% threshold... (see below);
/// 3. benefit not supported: point estimate < 5% or CI includes zero;
/// 4. trade-off failure: success rate more than 1 pp below BASE;
/// 5. origin p99 worse -> benefit not supported;
/// 6. otherwise benefit supported.
///
/// Interpretation of "CI cannot exclude a 5% effect": when the CI includes zero AND its upper bound is at or
/// above 5%, the data can neither show an effect nor exclude a >= 5% one: INCONCLUSIVE. When the CI includes
/// zero and its upper bound is below 5%, a >= 5% effect is excluded: BENEFIT_NOT_SUPPORTED.
pub fn evaluate_arm(e: &ArmEvidence) -> Verdict {
    let mut reasons = Vec::new();
    if !e.mechanism_pass {
        reasons.push("origin concurrency exceeded the cap in force in at least one run (H-B1 falsified)".to_string());
        return Verdict { outcome: Outcome::MechanismFailed, reasons };
    }
    if e.n_valid_pairs < MIN_VALID_RUNS {
        reasons.push(format!("only {} valid paired runs; {} required", e.n_valid_pairs, MIN_VALID_RUNS));
        return Verdict { outcome: Outcome::Inconclusive, reasons };
    }
    if e.contention_flagged {
        reasons.push("host contention flagged in at least one run".to_string());
        return Verdict { outcome: Outcome::Inconclusive, reasons };
    }
    let (mean, lo, hi) = match (e.rel_reduction_mean, e.rel_ci_lo, e.rel_ci_hi) {
        (Some(m), Some(l), Some(h)) => (m, l, h),
        _ => {
            reasons.push("paired reduction of peak origin concurrency unavailable".to_string());
            return Verdict { outcome: Outcome::Inconclusive, reasons };
        }
    };
    let ci_includes_zero = lo <= 0.0 && hi >= 0.0;
    if ci_includes_zero && hi >= BENEFIT_THRESHOLD {
        reasons.push(format!(
            "95% CI [{lo:.4}, {hi:.4}] includes zero and cannot exclude a {:.0}% reduction",
            BENEFIT_THRESHOLD * 100.0
        ));
        return Verdict { outcome: Outcome::Inconclusive, reasons };
    }
    if mean < BENEFIT_THRESHOLD || ci_includes_zero || lo <= 0.0 {
        reasons.push(format!(
            "peak-concurrency reduction {mean:.4} (95% CI [{lo:.4}, {hi:.4}]) does not meet >= {:.0}% with CI excluding zero",
            BENEFIT_THRESHOLD * 100.0
        ));
        return Verdict { outcome: Outcome::BenefitNotSupported, reasons };
    }
    let succ = e.success_diff_pp_mean;
    let p99 = e.origin_p99_diff_mean;
    if let Some(s) = succ {
        if s < -SUCCESS_TOLERANCE_PP {
            reasons.push(format!(
                "peak reduction {mean:.4} (CI [{lo:.4}, {hi:.4}]) but client success rate is {:.3} pp below BASE (limit {SUCCESS_TOLERANCE_PP} pp; shed 503s count as failures)",
                -s
            ));
            return Verdict { outcome: Outcome::TradeOffFailure, reasons };
        }
    } else {
        reasons.push("client success-rate difference unavailable".to_string());
        return Verdict { outcome: Outcome::Inconclusive, reasons };
    }
    match p99 {
        Some(d) if d > 0.0 => {
            reasons.push(format!("peak reduction {mean:.4} but origin p99 service time is worse by {d:.3} ms"));
            Verdict { outcome: Outcome::BenefitNotSupported, reasons }
        }
        Some(_) => {
            reasons.push(format!(
                "peak reduction {mean:.4} (95% CI [{lo:.4}, {hi:.4}]), origin p99 not worse, client success within {SUCCESS_TOLERANCE_PP} pp"
            ));
            Verdict { outcome: Outcome::BenefitSupported, reasons }
        }
        None => {
            reasons.push("origin p99 difference unavailable".to_string());
            Verdict { outcome: Outcome::Inconclusive, reasons }
        }
    }
}

//! Deterministic Statistical Analysis, Hypothesis Testing & Synthetic Validation (M6 / RT-1 / WP-19b).
//!
//! Features:
//! - Paired difference estimation across matched evaluation seeds.
//! - Non-parametric effect size estimation (Cliff's delta, relative median ratio).
//! - Deterministic BCa / percentile bootstrap 95% confidence intervals (B = 10,000).
//! - Holm-Bonferroni multiple testing correction.
//! - Synthetic known-effect validation (positive effect, negative effect, and null dataset).

use super::rng::StreamRng;

/// Paired summary statistics.
#[derive(Debug, Clone, PartialEq)]
pub struct PairedStats {
    pub n: usize,
    pub mean_diff: f64,
    pub median_diff: f64,
    pub rel_diff_median: f64,
    pub std_err: f64,
    pub ci_95_lower: f64,
    pub ci_95_upper: f64,
    pub cliffs_delta: f64,
    pub p_value: f64,
}

/// Computes paired differences and bootstrap 95% confidence intervals.
pub struct StatisticalEvaluator;

impl StatisticalEvaluator {
    /// Computes paired differences: diff_i = treatment_i - control_i.
    pub fn paired_difference(treatment: &[f64], control: &[f64]) -> Vec<f64> {
        assert_eq!(treatment.len(), control.len());
        treatment
            .iter()
            .zip(control.iter())
            .map(|(t, c)| t - c)
            .collect()
    }

    /// Computes median of a slice.
    pub fn median(values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len().is_multiple_of(2) {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    /// Computes non-parametric Cliff's delta between two distributions.
    pub fn cliffs_delta(treatment: &[f64], control: &[f64]) -> f64 {
        if treatment.is_empty() || control.is_empty() {
            return 0.0;
        }
        let mut more = 0i64;
        let mut less = 0i64;
        for &t in treatment {
            for &c in control {
                if t > c {
                    more += 1;
                } else if t < c {
                    less += 1;
                }
            }
        }
        (more - less) as f64 / (treatment.len() * control.len()) as f64
    }

    /// Deterministic bootstrap 95% confidence interval using SplitMix64 PRNG.
    pub fn bootstrap_ci(
        diffs: &[f64],
        resamples: usize,
        master_seed: u64,
    ) -> (f64, f64) {
        if diffs.is_empty() {
            return (0.0, 0.0);
        }
        let n = diffs.len();
        let mut rng = StreamRng::new(master_seed);
        let mut sample_means = Vec::with_capacity(resamples);

        for _ in 0..resamples {
            let mut sum = 0.0;
            for _ in 0..n {
                let idx = rng.next_range_u64(0, (n - 1) as u64) as usize;
                sum += diffs[idx];
            }
            sample_means.push(sum / n as f64);
        }

        sample_means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let lower_idx = ((resamples as f64) * 0.025).round() as usize;
        let upper_idx = ((resamples as f64) * 0.975).round() as usize;

        let lower = sample_means[lower_idx.min(resamples - 1)];
        let upper = sample_means[upper_idx.min(resamples - 1)];
        (lower, upper)
    }

    /// Full paired statistical evaluation.
    pub fn evaluate_paired(
        treatment: &[f64],
        control: &[f64],
        seed: u64,
    ) -> PairedStats {
        let n = treatment.len();
        assert!(n >= 5, "Minimum 5 samples required for statistical evaluation");
        let diffs = Self::paired_difference(treatment, control);

        let sum: f64 = diffs.iter().sum();
        let mean_diff = sum / n as f64;
        let median_diff = Self::median(&diffs);

        let med_ctl = Self::median(control).max(1e-9);
        let rel_diff_median = median_diff / med_ctl;

        // Variance & SE
        let variance = diffs
            .iter()
            .map(|&d| (d - mean_diff).powi(2))
            .sum::<f64>()
            / (n - 1) as f64;
        let std_err = (variance / n as f64).sqrt();

        // 10,000 resample bootstrap CI
        let (ci_95_lower, ci_95_upper) = Self::bootstrap_ci(&diffs, 10_000, seed);

        // Cliff's delta
        let delta = Self::cliffs_delta(treatment, control);

        // Approximate two-sided paired t-test p-value via normal approximation
        let z = if std_err > 1e-12 {
            mean_diff.abs() / std_err
        } else {
            0.0
        };
        // Erg-based complementary error function approximation
        let p_value = 2.0 * (1.0 - Self::phi(z));

        PairedStats {
            n,
            mean_diff,
            median_diff,
            rel_diff_median,
            std_err,
            ci_95_lower,
            ci_95_upper,
            cliffs_delta: delta,
            p_value: p_value.clamp(0.0, 1.0),
        }
    }

    /// Standard normal CDF approximation (Abramowitz & Stegun 26.2.17).
    fn phi(x: f64) -> f64 {
        if x <= 0.0 {
            return 0.5;
        }
        let t = 1.0 / (1.0 + 0.2316419 * x);
        let poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
        let pdf = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
        (1.0 - pdf * poly).clamp(0.0, 1.0)
    }
}

/// Holm-Bonferroni Family-Wise Error Rate (FWER) controller.
pub struct HolmBonferroni;

impl HolmBonferroni {
    /// Corrects an array of p-values using Holm-Bonferroni step-down.
    /// Returns vector of (original_idx, is_significant_at_alpha).
    pub fn correct(p_values: &[f64], alpha: f64) -> Vec<bool> {
        let m = p_values.len();
        let mut indexed: Vec<(usize, f64)> = p_values.iter().cloned().enumerate().collect();
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut significant = vec![false; m];
        for k in 0..m {
            let threshold = alpha / (m - k) as f64;
            if indexed[k].1 <= threshold {
                significant[indexed[k].0] = true;
            } else {
                // Step-down: once one fails, all subsequent hypotheses fail rejection
                break;
            }
        }
        significant
    }
}

/// Synthetic dataset generator for pre-data analysis verification.
pub struct SyntheticAnalysisValidator;

impl SyntheticAnalysisValidator {
    /// Generates a planted effect dataset: treatment has an effect of `delta` relative to baseline.
    pub fn generate_planted(
        n: usize,
        base_val: f64,
        delta: f64,
        noise_sd: f64,
        seed: u64,
    ) -> (Vec<f64>, Vec<f64>) {
        let mut rng = StreamRng::new(seed);
        let mut baseline = Vec::with_capacity(n);
        let mut treatment = Vec::with_capacity(n);

        for _ in 0..n {
            // Box-Muller transform for Gaussian noise
            let u1 = rng.next_f64().max(1e-12);
            let u2 = rng.next_f64();
            let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            let z1 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).sin();

            let base = base_val + z0 * noise_sd;
            let treat = base * (1.0 + delta) + z1 * (noise_sd * 0.5);

            baseline.push(base.max(0.0));
            treatment.push(treat.max(0.0));
        }

        (treatment, baseline)
    }

    /// Verifies analysis dry-run on synthetic data:
    /// - Planted +8% effect must be detected with positive CI.
    /// - Planted -8% penalty must be detected with negative CI.
    /// - Null dataset (delta = 0) must show CI spanning zero (no false positive).
    pub fn verify_analysis_pipeline() -> Result<(), String> {
        let seed = 42_424_242;

        // 1. Positive effect (+8%)
        let (pos_treat, pos_base) = Self::generate_planted(30, 100.0, 0.08, 1.0, seed);
        let pos_stats = StatisticalEvaluator::evaluate_paired(&pos_treat, &pos_base, seed);
        if pos_stats.ci_95_lower <= 0.0 {
            return Err(format!(
                "Synthetic Dry-Run Failure: Planted +8% effect not detected (CI lower = {:.4})",
                pos_stats.ci_95_lower
            ));
        }

        // 2. Negative effect (-8%)
        let (neg_treat, neg_base) = Self::generate_planted(30, 100.0, -0.08, 1.0, seed + 1);
        let neg_stats = StatisticalEvaluator::evaluate_paired(&neg_treat, &neg_base, seed + 1);
        if neg_stats.ci_95_upper >= 0.0 {
            return Err(format!(
                "Synthetic Dry-Run Failure: Planted -8% penalty not detected (CI upper = {:.4})",
                neg_stats.ci_95_upper
            ));
        }

        // 3. Null dataset (0% effect) -> false-positive protection
        let (null_treat, null_base) = Self::generate_planted(30, 100.0, 0.0, 1.0, seed + 2);
        let null_stats = StatisticalEvaluator::evaluate_paired(&null_treat, &null_base, seed + 2);
        if !(null_stats.ci_95_lower <= 0.0 && null_stats.ci_95_upper >= 0.0) {
            return Err(format!(
                "Synthetic Dry-Run Failure: Null dataset triggered false positive (CI = [{:.4}, {:.4}])",
                null_stats.ci_95_lower, null_stats.ci_95_upper
            ));
        }

        // 4. Holm-Bonferroni correction check
        let p_vals = vec![0.001, 0.01, 0.04, 0.06];
        let sig = HolmBonferroni::correct(&p_vals, 0.05);
        if !sig[0] || !sig[1] {
            return Err("Holm-Bonferroni failure on significant p-values".to_string());
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// M6 protocol v2 analysis: BCa bootstrap, sign-flip permutation test, Holm adjustment,
// consuming per-seed paired values extracted from RAW cell data (see eval.rs).
// ---------------------------------------------------------------------------

/// Inverse standard normal CDF (Acklam's rational approximation, relative error < 1.2e-9).
pub fn inv_norm_cdf(p: f64) -> f64 {
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    const A: [f64; 6] = [-3.969683028665376e+01, 2.209460984245205e+02, -2.759285104469687e+02, 1.38357751867269e+02, -3.066479806614716e+01, 2.506628277459239e+00];
    const B: [f64; 5] = [-5.447609879822406e+01, 1.615858368580409e+02, -1.556989798598866e+02, 6.680131188771972e+01, -1.328068155288572e+01];
    const C: [f64; 6] = [-7.784894002430293e-03, -3.223964580411365e-01, -2.400758277161838e+00, -2.549732539343734e+00, 4.374664141464968e+00, 2.938163982698783e+00];
    const D: [f64; 4] = [7.784695709041462e-03, 3.224671290700398e-01, 2.445134137142996e+00, 3.754408661907416e+00];
    let plow = 0.02425;
    if p < plow {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= 1.0 - plow {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    }
}

/// Standard normal CDF (via the same A&S approximation, symmetric).
pub fn norm_cdf(x: f64) -> f64 {
    if x >= 0.0 {
        StatisticalEvaluator::phi_pub(x)
    } else {
        1.0 - StatisticalEvaluator::phi_pub(-x)
    }
}

impl StatisticalEvaluator {
    pub fn phi_pub(x: f64) -> f64 {
        if x <= 0.0 {
            return 0.5;
        }
        Self::phi(x)
    }

    /// 95% BCa (bias-corrected and accelerated) bootstrap interval of the MEAN of `data`.
    /// Deterministic given `seed`. Degenerate (constant) data returns (mean, mean).
    pub fn bca_ci_mean(data: &[f64], resamples: usize, seed: u64) -> (f64, f64) {
        let n = data.len();
        if n == 0 {
            return (0.0, 0.0);
        }
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let theta = mean(data);
        if data.iter().all(|&x| (x - data[0]).abs() < 1e-15) {
            return (theta, theta);
        }
        let mut rng = StreamRng::new(seed);
        let mut boots = Vec::with_capacity(resamples);
        for _ in 0..resamples {
            let mut s = 0.0;
            for _ in 0..n {
                s += data[rng.next_range_u64(0, (n - 1) as u64) as usize];
            }
            boots.push(s / n as f64);
        }
        boots.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let below = boots.iter().filter(|&&b| b < theta).count() as f64;
        let z0 = inv_norm_cdf((below + 0.5) / (resamples as f64 + 1.0));
        // Jackknife acceleration
        let total: f64 = data.iter().sum();
        let jack: Vec<f64> = data.iter().map(|&x| (total - x) / (n as f64 - 1.0)).collect();
        let jm = mean(&jack);
        let num: f64 = jack.iter().map(|&j| (jm - j).powi(3)).sum();
        let den: f64 = jack.iter().map(|&j| (jm - j).powi(2)).sum::<f64>().powf(1.5);
        let a = if den > 0.0 { num / (6.0 * den) } else { 0.0 };
        let adj = |alpha: f64| -> f64 {
            let za = inv_norm_cdf(alpha);
            let t = z0 + za;
            norm_cdf(z0 + t / (1.0 - a * t))
        };
        let pick = |q: f64| -> f64 {
            let idx = (q * resamples as f64).floor() as usize;
            boots[idx.min(resamples - 1)]
        };
        (pick(adj(0.025)), pick(adj(0.975)))
    }

    /// Two-sided paired sign-flip permutation test on the mean of `diffs`.
    /// Exact for n <= 16 (2^n enumeration), otherwise 20,000 deterministic random sign flips.
    pub fn sign_flip_p_value(diffs: &[f64], seed: u64) -> f64 {
        let n = diffs.len();
        if n == 0 || diffs.iter().all(|&d| d == 0.0) {
            return 1.0;
        }
        let obs = diffs.iter().sum::<f64>().abs();
        let eps = 1e-12 * (1.0 + obs);
        if n <= 16 {
            let mut ge = 0u64;
            let total = 1u64 << n;
            for mask in 0..total {
                let mut s = 0.0;
                for (i, d) in diffs.iter().enumerate() {
                    s += if (mask >> i) & 1 == 1 { -d } else { *d };
                }
                if s.abs() >= obs - eps {
                    ge += 1;
                }
            }
            ge as f64 / total as f64
        } else {
            let mut rng = StreamRng::new(seed);
            let iters = 20_000u64;
            let mut ge = 0u64;
            for _ in 0..iters {
                let mut s = 0.0;
                for d in diffs {
                    s += if rng.next_u64() & 1 == 1 { -d } else { *d };
                }
                if s.abs() >= obs - eps {
                    ge += 1;
                }
            }
            (ge as f64 + 1.0) / (iters as f64 + 1.0)
        }
    }
}

impl HolmBonferroni {
    /// Holm-adjusted p-values (step-down, monotone), same order as input.
    pub fn adjust(p_values: &[f64]) -> Vec<f64> {
        let m = p_values.len();
        let mut idx: Vec<usize> = (0..m).collect();
        idx.sort_by(|&a, &b| p_values[a].partial_cmp(&p_values[b]).unwrap_or(std::cmp::Ordering::Equal));
        let mut adj = vec![0.0; m];
        let mut running = 0.0f64;
        for (k, &i) in idx.iter().enumerate() {
            let v = ((m - k) as f64 * p_values[i]).min(1.0);
            running = running.max(v);
            adj[i] = running;
        }
        adj
    }
}

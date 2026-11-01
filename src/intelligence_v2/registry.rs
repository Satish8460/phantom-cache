//! Static, versioned policy registry (design section 4). Compiled in; no runtime loading.

use super::{parse_mode, ConfigError, Mode};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lifecycle {
    Proposed = 0,
    Implemented = 1,
    UnitTested = 2,
    Integrated = 3,
    ShadowEvaluated = 4,
    Retained = 5,
    Narrowed = 6,
    Rejected = 7,
}

impl Lifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            Lifecycle::Proposed => "Proposed",
            Lifecycle::Implemented => "Implemented",
            Lifecycle::UnitTested => "UnitTested",
            Lifecycle::Integrated => "Integrated",
            Lifecycle::ShadowEvaluated => "ShadowEvaluated",
            Lifecycle::Retained => "Retained",
            Lifecycle::Narrowed => "Narrowed",
            Lifecycle::Rejected => "Rejected",
        }
    }
    fn rank(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParamSpec {
    pub name: &'static str,
    pub default: f64,
    pub min: f64,
    pub max: f64,
    pub unit: &'static str,
}

#[derive(Debug, Clone)]
pub struct PolicySpec {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: &'static str,
    pub implemented: bool,
    pub objective: &'static str,
    pub trigger: &'static str,
    /// Feature ids; the minimum sample count is `PieConfig::min_feature_samples`.
    pub required_inputs: &'static [&'static str],
    pub parameters: &'static [ParamSpec],
    pub safety_constraints: &'static [&'static str],
    pub expected_effect: &'static str,
    pub evaluation_method: &'static str,
    pub limitations: &'static str,
    pub version: &'static str,
    pub lifecycle: Lifecycle,
    /// Declared as text so that validation of a forbidden value ("canary", "active") is testable.
    pub max_runtime_state: &'static str,
}

const SAFETY: &[&str] = &[
    "staleness cap PHANTOM_MAX_STALE_MS is not read or written",
    "origin concurrency cap is not read or written",
    "memory budget of the cache is not read or written",
    "no write path to generations, epochs or invalidation",
    "output is advisory text only, never applied",
];

pub const P1_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "p1.hot_top1_share",
        default: 0.20,
        min: 0.05,
        max: 0.95,
        unit: "share",
    },
    ParamSpec {
        name: "p1.scan_one_hit_ratio",
        default: 0.60,
        min: 0.30,
        max: 0.99,
        unit: "share",
    },
    ParamSpec {
        name: "p1.zipf_min_alpha",
        default: 0.60,
        min: 0.20,
        max: 2.00,
        unit: "alpha",
    },
    ParamSpec {
        name: "p1.phase_jaccard_max",
        default: 0.30,
        min: 0.00,
        max: 0.90,
        unit: "jaccard",
    },
    ParamSpec {
        name: "p1.heavy_share_min",
        default: 0.02,
        min: 0.005,
        max: 0.50,
        unit: "share",
    },
    ParamSpec {
        name: "p1.burst_ratio",
        default: 3.0,
        min: 1.5,
        max: 20.0,
        unit: "x_baseline",
    },
];
pub const P2_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "p2.delta_min",
        default: 0.02,
        min: 0.0,
        max: 0.5,
        unit: "hit_ratio_abs",
    },
    ParamSpec {
        name: "p2.min_sampled",
        default: 300.0,
        min: 100.0,
        max: 1.0e6,
        unit: "sampled_requests",
    },
];
pub const P3_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "p3.min_sampled",
        default: 300.0,
        min: 100.0,
        max: 1.0e6,
        unit: "sampled_requests",
    },
    ParamSpec {
        name: "p3.one_hit_ratio_min",
        default: 0.50,
        min: 0.20,
        max: 0.99,
        unit: "share",
    },
    ParamSpec {
        name: "p3.delta_min",
        default: 0.02,
        min: 0.0,
        max: 0.5,
        unit: "hit_ratio_abs",
    },
    ParamSpec { name: "p3.one_hit_bytes_min", default: 0.3, min: 0.0, max: 0.99, unit: "share" },
    ParamSpec { name: "p3.origin_cost_warn_us", default: 200000.0, min: 1000.0, max: 60000000.0, unit: "us" },
];
pub const P5_PARAMS: &[ParamSpec] = &[ParamSpec {
    name: "p5.waiters_per_fetch_on",
    default: 8.0,
    min: 2.0,
    max: 1000.0,
    unit: "waiters_per_fetch",
}];
pub const P6_PARAMS: &[ParamSpec] = &[
    ParamSpec {
        name: "p6.p99_ms_on",
        default: 500.0,
        min: 10.0,
        max: 60000.0,
        unit: "ms",
    },
    ParamSpec {
        name: "p6.error_rate_on",
        default: 0.05,
        min: 0.001,
        max: 1.0,
        unit: "rate",
    },
];
pub const H_PARAMS: &[ParamSpec] = &[ParamSpec { name: "h.hot_share", default: 0.02, min: 0.005, max: 0.5, unit: "share" }];
pub const C_PARAMS: &[ParamSpec] = &[ParamSpec { name: "c.high_ratio", default: 0.5, min: 0.05, max: 0.99, unit: "ratio" }];
pub const PR_PARAMS: &[ParamSpec] = &[ParamSpec { name: "pr.min_windows", default: 5.0, min: 2.0, max: 100.0, unit: "windows" }];
pub const FC_PARAMS: &[ParamSpec] = &[
    ParamSpec { name: "fc.rate_slack", default: 0.5, min: 0.0, max: 5.0, unit: "x_baseline" },
    ParamSpec { name: "fc.rate_h", default: 1.5, min: 0.5, max: 50.0, unit: "cusum" },
    ParamSpec { name: "fc.conc_slack", default: 0.05, min: 0.0, max: 0.5, unit: "share" },
    ParamSpec { name: "fc.conc_h", default: 0.3, min: 0.05, max: 5.0, unit: "cusum" },
];

pub fn registry() -> Vec<PolicySpec> {
    vec![
        PolicySpec {
            id: "POL-CLASS-1",
            name: "Workload classification (P1)",
            kind: "WorkloadClassification",
            implemented: true,
            objective: "Label the traffic window (hot-key dominated, scan/cold, zipf-like, bursty, phase change, uniform-like). Judged on agreement with the generating label of seeded Lab workloads.",
            trigger: "every decision tick in advisory mode",
            required_inputs: &["F2", "F3"],
            parameters: P1_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "Descriptive only; applies nothing. Used as context for other advice.",
            evaluation_method: "Seeded Lab workloads, disagreement rate against the generating label (pre-registration required before any claim).",
            limitations: "One 1-second-class window of sampled keys; thresholds chosen on development seeds, not pre-registered; cannot see key sizes or costs.",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "advisory",
        },
        PolicySpec {
            id: "POL-EVICT-1",
            name: "Eviction shadow selection (P2)",
            kind: "Eviction",
            implemented: true,
            objective: "Recommend the eviction policy with the highest hit ratio on a spatially sampled key stream when the 95% interval of the gain over the incumbent excludes the minimum margin.",
            trigger: "enough sampled requests in the shadow caches (p2.min_sampled)",
            required_inputs: &["F2"],
            parameters: P2_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "A testable statement: the candidate would have had a higher hit ratio on the sampled stream. No number is promised for the serving path.",
            evaluation_method: "Held-out Lab traces with a tuned static baseline (pre-registration required).",
            limitations: "Objective is hit ratio on sampled keys, not origin-weighted miss cost; assumes the declared cache_capacity_entries; the interval treats the two shadow ratios as independent (conservative); S3-FIFO shadow uses a deterministic ghost FIFO (the frozen shadow evicts an arbitrary ghost key).",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "advisory",
        },
        PolicySpec {
            id: "POL-ADMIT-1",
            name: "Frequency-aware admission advice (P3)",
            kind: "Admission",
            implemented: true,
            objective: "Recommend a TinyLFU-style admission filter when one-hit wonders dominate and the shadow shows a gain over the incumbent.",
            trigger: "one-hit-wonder ratio above p3.one_hit_ratio_min",
            required_inputs: &["F17"],
            parameters: P3_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "Fewer one-hit insertions displacing reused keys, on the sampled stream only.",
            evaluation_method: "Scan-heavy Lab workloads against the incumbent policy (pre-registration required).",
            limitations: "One-hit ratio is measured inside one window; the shadow TinyLFU is the frozen miniature (4x256 sketch, no aging).",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "advisory",
        },
        PolicySpec {
            id: "POL-TTL-1",
            name: "TTL and freshness advice (P4)",
            kind: "RefreshAdvice",
            implemented: false,
            objective: "Not implemented in this batch.",
            trigger: "n/a",
            required_inputs: &[],
            parameters: &[],
            safety_constraints: SAFETY,
            expected_effect: "n/a",
            evaluation_method: "n/a",
            limitations: "Listed for completeness only; cannot run.",
            version: "0.0.0",
            lifecycle: Lifecycle::Proposed,
            max_runtime_state: "off",
        },
        PolicySpec {
            id: "POL-STAMPEDE-1",
            name: "Stampede-risk warning (P5)",
            kind: "StampedeWarning",
            implemented: true,
            objective: "Warn when coalesced waiters per origin fetch stay above a declared level (coalescing is carrying heavy pressure).",
            trigger: "F13 at or above p5.waiters_per_fetch_on (clears below 80% of it)",
            required_inputs: &["F13"],
            parameters: P5_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "Advisory text only.",
            evaluation_method: "Replayed stampede scenarios; false-warning rate bound (pre-registration required).",
            limitations: "High waiters per fetch also means coalescing works; the warning is a pressure indicator, not a failure detector. Concurrent fetches per key are not visible.",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "advisory",
        },
        PolicySpec {
            id: "POL-ORIGIN-1",
            name: "Origin-saturation warning (P6)",
            kind: "OriginSaturationWarning",
            implemented: true,
            objective: "Warn when origin p99 latency or error rate cross declared levels. The origin cap is not changed.",
            trigger: "F10 or F10b at or above threshold (clears below 80%)",
            required_inputs: &["F10", "F10b"],
            parameters: P6_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "Advisory text only.",
            evaluation_method: "Replayed brownout scenarios; must precede observed degradation (pre-registration required).",
            limitations: "p99 comes from a log-bucket histogram (upper bound, within about 25%); the origin concurrency gate snapshot is not read.",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "advisory",
        },
        PolicySpec {
            id: "POL-DIST-1",
            name: "Distributed advice (P7)",
            kind: "DistributedImbalanceAdvice",
            implemented: false,
            objective: "Not implemented: needs several nodes' counters (F15).",
            trigger: "n/a",
            required_inputs: &[],
            parameters: &[],
            safety_constraints: SAFETY,
            expected_effect: "n/a",
            evaluation_method: "n/a",
            limitations: "A single gateway never estimates another node's load.",
            version: "0.0.0",
            lifecycle: Lifecycle::Proposed,
            max_runtime_state: "off",
        },
        PolicySpec {
            id: "POL-HOT-1",
            name: "Hot-key detection (Space-Saving)",
            kind: "HotKeyDetection",
            implemented: true,
            objective: "Report the heaviest keys of each window (hash only) with count bounds, share and confidence; flag hot above a declared share.",
            trigger: "every window with enough samples",
            required_inputs: &["F3", "F30"],
            parameters: H_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "Observation only; no action.",
            evaluation_method: "Seeded Lab workloads with known heavy keys (pre-registration required before any claim).",
            limitations: "64 counters per window: keys below about 1/64 of the window may be missed; counts are Space-Saving bounds, confidence is lower/upper bound, not a probability.",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "observe",
        },
        PolicySpec {
            id: "POL-FREQ-1",
            name: "Frequency observation and one-hit-wonder detection",
            kind: "FrequencyObservation",
            implemented: true,
            objective: "Summarise how often keys repeat in a window (once, twice, 3+; share of requests in the hottest 1% and 10% of distinct keys).",
            trigger: "every window with at least 30 sampled distinct keys",
            required_inputs: &["F17", "F18"],
            parameters: &[],
            safety_constraints: SAFETY,
            expected_effect: "Observation only; no action.",
            evaluation_method: "Seeded Lab workloads with known frequency structure.",
            limitations: "Computed on a 1/16 spatial sample inside one window; keys that repeat only across windows look like one-hit wonders.",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "observe",
        },
        PolicySpec {
            id: "POL-CARD-1",
            name: "High-cardinality estimation (HyperLogLog p=10)",
            kind: "CardinalityEstimation",
            implemented: true,
            objective: "Estimate distinct keys per window (standard error about 3.25 percent), distinct/requests ratio, growth versus the previous window, and flag high cardinality.",
            trigger: "every window with enough samples",
            required_inputs: &["F23", "F24"],
            parameters: C_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "Observation only; no action.",
            evaluation_method: "Seeded Lab workloads with known distinct counts.",
            limitations: "1 KiB of registers; relative error about 3.25 percent (1 sigma), larger for tiny counts; one window at a time.",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "observe",
        },
        PolicySpec {
            id: "POL-PRED-1",
            name: "Hot-key prediction (rules plus EWMA statistics, not machine learning)",
            kind: "HotKeyPrediction",
            implemented: true,
            objective: "Predict which keys will be above the hot share in the next window from an EWMA of their share and its trend; score every prediction against the next window and against a persistence baseline.",
            trigger: "every window with enough samples",
            required_inputs: &["F30"],
            parameters: PR_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "Observation only; the scorecard says whether the predictor beats persistence.",
            evaluation_method: "Online scorecard (precision, recall, windows scored) against the next window's actual hot set; no training.",
            limitations: "Statistical rules only; no claim of usefulness until the scorecard shows it beats the persistence baseline on held-out traces.",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "observe",
        },
        PolicySpec {
            id: "POL-FLASH-1",
            name: "Flash-crowd detection with advisory text",
            kind: "FlashCrowdDetection",
            implemented: true,
            objective: "Detect a sudden rise in request rate or in the concentration of the hottest keys (CUSUM change points) and write an advisory record with evidence and a heuristic confidence.",
            trigger: "rate CUSUM or concentration CUSUM above its declared threshold",
            required_inputs: &["F3b", "F6"],
            parameters: FC_PARAMS,
            safety_constraints: SAFETY,
            expected_effect: "Advisory text only (for example consider pre-warming hot keys); nothing is applied.",
            evaluation_method: "Seeded flash-crowd Lab workloads and stationary controls (pre-registration required).",
            limitations: "Confidence is a rule score (0.6 one signal, 0.9 both), not a calibrated probability; the rate baseline needs 2 earlier windows.",
            version: "1.0.0",
            lifecycle: Lifecycle::UnitTested,
            max_runtime_state: "advisory",
        },
    ]
}

/// Registry validation (design section 4 rules). Returns the first violation.
pub fn validate(specs: &[PolicySpec]) -> Result<(), ConfigError> {
    let mut seen = BTreeMap::new();
    for s in specs {
        if seen.insert(s.id, ()).is_some() {
            return Err(ConfigError::Registry(format!(
                "duplicate policy id {}",
                s.id
            )));
        }
        let max = parse_mode(s.max_runtime_state)
            .map_err(|e| ConfigError::Registry(format!("{}: {}", s.id, e)))?;
        if max > Mode::Advisory {
            return Err(ConfigError::Registry(format!(
                "{}: state above advisory",
                s.id
            )));
        }
        if max == Mode::Advisory && s.lifecycle.rank() < Lifecycle::UnitTested.rank() {
            return Err(ConfigError::Registry(format!(
                "{}: advisory requires lifecycle >= UnitTested",
                s.id
            )));
        }
        if max != Mode::Off && s.lifecycle == Lifecycle::Rejected {
            return Err(ConfigError::Registry(format!(
                "{}: rejected policy cannot run",
                s.id
            )));
        }
        if !s.implemented && max != Mode::Off {
            return Err(ConfigError::Registry(format!(
                "{}: not implemented but may run",
                s.id
            )));
        }
        if s.safety_constraints.is_empty() {
            return Err(ConfigError::Registry(format!(
                "{}: no safety constraints declared",
                s.id
            )));
        }
        for p in s.parameters {
            if !(p.min.is_finite() && p.max.is_finite() && p.default.is_finite())
                || p.min > p.default
                || p.default > p.max
            {
                return Err(ConfigError::Registry(format!(
                    "{}: parameter {} has invalid bounds",
                    s.id, p.name
                )));
            }
        }
    }
    Ok(())
}

/// Effective parameters: defaults plus validated overrides.
#[derive(Debug, Clone)]
pub struct Params(BTreeMap<&'static str, f64>);

impl Params {
    pub fn from_overrides(overrides: &BTreeMap<String, f64>) -> Result<Params, ConfigError> {
        let specs = registry();
        let mut all: BTreeMap<&'static str, ParamSpec> = BTreeMap::new();
        for s in &specs {
            for p in s.parameters {
                all.insert(p.name, *p);
            }
        }
        for (k, v) in overrides {
            let Some(p) = all.get(k.as_str()) else {
                return Err(ConfigError::UnknownParameter(k.clone()));
            };
            if !v.is_finite() {
                return Err(ConfigError::NotFinite(k.clone()));
            }
            if *v < p.min || *v > p.max {
                return Err(ConfigError::OutOfBounds {
                    name: k.clone(),
                    value: *v,
                    min: p.min,
                    max: p.max,
                });
            }
        }
        let mut m = BTreeMap::new();
        for (name, p) in &all {
            let v = overrides.get(*name).copied().unwrap_or(p.default);
            m.insert(*name, v);
        }
        Ok(Params(m))
    }
    pub fn get(&self, name: &str) -> f64 {
        self.0.get(name).copied().unwrap_or(f64::NAN)
    }
}

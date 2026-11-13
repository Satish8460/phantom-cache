//! Shadow policies P1, P2, P3, P5, P6. Pure functions of a window plus shadow-cache reports; they change nothing.

use super::arbiter::{CandidateEval, Effect, InputUsed, Recommendation, Verdict};
use super::registry::Params;
use super::signals::{WindowSnapshot, SAMPLE_MODULUS};
use super::{mix64, FVal, Src};
use crate::intelligence::shadow::{
    LruShadow, ShadowEvaluationReport, ShadowEvaluator, ShadowPolicy, SieveShadow, TinyLfuShadow,
};
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};

pub const P1: &str = "POL-CLASS-1";
pub const P2: &str = "POL-EVICT-1";
pub const P3: &str = "POL-ADMIT-1";
pub const P5: &str = "POL-STAMPEDE-1";
pub const P6: &str = "POL-ORIGIN-1";
pub const POLICY_VERSION: &str = "1.0.0";
pub const ADVISORY_LABEL: &str = "advisory, not applied";

// ---------------------------------------------------------------------------------------------
// Deterministic S3-FIFO shadow. The frozen `S3FifoShadow` evicts an arbitrary ghost key via HashMap
// iteration order, which differs per process; this copy keeps the same algorithm with a FIFO ghost list.
// ---------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct DetS3Fifo {
    small_cap: usize,
    main_cap: usize,
    small_q: VecDeque<u64>,
    main_q: VecDeque<u64>,
    ghost_q: VecDeque<u64>,
    ghost_set: HashSet<u64>,
    freq: HashMap<u64, u8>,
    hits: u64,
    misses: u64,
}

impl DetS3Fifo {
    pub fn new(total_capacity: usize) -> Self {
        let cap = total_capacity.max(10);
        let small = (cap / 10).max(2);
        Self {
            small_cap: small,
            main_cap: cap - small,
            small_q: VecDeque::new(),
            main_q: VecDeque::new(),
            ghost_q: VecDeque::new(),
            ghost_set: HashSet::new(),
            freq: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }
    fn evict_main(&mut self) {
        while let Some(key) = self.main_q.pop_front() {
            let f = self.freq.get(&key).copied().unwrap_or(0);
            if f > 0 {
                self.freq.insert(key, f - 1);
                self.main_q.push_back(key);
            } else {
                self.freq.remove(&key);
                break;
            }
        }
    }
    fn evict(&mut self) {
        if self.small_q.len() >= self.small_cap {
            if let Some(key) = self.small_q.pop_front() {
                let f = self.freq.get(&key).copied().unwrap_or(0);
                if f > 0 {
                    if self.main_q.len() >= self.main_cap {
                        self.evict_main();
                    }
                    self.main_q.push_back(key);
                } else {
                    if self.ghost_q.len() >= self.small_cap + self.main_cap {
                        if let Some(old) = self.ghost_q.pop_front() {
                            self.ghost_set.remove(&old);
                        }
                    }
                    self.ghost_q.push_back(key);
                    self.ghost_set.insert(key);
                    self.freq.remove(&key);
                }
            }
        } else if self.main_q.len() >= self.main_cap {
            self.evict_main();
        }
    }
}

impl ShadowPolicy for DetS3Fifo {
    fn name(&self) -> &'static str {
        "S3-FIFO"
    }
    fn access(&mut self, key: u64) -> bool {
        if self.small_q.contains(&key) || self.main_q.contains(&key) {
            let c = self.freq.entry(key).or_insert(0);
            *c = (*c + 1).min(3);
            self.hits += 1;
            true
        } else {
            self.misses += 1;
            let in_ghost = self.ghost_set.remove(&key);
            if in_ghost {
                if let Some(p) = self.ghost_q.iter().position(|k| *k == key) {
                    self.ghost_q.remove(p);
                }
            }
            self.evict();
            if in_ghost {
                if self.main_q.len() >= self.main_cap {
                    self.evict_main();
                }
                self.main_q.push_back(key);
            } else {
                self.small_q.push_back(key);
            }
            self.freq.insert(key, 0);
            false
        }
    }
    fn hits(&self) -> u64 {
        self.hits
    }
    fn misses(&self) -> u64 {
        self.misses
    }
    fn reset(&mut self) {
        self.small_q.clear();
        self.main_q.clear();
        self.ghost_q.clear();
        self.ghost_set.clear();
        self.freq.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Incumbent {
    Lru,
    Sieve,
    S3Fifo,
}

impl Incumbent {
    pub fn as_str(self) -> &'static str {
        match self {
            Incumbent::Lru => "LRU",
            Incumbent::Sieve => "SIEVE",
            Incumbent::S3Fifo => "S3-FIFO",
        }
    }
}

fn make_policy(name: &str, cap: usize) -> Box<dyn ShadowPolicy> {
    match name {
        "LRU" => Box::new(LruShadow::new(cap)),
        "SIEVE" => Box::new(SieveShadow::new(cap)),
        "S3-FIFO" => Box::new(DetS3Fifo::new(cap)),
        _ => Box::new(TinyLfuShadow::new(cap)),
    }
}

/// Shadow caches on a spatially sampled key stream (one `ShadowEvaluator` per candidate, incumbent as baseline).
pub struct Shadows {
    pub incumbent: Incumbent,
    pub shadow_cap: usize,
    evals: Vec<(&'static str, ShadowEvaluator)>,
}

impl Shadows {
    pub fn new(incumbent: Incumbent, cache_capacity_entries: usize, modulus: u64) -> Shadows {
        let cap = (cache_capacity_entries / modulus.max(1) as usize).max(8);
        let mut evals = Vec::new();
        for c in ["LRU", "SIEVE", "S3-FIFO", "TinyLFU"] {
            if c == incumbent.as_str() {
                continue;
            }
            evals.push((
                c,
                ShadowEvaluator::new(
                    modulus,
                    make_policy(incumbent.as_str(), cap),
                    make_policy(c, cap),
                ),
            ));
        }
        Shadows {
            incumbent,
            shadow_cap: cap,
            evals,
        }
    }
    /// Feeds one request key. The key is mixed first so that `% modulus` sampling is uniform.
    pub fn feed(&mut self, key_hash: u64) {
        let m = mix64(key_hash);
        for (_, e) in self.evals.iter_mut() {
            e.observe(m);
        }
    }
    pub fn reset(&mut self) {
        for (_, e) in self.evals.iter_mut() {
            e.reset();
        }
    }
    pub fn sampled(&self) -> u64 {
        self.evals
            .first()
            .map(|(_, e)| e.generate_report(0.0).total_sampled_requests)
            .unwrap_or(0)
    }
    pub fn report(&self, name: &str, delta_min: f64) -> Option<ShadowEvaluationReport> {
        self.evals
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, e)| e.generate_report(delta_min))
    }
    pub fn eviction_candidates(&self) -> Vec<&'static str> {
        self.evals
            .iter()
            .map(|(n, _)| *n)
            .filter(|n| *n != "TinyLFU")
            .collect()
    }
    /// Upper bound of the heap used by the shadow caches (declared capacities, 64 bytes per entry per policy).
    pub fn memory_bytes_upper(
        incumbent: Incumbent,
        cache_capacity_entries: usize,
        modulus: u64,
    ) -> usize {
        let cap = (cache_capacity_entries / modulus.max(1) as usize).max(8);
        let n_eval = if incumbent == Incumbent::Lru
            || incumbent == Incumbent::Sieve
            || incumbent == Incumbent::S3Fifo
        {
            3
        } else {
            4
        };
        n_eval * 2 * (cap * 64 + 4096)
    }
}

/// Origin health reported by the gateway from its own measured metrics (optional input to P6).
#[derive(Debug, Clone, Copy)]
pub struct OriginHealthInput {
    pub p99_ms: f64,
    pub error_rate: f64,
    pub samples: u64,
    /// `None` means the caller forgot provenance: treated as poisoned input (fault).
    pub src: Option<Src>,
    pub received_ms: u64,
}

pub struct EvalCtx<'a> {
    pub win: &'a WindowSnapshot,
    pub params: &'a Params,
    pub min_samples: u64,
    pub lab_mode: bool,
    pub shadows: &'a Shadows,
    pub external: Option<&'a OriginHealthInput>,
    pub prev_active: bool,
    pub phase_recent: bool,
    pub burst_recent: bool,
    pub now_ms: u64,
    pub window_ms: u64,
}

#[derive(Debug, Clone)]
pub struct PolicyResult {
    pub policy_id: &'static str,
    pub verdict: Verdict,
    pub reason_code: String,
    pub inputs: Vec<InputUsed>,
    pub candidates: Vec<CandidateEval>,
    pub recommendation: Option<Recommendation>,
    pub rec_key: Option<String>,
    pub guardrails: Vec<String>,
    pub lever: &'static str,
    pub score: f64,
    pub label: Option<String>,
    pub warning_active: Option<bool>,
}

impl PolicyResult {
    fn new(policy_id: &'static str, lever: &'static str) -> PolicyResult {
        PolicyResult {
            policy_id,
            verdict: Verdict::NoAction,
            reason_code: String::new(),
            inputs: vec![],
            candidates: vec![],
            recommendation: None,
            rec_key: None,
            guardrails: vec![],
            lever,
            score: f64::NEG_INFINITY,
            label: None,
            warning_active: None,
        }
    }
    fn no_action(mut self, reason: impl Into<String>) -> PolicyResult {
        self.verdict = Verdict::NoAction;
        self.reason_code = reason.into();
        self
    }
}

fn min_for(id: &str, cfg_min: u64) -> u64 {
    match id {
        "F5" | "F17" => 30,
        _ => cfg_min,
    }
}

fn input_of(win: &WindowSnapshot, id: &'static str) -> InputUsed {
    match win.feat(id) {
        Some(f) => InputUsed {
            feature: id,
            value: f.value,
            src: f.src,
            samples: f.samples,
            window_ms: win.duration_ms(),
        },
        None => InputUsed {
            feature: id,
            value: None,
            src: Src::Unavailable,
            samples: 0,
            window_ms: win.duration_ms(),
        },
    }
}

type GateFail = (Vec<InputUsed>, &'static str, Vec<String>);

/// Provenance and sample gate. Err carries the reason code and guardrail events.
fn gate(
    win: &WindowSnapshot,
    ids: &[&'static str],
    cfg_min: u64,
    lab: bool,
) -> Result<Vec<InputUsed>, GateFail> {
    let mut inputs = Vec::new();
    let mut guard = Vec::new();
    let mut code: Option<(u8, &'static str)> = None; // lower number = higher priority
    let set = |prio: u8, c: &'static str, code: &mut Option<(u8, &'static str)>| {
        if code.map(|(p, _)| prio < p).unwrap_or(true) {
            *code = Some((prio, c));
        }
    };
    for id in ids {
        let inp = input_of(win, id);
        let reason = win.feat(id).and_then(|f| f.reason);
        match inp.src {
            Src::Simulated if !lab => {
                guard.push(format!("provenance_refused:{id}"));
                set(0, "provenance_refused", &mut code);
            }
            Src::Unavailable => {
                guard.push(format!("unavailable:{id}:{}", reason.unwrap_or("missing")));
                match reason {
                    Some("window_degraded") => set(1, "window_degraded", &mut code),
                    Some(r) if r.starts_with("insufficient") => {
                        set(2, "insufficient_samples", &mut code)
                    }
                    _ => set(3, "inputs_unavailable", &mut code),
                }
            }
            _ => {
                if inp.samples < min_for(id, cfg_min) {
                    guard.push(format!("insufficient_samples:{id}"));
                    set(2, "insufficient_samples", &mut code);
                }
            }
        }
        inputs.push(inp);
    }
    match code {
        Some((_, c)) => Err((inputs, c, guard)),
        None => Ok(inputs),
    }
}

fn f(win: &WindowSnapshot, id: &str) -> Option<f64> {
    win.feat(id).and_then(|x| x.value).map(|v| v.as_f64())
}

pub fn heavy_set(win: &WindowSnapshot, min_share: f64) -> Vec<u64> {
    if win.requests == 0 {
        return vec![];
    }
    let mut v: Vec<u64> = win
        .top_keys
        .iter()
        .filter(|(_, c)| (*c as f64) / (win.requests as f64) >= min_share)
        .map(|(k, _)| *k)
        .collect();
    v.sort_unstable();
    v
}

pub fn jaccard(a: &[u64], b: &[u64]) -> f64 {
    let sa: HashSet<u64> = a.iter().copied().collect();
    let sb: HashSet<u64> = b.iter().copied().collect();
    let inter = sa.intersection(&sb).count() as f64;
    let uni = sa.union(&sb).count() as f64;
    if uni == 0.0 {
        1.0
    } else {
        inter / uni
    }
}

// ---------------------------------------------------------------------------------------------
// P1 workload classification
// ---------------------------------------------------------------------------------------------
pub fn eval_p1(c: &EvalCtx) -> PolicyResult {
    let mut r = PolicyResult::new(P1, "none");
    match gate(c.win, &["F2", "F3"], c.min_samples, c.lab_mode) {
        Err((inputs, code, guard)) => {
            r.inputs = inputs;
            r.guardrails = guard;
            return r.no_action(code);
        }
        Ok(inputs) => r.inputs = inputs,
    }
    for id in ["F4", "F17", "F6"] {
        let i = input_of(c.win, id);
        if i.src.passes_gate() {
            r.inputs.push(i);
        }
    }
    let p = c.params;
    let top1 = f(c.win, "F3").unwrap_or(0.0);
    let one_hit = f(c.win, "F17");
    let alpha = f(c.win, "F4");
    let conds: [(&str, bool); 6] = [
        ("bursty_flash_crowd", c.burst_recent),
        ("phase_change", c.phase_recent),
        ("hot_key_dominated", top1 >= p.get("p1.hot_top1_share")),
        (
            "scan_cold",
            one_hit
                .map(|x| x >= p.get("p1.scan_one_hit_ratio"))
                .unwrap_or(false),
        ),
        (
            "zipf_like",
            alpha
                .map(|a| a >= p.get("p1.zipf_min_alpha"))
                .unwrap_or(false),
        ),
        ("uniform_like", true),
    ];
    let mut chosen: Option<&str> = None;
    for (name, ok) in conds.iter() {
        let sel = *ok && chosen.is_none();
        if sel {
            chosen = Some(name);
        }
        r.candidates.push(CandidateEval {
            candidate_id: (*name).to_string(),
            status: if sel { "selected" } else { "rejected" },
            rejection_reason: if sel { None } else { Some("trigger_not_met") },
            estimated_effect: None,
        });
    }
    let name = chosen.unwrap_or("uniform_like");
    r.verdict = Verdict::Observation;
    r.reason_code = format!("label_{name}");
    r.label = Some(name.to_string());
    r
}

// ---------------------------------------------------------------------------------------------
// P2 eviction shadow selection and P3 admission advice
// ---------------------------------------------------------------------------------------------
fn effect_of(rep: &ShadowEvaluationReport) -> Effect {
    Effect {
        metric: "hit_ratio_delta",
        value: rep.delta_hit_ratio,
        ci95: [rep.confidence_interval_95.0, rep.confidence_interval_95.1],
        src: Src::Derived,
        basis: "shadow_sampled_stream",
    }
}

pub fn eval_p2(c: &EvalCtx) -> PolicyResult {
    let mut r = PolicyResult::new(P2, "cache_replacement");
    match gate(c.win, &["F2"], c.min_samples, c.lab_mode) {
        Err((inputs, code, guard)) => {
            r.inputs = inputs;
            r.guardrails = guard;
            return r.no_action(code);
        }
        Ok(inputs) => r.inputs = inputs,
    }
    let delta_min = c.params.get("p2.delta_min");
    let sampled = c.shadows.sampled();
    let names = c.shadows.eviction_candidates();
    if (sampled as f64) < c.params.get("p2.min_sampled") {
        for n in names {
            r.candidates.push(CandidateEval {
                candidate_id: n.to_string(),
                status: "rejected",
                rejection_reason: Some("insufficient_samples"),
                estimated_effect: None,
            });
        }
        r.guardrails.push(format!("shadow_sampled:{sampled}"));
        return r.no_action("insufficient_shadow_samples");
    }
    let mut best: Option<(usize, f64)> = None;
    let mut reps = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let rep = c.shadows.report(n, delta_min);
        if let Some(rep) = &rep {
            let lo = rep.confidence_interval_95.0;
            if best.map(|(_, b)| lo > b).unwrap_or(true) {
                best = Some((i, lo));
            }
        }
        reps.push(rep);
    }
    let winner = best.and_then(|(i, _)| {
        reps[i]
            .as_ref()
            .filter(|rp| rp.is_statistically_significant)
            .map(|_| i)
    });
    for (i, n) in names.iter().enumerate() {
        let Some(rep) = &reps[i] else { continue };
        let sel = winner == Some(i);
        r.candidates.push(CandidateEval {
            candidate_id: n.to_string(),
            status: if sel { "selected" } else { "rejected" },
            rejection_reason: if sel {
                None
            } else if rep.is_statistically_significant {
                Some("dominated")
            } else {
                Some("below_margin")
            },
            estimated_effect: Some(effect_of(rep)),
        });
    }
    match winner {
        Some(i) => {
            let rep = reps[i].as_ref().unwrap();
            let n = names[i];
            r.verdict = Verdict::Recommend;
            r.reason_code = "candidate_exceeds_margin".into();
            r.score = rep.confidence_interval_95.0;
            r.rec_key = Some(format!("evict:{n}"));
            r.recommendation = Some(Recommendation {
                text: format!(
                    "On a 1/{} key sample, shadow {} would have a hit ratio {:+.2} percentage points versus the incumbent {} (95% interval [{:+.2}, {:+.2}]). Advisory only; nothing was changed.",
                    SAMPLE_MODULUS,
                    n,
                    rep.delta_hit_ratio * 100.0,
                    c.shadows.incumbent.as_str(),
                    rep.confidence_interval_95.0 * 100.0,
                    rep.confidence_interval_95.1 * 100.0
                ),
                structured: json!({"kind":"eviction_policy","candidate":n,"incumbent":c.shadows.incumbent.as_str(),"sampled_requests":sampled}),
                applied: false,
                label: ADVISORY_LABEL,
            });
            r
        }
        None => r.no_action("no_candidate_exceeds_margin"),
    }
}

pub fn eval_p3(c: &EvalCtx) -> PolicyResult {
    let mut r = PolicyResult::new(P3, "cache_replacement");
    match gate(c.win, &["F17"], c.min_samples, c.lab_mode) {
        Err((inputs, code, guard)) => {
            r.inputs = inputs;
            r.guardrails = guard;
            return r.no_action(code);
        }
        Ok(inputs) => r.inputs = inputs,
    }
    let one_hit = f(c.win, "F17").unwrap_or(0.0);
    // size and origin cost weighting (0 = unknown in the events -> unavailable -> unchanged behaviour, never extrapolated)
    let bytes_share = f(c.win, "F28");
    for id in ["F27", "F28", "F29"] {
        let i = input_of(c.win, id);
        if i.src.passes_gate() {
            r.inputs.push(i);
        } else {
            let why = c.win.feat(id).and_then(|x| x.reason).unwrap_or("missing");
            r.guardrails.push(format!("weight_unavailable:{id}:{why}"));
        }
    }
    let mean_size = f(c.win, "F27");
    let cost_share = f(c.win, "F29");
    let origin_p99_ms = f(c.win, "F10");
    let high_origin_cost = origin_p99_ms.map(|p| p * 1000.0 >= c.params.get("p3.origin_cost_warn_us")).unwrap_or(false);
    let delta_min = c.params.get("p3.delta_min");
    let sampled = c.shadows.sampled();
    let rep = c.shadows.report("TinyLFU", delta_min);
    let mut cand = CandidateEval {
        candidate_id: "TinyLFU-admission".into(),
        status: "rejected",
        rejection_reason: Some("trigger_not_met"),
        estimated_effect: rep.as_ref().map(effect_of),
    };
    if one_hit < c.params.get("p3.one_hit_ratio_min") {
        r.candidates.push(cand);
        return r.no_action("one_hit_ratio_below_trigger");
    }
    if let Some(b) = bytes_share {
        if b < c.params.get("p3.one_hit_bytes_min") {
            r.candidates.push(cand);
            r.guardrails.push(format!("one_hit_bytes_share:{b:.3}"));
            return r.no_action("one_hit_bytes_share_low");
        }
    }
    if (sampled as f64) < c.params.get("p3.min_sampled") {
        cand.rejection_reason = Some("insufficient_samples");
        r.candidates.push(cand);
        r.guardrails.push(format!("shadow_sampled:{sampled}"));
        return r.no_action("insufficient_shadow_samples");
    }
    match rep {
        Some(rep) if rep.is_statistically_significant => {
            cand.status = "selected";
            cand.rejection_reason = None;
            r.candidates.push(cand);
            r.verdict = Verdict::Recommend;
            r.reason_code = "admission_gain_exceeds_margin".into();
            r.score = rep.confidence_interval_95.0;
            r.rec_key = Some("admission:tinylfu".into());
            r.recommendation = Some(Recommendation {
                text: format!(
                    "One-hit wonders are {:.0}% of sampled requests; a TinyLFU-style admission filter would have a hit ratio {:+.2} percentage points versus the incumbent {} on the sampled stream (95% interval [{:+.2}, {:+.2}]). {}Advisory only; nothing was changed.",
                    one_hit * 100.0,
                    rep.delta_hit_ratio * 100.0,
                    c.shadows.incumbent.as_str(),
                    rep.confidence_interval_95.0 * 100.0,
                    rep.confidence_interval_95.1 * 100.0,
                    weight_note(bytes_share, cost_share, high_origin_cost)
                ),
                structured: json!({"kind":"admission_filter","candidate":"tinylfu","incumbent":c.shadows.incumbent.as_str(),"sampled_requests":sampled,"one_hit_bytes_share":bytes_share,"one_hit_origin_cost_share":cost_share,"mean_object_size_bytes":mean_size,"high_origin_cost":high_origin_cost,"weighting":if bytes_share.is_some() {"size_and_cost_considered"} else {"size_unknown_count_only"}}),
                applied: false,
                label: ADVISORY_LABEL,
            });
            r
        }
        _ => {
            cand.rejection_reason = Some("below_margin");
            r.candidates.push(cand);
            r.no_action("no_candidate_exceeds_margin")
        }
    }
}

// ---------------------------------------------------------------------------------------------
// P5 stampede risk, P6 origin saturation (advisory text only, with 20% hysteresis)
// ---------------------------------------------------------------------------------------------
pub(crate) fn hysteresis(value: f64, on: f64, prev_active: bool) -> bool {
    value >= on || (prev_active && value >= 0.8 * on)
}

pub fn eval_p5(c: &EvalCtx) -> PolicyResult {
    let mut r = PolicyResult::new(P5, "origin_pressure");
    match gate(c.win, &["F13"], c.min_samples, c.lab_mode) {
        Err((inputs, code, guard)) => {
            r.inputs = inputs;
            r.guardrails = guard;
            r.warning_active = Some(false);
            return r.no_action(code);
        }
        Ok(inputs) => r.inputs = inputs,
    }
    let v = f(c.win, "F13").unwrap_or(0.0);
    let on = c.params.get("p5.waiters_per_fetch_on");
    let active = hysteresis(v, on, c.prev_active);
    r.warning_active = Some(active);
    let mut cand = CandidateEval {
        candidate_id: "stampede_warning".into(),
        status: "rejected",
        rejection_reason: Some("trigger_not_met"),
        estimated_effect: None,
    };
    if active {
        cand.status = "selected";
        cand.rejection_reason = None;
        r.candidates.push(cand);
        r.verdict = Verdict::Recommend;
        r.reason_code = "waiters_per_fetch_high".into();
        r.score = v - on;
        r.rec_key = Some("stampede:warn".into());
        r.recommendation = Some(Recommendation {
            text: format!(
                "Coalescing is carrying {:.1} waiters per origin fetch (threshold {:.1}). Single-flight is under heavy pressure on hot keys; review origin concurrency and expiry spread. Advisory only; nothing was changed.",
                v, on
            ),
            structured: json!({"kind":"stampede_warning","waiters_per_fetch":v,"threshold":on}),
            applied: false,
            label: ADVISORY_LABEL,
        });
        r
    } else {
        r.candidates.push(cand);
        if c.prev_active {
            r.no_action("warning_cleared")
        } else {
            r.no_action("below_threshold")
        }
    }
}

pub fn eval_p6(c: &EvalCtx) -> PolicyResult {
    let mut r = PolicyResult::new(P6, "origin_pressure");
    let (p99, err, samples_ok);
    let fresh = c
        .external
        .filter(|e| c.now_ms.saturating_sub(e.received_ms) <= 3 * c.window_ms.max(1));
    if let Some(e) = fresh {
        let src = e.src.unwrap_or(Src::Unavailable);
        let mk = |id: &'static str, v: f64| InputUsed {
            feature: id,
            value: Some(FVal::F(v)),
            src,
            samples: e.samples,
            window_ms: c.window_ms,
        };
        r.inputs = vec![mk("F10", e.p99_ms), mk("F10b", e.error_rate)];
        r.guardrails
            .push("origin_health_source:gateway_metrics".into());
        if src == Src::Simulated && !c.lab_mode || src == Src::Unavailable {
            r.guardrails
                .push(format!("provenance_refused:F10:{}", src.as_str()));
            r.warning_active = Some(false);
            return r.no_action("provenance_refused");
        }
        samples_ok = e.samples >= c.min_samples;
        p99 = e.p99_ms;
        err = e.error_rate;
    } else {
        match gate(c.win, &["F10", "F10b"], c.min_samples, c.lab_mode) {
            Err((inputs, code, guard)) => {
                r.inputs = inputs;
                r.guardrails = guard;
                r.warning_active = Some(false);
                return r.no_action(code);
            }
            Ok(inputs) => r.inputs = inputs,
        }
        samples_ok = true;
        p99 = f(c.win, "F10").unwrap_or(0.0);
        err = f(c.win, "F10b").unwrap_or(0.0);
    }
    if !samples_ok {
        r.guardrails.push("insufficient_samples:F10".into());
        r.warning_active = Some(false);
        return r.no_action("insufficient_samples");
    }
    let on_p99 = c.params.get("p6.p99_ms_on");
    let on_err = c.params.get("p6.error_rate_on");
    let active = hysteresis(p99, on_p99, c.prev_active) || hysteresis(err, on_err, c.prev_active);
    r.warning_active = Some(active);
    let mut cand = CandidateEval {
        candidate_id: "origin_saturation_warning".into(),
        status: "rejected",
        rejection_reason: Some("trigger_not_met"),
        estimated_effect: None,
    };
    if active {
        cand.status = "selected";
        cand.rejection_reason = None;
        r.candidates.push(cand);
        r.verdict = Verdict::Recommend;
        r.reason_code = "origin_health_degraded".into();
        r.score = (p99 / on_p99).max(err / on_err);
        r.rec_key = Some("origin:warn".into());
        r.recommendation = Some(Recommendation {
            text: format!(
                "Origin fetch p99 is about {:.0} ms (threshold {:.0}) and error rate {:.1}% (threshold {:.1}%). The origin may be saturating; the origin concurrency cap was NOT changed. Advisory only.",
                p99,
                on_p99,
                err * 100.0,
                on_err * 100.0
            ),
            structured: json!({"kind":"origin_saturation_warning","p99_ms":p99,"error_rate":err}),
            applied: false,
            label: ADVISORY_LABEL,
        });
        r
    } else {
        r.candidates.push(cand);
        if c.prev_active {
            r.no_action("warning_cleared")
        } else {
            r.no_action("below_threshold")
        }
    }
}

fn weight_note(bytes: Option<f64>, cost: Option<f64>, high_cost: bool) -> String {
    let mut s = String::new();
    match bytes {
        Some(b) => s.push_str(&format!("One-hit keys hold {:.0}% of sampled object bytes. ", b * 100.0)),
        None => s.push_str("Object sizes are unknown, so the advice counts requests only. "),
    }
    if let Some(x) = cost {
        s.push_str(&format!("One-hit keys account for {:.0}% of sampled origin time. ", x * 100.0));
    }
    if high_cost {
        s.push_str("Origin latency is high, so delaying admission of keys seen twice is costly: validate before relying on it. ");
    }
    s
}

// ---------------------------------------------------------------------------------------------
// Flash-crowd detection (POL-FLASH-1): advisory text only
// ---------------------------------------------------------------------------------------------
pub const PFLASH: &str = "POL-FLASH-1";

pub fn eval_flash(win: &WindowSnapshot, lab_mode: bool) -> Option<PolicyResult> {
    let ev = win.flash.as_ref()?;
    let mut r = PolicyResult::new(PFLASH, "origin_pressure");
    let dur = win.duration_ms();
    let mut inputs = Vec::new();
    for id in ["F1", "F3b", "F6"] {
        inputs.push(input_of(win, id));
    }
    r.inputs = inputs;
    if r.inputs.iter().any(|i| i.src == Src::Simulated) && !lab_mode {
        r.guardrails.push("provenance_refused:flash".into());
        r.reason_code = "provenance_refused".into();
        return Some(r);
    }
    let keys: Vec<String> = ev.hot_keys.iter().map(|(k, s)| format!("{k:016x} ({:.1}% of window)", s * 100.0)).collect();
    r.verdict = Verdict::Recommend;
    r.reason_code = match (ev.rate_fired, ev.conc_fired) {
        (true, true) => "rate_and_concentration_change",
        (true, false) => "rate_change",
        _ => "concentration_change",
    }
    .into();
    r.score = ev.confidence;
    r.rec_key = Some("flash:prewarm".into());
    r.candidates.push(CandidateEval {
        candidate_id: "flash_crowd_advisory".into(),
        status: "selected",
        rejection_reason: None,
        estimated_effect: None,
    });
    r.recommendation = Some(Recommendation {
        text: format!(
            "Possible flash crowd (rule confidence {:.1}, heuristic, not a probability): request rate is {} of its baseline and the ten hottest keys carry {} of requests. Consider pre-warming these hot keys and a longer stale window: {}. Nothing was applied.",
            ev.confidence,
            ev.rate_ratio.map(|x| format!("{x:.1}x")).unwrap_or_else(|| "unknown".into()),
            ev.top10_share.map(|x| format!("{:.0}%", x * 100.0)).unwrap_or_else(|| "unknown".into()),
            if keys.is_empty() { "no keys reported".to_string() } else { keys.join(", ") }
        ),
        structured: json!({
            "kind":"flash_crowd_advisory","confidence":ev.confidence,"confidence_kind":"heuristic_rule_score",
            "window_ms":dur,"rate_ratio":ev.rate_ratio,"rate_cusum":ev.rate_cusum,"rate_fired":ev.rate_fired,
            "top10_share":ev.top10_share,"concentration_baseline":ev.conc_baseline,"concentration_cusum":ev.conc_cusum,
            "concentration_fired":ev.conc_fired,
            "hot_keys":ev.hot_keys.iter().map(|(k,s)| json!({"key_hash":format!("{k:016x}"),"share":s})).collect::<Vec<_>>()
        }),
        applied: false,
        label: ADVISORY_LABEL,
    });
    Some(r)
}

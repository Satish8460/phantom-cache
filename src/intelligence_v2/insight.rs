//! Window insight: hot-key tracking, frequency summary, HyperLogLog cardinality, size and origin-cost features,
//! statistical hot-key prediction with an online scorecard, and flash-crowd change-point detection.
//!
//! Everything here is rules plus simple statistics (Space-Saving, HyperLogLog, EWMA, CUSUM). There is NO machine
//! learning and no model training. All outputs are deterministic functions of the event stream. Nothing is applied.

use super::registry::Params;
use super::signals::{Accum, Feat};
use super::{mix64, FVal, Src};
use std::collections::BTreeMap;

pub const HLL_P: u32 = 10;
pub const HLL_M: usize = 1 << HLL_P; // 1024 registers = 1 KiB
pub const HLL_STD_ERR: f64 = 1.04 / 32.0; // 1.04 / sqrt(1024) = 3.25 percent
pub const TRACKS_CAP: usize = 128;

/// HyperLogLog with p = 10 and a deterministic 64-bit hash.
#[derive(Clone)]
pub struct Hll {
    regs: [u8; HLL_M],
}

impl Hll {
    pub fn new() -> Self {
        Hll { regs: [0; HLL_M] }
    }
    pub fn insert(&mut self, key_hash: u64) {
        let h = mix64(key_hash ^ 0x9E37_79B9_7F4A_7C15);
        let idx = (h >> (64 - HLL_P)) as usize;
        let w = (h << HLL_P) | (1u64 << (HLL_P - 1)); // sentinel bit caps the rank
        let rank = w.leading_zeros() as u8 + 1;
        if rank > self.regs[idx] {
            self.regs[idx] = rank;
        }
    }
    pub fn estimate(&self) -> f64 {
        let m = HLL_M as f64;
        let alpha = 0.7213 / (1.0 + 1.079 / m);
        let mut sum = 0.0;
        let mut zeros = 0usize;
        for r in self.regs.iter() {
            sum += 2f64.powi(-(*r as i32));
            if *r == 0 {
                zeros += 1;
            }
        }
        let e = alpha * m * m / sum;
        if e <= 2.5 * m && zeros > 0 {
            m * (m / zeros as f64).ln()
        } else {
            e
        }
    }
}

impl Default for Hll {
    fn default() -> Self {
        Self::new()
    }
}

/// One reported heavy hitter. `lower <= true count <= upper` is the Space-Saving guarantee.
#[derive(Debug, Clone)]
pub struct HotKey {
    pub key_hash: u64,
    pub lower: u32,
    pub upper: u32,
    pub error: u32,
    pub share: f64,
    pub confidence: f64,
    pub hot: bool,
}

#[derive(Debug, Clone)]
pub struct FlashEvidence {
    pub rate_ratio: Option<f64>,
    pub rate_cusum: f64,
    pub rate_fired: bool,
    pub top10_share: Option<f64>,
    pub conc_baseline: Option<f64>,
    pub conc_cusum: f64,
    pub conc_fired: bool,
    pub confidence: f64,
    pub hot_keys: Vec<(u64, f64)>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Score {
    pub tp: u64,
    pub fp: u64,
    pub fnn: u64,
    pub windows: u64,
}

impl Score {
    pub fn precision(&self) -> Option<f64> {
        (self.tp + self.fp > 0).then(|| self.tp as f64 / (self.tp + self.fp) as f64)
    }
    pub fn recall(&self) -> Option<f64> {
        (self.tp + self.fnn > 0).then(|| self.tp as f64 / (self.tp + self.fnn) as f64)
    }
    fn add(&mut self, predicted: &[u64], actual: &[u64]) {
        let tp = predicted.iter().filter(|k| actual.contains(k)).count() as u64;
        self.tp += tp;
        self.fp += predicted.len() as u64 - tp;
        self.fnn += actual.len() as u64 - tp;
        self.windows += 1;
    }
}

#[derive(Debug, Clone, Copy)]
struct KeyTrack {
    ewma: f64,
    trend: f64,
    last: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct InsightCfg {
    pub hot_share: f64,
    pub high_ratio: f64,
    pub pred_min_windows: u64,
    pub rate_slack: f64,
    pub rate_h: f64,
    pub conc_slack: f64,
    pub conc_h: f64,
}

impl Default for InsightCfg {
    fn default() -> Self {
        InsightCfg {
            hot_share: 0.02,
            high_ratio: 0.5,
            pred_min_windows: 5,
            rate_slack: 0.5,
            rate_h: 1.5,
            conc_slack: 0.05,
            conc_h: 0.3,
        }
    }
}

/// Cross-window state of the insight layer (bounded: at most `TRACKS_CAP` tracked keys).
pub struct InsightState {
    pub cfg: InsightCfg,
    prev_distinct: Option<f64>,
    tracks: BTreeMap<u64, KeyTrack>,
    pred_set: Option<Vec<u64>>,
    pub predicted_next: Vec<(u64, f64)>,
    prev_hot: Vec<u64>,
    pub score: Score,
    pub baseline_score: Score,
    rate_s: f64,
    conc_s: f64,
    conc_base: Option<f64>,
    conc_windows: u32,
}

impl InsightState {
    pub fn new() -> Self {
        InsightState {
            cfg: InsightCfg::default(),
            prev_distinct: None,
            tracks: BTreeMap::new(),
            pred_set: None,
            predicted_next: vec![],
            prev_hot: vec![],
            score: Score::default(),
            baseline_score: Score::default(),
            rate_s: 0.0,
            conc_s: 0.0,
            conc_base: None,
            conc_windows: 0,
        }
    }
    pub fn configure(&mut self, p: &Params) {
        self.cfg = InsightCfg {
            hot_share: p.get("h.hot_share"),
            high_ratio: p.get("c.high_ratio"),
            pred_min_windows: p.get("pr.min_windows") as u64,
            rate_slack: p.get("fc.rate_slack"),
            rate_h: p.get("fc.rate_h"),
            conc_slack: p.get("fc.conc_slack"),
            conc_h: p.get("fc.conc_h"),
        };
    }
    pub fn tracked_keys(&self) -> usize {
        self.tracks.len()
    }
}

impl Default for InsightState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Extra {
    pub feats: Vec<Feat>,
    pub hot: Vec<HotKey>,
    pub flash: Option<FlashEvidence>,
}

const IDS: &[(&str, &str, &str)] = &[
    ("F18", "key_share_seen_once", "share"),
    ("F19", "key_share_seen_twice", "share"),
    ("F20", "key_share_seen_3plus", "share"),
    ("F21", "top1pct_request_share", "share"),
    ("F22", "top10pct_request_share", "share"),
    ("F23", "distinct_keys_est", "keys"),
    ("F23b", "distinct_keys_std_err", "keys"),
    ("F24", "distinct_ratio", "ratio"),
    ("F25", "distinct_growth", "ratio"),
    ("F26", "high_cardinality", "bool"),
    ("F27", "mean_object_size_bytes", "bytes"),
    ("F28", "one_hit_bytes_share", "share"),
    ("F29", "one_hit_origin_cost_share", "share"),
    ("F30", "hot_key_count", "keys"),
];

/// All insight features unavailable (degraded or empty window).
pub fn na_features(reason: &'static str, samples: u64) -> Vec<Feat> {
    IDS.iter().map(|(i, n, u)| Feat::na(i, n, u, reason, samples)).collect()
}

fn set_of(hot: &[HotKey]) -> Vec<u64> {
    let mut v: Vec<u64> = hot.iter().filter(|h| h.hot).map(|h| h.key_hash).collect();
    v.sort_unstable();
    v
}

#[allow(clippy::too_many_arguments)]
pub fn compute(
    acc: &Accum,
    ins: &mut InsightState,
    req: u64,
    min: u64,
    burst_ratio: Option<f64>,
    top10_share: Option<f64>,
) -> Extra {
    let enough = req >= min;
    let mut f: Vec<Feat> = Vec::with_capacity(IDS.len());
    let d = Src::Derived;

    // ---- frequency distribution over the 1/16 spatial sample of distinct keys ----
    let distinct_s = acc.sample.len() as u64;
    if acc.sample_saturated {
        f.extend(
            ["F18", "F19", "F20", "F21", "F22"]
                .iter()
                .map(|id| na_of(id, "sampler_saturated", distinct_s)),
        );
    } else if distinct_s < 30 {
        f.extend(
            ["F18", "F19", "F20", "F21", "F22"]
                .iter()
                .map(|id| na_of(id, "insufficient_samples", distinct_s)),
        );
    } else {
        let mut counts: Vec<u32> = acc.sample.values().copied().collect();
        counts.sort_unstable_by(|a, b| b.cmp(a));
        let n = distinct_s as f64;
        let once = counts.iter().filter(|c| **c == 1).count() as f64;
        let twice = counts.iter().filter(|c| **c == 2).count() as f64;
        let three = n - once - twice;
        f.push(ok_of("F18", once / n, d, distinct_s));
        f.push(ok_of("F19", twice / n, d, distinct_s));
        f.push(ok_of("F20", three / n, d, distinct_s));
        let total: u64 = counts.iter().map(|c| *c as u64).sum::<u64>().max(1);
        let top = |frac: f64| -> f64 {
            let k = ((n * frac).ceil() as usize).max(1).min(counts.len());
            counts[..k].iter().map(|c| *c as u64).sum::<u64>() as f64 / total as f64
        };
        if distinct_s >= 100 {
            f.push(ok_of("F21", top(0.01), d, distinct_s));
        } else {
            f.push(na_of("F21", "insufficient_samples", distinct_s));
        }
        f.push(ok_of("F22", top(0.10), d, distinct_s));
    }

    // ---- cardinality (HyperLogLog p = 10, standard error about 3.25 percent) ----
    if enough {
        let est = acc.hll.estimate();
        let ratio = (est / req.max(1) as f64).min(1.0);
        f.push(ok_of("F23", est, d, req));
        f.push(ok_of("F23b", est * HLL_STD_ERR, d, req));
        f.push(ok_of("F24", ratio, d, req));
        match ins.prev_distinct {
            Some(p) if p > 0.0 => f.push(ok_of("F25", est / p - 1.0, d, req)),
            _ => f.push(na_of("F25", "no_previous_window", req)),
        }
        f.push(Feat::ok(
            "F26",
            "high_cardinality",
            "bool",
            FVal::B(ratio >= ins.cfg.high_ratio),
            d,
            req,
        ));
        ins.prev_distinct = Some(est);
    } else {
        for id in ["F23", "F23b", "F24", "F25", "F26"] {
            f.push(na_of(id, "insufficient_samples", req));
        }
    }

    // ---- object size and origin cost of sampled keys (0 means unknown, never extrapolated) ----
    let (mut ks, mut kb, mut sb, mut ko, mut to, mut so) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    for (k, (size, orig)) in acc.meta.iter() {
        let single = acc.sample.get(k).copied() == Some(1);
        if *size > 0 {
            ks += 1;
            kb += *size as u64;
            if single {
                sb += *size as u64;
            }
        }
        if *orig > 0 {
            ko += 1;
            to += *orig as u64;
            if single {
                so += *orig as u64;
            }
        }
    }
    let why = |known: u64, unknown: &'static str| if known == 0 { unknown } else { "insufficient_samples" };
    if ks >= 30 {
        f.push(ok_of("F27", kb as f64 / ks as f64, d, ks));
        f.push(ok_of("F28", sb as f64 / kb.max(1) as f64, d, ks));
    } else {
        f.push(na_of("F27", why(ks, "size_unknown"), ks));
        f.push(na_of("F28", why(ks, "size_unknown"), ks));
    }
    if ko >= 30 {
        f.push(ok_of("F29", so as f64 / to.max(1) as f64, d, ko));
    } else {
        f.push(na_of("F29", why(ko, "origin_cost_unknown"), ko));
    }

    // ---- hot keys (Space-Saving, guaranteed lower bounds) ----
    let mut hot: Vec<HotKey> = vec![];
    if enough {
        for (key, count, error) in acc.topk.entries().into_iter().take(10) {
            let lower = count - error;
            let share = lower as f64 / req as f64;
            hot.push(HotKey {
                key_hash: key,
                lower,
                upper: count,
                error,
                share: share.min(1.0),
                confidence: if count == 0 { 0.0 } else { lower as f64 / count as f64 },
                hot: share >= ins.cfg.hot_share,
            });
        }
        let n = hot.iter().filter(|h| h.hot).count();
        f.push(ok_of("F30", n as f64, d, req));
    } else {
        f.push(na_of("F30", "insufficient_samples", req));
    }

    // ---- statistical prediction and its scorecard ----
    if enough {
        let actual = set_of(&hot);
        if let Some(p) = ins.pred_set.take() {
            ins.score.add(&p, &actual);
            let prev = std::mem::take(&mut ins.prev_hot);
            ins.baseline_score.add(&prev, &actual);
        }
        ins.prev_hot = actual;
        let shares: BTreeMap<u64, f64> = hot.iter().map(|h| (h.key_hash, h.share)).collect();
        for h in &hot {
            ins.tracks.entry(h.key_hash).or_insert(KeyTrack { ewma: h.share, trend: 0.0, last: h.share });
        }
        for (k, t) in ins.tracks.iter_mut() {
            let s = shares.get(k).copied().unwrap_or(0.0);
            let delta = s - t.last;
            t.ewma = 0.5 * s + 0.5 * t.ewma;
            t.trend = 0.5 * delta + 0.5 * t.trend;
            t.last = s;
        }
        ins.tracks.retain(|_, t| t.ewma >= 0.001 || t.last > 0.0);
        while ins.tracks.len() > TRACKS_CAP {
            let drop = ins
                .tracks
                .iter()
                .min_by(|a, b| a.1.ewma.partial_cmp(&b.1.ewma).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(b.0)))
                .map(|(k, _)| *k);
            if let Some(k) = drop {
                ins.tracks.remove(&k);
            } else {
                break;
            }
        }
        let mut next: Vec<(u64, f64)> = ins
            .tracks
            .iter()
            .map(|(k, t)| (*k, (t.ewma + t.trend).clamp(0.0, 1.0)))
            .filter(|(_, p)| *p >= ins.cfg.hot_share)
            .collect();
        next.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
        let mut set: Vec<u64> = next.iter().map(|x| x.0).collect();
        set.sort_unstable();
        ins.pred_set = Some(set);
        ins.predicted_next = next;
    } else {
        ins.pred_set = None; // a gap breaks the prediction chain; never scored across a thin window
        ins.predicted_next.clear();
    }

    // ---- flash-crowd change points: CUSUM on rate ratio and on top-10 concentration ----
    let mut flash = None;
    if enough {
        if let Some(r) = burst_ratio {
            ins.rate_s = (ins.rate_s + (r - 1.0) - ins.cfg.rate_slack).max(0.0);
        }
        let rate_fired = ins.rate_s >= ins.cfg.rate_h;
        let mut conc_fired = false;
        let base_before = ins.conc_base;
        if let Some(c) = top10_share {
            if let (Some(b), true) = (ins.conc_base, ins.conc_windows >= 2) {
                ins.conc_s = (ins.conc_s + (c - b) - ins.cfg.conc_slack).max(0.0);
                conc_fired = ins.conc_s >= ins.cfg.conc_h;
            }
            if !conc_fired {
                ins.conc_base = Some(match ins.conc_base {
                    Some(b) => 0.8 * b + 0.2 * c,
                    None => c,
                });
            }
            ins.conc_windows = ins.conc_windows.saturating_add(1);
        }
        if rate_fired || conc_fired {
            // heuristic rule score, not a calibrated probability
            let confidence = match (rate_fired, conc_fired) {
                (true, true) => 0.9,
                _ => 0.6,
            };
            flash = Some(FlashEvidence {
                rate_ratio: burst_ratio,
                rate_cusum: ins.rate_s,
                rate_fired,
                top10_share,
                conc_baseline: base_before,
                conc_cusum: ins.conc_s,
                conc_fired,
                confidence,
                hot_keys: hot.iter().take(5).map(|h| (h.key_hash, h.share)).collect(),
            });
            ins.rate_s = 0.0;
            ins.conc_s = 0.0;
        }
    }
    Extra { feats: f, hot, flash }
}

fn name_of(id: &str) -> (&'static str, &'static str, &'static str) {
    IDS.iter().find(|x| x.0 == id).map(|x| (x.0, x.1, x.2)).unwrap_or(("?", "?", ""))
}
fn ok_of(id: &str, v: f64, src: Src, samples: u64) -> Feat {
    let (i, n, u) = name_of(id);
    Feat::ok(i, n, u, FVal::F(v), src, samples)
}
fn na_of(id: &str, reason: &'static str, samples: u64) -> Feat {
    let (i, n, u) = name_of(id);
    Feat::na(i, n, u, reason, samples)
}

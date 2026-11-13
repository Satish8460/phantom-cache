//! Signal layer: bounded non-blocking event queue and windowed features with provenance.
//!
//! Feature ids follow `src/intelligence/signals.rs` (F1..F12) plus F13, F15, F16, F17 from the design.
//! A feature below `min_feature_samples`, or in a degraded window, is `unavailable` and is never extrapolated.

use super::{mix64, FVal, Mode, Src};
use crate::intelligence::signals::{HotKeyEmergenceDetector, ZipfSkewDetector};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::sync::Arc;

pub const MAX_LATENCY_US: u32 = 600_000_000; // 10 minutes; anything above is treated as poisoned input
pub const TOPK_CAP: usize = 64;
pub const SAMPLE_MAP_CAP: usize = 4096;
pub const SAMPLE_MODULUS: u64 = 16;
pub const HIST_BUCKETS: usize = 124;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Hit,
    Miss,
    Coalesced,
    Stale,
    Peer,
    Invalidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginResult {
    None,
    Ok,
    Err,
}

/// One compact request-path event (24 bytes). Contains no key material, only a hash.
#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub key_hash: u64,
    pub latency_us: u32,
    /// Real origin fetch latency in microseconds (0 = unknown / no origin fetch).
    pub origin_latency_us: u32,
    pub outcome: Outcome,
    pub origin: OriginResult,
    /// Response body size in bytes (0 = unknown; features that need it then say `unavailable`).
    pub size_bytes: u32,
}

pub(crate) struct SinkShared {
    pub mode: AtomicU8,
    pub enqueued: AtomicU64,
    pub dropped: AtomicU64,
    pub dequeued: AtomicU64,
}

/// Cloneable hot-path handle. `try_record` never blocks and never allocates.
#[derive(Clone)]
pub struct EventSink {
    pub(crate) tx: SyncSender<Event>,
    pub(crate) shared: Arc<SinkShared>,
}

impl EventSink {
    /// Returns true if the event was queued. Off: one relaxed load and return. Full queue: drop and count.
    #[inline]
    pub fn try_record(&self, ev: Event) -> bool {
        if self.shared.mode.load(Ordering::Relaxed) == Mode::Off as u8 {
            return false;
        }
        match self.tx.try_send(ev) {
            Ok(()) => {
                self.shared.enqueued.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(TrySendError::Full(_)) => {
                self.shared.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        }
    }
}

// ---- latency histogram: 4 sub-buckets per octave (upper-bound quantiles, error <= 25%) ----

pub fn bucket_of(us: u32) -> usize {
    if us < 4 {
        us as usize
    } else {
        let o = 31 - us.leading_zeros() as usize; // >= 2
        4 * (o - 1) + (((us >> (o - 2)) & 3) as usize)
    }
}

pub fn bucket_upper_us(idx: usize) -> u64 {
    if idx < 4 {
        idx as u64 + 1
    } else {
        let o = idx / 4 + 1;
        let sub = (idx % 4) as u64;
        (5 + sub) << (o - 2)
    }
}

/// Bounded Space-Saving top-k. Reported counts are guaranteed lower bounds (count - error).
#[derive(Debug, Clone)]
pub struct TopK {
    entries: Vec<(u64, u32, u32)>, // key, count, error
}

impl TopK {
    pub fn new() -> Self {
        Self {
            entries: Vec::with_capacity(TOPK_CAP),
        }
    }
    pub fn observe(&mut self, key: u64) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.0 == key) {
            e.1 = e.1.saturating_add(1);
        } else if self.entries.len() < TOPK_CAP {
            self.entries.push((key, 1, 0));
        } else {
            let mut mi = 0;
            let mut mc = u32::MAX;
            for (i, e) in self.entries.iter().enumerate() {
                if e.1 < mc {
                    mc = e.1;
                    mi = i;
                }
            }
            let e = &mut self.entries[mi];
            e.0 = key;
            e.2 = e.1;
            e.1 = e.1.saturating_add(1);
        }
    }
    /// (key, count upper bound, error) sorted by guaranteed count (count - error) descending, ties by key.
    pub fn entries(&self) -> Vec<(u64, u32, u32)> {
        let mut v = self.entries.clone();
        v.sort_unstable_by(|a, b| (b.1 - b.2).cmp(&(a.1 - a.2)).then(a.0.cmp(&b.0)));
        v
    }
    /// (key, guaranteed count), sorted by count descending, ties by key ascending (deterministic).
    pub fn lower_bounds(&self) -> Vec<(u64, u32)> {
        let mut v: Vec<(u64, u32)> = self.entries.iter().map(|e| (e.0, e.1 - e.2)).collect();
        v.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    }
}

impl Default for TopK {
    fn default() -> Self {
        Self::new()
    }
}

/// Accumulator for the currently open window.
pub struct Accum {
    pub start_ms: u64,
    pub dropped_at_start: u64,
    pub events: u64,
    pub requests: u64,
    pub hits: u64,
    pub misses: u64,
    pub coalesced: u64,
    pub stale: u64,
    pub peer: u64,
    pub invalidations: u64,
    pub origin_ok: u64,
    pub origin_err: u64,
    pub hist: Box<[u32; HIST_BUCKETS]>,
    pub topk: TopK,
    pub sample: HashMap<u64, u32>,
    /// Per sampled key: (first known size in bytes, first known origin latency in us); 0 = unknown.
    pub meta: HashMap<u64, (u32, u32)>,
    pub hll: super::insight::Hll,
    pub sample_events: u64,
    pub sample_saturated: bool,
    pub truncated: bool,
}

impl Accum {
    pub fn new(start_ms: u64, dropped_at_start: u64) -> Self {
        Self {
            start_ms,
            dropped_at_start,
            events: 0,
            requests: 0,
            hits: 0,
            misses: 0,
            coalesced: 0,
            stale: 0,
            peer: 0,
            invalidations: 0,
            origin_ok: 0,
            origin_err: 0,
            hist: Box::new([0; HIST_BUCKETS]),
            topk: TopK::new(),
            sample: HashMap::with_capacity(256),
            meta: HashMap::new(),
            hll: super::insight::Hll::new(),
            sample_events: 0,
            sample_saturated: false,
            truncated: false,
        }
    }

    pub fn add(&mut self, ev: &Event) {
        self.events += 1;
        match ev.outcome {
            Outcome::Invalidation => {
                self.invalidations += 1;
                return;
            }
            Outcome::Hit => self.hits += 1,
            Outcome::Miss => self.misses += 1,
            Outcome::Coalesced => self.coalesced += 1,
            Outcome::Stale => self.stale += 1,
            Outcome::Peer => self.peer += 1,
        }
        self.requests += 1;
        match ev.origin {
            OriginResult::None => {}
            OriginResult::Ok | OriginResult::Err => {
                if ev.origin == OriginResult::Ok {
                    self.origin_ok += 1;
                } else {
                    self.origin_err += 1;
                }
                self.hist[bucket_of(ev.origin_latency_us)] += 1;
            }
        }
        self.topk.observe(ev.key_hash);
        self.hll.insert(ev.key_hash);
        let m = mix64(ev.key_hash);
        if m.is_multiple_of(SAMPLE_MODULUS) {
            if (ev.size_bytes > 0 || ev.origin_latency_us > 0)
                && (self.sample.contains_key(&ev.key_hash) || self.sample.len() < SAMPLE_MAP_CAP)
            {
                let e = self.meta.entry(ev.key_hash).or_insert((0, 0));
                if e.0 == 0 {
                    e.0 = ev.size_bytes;
                }
                if e.1 == 0 {
                    e.1 = ev.origin_latency_us;
                }
            }
            if let Some(c) = self.sample.get_mut(&ev.key_hash) {
                *c = c.saturating_add(1);
                self.sample_events += 1;
            } else if self.sample.len() < SAMPLE_MAP_CAP {
                self.sample.insert(ev.key_hash, 1);
                self.sample_events += 1;
            } else {
                self.sample_saturated = true;
            }
        }
    }

    pub fn origin_fetches(&self) -> u64 {
        self.origin_ok + self.origin_err
    }

    fn origin_quantile_ms(&self, q: f64) -> Option<f64> {
        let n = self.origin_fetches();
        if n == 0 {
            return None;
        }
        let target = ((n as f64) * q).ceil().max(1.0) as u64;
        let mut cum = 0u64;
        for (i, c) in self.hist.iter().enumerate() {
            cum += *c as u64;
            if cum >= target {
                return Some(bucket_upper_us(i) as f64 / 1000.0);
            }
        }
        None
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Feat {
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub value: Option<FVal>,
    pub src: Src,
    pub samples: u64,
    pub reason: Option<&'static str>,
}

impl Feat {
    pub(crate) fn ok(
        id: &'static str,
        name: &'static str,
        unit: &'static str,
        v: FVal,
        src: Src,
        samples: u64,
    ) -> Feat {
        Feat {
            id,
            name,
            unit,
            value: Some(v),
            src,
            samples,
            reason: None,
        }
    }
    pub(crate) fn na(
        id: &'static str,
        name: &'static str,
        unit: &'static str,
        reason: &'static str,
        samples: u64,
    ) -> Feat {
        Feat {
            id,
            name,
            unit,
            value: None,
            src: Src::Unavailable,
            samples,
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowSnapshot {
    pub index: u64,
    pub start_ms: u64,
    pub end_ms: u64,
    pub events: u64,
    pub requests: u64,
    pub dropped: u64,
    pub degraded: bool,
    pub degraded_reason: Option<&'static str>,
    pub features: Vec<Feat>,
    /// (key hash, guaranteed count), at most 10.
    pub top_keys: Vec<(u64, u32)>,
    /// Heavy hitters of the window (empty when the window is degraded or below min samples).
    pub hot: Vec<super::insight::HotKey>,
    /// Set when the change-point detectors fired in this window.
    pub flash: Option<super::insight::FlashEvidence>,
}

impl WindowSnapshot {
    pub fn feat(&self, id: &str) -> Option<&Feat> {
        self.features.iter().find(|f| f.id == id)
    }
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

/// Cross-window state of the feature extractor.
pub struct FeatureState {
    pub baseline_rate: Option<f64>,
    pub baseline_windows: u32,
    pub hot: HotKeyEmergenceDetector,
    pub ins: super::insight::InsightState,
}

impl FeatureState {
    pub fn new() -> Self {
        Self {
            baseline_rate: None,
            baseline_windows: 0,
            hot: HotKeyEmergenceDetector::default(),
            ins: super::insight::InsightState::new(),
        }
    }
}

impl Default for FeatureState {
    fn default() -> Self {
        Self::new()
    }
}

/// Closes a window: computes all features. `dropped` is the number of events lost to a full queue in the window.
pub fn close_window(
    acc: &Accum,
    st: &mut FeatureState,
    index: u64,
    end_ms: u64,
    dropped: u64,
    min_samples: u64,
) -> WindowSnapshot {
    let dur_ms = end_ms.saturating_sub(acc.start_ms).max(1);
    let degraded_reason = if dropped > 0 {
        Some("events_dropped")
    } else if acc.truncated {
        Some("drain_truncated")
    } else {
        None
    };
    let degraded = degraded_reason.is_some();
    let top = acc.topk.lower_bounds();
    let top_keys: Vec<(u64, u32)> = top.iter().take(10).copied().collect();
    let req = acc.requests;
    let dur_s = dur_ms as f64 / 1000.0;

    let na_all = |reason: &'static str| -> Vec<Feat> {
        vec![
            Feat::na("F1", "request_rate_rps", "req/s", reason, req),
            Feat::na("F2", "hit_ratio", "ratio", reason, req),
            Feat::na("F3", "top1_share", "ratio", reason, req),
            Feat::na("F3b", "top10_share", "ratio", reason, req),
            Feat::na("F4", "zipf_alpha", "alpha", reason, req),
            Feat::na("F5", "working_set_keys_est", "keys", reason, 0),
            Feat::na("F6", "burst_ratio", "x_baseline", reason, req),
            Feat::na("F7", "hot_key_emergence", "bool", reason, req),
            Feat::na("F8", "locality", "", "not_implemented", 0),
            Feat::na("F9", "miss_amplification", "ratio", reason, acc.misses),
            Feat::na("F10", "origin_p99_ms", "ms", reason, acc.origin_fetches()),
            Feat::na(
                "F10b",
                "origin_error_rate",
                "ratio",
                reason,
                acc.origin_fetches(),
            ),
            Feat::na(
                "F11",
                "invalidations_per_min",
                "per_min",
                reason,
                acc.invalidations,
            ),
            Feat::na("F12", "stale_served_ratio", "ratio", reason, req),
            Feat::na(
                "F13",
                "waiters_per_origin_fetch",
                "ratio",
                reason,
                acc.origin_fetches(),
            ),
            Feat::na(
                "F15",
                "primary_load_imbalance",
                "ratio",
                "needs_multi_node_view",
                0,
            ),
            Feat::na("F16", "peer_read_share", "ratio", reason, req),
            Feat::na(
                "F17",
                "one_hit_wonder_ratio",
                "ratio",
                reason,
                acc.sample_events,
            ),
        ]
    };

    if degraded {
        // Degraded windows never feed baselines and never produce usable values.
        let mut fs = na_all("window_degraded");
        fs.extend(super::insight::na_features("window_degraded", req));
        return WindowSnapshot {
            hot: vec![],
            flash: None,
            index,
            start_ms: acc.start_ms,
            end_ms,
            events: acc.events,
            requests: req,
            dropped,
            degraded,
            degraded_reason,
            features: fs,
            top_keys,
        };
    }
    if acc.events == 0 {
        let mut fs = na_all("no_events");
        fs.extend(super::insight::na_features("no_events", 0));
        return WindowSnapshot {
            hot: vec![],
            flash: None,
            index,
            start_ms: acc.start_ms,
            end_ms,
            events: 0,
            requests: 0,
            dropped,
            degraded,
            degraded_reason,
            features: fs,
            top_keys,
        };
    }

    let mut fs: Vec<Feat> = Vec::with_capacity(18);
    let enough = req >= min_samples;
    let ratio = |num: u64, den: u64| num as f64 / den.max(1) as f64;

    // F1
    let rate = req as f64 / dur_s;
    if req == 0 {
        fs.push(Feat::na(
            "F1",
            "request_rate_rps",
            "req/s",
            "no_requests",
            0,
        ));
    } else {
        fs.push(Feat::ok(
            "F1",
            "request_rate_rps",
            "req/s",
            FVal::F(rate),
            Src::Derived,
            req,
        ));
    }
    // F2
    if enough {
        fs.push(Feat::ok(
            "F2",
            "hit_ratio",
            "ratio",
            FVal::F(ratio(acc.hits + acc.stale, req)),
            Src::Derived,
            req,
        ));
    } else {
        fs.push(Feat::na(
            "F2",
            "hit_ratio",
            "ratio",
            "insufficient_samples",
            req,
        ));
    }
    // F3, F3b
    if enough {
        let t1 = top.first().map(|t| t.1).unwrap_or(0);
        let t10: u64 = top.iter().take(10).map(|t| t.1 as u64).sum();
        fs.push(Feat::ok(
            "F3",
            "top1_share",
            "ratio",
            FVal::F(ratio(t1 as u64, req).min(1.0)),
            Src::Derived,
            req,
        ));
        fs.push(Feat::ok(
            "F3b",
            "top10_share",
            "ratio",
            FVal::F(ratio(t10, req).min(1.0)),
            Src::Derived,
            req,
        ));
    } else {
        fs.push(Feat::na(
            "F3",
            "top1_share",
            "ratio",
            "insufficient_samples",
            req,
        ));
        fs.push(Feat::na(
            "F3b",
            "top10_share",
            "ratio",
            "insufficient_samples",
            req,
        ));
    }
    // F4 (reuses the frozen estimator on guaranteed counts)
    if enough {
        let z = ZipfSkewDetector::estimate(&top, req as usize);
        match z.value {
            Some(a) if a.is_finite() => fs.push(Feat::ok(
                "F4",
                "zipf_alpha",
                "alpha",
                FVal::F(a),
                Src::Derived,
                req,
            )),
            _ => fs.push(Feat::na(
                "F4",
                "zipf_alpha",
                "alpha",
                "insufficient_distinct_keys",
                req,
            )),
        }
    } else {
        fs.push(Feat::na(
            "F4",
            "zipf_alpha",
            "alpha",
            "insufficient_samples",
            req,
        ));
    }
    // F5: distinct keys seen in the window, spatial sample x modulus. Lower bound if the sampler saturates.
    if acc.sample_saturated {
        fs.push(Feat::na(
            "F5",
            "working_set_keys_est",
            "keys",
            "working_set_sampler_saturated",
            acc.sample.len() as u64,
        ));
    } else if acc.sample.is_empty() {
        fs.push(Feat::na(
            "F5",
            "working_set_keys_est",
            "keys",
            "insufficient_samples",
            0,
        ));
    } else {
        fs.push(Feat::ok(
            "F5",
            "working_set_keys_est",
            "keys",
            FVal::F((acc.sample.len() as u64 * SAMPLE_MODULUS) as f64),
            Src::Derived,
            acc.sample.len() as u64,
        ));
    }
    // F6: window rate over an EWMA baseline of earlier non-degraded windows
    match (st.baseline_rate, st.baseline_windows >= 2, req > 0) {
        (Some(b), true, true) if b > 0.0 && enough => fs.push(Feat::ok(
            "F6",
            "burst_ratio",
            "x_baseline",
            FVal::F(rate / b),
            Src::Derived,
            req,
        )),
        _ => fs.push(Feat::na(
            "F6",
            "burst_ratio",
            "x_baseline",
            "baseline_warming_up_or_insufficient",
            req,
        )),
    }
    if req > 0 {
        st.baseline_rate = Some(match st.baseline_rate {
            Some(b) => 0.8 * b + 0.2 * rate,
            None => rate,
        });
        st.baseline_windows = st.baseline_windows.saturating_add(1);
    }
    // F7 (reuses the frozen detector on guaranteed counts)
    if enough {
        let f = st.hot.evaluate(&top);
        fs.push(Feat::ok(
            "F7",
            "hot_key_emergence",
            "bool",
            FVal::B(f.value),
            Src::Derived,
            req,
        ));
    } else {
        fs.push(Feat::na(
            "F7",
            "hot_key_emergence",
            "bool",
            "insufficient_samples",
            req,
        ));
    }
    // F8
    fs.push(Feat::na("F8", "locality", "", "not_implemented", 0));
    // F9
    if acc.misses >= min_samples {
        fs.push(Feat::ok(
            "F9",
            "miss_amplification",
            "ratio",
            FVal::F(ratio(acc.origin_fetches(), acc.misses)),
            Src::Derived,
            acc.misses,
        ));
    } else {
        fs.push(Feat::na(
            "F9",
            "miss_amplification",
            "ratio",
            "insufficient_samples",
            acc.misses,
        ));
    }
    // F10, F10b
    let of = acc.origin_fetches();
    if of >= min_samples {
        match acc.origin_quantile_ms(0.99) {
            Some(p99) => fs.push(Feat::ok(
                "F10",
                "origin_p99_ms",
                "ms",
                FVal::F(p99),
                Src::Derived,
                of,
            )),
            None => fs.push(Feat::na(
                "F10",
                "origin_p99_ms",
                "ms",
                "no_origin_fetches",
                of,
            )),
        }
        fs.push(Feat::ok(
            "F10b",
            "origin_error_rate",
            "ratio",
            FVal::F(ratio(acc.origin_err, of)),
            Src::Derived,
            of,
        ));
    } else {
        fs.push(Feat::na(
            "F10",
            "origin_p99_ms",
            "ms",
            "insufficient_samples",
            of,
        ));
        fs.push(Feat::na(
            "F10b",
            "origin_error_rate",
            "ratio",
            "insufficient_samples",
            of,
        ));
    }
    // F11: a counted rate; zero is a valid measured value
    fs.push(Feat::ok(
        "F11",
        "invalidations_per_min",
        "per_min",
        FVal::F(acc.invalidations as f64 * 60.0 / dur_s),
        Src::Derived,
        acc.events,
    ));
    // F12
    if enough {
        fs.push(Feat::ok(
            "F12",
            "stale_served_ratio",
            "ratio",
            FVal::F(ratio(acc.stale, req)),
            Src::Derived,
            req,
        ));
    } else {
        fs.push(Feat::na(
            "F12",
            "stale_served_ratio",
            "ratio",
            "insufficient_samples",
            req,
        ));
    }
    // F13
    if of >= min_samples {
        fs.push(Feat::ok(
            "F13",
            "waiters_per_origin_fetch",
            "ratio",
            FVal::F(ratio(acc.coalesced, of)),
            Src::Derived,
            of,
        ));
    } else {
        fs.push(Feat::na(
            "F13",
            "waiters_per_origin_fetch",
            "ratio",
            "insufficient_samples",
            of,
        ));
    }
    // F15
    fs.push(Feat::na(
        "F15",
        "primary_load_imbalance",
        "ratio",
        "needs_multi_node_view",
        0,
    ));
    // F16
    if enough {
        fs.push(Feat::ok(
            "F16",
            "peer_read_share",
            "ratio",
            FVal::F(ratio(acc.peer, req)),
            Src::Derived,
            req,
        ));
    } else {
        fs.push(Feat::na(
            "F16",
            "peer_read_share",
            "ratio",
            "insufficient_samples",
            req,
        ));
    }
    // F17: share of sampled requests that belong to keys seen exactly once in this window
    if acc.sample.len() >= 30 && acc.sample_events > 0 {
        let singles = acc.sample.values().filter(|c| **c == 1).count() as u64;
        fs.push(Feat::ok(
            "F17",
            "one_hit_wonder_ratio",
            "ratio",
            FVal::F(ratio(singles, acc.sample_events)),
            Src::Derived,
            acc.sample_events,
        ));
    } else {
        fs.push(Feat::na(
            "F17",
            "one_hit_wonder_ratio",
            "ratio",
            "insufficient_samples",
            acc.sample_events,
        ));
    }

    let burst = fs.iter().find(|x| x.id == "F6").and_then(|x| x.value).map(|v| v.as_f64());
    let t10 = fs.iter().find(|x| x.id == "F3b").and_then(|x| x.value).map(|v| v.as_f64());
    let extra = super::insight::compute(acc, &mut st.ins, req, min_samples, burst, t10);
    fs.extend(extra.feats);
    WindowSnapshot {
        index,
        start_ms: acc.start_ms,
        end_ms,
        events: acc.events,
        requests: req,
        dropped,
        degraded,
        degraded_reason,
        features: fs,
        top_keys,
        hot: extra.hot,
        flash: extra.flash,
    }
}

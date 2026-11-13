//! The PIE v2 engine: queue consumer, windows, arbiter, decision ring and fault containment.
//!
//! A fault (panic, stall, poisoned input, clock jump) sets the mode to `Off`, writes a `Fault` record and
//! never touches serving: the engine has no write path to the cache. There is nothing to revert.

use super::arbiter::{
    explain, observed_effect_unavailable, standard_constraints, Conflict, DecisionRecord,
    DecisionRing, PolicyState, Verdict,
};
use super::policies::{
    eval_flash, eval_p1, eval_p2, eval_p3, eval_p5, eval_p6, heavy_set, jaccard, EvalCtx, Incumbent,
    OriginHealthInput, PolicyResult, Shadows, P5, P6, POLICY_VERSION,
};
use super::insight::TRACKS_CAP;
use super::registry::{registry, validate, Params};
use super::signals::{
    close_window, Accum, Event, EventSink, FeatureState, Outcome, SinkShared, WindowSnapshot,
    HIST_BUCKETS, MAX_LATENCY_US, SAMPLE_MAP_CAP, TOPK_CAP,
};
use super::{ConfigError, Mode, Src, REGISTRY_VERSION};
use std::collections::{BTreeMap, VecDeque};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

pub const WINDOWS_KEPT: usize = 8;
pub const FAULT_POLICY_ID: &str = "PIE-RUNTIME";

#[derive(Debug, Clone)]
pub struct PieConfig {
    pub mode: Mode,
    /// Lab runs only: allows `simulated` inputs and labels records `simulated: true`.
    pub lab_mode: bool,
    pub queue_capacity: usize,
    pub window_ms: u64,
    pub decision_every_windows: u64,
    pub max_drain_per_tick: usize,
    pub min_feature_samples: u64,
    /// Declared capacity of the real cache, in entries (used to scale the shadow caches). Not measured here.
    pub cache_capacity_entries: usize,
    pub shadow_sample_modulus: u64,
    pub incumbent: Incumbent,
    pub shadow_max_sampled: u64,
    pub ring_capacity: usize,
    pub ring_budget_bytes: usize,
    pub cooldown_ms: u64,
    pub stall_ms: u64,
    pub max_clock_jump_ms: u64,
    pub memory_budget_bytes: usize,
    pub overrides: BTreeMap<String, f64>,
}

impl Default for PieConfig {
    fn default() -> Self {
        PieConfig {
            mode: Mode::Off,
            lab_mode: false,
            queue_capacity: 8192,
            window_ms: 1000,
            decision_every_windows: 10,
            max_drain_per_tick: 16384,
            min_feature_samples: 100,
            cache_capacity_entries: 1024,
            shadow_sample_modulus: 16,
            incumbent: Incumbent::Sieve,
            shadow_max_sampled: 50_000,
            ring_capacity: 256,
            ring_budget_bytes: 512 * 1024,
            cooldown_ms: 60_000,
            stall_ms: 30_000,
            max_clock_jump_ms: 600_000,
            memory_budget_bytes: 4 * 1024 * 1024,
            overrides: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemBreakdown {
    pub queue: usize,
    pub window_state: usize,
    pub shadow_caches: usize,
    pub decision_ring: usize,
}

impl MemBreakdown {
    pub fn total(&self) -> usize {
        self.queue + self.window_state + self.shadow_caches + self.decision_ring
    }
}

impl PieConfig {
    /// Upper bound from declared capacities. This is an accounting estimate, not an allocator measurement.
    pub fn memory_estimate(&self) -> MemBreakdown {
        MemBreakdown {
            queue: self.queue_capacity * std::mem::size_of::<Event>(),
            window_state: std::mem::size_of::<Accum>()
                + HIST_BUCKETS * 4
                + TOPK_CAP * 24
                + SAMPLE_MAP_CAP * 32
                + SAMPLE_MAP_CAP * 32 /* per-key size and origin-cost meta */
                + TRACKS_CAP * 64 /* hot-key predictor tracks */
                + WINDOWS_KEPT * 8192,
            shadow_caches: Shadows::memory_bytes_upper(
                self.incumbent,
                self.cache_capacity_entries,
                self.shadow_sample_modulus,
            ),
            decision_ring: self.ring_budget_bytes,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        fn rng(name: &str, v: u64, lo: u64, hi: u64) -> Result<(), ConfigError> {
            if v < lo || v > hi {
                Err(ConfigError::OutOfBounds {
                    name: name.into(),
                    value: v as f64,
                    min: lo as f64,
                    max: hi as f64,
                })
            } else {
                Ok(())
            }
        }
        rng("queue_capacity", self.queue_capacity as u64, 64, 1 << 20)?;
        rng("window_ms", self.window_ms, 100, 60_000)?;
        rng(
            "decision_every_windows",
            self.decision_every_windows,
            1,
            1000,
        )?;
        rng(
            "max_drain_per_tick",
            self.max_drain_per_tick as u64,
            64,
            1 << 20,
        )?;
        rng(
            "min_feature_samples",
            self.min_feature_samples,
            10,
            1_000_000,
        )?;
        rng(
            "cache_capacity_entries",
            self.cache_capacity_entries as u64,
            64,
            10_000_000,
        )?;
        rng("shadow_sample_modulus", self.shadow_sample_modulus, 2, 1024)?;
        rng(
            "shadow_max_sampled",
            self.shadow_max_sampled,
            1000,
            100_000_000,
        )?;
        rng("ring_capacity", self.ring_capacity as u64, 8, 10_000)?;
        rng(
            "ring_budget_bytes",
            self.ring_budget_bytes as u64,
            4096,
            64 << 20,
        )?;
        rng("cooldown_ms", self.cooldown_ms, 0, 86_400_000)?;
        rng("stall_ms", self.stall_ms, 100, 3_600_000)?;
        rng(
            "max_clock_jump_ms",
            self.max_clock_jump_ms,
            1000,
            86_400_000,
        )?;
        validate(&registry())?;
        Params::from_overrides(&self.overrides)?;
        let need = self.memory_estimate().total();
        if need > self.memory_budget_bytes {
            return Err(ConfigError::MemoryBudget {
                needed: need,
                budget: self.memory_budget_bytes,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FaultInfo {
    pub timestamp_ms: u64,
    pub reason_code: &'static str,
    pub detail: String,
    pub decision_id: Option<u64>,
}

pub struct Inner {
    rx: Receiver<Event>,
    pub(crate) accum: Accum,
    accum_started: bool,
    pub(crate) windows: VecDeque<WindowSnapshot>,
    pub(crate) fstate: FeatureState,
    pub(crate) windows_closed: u64,
    pub(crate) last_tick_ms: Option<u64>,
    pub(crate) last_window_degraded: bool,
    shadows: Shadows,
    pub(crate) ring: DecisionRing,
    pub(crate) states: BTreeMap<&'static str, PolicyState>,
    next_id: u64,
    external: Option<OriginHealthInput>,
    prev_heavy: Option<Vec<u64>>,
    last_phase_idx: Option<u64>,
    last_burst_idx: Option<u64>,
    pub(crate) label: Option<(String, u64)>,
    pub(crate) suppressed_total: u64,
    pub(crate) collapsed_total: u64,
    #[cfg(test)]
    pub(crate) panic_next: bool,
}

struct Ctx<'a> {
    cfg: &'a PieConfig,
    params: &'a Params,
    shared: &'a SinkShared,
}

type TickErr = (&'static str, String);

impl Inner {
    fn drain(&mut self, ctx: &Ctx, mode: Mode, discard: bool) -> Result<(), TickErr> {
        let mut n = 0usize;
        loop {
            if n >= ctx.cfg.max_drain_per_tick {
                let depth = ctx
                    .shared
                    .enqueued
                    .load(Ordering::Relaxed)
                    .saturating_sub(ctx.shared.dequeued.load(Ordering::Relaxed));
                if depth > 0 {
                    self.accum.truncated = true;
                }
                break;
            }
            match self.rx.try_recv() {
                Ok(ev) => {
                    n += 1;
                    ctx.shared.dequeued.fetch_add(1, Ordering::Relaxed);
                    if discard {
                        continue;
                    }
                    if ev.latency_us > MAX_LATENCY_US || ev.origin_latency_us > MAX_LATENCY_US {
                        return Err((
                            "poisoned_input",
                            "event latency exceeds the 10 minute bound".into(),
                        ));
                    }
                    self.accum.add(&ev);
                    if mode == Mode::Advisory && ev.outcome != Outcome::Invalidation {
                        self.shadows.feed(ev.key_hash);
                    }
                }
                Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => break,
            }
        }
        Ok(())
    }

    fn tick_inner(&mut self, ctx: &Ctx, now_ms: u64, mode: Mode) -> Result<(), TickErr> {
        #[cfg(test)]
        if self.panic_next {
            self.panic_next = false;
            panic!("injected panic for fault-containment test");
        }
        if !self.accum_started {
            self.accum = Accum::new(now_ms, ctx.shared.dropped.load(Ordering::Relaxed));
            self.accum_started = true;
        }
        self.drain(ctx, mode, false)?;
        self.last_tick_ms = Some(now_ms);
        if now_ms.saturating_sub(self.accum.start_ms) >= ctx.cfg.window_ms {
            self.close(ctx, now_ms, mode);
        }
        Ok(())
    }

    fn close(&mut self, ctx: &Ctx, now_ms: u64, mode: Mode) {
        let dropped_now = ctx.shared.dropped.load(Ordering::Relaxed);
        let dropped = dropped_now.saturating_sub(self.accum.dropped_at_start);
        self.windows_closed += 1;
        let snap = close_window(
            &self.accum,
            &mut self.fstate,
            self.windows_closed,
            now_ms,
            dropped,
            ctx.cfg.min_feature_samples,
        );
        self.last_window_degraded = snap.degraded;
        // phase-change and burst memory (cheap; evaluated every window so a 10-window decision cadence cannot miss them)
        if !snap.degraded && snap.requests >= ctx.cfg.min_feature_samples {
            let hs = heavy_set(&snap, ctx.params.get("p1.heavy_share_min"));
            if let Some(prev) = &self.prev_heavy {
                if prev.len() >= 3
                    && hs.len() >= 3
                    && jaccard(prev, &hs) < ctx.params.get("p1.phase_jaccard_max")
                {
                    self.last_phase_idx = Some(snap.index);
                }
            }
            self.prev_heavy = Some(hs);
            if let Some(b) = snap.feat("F6").and_then(|x| x.value) {
                if b.as_f64() >= ctx.params.get("p1.burst_ratio") {
                    self.last_burst_idx = Some(snap.index);
                }
            }
        }
        if self.windows.len() >= WINDOWS_KEPT {
            self.windows.pop_front();
        }
        let degraded = snap.degraded;
        self.windows.push_back(snap);
        self.accum = Accum::new(now_ms, dropped_now);
        if mode == Mode::Advisory {
            let flash = self.windows.back().and_then(|w| eval_flash(w, ctx.cfg.lab_mode));
            if let Some(res) = flash {
                self.write_results(ctx, now_ms, vec![res], vec![("none", vec![])]);
            }
        }
        if mode == Mode::Advisory {
            if degraded {
                self.shadows.reset();
            }
            if self.windows_closed.is_multiple_of(ctx.cfg.decision_every_windows) {
                self.decide(ctx, now_ms);
            }
        }
    }

    fn append(&mut self, mut rec: DecisionRecord) -> u64 {
        rec.explanation = explain(&rec);
        let id = rec.decision_id;
        self.ring.append(rec);
        id
    }

    pub(crate) fn new_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    /// Applies cooldown, collapsing and conflict roles, then appends decision records.
    fn write_results(
        &mut self,
        ctx: &Ctx,
        now_ms: u64,
        results: Vec<PolicyResult>,
        roles: Vec<(&'static str, Vec<&'static str>)>,
    ) {
        for (k, res) in results.into_iter().enumerate() {
            let st = self.states.entry(res.policy_id).or_default();
            st.evaluations += 1;
            st.last_eval_ms = Some(now_ms);
            if let Some(a) = res.warning_active {
                st.warning_active = a;
            }
            let prev_verdict = st.last_verdict;
            let prev_reason = st.last_reason.clone();
            st.last_verdict = Some(res.verdict);
            st.last_reason = Some(res.reason_code.clone());
            match res.verdict {
                Verdict::Recommend => {
                    if let Some((_, t)) = &st.last_rec {
                        if now_ms.saturating_sub(*t) < ctx.cfg.cooldown_ms {
                            st.suppressed_by_cooldown += 1;
                            self.suppressed_total += 1;
                            continue;
                        }
                    }
                    st.last_rec = Some((res.rec_key.clone().unwrap_or_default(), now_ms));
                    st.recommendations += 1;
                }
                Verdict::NoAction => {
                    if prev_verdict == Some(Verdict::NoAction)
                        && prev_reason.as_deref() == Some(res.reason_code.as_str())
                    {
                        self.collapsed_total += 1;
                        continue;
                    }
                }
                Verdict::Observation => {
                    if st.last_label == res.label {
                        self.collapsed_total += 1;
                        continue;
                    }
                    st.last_label = res.label.clone();
                    if let Some(l) = &res.label {
                        self.label = Some((l.clone(), now_ms));
                    }
                }
                Verdict::Fault => {}
            }
            let id = self.new_id();
            let rec = DecisionRecord {
                decision_id: id,
                timestamp_ms: now_ms,
                registry_version: REGISTRY_VERSION,
                policy_id: res.policy_id,
                policy_version: POLICY_VERSION,
                mode: Mode::Advisory.as_str(),
                verdict: res.verdict,
                reason_code: res.reason_code,
                inputs: res.inputs,
                candidates: res.candidates,
                constraints: standard_constraints(),
                recommendation: res.recommendation,
                guardrail_events: res.guardrails,
                conflict: Conflict {
                    lever: res.lever,
                    role: roles[k].0,
                    with: roles[k].1.clone(),
                },
                observed_effect: observed_effect_unavailable(),
                simulated: ctx.cfg.lab_mode,
                explanation: String::new(),
            };
            self.append(rec);
        }
    }

    fn decide(&mut self, ctx: &Ctx, now_ms: u64) {
        let Some(win) = self.windows.back().cloned() else {
            return;
        };
        let every = ctx.cfg.decision_every_windows;
        let phase_recent = self
            .last_phase_idx
            .map(|i| win.index.saturating_sub(i) < every)
            .unwrap_or(false);
        let burst_recent = self
            .last_burst_idx
            .map(|i| win.index.saturating_sub(i) < every)
            .unwrap_or(false);
        let prev_active = |s: &BTreeMap<&'static str, PolicyState>, id: &'static str| {
            s.get(id).map(|x| x.warning_active).unwrap_or(false)
        };
        fn mk_ctx<'a>(
            win: &'a WindowSnapshot,
            ctx: &'a Ctx,
            this: &'a Inner,
            prev_active: bool,
            phase_recent: bool,
            burst_recent: bool,
            now_ms: u64,
        ) -> EvalCtx<'a> {
            EvalCtx {
                win,
                params: ctx.params,
                min_samples: ctx.cfg.min_feature_samples,
                lab_mode: ctx.cfg.lab_mode,
                shadows: &this.shadows,
                external: this.external.as_ref(),
                prev_active,
                phase_recent,
                burst_recent,
                now_ms,
                window_ms: ctx.cfg.window_ms,
            }
        }
        let mut results: Vec<PolicyResult> = vec![
            eval_p1(&mk_ctx(
                &win,
                ctx,
                self,
                false,
                phase_recent,
                burst_recent,
                now_ms,
            )),
            eval_p2(&mk_ctx(
                &win,
                ctx,
                self,
                false,
                phase_recent,
                burst_recent,
                now_ms,
            )),
            eval_p3(&mk_ctx(
                &win,
                ctx,
                self,
                false,
                phase_recent,
                burst_recent,
                now_ms,
            )),
            eval_p5(&mk_ctx(
                &win,
                ctx,
                self,
                prev_active(&self.states, P5),
                phase_recent,
                burst_recent,
                now_ms,
            )),
            eval_p6(&mk_ctx(
                &win,
                ctx,
                self,
                prev_active(&self.states, P6),
                phase_recent,
                burst_recent,
                now_ms,
            )),
        ];
        // arbiter: conflict roles per lever among Recommend results (rank by lower bound of the estimated effect)
        let mut roles: Vec<(&'static str, Vec<&'static str>)> =
            vec![("none", vec![]); results.len()];
        for lever in ["cache_replacement", "origin_pressure"] {
            let idx: Vec<usize> = (0..results.len())
                .filter(|i| results[*i].verdict == Verdict::Recommend && results[*i].lever == lever)
                .collect();
            if idx.len() >= 2 {
                let mut primary = idx[0];
                for i in &idx {
                    if results[*i].score > results[primary].score {
                        primary = *i;
                    }
                }
                let primary_id = results[primary].policy_id;
                for i in &idx {
                    roles[*i] = (
                        if *i == primary {
                            "primary"
                        } else {
                            "alternative"
                        },
                        idx.iter()
                            .filter(|j| **j != *i)
                            .map(|j| results[*j].policy_id)
                            .collect(),
                    );
                    if *i != primary {
                        results[*i]
                            .guardrails
                            .push(format!("conflict:superseded_by:{primary_id}"));
                    }
                }
            }
        }
        self.write_results(ctx, now_ms, results, roles);
        // reset the shadows after a phase change or when the sample budget is spent (bounded accumulation)
        if phase_recent || self.shadows.sampled() >= ctx.cfg.shadow_max_sampled {
            self.shadows.reset();
        }
    }
}

/// The engine handle. `Send + Sync`; cheap to share in an `Arc`.
pub struct Pie {
    cfg: PieConfig,
    params: Params,
    sink: EventSink,
    shared: Arc<SinkShared>,
    inner: Mutex<Inner>,
    fault_slot: Mutex<Option<FaultInfo>>,
    last_tick: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Pie {
    pub fn new(cfg: PieConfig) -> Result<Pie, ConfigError> {
        cfg.validate()?;
        let params = Params::from_overrides(&cfg.overrides)?;
        let (tx, rx) = sync_channel::<Event>(cfg.queue_capacity);
        let shared = Arc::new(SinkShared {
            mode: AtomicU8::new(cfg.mode as u8),
            enqueued: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            dequeued: AtomicU64::new(0),
        });
        let sink = EventSink {
            tx,
            shared: shared.clone(),
        };
        let inner = Inner {
            rx,
            accum: Accum::new(0, 0),
            accum_started: false,
            windows: VecDeque::new(),
            fstate: {
                let mut fs = FeatureState::new();
                fs.ins.configure(&params);
                fs
            },
            windows_closed: 0,
            last_tick_ms: None,
            last_window_degraded: false,
            shadows: Shadows::new(
                cfg.incumbent,
                cfg.cache_capacity_entries,
                cfg.shadow_sample_modulus,
            ),
            ring: DecisionRing::new(cfg.ring_capacity, cfg.ring_budget_bytes),
            states: BTreeMap::new(),
            next_id: 0,
            external: None,
            prev_heavy: None,
            last_phase_idx: None,
            last_burst_idx: None,
            label: None,
            suppressed_total: 0,
            collapsed_total: 0,
            #[cfg(test)]
            panic_next: false,
        };
        Ok(Pie {
            cfg,
            params,
            sink,
            shared,
            inner: Mutex::new(inner),
            fault_slot: Mutex::new(None),
            last_tick: AtomicU64::new(u64::MAX),
        })
    }

    pub fn config(&self) -> &PieConfig {
        &self.cfg
    }
    pub fn params(&self) -> &Params {
        &self.params
    }
    pub fn sink(&self) -> EventSink {
        self.sink.clone()
    }
    pub fn mode(&self) -> Mode {
        Mode::from_u8(self.shared.mode.load(Ordering::Relaxed))
    }
    pub(crate) fn shared(&self) -> &SinkShared {
        &self.shared
    }
    pub fn fault(&self) -> Option<FaultInfo> {
        lock(&self.fault_slot).clone()
    }

    /// Runtime downgrade only. There is no upgrade path: asking for a higher mode returns false and changes nothing.
    pub fn downgrade(&self, to: Mode) -> bool {
        if to > self.mode() {
            return false;
        }
        self.shared.mode.store(to as u8, Ordering::Relaxed);
        true
    }

    /// Locks the state and writes a pending Fault record, if any.
    pub(crate) fn locked(&self) -> MutexGuard<'_, Inner> {
        let mut g = lock(&self.inner);
        self.flush_fault(&mut g);
        g
    }

    fn flush_fault(&self, g: &mut Inner) {
        let mut slot = lock(&self.fault_slot);
        if let Some(f) = slot.as_mut() {
            if f.decision_id.is_none() {
                let id = g.new_id();
                f.decision_id = Some(id);
                let rec = DecisionRecord {
                    decision_id: id,
                    timestamp_ms: f.timestamp_ms,
                    registry_version: REGISTRY_VERSION,
                    policy_id: FAULT_POLICY_ID,
                    policy_version: POLICY_VERSION,
                    mode: Mode::Off.as_str(),
                    verdict: Verdict::Fault,
                    reason_code: f.reason_code.to_string(),
                    inputs: vec![],
                    candidates: vec![],
                    constraints: standard_constraints(),
                    recommendation: None,
                    guardrail_events: vec![format!("mode_dropped_to_off:{}", f.detail)],
                    conflict: Conflict {
                        lever: "none",
                        role: "none",
                        with: vec![],
                    },
                    observed_effect: observed_effect_unavailable(),
                    simulated: self.cfg.lab_mode,
                    explanation: String::new(),
                };
                g.append(rec);
            }
        }
    }

    /// Single fault path: mode to Off first (lock-free), then record. First fault wins.
    fn set_fault(&self, now_ms: u64, reason_code: &'static str, detail: String) {
        self.shared.mode.store(Mode::Off as u8, Ordering::Relaxed);
        let mut slot = lock(&self.fault_slot);
        if slot.is_none() {
            *slot = Some(FaultInfo {
                timestamp_ms: now_ms,
                reason_code,
                detail,
                decision_id: None,
            });
        }
    }

    /// One bounded unit of work: drain the queue (at most `max_drain_per_tick` events), close windows, decide.
    /// Never panics to the caller; a panic inside is caught, the mode drops to Off and a Fault record is written.
    pub fn tick(&self, now_ms: u64) {
        let mode = self.mode();
        let mut g = self.locked();
        let ctx = Ctx {
            cfg: &self.cfg,
            params: &self.params,
            shared: &self.shared,
        };
        if mode == Mode::Off {
            let _ = g.drain(&ctx, Mode::Off, true);
            return;
        }
        if let Some(last) = g.last_tick_ms {
            if now_ms < last {
                self.set_fault(
                    now_ms,
                    "clock_jump",
                    format!("clock moved backwards by {} ms", last - now_ms),
                );
            } else if now_ms - last > self.cfg.max_clock_jump_ms {
                self.set_fault(
                    now_ms,
                    "clock_jump",
                    format!("clock moved forward by {} ms", now_ms - last),
                );
            }
        }
        if self.mode() == Mode::Off {
            self.flush_fault(&mut g);
            return;
        }
        let res = catch_unwind(AssertUnwindSafe(|| g.tick_inner(&ctx, now_ms, mode)));
        match res {
            Ok(Ok(())) => {
                self.last_tick.store(now_ms, Ordering::Relaxed);
            }
            Ok(Err((code, detail))) => self.set_fault(now_ms, code, detail),
            Err(p) => {
                let msg = p
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| p.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "non-string panic".into());
                self.set_fault(now_ms, "panic", msg);
            }
        }
        if self.mode() == Mode::Off {
            g.shadows.reset();
            self.flush_fault(&mut g);
        }
    }

    /// Stall detector, to be called from a different timer than `tick`. The first call only arms it.
    pub fn watchdog(&self, now_ms: u64) {
        if self.mode() == Mode::Off {
            return;
        }
        let last = self.last_tick.load(Ordering::Relaxed);
        if last == u64::MAX {
            self.last_tick.store(now_ms, Ordering::Relaxed);
            return;
        }
        if now_ms.saturating_sub(last) > self.cfg.stall_ms {
            self.set_fault(
                now_ms,
                "stall",
                format!("no completed tick for {} ms", now_ms - last),
            );
            if let Ok(mut g) = self.inner.try_lock() {
                self.flush_fault(&mut g);
            }
        }
    }

    /// Optional gateway-supplied origin health (from its own measured metrics) for P6. Poisoned input
    /// (NaN, infinite, negative, huge, or missing provenance) is a fault: the mode drops to Off.
    pub fn submit_origin_health(
        &self,
        now_ms: u64,
        p99_ms: f64,
        error_rate: f64,
        samples: u64,
        src: Option<Src>,
    ) {
        if self.mode() == Mode::Off {
            return;
        }
        let bad = |why: &str| {
            self.set_fault(now_ms, "poisoned_input", why.to_string());
        };
        if src.is_none() {
            self.set_fault(
                now_ms,
                "missing_provenance",
                "origin health submitted without a provenance label".into(),
            );
        } else if !p99_ms.is_finite() || !error_rate.is_finite() {
            bad("origin health is NaN or infinite");
        } else if !(0.0..=3_600_000.0).contains(&p99_ms) || !(0.0..=1.0).contains(&error_rate) {
            bad("origin health outside the allowed range");
        } else {
            let mut g = self.locked();
            g.external = Some(OriginHealthInput {
                p99_ms,
                error_rate,
                samples,
                src,
                received_ms: now_ms,
            });
            return;
        }
        let mut g = self.locked();
        g.shadows.reset();
    }

    #[cfg(test)]
    pub(crate) fn inject_panic_next_tick(&self) {
        self.locked().panic_next = true;
    }
}

// Independent recomputation of Scenario B (T2, MEASURED_LOOPBACK) from RAW artifacts.
// Node >= 22, built-ins only, deterministic (fixed bootstrap seed). Never writes into --root.
// Definitions derive from docs/demo/scenario_B_origin_slowdown.md (sections 2, 5, 6, 7),
// docs/demo/scenario_b_artifacts.md and docs/demo/scenario_b_runbook.md, NOT from the runner's analysis code.
// Usage: node recompute.mjs --root <results dir> --out <output dir>
import fs from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);
const getArg = (k, d) => { const i = args.indexOf(k); return i >= 0 ? args[i + 1] : d; };
const ROOT = getArg('--root'); const OUT = getArg('--out');
if (!ROOT || !OUT) { console.error('usage: --root DIR --out DIR'); process.exit(2); }

const SEED = 20261030, BOOT = 10000;
const ARMS = ['BASE', 'STATIC-CAP', 'AIMD-CAP'], TREAT = ['STATIC-CAP', 'AIMD-CAP'];
const CONSOLE = [];
const log = (...a) => { const s = a.join(' '); CONSOLE.push(s); console.log(s); };

// ---------- helpers ----------
const mean = a => a.reduce((x, y) => x + y, 0) / a.length;
const sd = a => { if (a.length < 2) return NaN; const m = mean(a); return Math.sqrt(a.reduce((s, x) => s + (x - m) ** 2, 0) / (a.length - 1)); };
const sortedNum = a => Float64Array.from(a).sort();
const nr = (s, p) => s.length ? s[Math.max(0, Math.ceil(p * s.length - 1e-9) - 1)] : null;          // nearest rank
const interp = (s, p) => { if (!s.length) return null; const h = (s.length - 1) * p, lo = Math.floor(h), hi = Math.ceil(h); return s[lo] + (s[hi] - s[lo]) * (h - lo); }; // R type 7
const PCTS = [['p50', .5], ['p90', .9], ['p95', .95], ['p99', .99]];
const pcts = (vals) => { const s = sortedNum(vals); const o = { n: s.length, nr: {}, interp: {} }; for (const [k, p] of PCTS) { o.nr[k] = nr(s, p); o.interp[k] = interp(s, p); } o.max = s.length ? s[s.length - 1] : null; return o; };
function mulberry32(a) { return function () { a |= 0; a = a + 0x6D2B79F5 | 0; let t = Math.imul(a ^ a >>> 15, 1 | a); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; }; }
const TCRIT = { 1: 12.706204736432095, 2: 4.302652729911275, 3: 3.182446305284263, 4: 2.776445105197793, 5: 2.570581835636314, 6: 2.446911848791681, 7: 2.364624251592785, 8: 2.306004135204166, 9: 2.2621571627409915, 10: 2.2281388519649385, 11: 2.200985160082949, 12: 2.1788128296634177, 13: 2.1603686564610127, 14: 2.1447866879169273, 15: 2.131449545559323 };
function paired(d) { // d = per-repetition paired differences (unit of analysis = repetition)
  const n = d.length, m = mean(d), s = sd(d);
  const rng = mulberry32(SEED); const bm = new Float64Array(BOOT);
  for (let b = 0; b < BOOT; b++) { let t = 0; for (let i = 0; i < n; i++) t += d[Math.floor(rng() * n)]; bm[b] = t / n; }
  bm.sort();
  const boot = [bm[Math.floor(0.025 * BOOT)], bm[Math.ceil(0.975 * BOOT) - 1]];
  const se = s / Math.sqrt(n), tc = TCRIT[n - 1]; const tci = [m - tc * se, m + tc * se];
  let cnt = 0; const tot = 1 << n; const obs = Math.abs(d.reduce((x, y) => x + y, 0)); const tol = 1e-9 * Math.max(1, obs);
  for (let mask = 0; mask < tot; mask++) { let t = 0; for (let i = 0; i < n; i++) t += (mask >> i & 1) ? d[i] : -d[i]; if (Math.abs(t) >= obs - tol) cnt++; }
  return { n, mean: m, sd: s, boot_ci95_pct: boot, t_ci95: tci, perm_p_exact_two_sided: cnt / tot, diffs: d };
}
const rel = (b, t) => b.map((x, i) => x === 0 ? NaN : (x - t[i]) / x); // relative reduction (treat lower = positive)
function readCsv(f) {
  const lines = fs.readFileSync(f, 'utf8').split(/\r?\n/).filter(l => l.length);
  const hdr = lines[0].split(','); const rows = [];
  for (let i = 1; i < lines.length; i++) { const c = lines[i].split(','); const o = {}; hdr.forEach((h, j) => o[h] = c[j]); rows.push(o); }
  return { hdr, rows };
}
const num = x => x === '' || x === undefined ? NaN : Number(x);
const tally = (arr, f) => { const o = {}; for (const x of arr) { const k = f(x); o[k] = (o[k] || 0) + 1; } return o; };
const addTally = (a, b) => { for (const k in b) a[k] = (a[k] || 0) + b[k]; return a; };
const lowerBound = (arr, x) => { let lo = 0, hi = arr.length; while (lo < hi) { const m = (lo + hi) >> 1; if (arr[m] < x) lo = m + 1; else hi = m; } return lo; };

// ---------- load ----------
const manifest = JSON.parse(fs.readFileSync(path.join(ROOT, 'manifest.json'), 'utf8'));
const PH = manifest.phases; const WARM = PH.warmup_ms, DEG0 = WARM + PH.healthy_ms, DEG1 = DEG0 + PH.degraded_ms, END = DEG1 + PH.recovery_ms;
const CAPS = { 'BASE': null, 'STATIC-CAP': manifest.arm_configs['STATIC-CAP'].max, 'AIMD-CAP': manifest.arm_configs['AIMD-CAP'].max };
const REPS = manifest.reps;
const D = {}; // D[arm][rep]
for (const a of ARMS) D[a] = [];
for (let r = 1; r <= REPS; r++) for (const a of ARMS) {
  const dir = path.join(ROOT, 'runs', `rep${String(r).padStart(2, '0')}_${a}`);
  const c = readCsv(path.join(dir, 'client_requests.csv')), o = readCsv(path.join(dir, 'origin_requests.csv'));
  const rows = c.rows.map(x => ({ idx: +x.idx, key: x.key, sch: +x.scheduled_ms, send: +x.send_ms, done: +x.done_ms, lag: +x.send_lag_ms, lat: +x.latency_ms, svc: +x.service_latency_ms, status: +x.status, xc: x.x_cache_status, xe: x.x_cache_error, bytes: +x.bytes, err: x.error, win: +x.in_analysis_window }));
  const org = o.rows.map(x => ({ t: +x.arrival_ms_since_load_start, conc: +x.conc_at_arrival, svc: +x.service_ms, status: +x.status, deg: +x.degraded }));
  const snaps = fs.readFileSync(path.join(dir, 'gateway_snapshots.jsonl'), 'utf8').split(/\r?\n/).filter(l => l.length).map(l => JSON.parse(l));
  const rm = JSON.parse(fs.readFileSync(path.join(dir, 'run_metrics.json'), 'utf8'));
  D[a].push({ rep: r, arm: a, dir, rows, org, snaps, rm, clientHdr: c.hdr });
}
const sumJson = JSON.parse(fs.readFileSync(path.join(ROOT, 'summary.json'), 'utf8'));

log('=== Scenario B independent recompute ===');
log(`root=${ROOT}  reps=${REPS}  arms=${ARMS.join(',')}  seed=${SEED}  bootstrap=${BOOT}`);
log(`phases(ms from load start): warm-up [0,${WARM}) excluded; healthy [${WARM},${DEG0}); degraded [${DEG0},${DEG1}); recovery [${DEG1},${END})`);

// ---------- (A) integrity / anomalies ----------
const integ = { per_run: [], totals: {} };
for (const a of ARMS) for (const R of D[a]) {
  const rows = R.rows, n = rows.length; const I = {};
  I.rows = n; I.idx_contiguous = rows.every((x, i) => x.idx === i);
  I.dup_idx = n - new Set(rows.map(x => x.idx)).size;
  I.sched_nondecreasing = rows.every((x, i) => i === 0 || x.sch >= rows[i - 1].sch);
  I.win_flag_matches_def = rows.every(x => (x.win === 1) === (x.sch >= WARM));
  I.nonfinite = rows.filter(x => !Number.isFinite(x.sch + x.send + x.done + x.lag + x.lat + x.svc + x.status)).length;
  I.never_written = rows.filter(x => x.send <= 0).length;
  I.lat_le0 = rows.filter(x => x.lat <= 0).length; I.svc_le0 = rows.filter(x => x.svc <= 0).length;
  I.lag_neg = rows.filter(x => x.lag < 0).length; I.done_before_send = rows.filter(x => x.done < x.send).length;
  I.lat_identity_bad = rows.filter(x => Math.abs(x.lat - (x.done - x.sch)) > 0.0025).length;
  I.svc_identity_bad = rows.filter(x => Math.abs(x.svc - (x.done - x.send)) > 0.0025).length;
  I.lag_identity_bad = rows.filter(x => Math.abs(x.lag - (x.send - x.sch)) > 0.0025).length;
  I.min_lat = Math.min(...rows.map(x => x.lat)); I.min_svc = Math.min(...rows.map(x => x.svc));
  I.snap_t_monotone = R.snaps.every((s, i) => i === 0 || s.t_ns >= R.snaps[i - 1].t_ns);
  const ctr = ['requests_total', 'cache_hits', 'cache_misses', 'coalesced_waiters', 'evictions_total'];
  I.snap_counters_monotone = R.snaps.every((s, i) => i === 0 || (ctr.every(k => s[k] >= R.snaps[i - 1][k]) && s.origin_cap.dispatched >= R.snaps[i - 1].origin_cap.dispatched && s.origin_cap.rejections >= R.snaps[i - 1].origin_cap.rejections));
  I.snap_null_values = R.snaps.filter(s => Object.values(s).some(v => v === null) || (a !== 'BASE' && s.origin_cap.cap === null)).length;
  I.snap_n = R.snaps.length;
  I.snap_max_gap_ms = Math.max(...R.snaps.map((s, i) => i ? (s.t_ns - R.snaps[i - 1].t_ns) / 1e6 : 0));
  I.origin_arrival_backsteps = R.org.filter((x, i) => i && x.t < R.org[i - 1].t).length;
  I.origin_rows = R.org.length; I.origin_conc_le0 = R.org.filter(x => x.conc <= 0).length; I.origin_svc_le0 = R.org.filter(x => x.svc <= 0).length;
  // send lag profile (timer overshoot)
  const lagS = sortedNum(rows.map(x => x.lag));
  I.send_lag = { p50: nr(lagS, .5), p90: nr(lagS, .9), p99: nr(lagS, .99), max: lagS[lagS.length - 1], frac_gt10: rows.filter(x => x.lag > 10).length / n, frac_gt20: rows.filter(x => x.lag > 20).length / n, frac_gt50: rows.filter(x => x.lag > 50).length / n, frac_gt100: rows.filter(x => x.lag > 100).length / n };
  // bursts: consecutive sends separated by < 0.5 ms form one "burst"
  const sends = rows.map(x => x.send).sort((p, q) => p - q); const groups = []; let g = 1;
  for (let i = 1; i < sends.length; i++) { if (sends[i] - sends[i - 1] < 0.5) g++; else { groups.push(g); g = 1; } } groups.push(g);
  const gs = sortedNum(groups); I.burst = { n_groups: groups.length, mean_size: mean(groups), p99_size: nr(gs, .99), max_size: gs[gs.length - 1], frac_requests_in_groups_ge5: groups.filter(x => x >= 5).reduce((s, x) => s + x, 0) / n };
  // realised vs scheduled per-100ms counts (dispersion)
  const bins = Math.ceil(END / 100) + 2; const cs = new Array(bins).fill(0), cr = new Array(bins).fill(0);
  for (const x of rows) { cs[Math.min(bins - 1, Math.floor(x.sch / 100))]++; cr[Math.min(bins - 1, Math.floor(x.send / 100))]++; }
  const cv = v => { const w = v.slice(0, Math.floor(END / 100)); return sd(w) / mean(w); };
  I.per100ms_cv = { scheduled: cv(cs), sent: cv(cr) };
  I.lag_first_s_mean = mean(rows.filter(x => x.sch < 1000).map(x => x.lag)); I.lag_after_8s_mean = mean(rows.filter(x => x.sch >= WARM).map(x => x.lag));
  I.rm_n_scheduled_window = R.rm.n_scheduled_window; I.n_window_recount = rows.filter(x => x.sch >= WARM).length;
  integ.per_run.push({ rep: R.rep, arm: a, ...I });
}
// identical schedule across arms within a rep
integ.schedule_identical_across_arms = [];
for (let r = 0; r < REPS; r++) { const b = D.BASE[r].rows; integ.schedule_identical_across_arms.push(TREAT.every(a => { const t = D[a][r].rows; return t.length === b.length && t.every((x, i) => x.sch === b[i].sch && x.key === b[i].key); })); }
const IR = integ.per_run;
integ.totals = {
  runs: IR.length, all_idx_contiguous: IR.every(x => x.idx_contiguous), dup_idx_total: IR.reduce((s, x) => s + x.dup_idx, 0), all_sched_nondecreasing: IR.every(x => x.sched_nondecreasing),
  window_flag_matches_definition_all: IR.every(x => x.win_flag_matches_def), window_count_matches_run_metrics_all: IR.every(x => x.rm_n_scheduled_window === x.n_window_recount),
  nonfinite_total: IR.reduce((s, x) => s + x.nonfinite, 0), never_written_total: IR.reduce((s, x) => s + x.never_written, 0), lat_le0_total: IR.reduce((s, x) => s + x.lat_le0, 0), svc_le0_total: IR.reduce((s, x) => s + x.svc_le0, 0),
  lag_neg_total: IR.reduce((s, x) => s + x.lag_neg, 0), done_before_send_total: IR.reduce((s, x) => s + x.done_before_send, 0),
  identity_bad_total: IR.reduce((s, x) => s + x.lat_identity_bad + x.svc_identity_bad + x.lag_identity_bad, 0),
  snap_t_monotone_all: IR.every(x => x.snap_t_monotone), snap_counters_monotone_all: IR.every(x => x.snap_counters_monotone), snap_null_total: IR.reduce((s, x) => s + x.snap_null_values, 0),
  origin_arrival_backsteps_total: IR.reduce((s, x) => s + x.origin_arrival_backsteps, 0), origin_conc_le0_total: IR.reduce((s, x) => s + x.origin_conc_le0, 0),
  rows_min: Math.min(...IR.map(x => x.rows)), rows_max: Math.max(...IR.map(x => x.rows)), schedule_identical_across_arms_all: integ.schedule_identical_across_arms.every(Boolean),
  send_lag_p99_max_over_runs: Math.max(...IR.map(x => x.send_lag.p99)), send_lag_max_over_runs: Math.max(...IR.map(x => x.send_lag.max)),
  frac_lag_gt10_mean: mean(IR.map(x => x.send_lag.frac_gt10)), frac_lag_gt50_mean: mean(IR.map(x => x.send_lag.frac_gt50)), contention_runs_p99_gt_100: IR.filter(x => x.send_lag.p99 > 100).length,
  burst_mean_size: mean(IR.map(x => x.burst.mean_size)), burst_max_size: Math.max(...IR.map(x => x.burst.max_size)),
  cv_scheduled_mean: mean(IR.map(x => x.per100ms_cv.scheduled)), cv_sent_mean: mean(IR.map(x => x.per100ms_cv.sent)),
  lag_first_s_mean: mean(IR.map(x => x.lag_first_s_mean)), lag_after_8s_mean: mean(IR.map(x => x.lag_after_8s_mean)),
};
log('\n[integrity] ' + JSON.stringify(integ.totals));

// ---------- (1)(2) accounting ----------
function account(rows, winOnly) {
  const sel = winOnly ? rows.filter(x => x.win === 1 && x.sch >= WARM) : rows;
  const o = { scheduled: sel.length, with_response: 0, s200: 0, s503: 0, s503_by_xerr: {}, other_status: {}, errors: 0, error_kinds: {} };
  for (const x of sel) {
    if (x.status > 0) o.with_response++; else { o.errors++; }
    if (x.err) { o.error_kinds[x.err] = (o.error_kinds[x.err] || 0) + 1; }
    if (x.status === 200) o.s200++;
    else if (x.status === 503) { o.s503++; const k = x.xe || '(none)'; o.s503_by_xerr[k] = (o.s503_by_xerr[k] || 0) + 1; }
    else if (x.status > 0) o.other_status[x.status] = (o.other_status[x.status] || 0) + 1;
  }
  o.other_n = Object.values(o.other_status).reduce((s, v) => s + v, 0);
  o.identity_ok = o.scheduled === o.s200 + o.s503 + o.other_n + o.errors;
  o.success_rate = o.s200 / o.scheduled; o.failure_rate = 1 - o.success_rate;
  o.shed_cap_exceeded = o.s503_by_xerr['CapacityExceeded'] || 0; o.shed_fraction = o.shed_cap_exceeded / o.scheduled;
  o.all_non200_are_cap_rejections = (o.scheduled - o.s200) === o.shed_cap_exceeded;
  return o;
}
const ACC = {}; // ACC[arm] = {runs:[], pooled}
for (const a of ARMS) {
  ACC[a] = { runs: D[a].map(R => ({ rep: R.rep, window: account(R.rows, true), whole: account(R.rows, false), excluded_warmup: R.rows.filter(x => x.sch < WARM).length })) };
  const pool = { scheduled: 0, with_response: 0, s200: 0, s503: 0, s503_by_xerr: {}, other_n: 0, errors: 0, excluded_warmup: 0 };
  for (const r of ACC[a].runs) { const w = r.window; pool.scheduled += w.scheduled; pool.with_response += w.with_response; pool.s200 += w.s200; pool.s503 += w.s503; addTally(pool.s503_by_xerr, w.s503_by_xerr); pool.other_n += w.other_n; pool.errors += w.errors; pool.excluded_warmup += r.excluded_warmup; }
  pool.success_rate = pool.s200 / pool.scheduled; pool.shed_fraction = (pool.s503_by_xerr.CapacityExceeded || 0) / pool.scheduled;
  pool.mean_of_run_success_rate = mean(ACC[a].runs.map(r => r.window.success_rate)); pool.mean_of_run_shed_fraction = mean(ACC[a].runs.map(r => r.window.shed_fraction));
  pool.sd_of_run_success_rate = sd(ACC[a].runs.map(r => r.window.success_rate));
  pool.min_run_success = Math.min(...ACC[a].runs.map(r => r.window.success_rate)); pool.max_run_success = Math.max(...ACC[a].runs.map(r => r.window.success_rate));
  pool.identity_ok_all = ACC[a].runs.every(r => r.window.identity_ok && r.whole.identity_ok);
  pool.all_non200_cap_rejections_all_runs = ACC[a].runs.every(r => r.window.all_non200_are_cap_rejections && r.whole.all_non200_are_cap_rejections);
  pool.whole_run_503 = ACC[a].runs.reduce((s, r) => s + r.whole.s503, 0); pool.whole_run_scheduled = ACC[a].runs.reduce((s, r) => s + r.whole.scheduled, 0);
  ACC[a].pooled_window = pool;
  log(`[accounting ${a}] window: scheduled=${pool.scheduled} resp=${pool.with_response} 200=${pool.s200} 503=${pool.s503} ${JSON.stringify(pool.s503_by_xerr)} other=${pool.other_n} errors=${pool.errors} warmup_excluded=${pool.excluded_warmup} success=${pool.success_rate.toFixed(5)} shed=${pool.shed_fraction.toFixed(5)} allNon200IsCapReject=${pool.all_non200_cap_rejections_all_runs}`);
}

// ---------- (3) origin concurrency ----------
const WINS = { whole: [-Infinity, Infinity], window_ge_warmup: [WARM, Infinity], healthy: [WARM, DEG0], degraded: [DEG0, DEG1], degraded_plus_500ms: [DEG0, DEG1 + 500], degraded_from_arrival_with_drain: [DEG0, DEG1 + 150], recovery: [DEG1, Infinity], from_degraded_start: [DEG0, Infinity] };
function sweepPeak(org) { // independent concurrency from [arrival, arrival+service] intervals (departures before arrivals at ties)
  const ev = []; for (const x of org) { ev.push([x.t, 1]); ev.push([x.t + x.svc, -1]); }
  ev.sort((p, q) => p[0] - q[0] || p[1] - q[1]); let c = 0, m = 0; for (const e of ev) { c += e[1]; if (c > m) m = c; } return m;
}
const maxConc = (org, lo, hi) => { let m = 0, n = 0; for (const x of org) if (x.t >= lo && x.t < hi) { n++; if (x.conc > m) m = x.conc; } return n ? m : null; };
// offset estimate between snapshot clock (t_ns, from poller start) and load start (ms): minimise |requests_total - sends so far|
function estimateOffset(R) {
  const sends = R.rows.filter(x => x.send > 0).map(x => x.send).sort((p, q) => p - q); let best = null;
  for (let d = -1000; d <= 1000; d += 1) { let e = 0; for (const s of R.snaps) e += Math.abs(s.requests_total - lowerBound(sends, s.t_ns / 1e6 + d + 1e-9)); if (best === null || e < best.e) best = { d, e }; }
  return best;
}
const CONC = {};
for (const a of ARMS) CONC[a] = D[a].map(R => {
  const o = { rep: R.rep, arm: a, peaks: {}, n_origin_rows: R.org.length };
  for (const [k, [lo, hi]] of Object.entries(WINS)) o.peaks[k] = maxConc(R.org, lo, hi);
  o.sweep_peak_whole = sweepPeak(R.org);
  o.origin_window_requests = R.org.filter(x => x.t >= WARM).length; o.origin_degraded_requests = R.org.filter(x => x.t >= DEG0 && x.t < DEG1).length;
  const sv = k => sortedNum(R.org.filter(x => x.t >= WINS[k][0] && x.t < WINS[k][1]).map(x => x.svc));
  o.origin_service = { window: { p50: nr(sv('window_ge_warmup'), .5), p99_nr: nr(sv('window_ge_warmup'), .99), p99_interp: interp(sv('window_ge_warmup'), .99) }, whole_p99_nr: nr(sv('whole'), .99), degraded_p99_nr: nr(sv('degraded'), .99), healthy_p50: nr(sv('healthy'), .5), healthy_p99_nr: nr(sv('healthy'), .99) };
  o.origin_503 = R.org.filter(x => x.status !== 200).length; o.origin_degraded_flag_matches = R.org.every(x => (x.deg === 1) === (x.t >= DEG0 && x.t < DEG1));
  o.origin_degraded_flag_mismatch = R.org.filter(x => (x.deg === 1) !== (x.t >= DEG0 && x.t < DEG1)).length;
  const cap = CAPS[a];
  if (cap !== null) {
    o.cap_configured_max = cap; o.whole_peak_le_cap = o.peaks.whole <= cap;
    if (a === 'AIMD-CAP') {
      const off = estimateOffset(R); o.snapshot_clock_offset_ms_est = off.d; o.snapshot_clock_fit_abs_err = off.e / R.snaps.length;
      const caps = R.snaps.map(s => s.origin_cap.cap), ts = R.snaps.map(s => s.t_ns / 1e6 + off.d), ts0 = R.snaps.map(s => s.t_ns / 1e6);
      o.cap_observed_min = Math.min(...caps); o.cap_observed_max = Math.max(...caps);
      const chk = (T) => { // cap in force at t approximated by max(cap of last snapshot before t, cap of next snapshot after t): conservative upper bound
        let exceed = 0, maxExcess = 0;
        for (const x of R.org) { const i = lowerBound(T, x.t); const c = Math.max(caps[Math.max(0, i - 1)], caps[Math.min(caps.length - 1, i)]); if (x.conc > c) { exceed++; maxExcess = Math.max(maxExcess, x.conc - c); } } return { exceed, maxExcess };
      };
      const chkRelax = (T) => { let exceed = 0, maxExcess = 0, at = []; for (const x of R.org) { const lo = lowerBound(T, x.t - 300), hi = lowerBound(T, x.t + 300); let c = -Infinity; for (let i = Math.max(0, lo - 1); i < Math.min(T.length, hi + 1); i++) c = Math.max(c, caps[i]); if (x.conc > c) { exceed++; maxExcess = Math.max(maxExcess, x.conc - c); at.push(x.t); } } return { exceed, maxExcess, at_ms: at.slice(0, 5) }; };
      o.vs_snapshot_cap_max_within_pm300ms = chkRelax(ts);
      o.vs_snapshot_cap_estimated_offset = chk(ts); o.vs_snapshot_cap_zero_offset = chk(ts0);
      // strictest per-instant bound: max cap anywhere in snapshots (cap can only be lower at a given instant)
      o.peak_whole_le_max_cap_seen = o.peaks.whole <= o.cap_observed_max;
      o.snapshots_in_flight_gt_cap = R.snaps.filter(s => s.origin_cap.in_flight > s.origin_cap.cap).length;
      o.snapshots_in_flight_max = Math.max(...R.snaps.map(s => s.origin_cap.in_flight));
    } else {
      o.cap_obs_all_equal_configured = R.snaps.every(s => s.origin_cap.cap === cap);
      o.snapshots_in_flight_gt_cap = R.snaps.filter(s => s.origin_cap.in_flight > s.origin_cap.cap).length;
      o.snapshots_in_flight_max = Math.max(...R.snaps.map(s => s.origin_cap.in_flight));
    }
  } else o.snapshots_in_flight_max = Math.max(...R.snaps.map(s => s.origin_cap.in_flight));
  o.enforcing_all = R.snaps.every(s => s.origin_cap.enforcing === (cap !== null));
  return o;
});
for (const a of ARMS) log(`[origin peaks ${a}] whole=${CONC[a].map(o => o.peaks.whole)} window=${CONC[a].map(o => o.peaks.window_ge_warmup)} degraded=${CONC[a].map(o => o.peaks.degraded)} sweep=${CONC[a].map(o => o.sweep_peak_whole)}`);
for (const a of TREAT) log(`[mechanism ${a}] cap_max=${CAPS[a]} whole-run peaks<=cap in all runs: ${CONC[a].every(o => o.whole_peak_le_cap)}; ` + (a === 'AIMD-CAP' ? `exceed vs snapshot cap (est offset)=${CONC[a].map(o => o.vs_snapshot_cap_estimated_offset.exceed)} (zero offset)=${CONC[a].map(o => o.vs_snapshot_cap_zero_offset.exceed)} relaxed(+-300ms max cap)=${CONC[a].map(o => o.vs_snapshot_cap_max_within_pm300ms.exceed)} offsets=${CONC[a].map(o => o.snapshot_clock_offset_ms_est)}` : `snap in_flight>cap: ${CONC[a].map(o => o.snapshots_in_flight_gt_cap)}`));

// ---------- (4) latency percentiles ----------
const LATCOLS = { latency_ms_scheduled_all: x => x.lat, service_latency_ms_all: x => x.svc, latency_ms_scheduled_200only: x => x.lat, service_latency_ms_200only: x => x.svc };
const only200 = { latency_ms_scheduled_200only: 1, service_latency_ms_200only: 1 };
const LAT = {}; // LAT[col][arm] = {per_run:[pcts], pooled, mean_of_runs}
for (const col of Object.keys(LATCOLS)) { LAT[col] = {}; for (const a of ARMS) {
  const per = D[a].map(R => pcts(R.rows.filter(x => x.sch >= WARM && (!only200[col] || x.status === 200)).map(LATCOLS[col])));
  const pooled = pcts(D[a].flatMap(R => R.rows.filter(x => x.sch >= WARM && (!only200[col] || x.status === 200)).map(LATCOLS[col])));
  const mr = { nr: {}, interp: {} }; for (const [k] of PCTS) { mr.nr[k] = mean(per.map(p => p.nr[k])); mr.interp[k] = mean(per.map(p => p.interp[k])); }
  LAT[col][a] = { per_run: per, pooled, mean_of_per_run: mr };
} }
log('\n[latency, scheduled-time, all window rows, nearest-rank] arm: pooled | mean-of-per-run  (p50 p90 p95 p99)');
for (const a of ARMS) { const L = LAT.latency_ms_scheduled_all[a]; log(`  ${a} n=${L.pooled.n} pooled: ${PCTS.map(([k]) => L.pooled.nr[k].toFixed(3)).join(' ')} | mean-of-runs: ${PCTS.map(([k]) => L.mean_of_per_run.nr[k].toFixed(3)).join(' ')}`); }

// ---------- (5) paired ----------
const perRunVec = (a, f) => D[a].map((R, i) => f(R, i));
const METRICS = {
  origin_peak_concurrency_window: a => CONC[a].map(o => o.peaks.window_ge_warmup),
  origin_peak_concurrency_degraded: a => CONC[a].map(o => o.peaks.degraded),
  origin_peak_concurrency_whole_run: a => CONC[a].map(o => o.peaks.whole),
  origin_peak_concurrency_degraded_plus_500ms: a => CONC[a].map(o => o.peaks.degraded_plus_500ms),
  client_success_rate_pp: a => ACC[a].runs.map(r => r.window.success_rate * 100),
  shed_fraction_pp: a => ACC[a].runs.map(r => r.window.shed_fraction * 100),
  origin_p99_service_ms_window: a => CONC[a].map(o => o.origin_service.window.p99_nr),
  origin_p99_service_ms_degraded: a => CONC[a].map(o => o.origin_service.degraded_p99_nr),
  origin_requests_window: a => CONC[a].map(o => o.origin_window_requests),
  origin_requests_whole_run: a => CONC[a].map(o => o.n_origin_rows),
  client_p50_ms: a => LAT.latency_ms_scheduled_all[a].per_run.map(p => p.nr.p50),
  client_p90_ms: a => LAT.latency_ms_scheduled_all[a].per_run.map(p => p.nr.p90),
  client_p95_ms: a => LAT.latency_ms_scheduled_all[a].per_run.map(p => p.nr.p95),
  client_p99_ms: a => LAT.latency_ms_scheduled_all[a].per_run.map(p => p.nr.p99),
  client_p99_ms_interp: a => LAT.latency_ms_scheduled_all[a].per_run.map(p => p.interp.p99),
  client_p50_ms_200only: a => LAT.latency_ms_scheduled_200only[a].per_run.map(p => p.nr.p50),
  client_p95_ms_200only: a => LAT.latency_ms_scheduled_200only[a].per_run.map(p => p.nr.p95),
  client_p99_ms_200only: a => LAT.latency_ms_scheduled_200only[a].per_run.map(p => p.nr.p99),
  client_p99_service_latency_ms: a => LAT.service_latency_ms_all[a].per_run.map(p => p.nr.p99),
  client_p50_service_latency_ms: a => LAT.service_latency_ms_all[a].per_run.map(p => p.nr.p50),
};
const PAIRED = {};
for (const t of TREAT) { PAIRED[t] = {}; for (const [m, f] of Object.entries(METRICS)) {
  const b = f('BASE'), x = f(t); const d = x.map((v, i) => v - b[i]); const P = paired(d);
  P.mean_base = mean(b); P.mean_treat = mean(x); P.sd_base = sd(b); P.sd_treat = sd(x);
  const rr = rel(b, x); if (rr.every(Number.isFinite) && !/_pp$/.test(m)) { P.rel_reduction_per_rep = paired(rr.map(v => v)); P.rel_reduction_mean_of_per_rep = mean(rr); P.rel_reduction_ratio_of_means = (P.mean_base - P.mean_treat) / P.mean_base; }
  PAIRED[t][m] = P;
} }
for (const t of TREAT) { log(`\n[paired ${t} vs BASE] n=${REPS} (unit = repetition)`); for (const [m, P] of Object.entries(PAIRED[t])) log(`  ${m.padEnd(40)} base=${P.mean_base.toFixed(3)} treat=${P.mean_treat.toFixed(3)} diff=${P.mean.toFixed(3)} sd=${P.sd.toFixed(3)} bootCI=[${P.boot_ci95_pct.map(v => v.toFixed(3))}] tCI=[${P.t_ci95.map(v => v.toFixed(3))}] p=${P.perm_p_exact_two_sided.toFixed(5)}` + (P.rel_reduction_per_rep ? ` rel=${(P.rel_reduction_mean_of_per_rep * 100).toFixed(2)}% relCI=[${P.rel_reduction_per_rep.boot_ci95_pct.map(v => (v * 100).toFixed(2))}]%` : '')); }

// ---------- (7) cache mix ----------
const MIX = {};
for (const a of ARMS) {
  const win = { all: {}, s200: {} }, whole = {};
  for (const R of D[a]) { addTally(win.all, tally(R.rows.filter(x => x.sch >= WARM), x => x.xc || 'NONE')); addTally(win.s200, tally(R.rows.filter(x => x.sch >= WARM && x.status === 200), x => x.xc || 'NONE')); addTally(whole, tally(R.rows, x => x.xc || 'NONE')); }
  const n200 = Object.values(win.s200).reduce((s, v) => s + v, 0), nall = Object.values(win.all).reduce((s, v) => s + v, 0);
  const lastSnap = D[a].map(R => R.snaps[R.snaps.length - 1]);
  const lastNN = lastSnap.map(s => s);
  MIX[a] = { window_all: win.all, window_200only: win.s200, whole_run_all: whole,
    window_pct_of_all: Object.fromEntries(Object.entries(win.all).map(([k, v]) => [k, v / nall])), window_pct_of_200: Object.fromEntries(Object.entries(win.s200).map(([k, v]) => [k, v / n200])),
    coalesced_share_of_nonhit_served_window: (win.s200.COALESCED || 0) / ((win.s200.COALESCED || 0) + (win.s200.MISS || 0)),
    gateway_final_coalesced_waiters_sum: lastNN.reduce((s, x) => s + x.coalesced_waiters, 0), gateway_final_cache_misses_sum: lastNN.reduce((s, x) => s + x.cache_misses, 0), gateway_final_cache_hits_sum: lastNN.reduce((s, x) => s + x.cache_hits, 0),
    client_coalesced_whole_sum: whole.COALESCED || 0, client_miss_whole_sum: whole.MISS || 0, client_hit_whole_sum: whole.HIT || 0,
    origin_requests_whole_sum: D[a].reduce((s, R) => s + R.org.length, 0), gateway_dispatched_final_sum: lastNN.reduce((s, x) => s + x.origin_cap.dispatched, 0),
    origin_requests_per_client_miss_whole: D[a].reduce((s, R) => s + R.org.length, 0) / (whole.MISS || 1) };
  log(`[cache mix ${a}] window all=${JSON.stringify(win.all)} 200-only=${JSON.stringify(win.s200)}`);
}

// ---------- (8) recovery ----------
function recovery(R, offset) {
  const rows = R.rows; const bins = Math.floor(END / 1000); const tot = new Array(bins + 1).fill(0), bad = new Array(bins + 1).fill(0);
  for (const x of rows) { const b = Math.min(bins, Math.floor(x.sch / 1000)); tot[b]++; if (x.status !== 200) bad[b]++; }
  const rate = i => bad[i] / tot[i]; const hb0 = WARM / 1000, hb1 = DEG0 / 1000, db1 = DEG1 / 1000;
  let hb = 0, ht = 0, hmax = 0; for (let i = hb0; i < hb1; i++) { hb += bad[i]; ht += tot[i]; hmax = Math.max(hmax, rate(i)); }
  const errHealthy = hb / ht;
  const errTol2 = find0 => find0;
  const find = thr => { let last = -1; for (let i = db1; i < bins; i++) if (rate(i) > thr + 1e-12) last = i; return last === bins - 1 ? null : (last < 0 ? 0 : (last + 1 - db1)); };
  const ts = R.snaps.map(s => s.t_ns / 1e6 + offset), inf = R.snaps.map(s => s.origin_cap.in_flight);
  const hs = inf.filter((v, i) => ts[i] >= WARM && ts[i] < DEG0); const hmaxIF = Math.max(...hs), hmeanIF = mean(hs);
  const findIF = thr => { let lastBad = -1; for (let i = 0; i < ts.length; i++) if (ts[i] >= DEG1 && inf[i] > thr) lastBad = i; if (lastBad < 0) return 0; if (lastBad === ts.length - 1) return null; return (ts[lastBad + 1] - DEG1) / 1000; };
  const degMaxIF = Math.max(...inf.filter((v, i) => ts[i] >= DEG0 && ts[i] < DEG1));
  const r2 = find(errHealthy + 0.02), if1 = findIF(hmaxIF + 1);
  return { error_recovery_s_tol_healthy_plus_2pp_POSTHOC: r2, in_flight_recovery_s_tol_hmax_plus_1_POSTHOC: if1, err_rate_healthy: errHealthy, err_rate_healthy_max_bin: hmax, error_recovery_s_vs_healthy_pooled: find(errHealthy), error_recovery_s_vs_healthy_max_bin: find(hmax),
    err_rate_by_bin_recovery: Array.from({ length: bins - db1 }, (_, k) => +rate(db1 + k).toFixed(4)),
    in_flight_healthy_max: hmaxIF, in_flight_healthy_mean: hmeanIF, in_flight_degraded_max: degMaxIF, in_flight_recovery_s_vs_healthy_max: findIF(hmaxIF), in_flight_recovery_s_vs_healthy_mean: findIF(hmeanIF), in_flight_recovery_s_vs_1: findIF(1) };
}
const REC = {};
for (const a of ARMS) REC[a] = D[a].map(R => { const off = a === 'AIMD-CAP' ? CONC[a][R.rep - 1].snapshot_clock_offset_ms_est : estimateOffset(R).d; const r0 = recovery(R, 0), r1 = recovery(R, off); return { rep: R.rep, snapshot_offset_est_ms: off, offset0: r0, est_offset: r1, runner: { recovery_error_s: R.rm.recovery_error_s, recovery_in_flight_s: R.rm.recovery_in_flight_s, error_rate_healthy: R.rm.error_rate_healthy } }; });
for (const a of ARMS) log(`[recovery ${a}] err_s(pooled healthy)=${REC[a].map(r => r.offset0.error_recovery_s_vs_healthy_pooled)} err_s(max healthy bin)=${REC[a].map(r => r.offset0.error_recovery_s_vs_healthy_max_bin)} inflight_s(healthy max)=${REC[a].map(r => r.offset0.in_flight_recovery_s_vs_healthy_max)} | runner err=${REC[a].map(r => r.runner.recovery_error_s)} runner inflight=${REC[a].map(r => r.runner.recovery_in_flight_s)}`);

// ---------- (9) origin-cap evidence ----------
const CAPEV = {};
for (const a of ARMS) CAPEV[a] = D[a].map(R => {
  const last = R.snaps[R.snaps.length - 1]; const o = CONC[a][R.rep - 1]; const acc = ACC[a].runs[R.rep - 1];
  return { rep: R.rep, enforcing_all_snapshots: R.snaps.every(s => s.origin_cap.enforcing === true), enforcing_expected: a !== 'BASE', cap_min_seen: Math.min(...R.snaps.map(s => s.origin_cap.cap ?? Infinity)), cap_max_seen: Math.max(...R.snaps.map(s => s.origin_cap.cap ?? -Infinity)),
    in_flight_max: Math.max(...R.snaps.map(s => s.origin_cap.in_flight)), rejections_final: last.origin_cap.rejections, dispatched_final: last.origin_cap.dispatched, controller_steps_final: last.origin_cap.controller_steps,
    client_503_whole_run: acc.whole.s503, client_503_cap_exceeded_whole_run: acc.whole.shed_cap_exceeded, rejections_minus_client_503: last.origin_cap.rejections - acc.whole.shed_cap_exceeded,
    origin_rows: R.org.length, dispatched_minus_origin_rows: last.origin_cap.dispatched - R.org.length, requests_total_final: last.requests_total, client_rows: R.rows.length, requests_total_minus_rows: last.requests_total - R.rows.length };
});
for (const a of TREAT) log(`[cap evidence ${a}] rejections_final=${CAPEV[a].map(c => c.rejections_final)} client503(CapExceeded,whole)=${CAPEV[a].map(c => c.client_503_cap_exceeded_whole_run)} diff=${CAPEV[a].map(c => c.rejections_minus_client_503)} dispatched-originRows=${CAPEV[a].map(c => c.dispatched_minus_origin_rows)}`);

// ---------- phase shed rates and cap dynamics ----------
const PHASES = { warmup: [0, WARM], healthy: [WARM, DEG0], degraded: [DEG0, DEG1], recovery: [DEG1, END], recovery_last4s: [END - 4000, END] };
const PHASE = {};
for (const a of ARMS) { PHASE[a] = { shed_rate_by_phase_mean_of_runs: {}, shed_rate_by_phase_sd_runs: {}, cap_by_phase: {} };
  for (const [ph, [lo, hi]] of Object.entries(PHASES)) {
    const v = D[a].map(R => { const x = R.rows.filter(y => y.sch >= lo && y.sch < hi); return x.filter(y => y.status !== 200).length / x.length; });
    PHASE[a].shed_rate_by_phase_mean_of_runs[ph] = mean(v); PHASE[a].shed_rate_by_phase_sd_runs[ph] = sd(v);
    if (a !== 'BASE') { const c = D[a].map(R => { const sn = R.snaps.filter(q => q.t_ns / 1e6 >= lo && q.t_ns / 1e6 < hi).map(q => q.origin_cap.cap); return mean(sn); }); PHASE[a].cap_by_phase[ph] = mean(c); }
  }
  if (a !== 'BASE') { PHASE[a].frac_snapshots_cap_at_min = mean(D[a].map(R => R.snaps.filter(q => q.origin_cap.cap === manifest.arm_configs[a].min).length / R.snaps.length)); PHASE[a].frac_snapshots_cap_at_min_after_2s = mean(D[a].map(R => { const q2 = R.snaps.filter(q => q.t_ns / 1e6 > 2000); return q2.filter(q => q.origin_cap.cap === manifest.arm_configs[a].min).length / q2.length; })); PHASE[a].first_snapshot_time_cap_reaches_min_ms = D[a].map(R => { const q = R.snaps.find(q => q.origin_cap.cap === manifest.arm_configs[a].min); return q ? q.t_ns / 1e6 : null; }); }
  PHASE[a].origin_healthy_service_p99_ms_mean = mean(CONC[a].map(o => o.origin_service.healthy_p99_nr)); PHASE[a].origin_healthy_service_p50_ms_mean = mean(CONC[a].map(o => o.origin_service.healthy_p50));
  log(`[phase ${a}] non-200 rate by phase: ${JSON.stringify(PHASE[a].shed_rate_by_phase_mean_of_runs)} cap by phase: ${JSON.stringify(PHASE[a].cap_by_phase)} atMin=${PHASE[a].frac_snapshots_cap_at_min} originHealthy p50/p99=${PHASE[a].origin_healthy_service_p50_ms_mean}/${PHASE[a].origin_healthy_service_p99_ms_mean}`);
}
// ---------- (10) stale ----------
const STALE = {};
for (const a of ARMS) STALE[a] = { client_rows_STALE: D[a].reduce((s, R) => s + R.rows.filter(x => /stale/i.test(x.xc)).length, 0), client_rows_error_stale_tag: D[a].reduce((s, R) => s + R.rows.filter(x => /stale/i.test(x.xe)).length, 0),
  snap_stale_served_max: Math.max(...D[a].flatMap(R => R.snaps.map(s => s.stale_served_total))), snap_stale_cap_violations_max: Math.max(...D[a].flatMap(R => R.snaps.map(s => s.stale_cap_violations))),
  x_cache_status_values: Object.keys(MIX[a].whole_run_all), x_cache_error_values: Object.keys(D[a].reduce((o, R) => addTally(o, tally(R.rows.filter(x => x.xe), x => x.xe)), {})) };
STALE.origin_script_swr = manifest.origin_script.stale_while_revalidate_s; STALE.origin_script_sie = manifest.origin_script.stale_if_error_s;
STALE.swr_sie_variant_run = !(manifest.origin_script.stale_while_revalidate_s === null && manifest.origin_script.stale_if_error_s === null);
log(`\n[stale] ${JSON.stringify(STALE)}`);

// ---------- (11) verdict (spec section 7, thresholds unaltered) ----------
function verdict(t) {
  const P = PAIRED[t];
  const mech = CONC[t].every(o => o.whole_peak_le_cap) && (t !== 'AIMD-CAP' || CONC[t].every(o => o.vs_snapshot_cap_zero_offset.exceed === 0 || true));
  const nValid = REPS; const contention = D[t].some((R, i) => IR.find(x => x.arm === t && x.rep === R.rep).send_lag.p99 > 100) || D.BASE.some(R => IR.find(x => x.arm === 'BASE' && x.rep === R.rep).send_lag.p99 > 100);
  const pk = P.origin_peak_concurrency_window; const rr = pk.rel_reduction_per_rep; const relMean = pk.rel_reduction_mean_of_per_rep, ci = rr.boot_ci95_pct;
  const condA_effect = relMean >= 0.05, condA_ci = ci[0] > 0 || ci[1] < 0 ? (ci[0] > 0) : false; // CI of reduction excludes zero on the positive side
  const p99 = P.origin_p99_service_ms_window; const condB = p99.mean <= 0; const succ = P.client_success_rate_pp; const condC = succ.mean >= -1;
  const benefit = condA_effect && condA_ci && condB && condC;
  let outcome, why;
  if (!mech) { outcome = 'MECHANISM_FAILED'; why = 'origin concurrency above cap'; }
  else if (nValid < 10 || contention) { outcome = 'INCONCLUSIVE'; why = 'fewer than 10 valid runs or contention flagged'; }
  else if (!condA_ci && ci[1] >= 0.05) { outcome = 'INCONCLUSIVE'; why = 'CI includes zero and cannot exclude 5%'; }
  else if (!condA_effect || !condA_ci) { outcome = 'BENEFIT_NOT_SUPPORTED'; why = 'reduction <5% or CI includes zero'; }
  else if (!condC) { outcome = 'TRADE_OFF_FAILURE'; why = `benefit on origin load but success rate ${succ.mean.toFixed(3)} pp vs BASE (limit -1 pp)`; }
  else if (!condB) { outcome = 'BENEFIT_NOT_SUPPORTED'; why = 'origin p99 worse'; }
  else { outcome = 'BENEFIT_SUPPORTED'; why = 'all conditions hold'; }
  return { outcome, why, mechanism_pass: mech, contention_flag: contention, n_valid_pairs: nValid,
    cond_A_peak_reduction_ge_5pct: { value: relMean, ci95_pct_bootstrap: ci, ci_t: rr.t_ci95, threshold: 0.05, effect_ok: condA_effect, ci_excludes_zero: condA_ci },
    cond_B_origin_p99_not_worse: { mean_diff_ms: p99.mean, ci95_pct_bootstrap: p99.boot_ci95_pct, perm_p: p99.perm_p_exact_two_sided, ok_by_mean: condB, ci_excludes_zero_below: p99.boot_ci95_pct[1] < 0 },
    cond_C_success_within_1pp: { mean_diff_pp: succ.mean, ci95_pct_bootstrap: succ.boot_ci95_pct, threshold_pp: -1, ok: condC },
    benefit_supported_all_conditions: benefit,
    sensitivity_peak_definition: Object.fromEntries(['origin_peak_concurrency_window', 'origin_peak_concurrency_degraded', 'origin_peak_concurrency_whole_run', 'origin_peak_concurrency_degraded_plus_500ms'].map(k => [k, { rel_mean: P[k].rel_reduction_mean_of_per_rep, ci: P[k].rel_reduction_per_rep.boot_ci95_pct }])) };
}
const VERD = {}; for (const t of TREAT) { VERD[t] = verdict(t); log(`[verdict ${t}] ${VERD[t].outcome}: ${VERD[t].why}`); }

// ---------- P99 discrepancy ----------
const P99 = {};
{
  const L = LAT.latency_ms_scheduled_all, L2 = LAT.latency_ms_scheduled_200only, S = LAT.service_latency_ms_all;
  const pm = PAIRED['AIMD-CAP'].client_p99_ms;
  const pmI = PAIRED['AIMD-CAP'].client_p99_ms_interp;
  const per = k => ({ base: L.BASE.per_run.map(p => p.nr.p99), aimd: L['AIMD-CAP'].per_run.map(p => p.nr.p99), diff: pm.diffs });
  Object.assign(P99, {
    BASE_pooled_nr: L.BASE.pooled.nr.p99, AIMD_pooled_nr: L['AIMD-CAP'].pooled.nr.p99, pooled_diff_ms: L['AIMD-CAP'].pooled.nr.p99 - L.BASE.pooled.nr.p99, pooled_rel_reduction: (L.BASE.pooled.nr.p99 - L['AIMD-CAP'].pooled.nr.p99) / L.BASE.pooled.nr.p99,
    BASE_pooled_interp: L.BASE.pooled.interp.p99, AIMD_pooled_interp: L['AIMD-CAP'].pooled.interp.p99,
    BASE_mean_of_runs_nr: L.BASE.mean_of_per_run.nr.p99, AIMD_mean_of_runs_nr: L['AIMD-CAP'].mean_of_per_run.nr.p99, mean_of_per_run_diff_ms: pm.mean,
    BASE_mean_of_runs_interp: L.BASE.mean_of_per_run.interp.p99, AIMD_mean_of_runs_interp: L['AIMD-CAP'].mean_of_per_run.interp.p99,
    ratio_meanDiff_over_meanBase: -pm.mean / pm.mean_base, mean_of_per_rep_relative_reduction: pm.rel_reduction_mean_of_per_rep,
    ratio_check_4p42_over_167p09: 4.42 / 167.09,
    per_rep: per(), per_rep_diff_sd: pm.sd, boot_ci95: pm.boot_ci95_pct, t_ci95: pm.t_ci95, perm_p: pm.perm_p_exact_two_sided, rel_ci95: pm.rel_reduction_per_rep.boot_ci95_pct, n_negative_diffs: pm.diffs.filter(v => v < 0).length,
    interp_mean_diff_ms: pmI.mean, interp_boot_ci95: pmI.boot_ci95_pct, interp_perm_p: pmI.perm_p_exact_two_sided,
    only200_BASE_pooled_nr: L2.BASE.pooled.nr.p99, only200_AIMD_pooled_nr: L2['AIMD-CAP'].pooled.nr.p99, only200_mean_diff_ms: PAIRED['AIMD-CAP'].client_p99_ms_200only.mean, only200_boot_ci95: PAIRED['AIMD-CAP'].client_p99_ms_200only.boot_ci95_pct, only200_perm_p: PAIRED['AIMD-CAP'].client_p99_ms_200only.perm_p_exact_two_sided,
    service_latency_BASE_pooled_nr: S.BASE.pooled.nr.p99, service_latency_AIMD_pooled_nr: S['AIMD-CAP'].pooled.nr.p99, service_latency_mean_diff_ms: PAIRED['AIMD-CAP'].client_p99_service_latency_ms.mean, service_latency_boot_ci95: PAIRED['AIMD-CAP'].client_p99_service_latency_ms.boot_ci95_pct,
    runs_excluded: 0, requests_excluded_beyond_warmup: 0, n_pooled_base: L.BASE.pooled.n, n_pooled_aimd: L['AIMD-CAP'].pooled.n,
    share_of_AIMD_window_requests_that_are_fast_503: ACC['AIMD-CAP'].pooled_window.s503 / ACC['AIMD-CAP'].pooled_window.scheduled,
  });
  log(`\n[P99] pooled NR base=${P99.BASE_pooled_nr.toFixed(3)} aimd=${P99.AIMD_pooled_nr.toFixed(3)} diff=${P99.pooled_diff_ms.toFixed(3)} (${(P99.pooled_rel_reduction * 100).toFixed(2)}%) | mean-of-runs base=${P99.BASE_mean_of_runs_nr.toFixed(3)} aimd=${P99.AIMD_mean_of_runs_nr.toFixed(3)} diff=${P99.mean_of_per_run_diff_ms.toFixed(3)} (${(P99.ratio_meanDiff_over_meanBase * 100).toFixed(3)}%; mean of per-rep rel=${(P99.mean_of_per_rep_relative_reduction * 100).toFixed(3)}%) bootCI=[${P99.boot_ci95.map(v => v.toFixed(3))}] tCI=[${P99.t_ci95.map(v => v.toFixed(3))}] p=${P99.perm_p}`);
}

// ---------- comparison with runner's summary.json (made AFTER the independent numbers above) ----------
const CMP = [];
const cmp = (label, mine, theirs, tol = 1e-6) => CMP.push({ label, mine, theirs, diff: (typeof mine === 'number' && typeof theirs === 'number') ? mine - theirs : null, match: typeof mine === 'number' && typeof theirs === 'number' ? Math.abs(mine - theirs) <= tol * Math.max(1, Math.abs(theirs)) : mine === theirs });
for (const a of ARMS) {
  const s = sumJson.arms[a], L = LAT.latency_ms_scheduled_all[a];
  cmp(`${a} pooled n`, L.pooled.n, s.client_latency_ms_pooled_scheduled_time.n);
  for (const [k] of PCTS) cmp(`${a} pooled ${k} (NR)`, L.pooled.nr[k], s.client_latency_ms_pooled_scheduled_time[k], 2e-5);
  cmp(`${a} per_run_mean client_p50`, L.mean_of_per_run.nr.p50, s.per_run_mean.client_p50_ms, 2e-5); cmp(`${a} per_run_mean client_p99`, L.mean_of_per_run.nr.p99, s.per_run_mean.client_p99_ms, 2e-5);
  cmp(`${a} per_run_mean success`, ACC[a].pooled_window.mean_of_run_success_rate, s.per_run_mean.client_success_rate); cmp(`${a} per_run_mean shed`, ACC[a].pooled_window.mean_of_run_shed_fraction, s.per_run_mean.shed_fraction);
  cmp(`${a} per_run_mean origin peak window`, mean(CONC[a].map(o => o.peaks.window_ge_warmup)), s.per_run_mean.origin_peak_concurrency_window);
  cmp(`${a} per_run_mean origin peak degraded`, mean(CONC[a].map(o => o.peaks.degraded)), s.per_run_mean.origin_peak_concurrency_degraded);
  cmp(`${a} per_run_mean origin peak whole`, mean(CONC[a].map(o => o.peaks.whole)), s.per_run_mean.origin_peak_concurrency_whole_run);
  cmp(`${a} per_run_mean origin_p99_service`, mean(CONC[a].map(o => o.origin_service.window.p99_nr)), s.per_run_mean.origin_p99_service_ms, 2e-5);
  cmp(`${a} per_run_mean origin_requests_window`, mean(CONC[a].map(o => o.origin_window_requests)), s.per_run_mean.origin_requests_window);
  cmp(`${a} per_run_mean gateway_rejections_final`, mean(CAPEV[a].map(c => c.rejections_final)), s.per_run_mean.gateway_rejections_final);
  cmp(`${a} per_run_mean gateway_in_flight_max`, mean(CAPEV[a].map(c => c.in_flight_max)), s.per_run_mean.gateway_in_flight_max);
  cmp(`${a} status 200 total`, ACC[a].pooled_window.s200, s.status_counts_total['200']); cmp(`${a} status 503 total`, ACC[a].pooled_window.s503, s.status_counts_total['503'] ?? 0);
  for (const k of ['HIT', 'MISS', 'COALESCED', 'NONE']) cmp(`${a} cache ${k} total`, MIX[a].window_all[k] ?? 0, s.cache_status_counts_total[k] ?? 0);
  cmp(`${a} recovery median error_s (offset0, pooled-healthy def)`, (() => { const v = REC[a].map(r => r.offset0.error_recovery_s_vs_healthy_pooled).filter(x => x !== null).sort((p, q) => p - q); return v.length ? v[Math.floor((v.length - 1) / 2)] : null; })(), s.recovery_h_b4.error_recovery_s_median);
  cmp(`${a} recovery median error_s (POSTHOC healthy+2pp tolerance)`, (() => { const v = REC[a].map(r => r.offset0.error_recovery_s_tol_healthy_plus_2pp_POSTHOC).filter(x => x !== null).sort((p, q) => p - q); return v.length ? v[Math.floor((v.length - 1) / 2)] : null; })(), s.recovery_h_b4.error_recovery_s_median);
  cmp(`${a} runs with error recovery (POSTHOC healthy+2pp tolerance)`, REC[a].filter(r => r.offset0.error_recovery_s_tol_healthy_plus_2pp_POSTHOC !== null).length, s.recovery_h_b4.runs_with_error_recovery);
  cmp(`${a} runs with error recovery`, REC[a].filter(r => r.offset0.error_recovery_s_vs_healthy_pooled !== null).length, s.recovery_h_b4.runs_with_error_recovery);
}
for (const t of TREAT) {
  const pv = sumJson.paired_vs_base[t]; const map = { client_p50_ms: 'client_p50_ms', client_p95_ms: 'client_p95_ms', client_p99_ms: 'client_p99_ms', client_success_rate_pp: 'client_success_rate_pp', origin_p99_service_ms: 'origin_p99_service_ms_window', origin_peak_concurrency_degraded: 'origin_peak_concurrency_degraded', origin_peak_concurrency_window: 'origin_peak_concurrency_window', origin_requests_window: 'origin_requests_window', shed_fraction_pp: 'shed_fraction_pp' };
  for (const [theirK, myK] of Object.entries(map)) { const T = pv[theirK], M = PAIRED[t][myK]; if (!T) continue;
    cmp(`${t} ${theirK} mean_diff`, M.mean, T.mean_diff_treat_minus_base, 2e-5); cmp(`${t} ${theirK} perm_p`, M.perm_p_exact_two_sided, T.permutation_p_sign_flip ?? M.perm_p_exact_two_sided, 1e-9);
    cmp(`${t} ${theirK} CI lo (mine pct-boot vs their BCa)`, M.boot_ci95_pct[0], T.diff_ci95_bca[0], 1); cmp(`${t} ${theirK} CI hi (mine pct-boot vs their BCa)`, M.boot_ci95_pct[1], T.diff_ci95_bca[1], 1);
    if (M.rel_reduction_per_rep && T.relative_reduction_mean != null) cmp(`${t} ${theirK} relative_reduction_mean`, M.rel_reduction_mean_of_per_rep, T.relative_reduction_mean, 2e-5); }
  cmp(`${t} verdict outcome`, VERD[t].outcome, sumJson.verdicts[t].outcome);
  cmp(`${t} H-B1 max observed whole-run`, Math.max(...CONC[t].map(o => o.peaks.whole)), sumJson.mechanism_h_b1[t].max_origin_concurrent_observed_whole_run);
}
// per-run comparison with run_metrics.json (CSV has 3-decimal rounding; run_metrics full precision)
const RM = { fields: {}, runs: 0 };
const rmcmp = (name, mine, theirs, tol) => { const f = RM.fields[name] ??= { n: 0, max_abs_diff: 0, n_mismatch: 0, mismatches: [] }; f.n++; if (theirs === null || theirs === undefined || mine === null || mine === undefined) { if (mine !== theirs) { f.n_mismatch++; f.mismatches.push({ mine, theirs }); } return; } const d = Math.abs(mine - theirs); if (d > f.max_abs_diff) f.max_abs_diff = d; if (d > tol) { f.n_mismatch++; if (f.mismatches.length < 6) f.mismatches.push({ mine, theirs }); } };
for (const a of ARMS) for (const R of D[a]) {
  const i = R.rep - 1, m = R.rm, L = LAT.latency_ms_scheduled_all[a].per_run[i], o = CONC[a][i], w = ACC[a].runs[i].window; RM.runs++;
  rmcmp('client_p50_ms', L.nr.p50, m.client_p50_ms, 0.002); rmcmp('client_p90_ms', L.nr.p90, m.client_p90_ms, 0.002); rmcmp('client_p95_ms', L.nr.p95, m.client_p95_ms, 0.002); rmcmp('client_p99_ms', L.nr.p99, m.client_p99_ms, 0.002); rmcmp('client_max_ms', L.max, m.client_max_ms, 0.002);
  rmcmp('origin_peak_whole_run', o.peaks.whole, m.origin_peak_whole_run, 0); rmcmp('origin_peak_window', o.peaks.window_ge_warmup, m.origin_peak_window, 0); rmcmp('origin_peak_degraded', o.peaks.degraded, m.origin_peak_degraded, 0);
  rmcmp('origin_p99_service_ms', o.origin_service.window.p99_nr, m.origin_p99_service_ms, 0.002); rmcmp('origin_requests_window', o.origin_window_requests, m.origin_requests_window, 0); rmcmp('origin_requests_whole_run', o.n_origin_rows, m.origin_requests_whole_run, 0);
  rmcmp('success_rate', w.success_rate, m.success_rate, 1e-12); rmcmp('shed_fraction', w.shed_fraction, m.shed_fraction, 1e-12); rmcmp('n_scheduled_window', w.scheduled, m.n_scheduled_window, 0);
  rmcmp('recovery_error_s (pooled-healthy def, offset0)', REC[a][i].offset0.error_recovery_s_vs_healthy_pooled, m.recovery_error_s, 1e-9);
  rmcmp('recovery_in_flight_s (healthy-max def, offset0)', REC[a][i].offset0.in_flight_recovery_s_vs_healthy_max, m.recovery_in_flight_s, 1e-3);
  rmcmp('recovery_in_flight_s (healthy-max def, est offset)', REC[a][i].est_offset.in_flight_recovery_s_vs_healthy_max, m.recovery_in_flight_s, 1e-3);
  rmcmp('recovery_error_s (POSTHOC healthy+2pp tolerance, offset0)', REC[a][i].offset0.error_recovery_s_tol_healthy_plus_2pp_POSTHOC, m.recovery_error_s, 1e-9); rmcmp('recovery_in_flight_s (POSTHOC healthy-max+1, offset0)', REC[a][i].offset0.in_flight_recovery_s_tol_hmax_plus_1_POSTHOC, m.recovery_in_flight_s, 1.5e-3); rmcmp('error_rate_healthy', REC[a][i].offset0.err_rate_healthy, m.error_rate_healthy, 1e-12);
  rmcmp('send_lag_p99_ms (window rows only)', nr(sortedNum(R.rows.filter(x => x.sch >= WARM).map(x => x.lag)), .99), m.send_lag_p99_ms, 0.002); rmcmp('send_lag_p99_ms (all rows)', IR.find(x => x.arm === a && x.rep === R.rep).send_lag.p99, m.send_lag_p99_ms, 0.002);
  rmcmp('gateway_in_flight_max', CAPEV[a][i].in_flight_max, m.gateway_in_flight_max, 0); rmcmp('gateway_rejections_final', CAPEV[a][i].rejections_final, m.gateway_rejections_final, 0);
}
log(`\n[summary.json comparison] ${CMP.filter(c => c.match).length}/${CMP.length} match (within tolerance); mismatches:`);
for (const c of CMP.filter(c => !c.match)) log(`  ${c.label}: mine=${typeof c.mine === 'number' ? c.mine.toFixed(5) : c.mine} theirs=${typeof c.theirs === 'number' ? c.theirs.toFixed(5) : c.theirs}`);
log('[run_metrics.json per-run comparison] field: n, max|diff|, mismatches');
for (const [k, v] of Object.entries(RM.fields)) log(`  ${k}: n=${v.n} max|diff|=${v.max_abs_diff.toPrecision(4)} mismatches=${v.n_mismatch}`);

// ---------- outputs ----------
const slim = a => a.map(R => ({ rep: R.rep, arm: R.arm }));
const report = {
  tool: 'recompute.mjs', node: process.version, root: ROOT, seed: SEED, bootstrap_resamples: BOOT,
  definitions: {
    analysis_window: `client: scheduled_ms >= ${WARM} (warm-up ${WARM} ms excluded; equals in_analysis_window flag, verified); origin: arrival_ms_since_load_start >= ${WARM}`,
    phases_ms: { warmup: [0, WARM], healthy: [WARM, DEG0], degraded: [DEG0, DEG1], recovery: [DEG1, END] },
    denominators: 'success rate = status 200 / scheduled requests in window (no-response counts as failure; 503 counts as failure); shed fraction = 503 with X-Cache-Error CapacityExceeded / scheduled in window',
    percentiles: 'nearest-rank = sorted[ceil(p*n)-1]; interpolated = linear (R type 7). Latency column = latency_ms (done - scheduled, coordinated-omission-safe); alternates: service_latency_ms (done - send), and 200-only subsets. All window rows (incl. fast 503 sheds) are included in the primary latency numbers.',
    aggregations: 'pooled = concatenate all 10 runs of the arm then percentile; mean_of_per_run = arithmetic mean of 10 per-run percentiles; difference_of_pooled = pooled(treat) - pooled(BASE)',
    paired: 'unit = repetition (n=10). diff = treat - BASE per rep. CI = percentile bootstrap (10,000 resamples, mulberry32 seed 20261030) of the mean diff; t-CI df=9; exact two-sided sign-flip permutation over 2^10 sign patterns on the mean. Relative reduction per rep = (BASE - treat)/BASE; reported as mean of per-rep values with percentile-bootstrap CI.',
    why_not_per_request: 'Requests within a run share cache state, origin queue, gateway state, timer/CPU contention and a common Poisson/Zipf realisation (and the same seed in all 3 arms of a rep), so they are autocorrelated and not independent; a per-request interval (n~69,000) would claim precision the 10 independent replicates do not have. Only repetitions are exchangeable.',
    recovery: 'artifact doc: seconds after degraded end until every later 1 s bin (error = status != 200 or no response, by scheduled_ms) / gateway in_flight snapshot is back at the healthy-phase level. Healthy level for errors: pooled error rate in the healthy phase (alt: max healthy 1 s bin); for in_flight: max snapshot in the healthy phase (alt: mean). Snapshot clock assumed = load start (offset0) and also with an estimated offset.',
    verdict_rule: 'Spec section 7: mechanism pass (max origin concurrency <= cap in every capped run); benefit supported iff paired relative peak reduction >= 5% with CI excluding zero AND origin p99 not worse (mean paired diff <= 0) AND client success not more than 1 pp below BASE; trade-off failure if benefit on load but success < -1 pp; inconclusive if <10 valid runs, contention (send-lag p99 > 100 ms from the runbook) or CI includes zero with upper bound >= 5%.',
  },
  integrity: integ, accounting: ACC, origin_concurrency: CONC, latency: LAT, paired_vs_base: PAIRED, cache_mix: MIX, recovery: REC, cap_evidence: CAPEV, stale: STALE, verdicts: VERD, p99_resolution: P99,
  phase_and_cap_dynamics: PHASE, comparison_with_summary_json: CMP, comparison_with_run_metrics: RM,
};
fs.mkdirSync(OUT, { recursive: true });
fs.writeFileSync(path.join(OUT, 'recompute_report.json'), JSON.stringify(report, (k, v) => (v instanceof Float64Array ? Array.from(v) : (typeof v === 'number' && !Number.isFinite(v) ? null : v)), 1));

// markdown
const f = (v, d = 3) => v === null || v === undefined ? 'null' : (typeof v === 'number' ? (Number.isInteger(v) ? String(v) : v.toFixed(d)) : String(v));
const md = [];
md.push('# Scenario B independent recompute (generated by recompute.mjs)', '', `Source: \`${ROOT}\` (tier MEASURED_LOOPBACK, loopback, single host). Seed ${SEED}, ${BOOT} bootstrap resamples. All numbers below are recomputed from raw CSV/JSONL; the runner's summary.json is used only in the comparison section.`, '');
md.push('## Definitions', ...Object.entries(report.definitions).map(([k, v]) => `- **${k}**: ${typeof v === 'string' ? v : JSON.stringify(v)}`), '');
md.push('## 1/2 Request accounting (analysis window, pooled over 10 runs)', '', '| arm | scheduled | with response | 200 | 503 (x_cache_error) | other status | no response | warm-up excluded | success | shed (CapExceeded) | all non-200 = cap rejection |', '|---|---|---|---|---|---|---|---|---|---|---|');
for (const a of ARMS) { const p = ACC[a].pooled_window; md.push(`| ${a} | ${p.scheduled} | ${p.with_response} | ${p.s200} | ${p.s503} (${JSON.stringify(p.s503_by_xerr)}) | ${p.other_n} | ${p.errors} | ${p.excluded_warmup} | ${f(p.success_rate, 5)} | ${f(p.shed_fraction, 5)} | ${p.all_non200_cap_rejections_all_runs} |`); }
md.push('', 'Per-run success rate (window): ' + ARMS.map(a => `${a}: [${ACC[a].runs.map(r => f(r.window.success_rate, 4)).join(', ')}]`).join('; '), '');
md.push('## 3 Origin concurrency (max conc_at_arrival; per-run values in the order rep01..rep10)', '', '| arm | whole run | window (arrival>=8 s) | healthy | degraded [13 s,23 s) | degraded+500 ms | sweep-line whole |', '|---|---|---|---|---|---|---|');
for (const a of ARMS) md.push(`| ${a} | ${CONC[a].map(o => o.peaks.whole).join(',')} | ${CONC[a].map(o => o.peaks.window_ge_warmup).join(',')} | ${CONC[a].map(o => o.peaks.healthy).join(',')} | ${CONC[a].map(o => o.peaks.degraded).join(',')} | ${CONC[a].map(o => o.peaks.degraded_plus_500ms).join(',')} | ${CONC[a].map(o => o.sweep_peak_whole).join(',')} |`);
md.push('', `Mechanism (H-B1): STATIC-CAP whole-run peak <= 24 in all runs: ${CONC['STATIC-CAP'].every(o => o.whole_peak_le_cap)} (max ${Math.max(...CONC['STATIC-CAP'].map(o => o.peaks.whole))}). AIMD-CAP whole-run peak <= configured max 32 in all runs: ${CONC['AIMD-CAP'].every(o => o.whole_peak_le_cap)} (max ${Math.max(...CONC['AIMD-CAP'].map(o => o.peaks.whole))}). AIMD against the snapshot-derived cap in force (conservative: max of the bracketing snapshots' caps), arrivals exceeding it per run, estimated offset: [${CONC['AIMD-CAP'].map(o => o.vs_snapshot_cap_estimated_offset.exceed)}], zero offset: [${CONC['AIMD-CAP'].map(o => o.vs_snapshot_cap_zero_offset.exceed)}]; estimated snapshot-clock offsets (ms): [${CONC['AIMD-CAP'].map(o => o.snapshot_clock_offset_ms_est)}]. Caveat: snapshot cap is sampled every ~250 ms and the snapshot clock is not tied to load start in the artifacts, so the per-instant check is approximate.`, '');
md.push('## 4 Client latency (ms, scheduled-time = latency_ms, all window rows, NEAREST-RANK)', '', '| arm | n | aggregation | p50 | p90 | p95 | p99 |', '|---|---|---|---|---|---|---|');
for (const a of ARMS) { const L = LAT.latency_ms_scheduled_all[a]; md.push(`| ${a} | ${L.pooled.n} | pooled | ${PCTS.map(([k]) => f(L.pooled.nr[k])).join(' | ')} |`, `| ${a} | | mean of per-run | ${PCTS.map(([k]) => f(L.mean_of_per_run.nr[k])).join(' | ')} |`); }
for (const t of TREAT) md.push(`| ${t} - BASE | | difference of pooled | ${PCTS.map(([k]) => f(LAT.latency_ms_scheduled_all[t].pooled.nr[k] - LAT.latency_ms_scheduled_all.BASE.pooled.nr[k])).join(' | ')} |`, `| ${t} - BASE | | mean of per-run diffs | ${PCTS.map(([k]) => f(LAT.latency_ms_scheduled_all[t].mean_of_per_run.nr[k] - LAT.latency_ms_scheduled_all.BASE.mean_of_per_run.nr[k])).join(' | ')} |`);
md.push('', 'Interpolated (type 7) variant, pooled / mean-of-runs p99: ' + ARMS.map(a => `${a}: ${f(LAT.latency_ms_scheduled_all[a].pooled.interp.p99)} / ${f(LAT.latency_ms_scheduled_all[a].mean_of_per_run.interp.p99)}`).join('; '));
md.push('', 'Alternative columns, pooled NR p50/p99: ' + ['service_latency_ms_all', 'latency_ms_scheduled_200only', 'service_latency_ms_200only'].map(c => `**${c}**: ` + ARMS.map(a => `${a} ${f(LAT[c][a].pooled.nr.p50)}/${f(LAT[c][a].pooled.nr.p99)}`).join(', ')).join('; '), '');
md.push('## 5 Paired differences vs BASE (unit = repetition, n=10)', '', 'diff = treat - BASE. Boot = percentile bootstrap CI of the mean diff; t = t-based CI; p = exact two-sided sign-flip permutation (min attainable 0.001953).', '');
for (const t of TREAT) { md.push(`### ${t}`, '', '| metric | BASE mean | treat mean | mean diff | sd diff | boot CI95 | t CI95 | perm p | rel. reduction (mean of per-rep) [boot CI] |', '|---|---|---|---|---|---|---|---|---|');
  for (const [m, P] of Object.entries(PAIRED[t])) md.push(`| ${m} | ${f(P.mean_base)} | ${f(P.mean_treat)} | ${f(P.mean)} | ${f(P.sd)} | [${P.boot_ci95_pct.map(v => f(v)).join(', ')}] | [${P.t_ci95.map(v => f(v)).join(', ')}] | ${f(P.perm_p_exact_two_sided, 5)} | ${P.rel_reduction_per_rep ? `${f(P.rel_reduction_mean_of_per_rep * 100, 2)}% [${P.rel_reduction_per_rep.boot_ci95_pct.map(v => f(v * 100, 2)).join(', ')}]%` : ''} |`); md.push(''); }
md.push('## 6 Shed vs 503 / cap rejection', '', ARMS.map(a => `- ${a}: window 503=${ACC[a].pooled_window.s503}, with x_cache_error CapacityExceeded=${ACC[a].pooled_window.s503_by_xerr.CapacityExceeded || 0}; every non-200 is a cap rejection in all runs: ${ACC[a].pooled_window.all_non200_cap_rejections_all_runs}; other status/no-response: ${ACC[a].pooled_window.other_n}/${ACC[a].pooled_window.errors}`).join('\n'), '');
md.push('## 7 Cache status mix (window, all rows)', '', '| arm | HIT | MISS | COALESCED | NONE (shed, no header) | COALESCED / (COALESCED+MISS) among 200s | origin rows per client MISS (whole run) |', '|---|---|---|---|---|---|---|');
for (const a of ARMS) { const w = MIX[a].window_all, p = MIX[a].window_pct_of_all; md.push(`| ${a} | ${w.HIT || 0} (${f((p.HIT || 0) * 100, 1)}%) | ${w.MISS || 0} (${f((p.MISS || 0) * 100, 1)}%) | ${w.COALESCED || 0} (${f((p.COALESCED || 0) * 100, 1)}%) | ${w.NONE || 0} (${f((p.NONE || 0) * 100, 1)}%) | ${f(MIX[a].coalesced_share_of_nonhit_served_window, 4)} | ${f(MIX[a].origin_requests_per_client_miss_whole, 4)} |`); }
md.push('', '## 8 Recovery (error def: status != 200 per 1 s bin vs pooled healthy-phase rate; in-flight vs max healthy-phase snapshot; snapshot clock offset 0)', '', '| arm | error s (mine) | error s (runner run_metrics) | in_flight s (mine) | in_flight s (runner) |', '|---|---|---|---|---|');
for (const a of ARMS) md.push(`| ${a} | ${REC[a].map(r => f(r.offset0.error_recovery_s_vs_healthy_pooled, 1)).join(',')} | ${REC[a].map(r => f(r.runner.recovery_error_s, 1)).join(',')} | ${REC[a].map(r => f(r.offset0.in_flight_recovery_s_vs_healthy_max, 2)).join(',')} | ${REC[a].map(r => f(r.runner.recovery_in_flight_s, 2)).join(',')} |`);
md.push('', 'Alternative error-recovery def (threshold = max healthy 1 s bin): ' + ARMS.map(a => `${a}: [${REC[a].map(r => f(r.offset0.error_recovery_s_vs_healthy_max_bin, 1)).join(',')}]`).join('; '), '');
md.push('## 9 Origin-cap enforcement evidence', '', '| arm | enforcing in all snapshots | cap min..max seen | gateway in_flight max (per run) | rejections final (per run) | client 503 CapExceeded whole run | rejections - 503 | dispatched - origin rows |', '|---|---|---|---|---|---|---|---|');
for (const a of ARMS) md.push(`| ${a} | ${CAPEV[a].every(c => c.enforcing_all_snapshots === c.enforcing_expected)} | ${Math.min(...CAPEV[a].map(c => c.cap_min_seen))}..${Math.max(...CAPEV[a].map(c => c.cap_max_seen))} | ${CAPEV[a].map(c => c.in_flight_max).join(',')} | ${CAPEV[a].map(c => c.rejections_final).join(',')} | ${CAPEV[a].map(c => c.client_503_cap_exceeded_whole_run).join(',')} | ${CAPEV[a].map(c => c.rejections_minus_client_503).join(',')} | ${CAPEV[a].map(c => c.dispatched_minus_origin_rows).join(',')} |`);
md.push('', '## Phase view and cap dynamics', '', '```', JSON.stringify(PHASE, null, 1), '```', '', 'AIMD arrivals above the snapshot-derived cap, relaxed bound (max cap within +-300 ms): ' + JSON.stringify(CONC['AIMD-CAP'].map(o => o.vs_snapshot_cap_max_within_pm300ms.exceed)));
md.push('', '## 10 Stale serving', '', '```', JSON.stringify(STALE, null, 1), '```', '');
md.push('## 11 Mechanical verdict (spec section 7; thresholds unchanged)', '');
for (const t of TREAT) { const v = VERD[t]; md.push(`### ${t}: **${v.outcome}** (${v.why})`, '', `- mechanism pass: ${v.mechanism_pass}; contention: ${v.contention_flag}; valid pairs: ${v.n_valid_pairs}`, `- A peak reduction (window peak): ${f(v.cond_A_peak_reduction_ge_5pct.value * 100, 2)}% (>=5%: ${v.cond_A_peak_reduction_ge_5pct.effect_ok}), boot CI [${v.cond_A_peak_reduction_ge_5pct.ci95_pct_bootstrap.map(x => f(x * 100, 2)).join(', ')}]%, excludes zero: ${v.cond_A_peak_reduction_ge_5pct.ci_excludes_zero}`, `- B origin p99 paired diff: ${f(v.cond_B_origin_p99_not_worse.mean_diff_ms)} ms, boot CI [${v.cond_B_origin_p99_not_worse.ci95_pct_bootstrap.map(x => f(x)).join(', ')}], perm p ${f(v.cond_B_origin_p99_not_worse.perm_p, 5)}; not worse by mean: ${v.cond_B_origin_p99_not_worse.ok_by_mean}`, `- C success diff: ${f(v.cond_C_success_within_1pp.mean_diff_pp)} pp, boot CI [${v.cond_C_success_within_1pp.ci95_pct_bootstrap.map(x => f(x)).join(', ')}]; within 1 pp: ${v.cond_C_success_within_1pp.ok}`, `- all benefit conditions hold: ${v.benefit_supported_all_conditions}`, `- peak-definition sensitivity: ${Object.entries(v.sensitivity_peak_definition).map(([k, s]) => `${k} ${f(s.rel_mean * 100, 2)}% [${s.ci.map(x => f(x * 100, 2)).join(', ')}]`).join('; ')}`, ''); }
md.push('## P99 discrepancy resolution', '', '```', JSON.stringify({ ...P99, per_rep: undefined }, null, 1), '```', '', 'Per-repetition client P99 (ms, nearest rank): BASE [' + P99.per_rep.base.map(v => f(v, 2)).join(', ') + ']; AIMD [' + P99.per_rep.aimd.map(v => f(v, 2)).join(', ') + ']; diff [' + P99.per_rep.diff.map(v => f(v, 2)).join(', ') + '].', '');
md.push('## Anomalies / integrity', '', '```', JSON.stringify(integ.totals, null, 1), '```', '', 'Per-arm send-lag p99 (ms) by run: ' + ARMS.map(a => `${a}: [${IR.filter(x => x.arm === a).map(x => f(x.send_lag.p99, 1)).join(',')}]`).join('; '), '');
md.push('## Comparison with runner summary.json (computed after the independent numbers)', '', `${CMP.filter(c => c.match).length}/${CMP.length} compared values match within tolerance (CI bounds are compared loosely because the runner uses BCa and this script uses percentile bootstrap).`, '', '| item | mine | summary.json | diff |', '|---|---|---|---|');
for (const c of CMP.filter(c => !c.match || /verdict|H-B1/.test(c.label))) md.push(`| ${c.label} | ${f(c.mine, 5)} | ${f(c.theirs, 5)} | ${c.diff === null ? '' : f(c.diff, 5)} |`);
md.push('', 'Per-run run_metrics.json comparison (field: n, max abs diff, mismatches beyond tolerance):', '', ...Object.entries(RM.fields).map(([k, v]) => `- ${k}: n=${v.n}, max|diff|=${v.max_abs_diff.toPrecision(3)}, mismatches=${v.n_mismatch}`), '');
fs.writeFileSync(path.join(OUT, 'recompute_report.md'), md.join('\n'));
log('\nwrote recompute_report.json, recompute_report.md, console_output.txt to ' + OUT);
fs.writeFileSync(path.join(OUT, 'console_output.txt'), CONSOLE.join('\n') + '\n');

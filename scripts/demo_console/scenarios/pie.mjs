#!/usr/bin/env node
// scripts/demo_console/scenarios/pie.mjs -- simple, honest policy-engine (PIE) demonstrations for the Demo Hub.
//   node scripts/demo_console/scenarios/pie.mjs --id pie-hotkey|pie-flashcrowd|pie-shift|pie-eviction|pie-admission --dir <cluster dir> [--out-dir d] [--events]
// Real traffic goes through node 1 of the running demo cluster (policy engine in Advisory mode); signals and decision records are read
// from node 1 with the tenant A token that the driver holds (the token is never printed). Status words used on every result:
//   OBSERVED (measured from real traffic) / SHADOW-EVALUATED (a what-if computed beside serving) / PROPOSED (advice only)
//   and always: APPLIED: none: the engine never changes how requests are served.
import { join } from 'node:path';
import { argv, recorder, sleep } from '../../lib/scenario_common.mjs';
import { ev } from '../../lib/events.mjs';
import { loadCluster, httpRequest, getJson, originManifest } from '../../lib/traffic.mjs';

const { opt } = argv();
const ID = opt('id'); const DIR = opt('dir'); const OUT = opt('out-dir', join(DIR || '.', 'hub', ID || 'pie'));
const R = recorder(ID, OUT, { prefix: String(ID).toUpperCase() });
const st = loadCluster(DIR); const TOKEN = st.secure ? st.tokens.tenantA.token : null; const PORT = st.ports[0];
const RUN = Date.now().toString(36);
const APPLIED = 'APPLIED: none: the engine never changes how requests are served';
const obs = (text, means, extra = {}) => ev.observe(text, { ribbon: 'OBSERVED', ...(means ? { means } : {}), ...extra });
const shadow = (text, means) => ev.observe(text, { ribbon: 'SHADOW-EVALUATED', ...(means ? { means } : {}) });
const proposed = (text) => ev.observe(text + ' ' + APPLIED + '.', { ribbon: 'PROPOSED', means: 'Advice only. Nothing was applied.' });
const sig = () => getJson(PORT, '/api/v1/intelligence/signals', TOKEN);
const decs = async (policy) => { const j = await getJson(PORT, `/api/v1/intelligence/decisions?limit=200${policy ? '&policy_id=' + policy : ''}`, TOKEN); return (j && (j.records || j.decisions)) || []; };
const fv = (s, id) => { const f = s && s.features && s.features[id]; return f && f.value !== null && f.value !== undefined ? { v: f.value, n: f.samples } : { v: null, n: f ? f.samples : 0, reason: f && f.reason }; };
const num = (x, d = 2) => (typeof x === 'number' ? +x.toFixed(d) : x);
const label = (s) => { const c = s && s.classification; return c && c.value ? String(c.value) : (c && c.reason ? `unavailable (${c.reason})` : 'unavailable'); };

async function preflight() {
  const reg = await getJson(PORT, '/api/v1/intelligence/registry', TOKEN);
  if (reg.error || !reg.max_mode) { R.check(false, 'this gateway serves the policy engine API', 'This gateway build does not include the policy engine routes (or the demo cluster is not in Observe/Advisory mode).'); return false; }
  ev.action(`Policy engine: mode ${reg.max_mode === 'advisory' ? 'Advisory' : 'Observe'} (it observes and advises; it never changes how requests are served)`, { ribbon: 'OBSERVED', means: APPLIED });
  return true;
}
const keyPath = (p) => `/api/v1/cache${p}?r=${RUN}`;
/** Send real traffic for ms; pick(i) -> path; workers concurrent loops; pauseMs between requests per worker. Returns the client log. */
async function drive(ms, pick, { workers = 10, pauseMs = 20, poll = 2500, tag = '' } = {}) {
  const t0 = Date.now(); const log = []; let i = 0; let stop = false; const best = { flash: null };
  const loops = Array.from({ length: workers }, async () => { while (!stop) { const p = pick(i++); const r = await httpRequest({ port: PORT, path: keyPath(p), token: TOKEN }); log.push({ t: Date.now(), p, ok: r.status === 200 }); if (pauseMs) await sleep(pauseMs); } });
  let lastPoll = 0; const snaps = [];
  while (Date.now() - t0 < ms) {
    await sleep(250);
    if (Date.now() - lastPoll >= poll) { lastPoll = Date.now(); const s = await sig(); snaps.push({ t: Date.now(), s }); const w = s.window; ev.observe(`${tag}${log.length} requests sent so far; engine window #${w ? w.index : '-'}: ${w ? w.events : 0} events, request rate ${num(fv(s, 'F1').v, 0)}/s, classification ${label(s)}`, { ribbon: 'OBSERVED' }); }
  }
  stop = true; await sleep(300); await Promise.all(loops);
  return { log, snaps, seconds: (Date.now() - t0) / 1000 };
}
/** The richest snapshot taken while traffic was flowing (a window read after traffic stops is empty). */
const richest = (snaps, score) => snaps.map((x) => x.s).filter((x) => x && x.window).sort((a, b) => score(b) - score(a))[0] || null;
const distinct = (log) => new Set(log.map((x) => x.p)).size;
const allOk = (log) => log.every((x) => x.ok);

// ------------------------------------------------------------------ pie-hotkey
async function hotkey() {
  if (!(await preflight())) return;
  const manifest = await originManifest(st.originPort); const pool = Object.keys(manifest).filter((k) => k.startsWith('/msg/') || k.startsWith('/text/')).slice(0, 201);
  const HOT = pool[0]; const others = pool.slice(1, 201);
  ev.action(`Sending real traffic for about 16 seconds: one key (${HOT}) gets about a third of the requests, ${others.length} other keys share the rest`, { means: 'A hot key is one popular object that dominates the traffic.' });
  const rs = Math.random; const pick = () => (rs() < 0.33 ? HOT : others[Math.floor(rs() * others.length)]);
  const { log } = await drive(16000, pick, { workers: 12, pauseMs: 15 });
  await sleep(1200);
  const s = await sig(); const w = s.window; const trueDistinctAll = distinct(log);
  const lastN = w ? log.slice(-w.events) : []; const trueWin = distinct(lastN);
  const hk = s.hot_keys && s.hot_keys.items && s.hot_keys.items[0];
  obs(`traffic sent: ${log.length} requests, ${trueDistinctAll} different keys, hot key share by the client ${(log.filter((x) => x.p === HOT).length / log.length * 100).toFixed(1)}%`, 'This is the truth the engine is compared with.');
  if (hk) obs(`hot key seen by the engine: hash ${hk.key_hash}, count between ${hk.count_lower} and ${hk.count_upper}, share ${num(hk.share, 3)}, confidence ${num(hk.confidence, 2)} (lower/upper bound ratio, not a probability)`, 'Only a hash of the key is kept, never the key.');
  obs(`classification: ${label(s)}; request rate ${num(fv(s, 'F1').v, 0)}/s; top-1 share ${num(fv(s, 'F3').v, 3)}`, 'The classification is re-evaluated every 10 one-second windows.');
  const d = fv(s, 'F23'), se = fv(s, 'F23b');
  obs(`distinct keys in the latest 1-second window: the engine estimates ${num(d.v, 0)} (standard error ${num(se.v, 1)}); the client sent ${trueWin} different keys in its last ${lastN.length} requests`, 'HyperLogLog estimate versus the true count of the same stretch of requests (approximate alignment).');
  R.check(allOk(log), 'every request was answered 200', `${log.length} requests`);
  R.check(!!hk && hk.hot === true && hk.share > 0.2 && hk.share < 0.5, 'the engine found the hot key and its share is about a third', hk ? `share ${num(hk.share, 3)}, hot ${hk.hot}` : 'no hot key reported');
  R.check(d.v !== null && Math.abs(d.v - trueWin) <= Math.max(3 * (se.v || 0), 0.3 * trueWin), 'the distinct-keys estimate is close to the true count (within 3 standard errors or 30%)', `estimate ${num(d.v, 0)}, true ${trueWin}`);
  if (/hot_key/.test(label(s))) obs('classification is hot_key_dominated, as expected for this traffic');
  proposed('A hot key like this could be replicated or kept longer.');
}

// ------------------------------------------------------------------ pie-flashcrowd
async function flashcrowd() {
  if (!(await preflight())) return;
  const manifest = await originManifest(st.originPort); const pool = Object.keys(manifest).filter((k) => k.startsWith('/msg/')).slice(0, 150); const HOT = '/text/007.txt';
  ev.action('Calm traffic for 9 seconds (spread over 150 keys, about 60 requests per second): the engine needs several calm windows as a baseline', { means: 'It compares each second with what came before.' });
  await drive(9000, () => pool[Math.floor(Math.random() * pool.length)], { workers: 3, pauseMs: 50, poll: 3000, tag: 'calm: ' });
  ev.action(`A flash crowd: about 7 seconds of heavy traffic on ONE key (${HOT}) from many clients at once`, { means: 'A sudden burst concentrated on a single object.' });
  const t0 = Date.now(); let first = null; let seen = null;
  const watch = (async () => { while (Date.now() - t0 < 9000) { await sleep(400); const s = await sig(); const fc = s.flash_crowd; if (fc) { if (!seen || (fc.rate_ratio || 0) > (seen.rate_ratio || 0)) seen = fc; if (fc.detected_in_last_window && !first) first = { ms: Date.now() - t0, fc }; } } })();
  const { log } = await drive(7000, () => HOT, { workers: 40, pauseMs: 0, poll: 3000, tag: 'burst: ' });
  await watch;
  const recs = (await decs('POL-FLASH-1'));
  R.check(allOk(log), 'every request was answered 200', `${log.length} burst requests`);
  R.check(!!seen, 'the engine exposes the flash-crowd block (rates, concentration, thresholds)', seen ? `max rate_ratio seen ${num(seen.rate_ratio)}` : 'no block');
  if (first) {
    obs(`flash crowd DETECTED about ${first.ms} ms after the burst began: rate ratio ${num(first.fc.rate_ratio)}, rate CUSUM ${num(first.fc.rate_cusum)}, top-10 share ${num(first.fc.top10_share, 2)} against a baseline of ${num(first.fc.concentration_baseline, 2)}; heuristic confidence ${num(first.fc.confidence, 2)} (a declared rule score, not a probability)`, 'Evidence: a jump in rate and a jump in concentration.');
    if (recs.length) { const r = recs[recs.length - 1]; obs(`decision record POL-FLASH-1: ${r.verdict} (${r.reason_code})${r.recommendation ? ': ' + r.recommendation.text : ''}`, `applied: ${r.recommendation ? r.recommendation.applied : 'n/a'}`); }
    proposed('Consider pre-warming the hot key and a longer stale window.');
  } else {
    obs(`the flash-crowd detector did NOT fire in this run. Largest values seen: rate ratio ${seen ? num(seen.rate_ratio) : 'n/a'} (threshold ${seen && seen.thresholds ? seen.thresholds.rate_h : '1.5'}), top-10 share ${seen ? num(seen.top10_share, 2) : 'n/a'} vs baseline ${seen ? num(seen.concentration_baseline, 2) : 'n/a'}`, 'No detection is shown because none happened. This is what the engine saw.');
    ev.observe('A rate_ratio is measured against the calm baseline; on this fast loopback client the baseline may already be high.', { ribbon: 'OBSERVED' });
  }
}

// ------------------------------------------------------------------ pie-shift
async function shift() {
  if (!(await preflight())) return;
  const manifest = await originManifest(st.originPort); const pool = Object.keys(manifest).filter((k) => !k.includes('huge') && !k.includes('/img/') && !k.includes('short')).slice(0, 290);
  const before = (await decs()).length;
  ev.action('Workload A for 22 seconds: requests spread evenly over 290 keys', { means: 'No key stands out.' });
  // The engine looks at one-second windows, and the window right after traffic stops is empty: keep the BUSIEST window seen
  // while each workload is running, and compare those.
  const busiest = (stop, slot) => (async () => { while (!stop.v) { await sleep(700); try { const s = await sig(); const n = (s.window && s.window.events) || 0; if (n >= 100 && (!slot.s || n >= slot.s.window.events)) slot.s = s; } catch { /* the next poll will try again */ } } })();
  const slotA = {}, stopA = { v: false }, pollA = busiest(stopA, slotA);
  const A = await drive(22000, () => pool[Math.floor(Math.random() * pool.length)], { workers: 8, pauseMs: 15, poll: 5000, tag: 'A: ' });
  stopA.v = true; await pollA;
  const sa = slotA.s || (await sig()); const mid = (await decs()).length;
  obs(`after workload A: classification ${label(sa)}, top-1 share ${num(fv(sa, 'F3').v, 3)}, distinct keys estimate ${num(fv(sa, 'F23').v, 0)}, decision records so far ${mid - before} new`);
  ev.action('Workload B for 22 seconds: one key now gets half of all requests', { means: 'The shape of the traffic changes.' });
  const slotB = {}, stopB = { v: false }, pollB = busiest(stopB, slotB);
  const B = await drive(22000, () => (Math.random() < 0.5 ? pool[0] : pool[Math.floor(Math.random() * pool.length)]), { workers: 8, pauseMs: 15, poll: 5000, tag: 'B: ' });
  stopB.v = true; await pollB;
  const sb = slotB.s || (await sig()); const after = await decs();
  obs(`after workload B: classification ${label(sb)}, top-1 share ${num(fv(sb, 'F3').v, 3)}, new decision records since A: ${after.length - mid}`);
  const la = label(sa), lb = label(sb);
  R.check(allOk(A.log) && allOk(B.log), 'every request was answered 200', `${A.log.length + B.log.length} requests`);
  R.check(fv(sb, 'F3').v !== null && fv(sa, 'F3').v !== null && fv(sb, 'F3').v > fv(sa, 'F3').v + 0.2, 'the top-1 share rose clearly when the workload became hot-key dominated', `${num(fv(sa, 'F3').v, 3)} -> ${num(fv(sb, 'F3').v, 3)}`);
  if (la !== lb) obs(`the classification label changed: ${la} -> ${lb}`, 'The engine noticed the shape of the traffic changed.');
  else obs(`the classification label stayed ${lb} (it is only re-evaluated every 10 windows; the top-1 share above shows the change)`);
  for (const r of after.slice(-3)) obs(`decision ${r.policy_id}: ${r.verdict} (${r.reason_code})`, r.recommendation ? 'A shadow suggestion; not applied.' : 'Nothing to change.');
  proposed('Re-check the eviction advice for the new workload.');
}

// ------------------------------------------------------------------ pie-eviction
async function eviction() {
  if (!(await preflight())) return;
  const manifest = await originManifest(st.originPort); const hotset = Object.keys(manifest).filter((k) => k.startsWith('/msg/')).slice(0, 20); const scan = Object.keys(manifest).filter((k) => k.startsWith('/msg/') || k.startsWith('/text/'));
  let sc = 0;
  ev.action('Sending a scan (every request a new key, never repeated) mixed with a hot set of 20 keys read again and again, for about 26 seconds', { means: 'A scan can push useful entries out of a cache; the engine compares replacement policies on a sample of this stream.' });
  const { log, snaps } = await drive(26000, () => (Math.random() < 0.3 ? hotset[Math.floor(Math.random() * 20)] : `${scan[sc % scan.length]}?scan=${sc++}`), { workers: 20, pauseMs: 2, poll: 1500 });
  const s = richest(snaps, (x) => (x.window ? x.window.events : 0)) || await sig(); const ds = await decs('POL-EVICT-1'); const all = await decs();
  R.check(allOk(log), 'every request was answered 200', `${log.length} requests`);
  obs(`traffic: ${log.length} requests; hit ratio ${num(fv(s, 'F2').v, 3)}, working-set estimate ${num(fv(s, 'F5').v, 0)} keys, classification ${label(s)}`);
  const rec = ds.filter((r) => r.candidates && r.candidates.length).slice(-1)[0];
  if (rec) {
    shadow(`eviction decision ${rec.policy_id} (${rec.verdict}, ${rec.reason_code}): what-if hit ratio change against the incumbent, computed on a sampled key stream`, 'Shadow caches hold a fixed 1,024 entries and see about 1 in 16 keys. Real serving uses SIEVE and was not changed.');
    for (const c of rec.candidates) { const e = c.estimated_effect; shadow(`${c.candidate_id}: ${c.status}${c.rejection_reason ? ' (' + c.rejection_reason + ')' : ''}${e ? `, estimated hit-ratio change ${num(e.value, 3)} with 95% interval [${e.ci95 ? e.ci95.map((x) => num(x, 3)).join(', ') : '-'}]` : ''}`); }
    const have = new Set(rec.candidates.map((c) => c.candidate_id));
    const missing = ['LRU', 'SIEVE', 'S3-FIFO', 'TinyLFU'].filter((n) => !have.has(n));
    if (missing.length) shadow(`not listed as candidates in this record: ${missing.join(', ')} (SIEVE is the incumbent; others may be absent in this engine version)`);
    if (rec.recommendation) proposed(rec.recommendation.text);
    else proposed('No change to the replacement policy is advised.');
  } else {
    obs(`the engine produced no eviction record in this window (records seen: ${all.length}, policies: ${[...new Set(all.map((r) => r.policy_id))].join(', ') || 'none'}). Numbers it did produce: hit ratio ${num(fv(s, 'F2').v, 3)}, working set ${num(fv(s, 'F5').v, 0)} keys`, 'Nothing is invented: this is what was available.');
    shadow('shadow capacity is a fixed 1,024 entries and real serving uses SIEVE; neither was changed');
    proposed('No eviction advice was produced for this run.');
  }
}

// ------------------------------------------------------------------ pie-admission
async function admission() {
  if (!(await preflight())) return;
  const manifest = await originManifest(st.originPort); const base = Object.keys(manifest).filter((k) => k.startsWith('/text/') || k.startsWith('/msg/')); let u = 0;
  ev.action('Sending traffic made mostly of keys seen only once (one-hit wonders) plus a small hot set, as fast as possible for about 14 seconds', { means: 'The frequency features need several thousand different keys in one window: they sample 1 in 16 keys and need at least 30 sampled keys.' });
  const { log, snaps } = await drive(14000, () => (Math.random() < 0.1 ? base[Math.floor(Math.random() * 8)] : `${base[u % base.length]}?once=${u++}`), { workers: 80, pauseMs: 0, poll: 1500 });
  const s = richest(snaps, (x) => ['F18', 'F19', 'F20', 'F21', 'F22'].filter((i) => fv(x, i).v !== null).length * 1e6 + (x.window ? x.window.events : 0)) || await sig();
  obs(`window read while traffic was flowing: #${s.window ? s.window.index : '-'} with ${s.window ? s.window.events : 0} events (${num(s.window ? s.window.events : 0, 0)} requests in one second)`);
  const ds = await decs('POL-ADMIT-1');
  const names = [['F18', 'seen once'], ['F19', 'seen twice'], ['F20', 'seen 3+ times'], ['F21', 'top 1% of keys: request share'], ['F22', 'top 10% of keys: request share']];
  let avail = 0;
  for (const [id, n] of names) { const f = fv(s, id); if (f.v !== null) { avail += 1; obs(`${id} ${n}: ${num(f.v, 3)} (${f.n} samples)`); } else obs(`${id} ${n}: unavailable, insufficient samples (${f.n} sampled keys)`, 'The engine never extrapolates below the minimum sample count.'); }
  obs(`distinct keys in the latest window (estimate): ${num(fv(s, 'F23').v, 0)}; request rate ${num(fv(s, 'F1').v, 0)}/s; one-hit-wonder ratio ${fv(s, 'F17').v === null ? 'unavailable' : num(fv(s, 'F17').v, 3)}`);
  R.check(allOk(log), 'every request was answered 200', `${log.length} requests`);
  R.check(fv(s, 'F23').v !== null, 'the cardinality estimate is available', `${num(fv(s, 'F23').v, 0)} distinct keys in the window`);
  if (ds.length) { const r = ds[ds.length - 1]; obs(`admission record POL-ADMIT-1: ${r.verdict} (${r.reason_code})${r.recommendation ? ': ' + r.recommendation.text : ''}`, 'Admission advice: whether keys seen once should be kept at all.'); proposed(r.recommendation ? r.recommendation.text : 'No admission change is advised.'); }
  else { obs('no admission advice record was produced in this run'); if (!avail) ev.observe('The frequency features stayed unavailable: this run did not reach the minimum sample count in a single window (a slower client or origin gives fewer keys per second).', { ribbon: 'OBSERVED' }); proposed('No admission advice was produced for this run.'); }
}

const fns = { 'pie-hotkey': hotkey, 'pie-flashcrowd': flashcrowd, 'pie-shift': shift, 'pie-eviction': eviction, 'pie-admission': admission };
if (!fns[ID] || !DIR) { console.error('usage: --id ' + Object.keys(fns).join('|') + ' --dir <cluster dir>'); process.exit(1); }
try { await fns[ID](); } catch (e) { R.check(false, 'the scenario hit an unexpected problem', String(e && e.message || e).slice(0, 200)); }
ev.observe(APPLIED, { ribbon: 'APPLIED' });
process.exit(R.finish() ? 0 : 1);

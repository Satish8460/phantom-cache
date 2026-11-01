// D-7 scripted browser journeys (observe-only; never edits the repo). Dependency-free, drives a real Chrome over CDP.
//   node ui/scripts/browser-journey.mjs --phase anon --evidence <dir> --ui http://localhost:3000 --gateway http://localhost:8085 --ctl D:/Temp/d7/d7ctl.mjs
//   node ui/scripts/browser-journey.mjs --phase jwt  --evidence <dir> --ui http://localhost:3000 --gateway http://localhost:18086 --tokens D:/Temp/d7/tokens.json
// Phase "anon": read flow, evidence detail, run form, Scenario B small run to completion, artifacts, second run + cancel,
//   reload mid-run, gateway kill/restart (only the PID recorded by the ctl script).
// Phase "jwt": no token / observer / operator-only / admin / expired tokens; token-leak inspection.
import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { phase, evidence, ui = 'http://localhost:3000', gateway, ctl, tokens } = args;
if (!phase || !evidence || !gateway) { console.error('required: --phase --evidence --gateway'); process.exit(2); }
const R = new Results(join(evidence, `journey_${phase}.results.json`));
const b = await new Browser({ evidenceDir: evidence, label: `j${phase}` }).launch();
const gwQuery = gateway === 'http://localhost:8085' ? '' : `?gateway=${encodeURIComponent(gateway)}`;
const page = (hash) => `${ui}/${gwQuery}${hash}`;
const txt = (sel) => b.eval(`(document.querySelector(${JSON.stringify(sel)})||{}).innerText||null`);
const body = () => b.eval('document.body.innerText');
const ctlRun = (a) => execSync(`node ${ctl} ${a}`).toString().trim();
let n = 0;
const shot = (name) => b.shot(`${phase}_${String(++n).padStart(2, '0')}_${name}.png`, 'shots');

async function openForm(kind) {
  await b.goto(page('#/lab/run'), 1500);
  await b.waitFor(`document.querySelector('form[aria-label="Run specification"]')`, 15000);
  if (kind === 'scenario_b') await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
  await sleep(300);
}
async function fillSb({ reps, warm, healthy, degraded, recovery, rate, keys, dropAimd = true }) {
  const out = [];
  out.push(await b.setField('Repetitions per arm', reps));
  out.push(await b.setField('Warm-up', warm)); out.push(await b.setField('Healthy', healthy));
  out.push(await b.setField('Degraded', degraded)); out.push(await b.setField('Recovery', recovery));
  out.push(await b.setField('Arrival rate', rate)); out.push(await b.setField('Key catalogue', keys));
  if (dropAimd) await b.eval(`(() => { const c=[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.startsWith('AIMD-CAP')); const i=c&&c.querySelector('input'); if (i && i.checked) i.click(); })()`);
  await sleep(400);
  return out;
}
async function startRun(label) {
  const clicked = await b.clickText('Start run');
  const ok = await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000);
  const id = ok ? await b.eval(`document.querySelector('[data-run-detail]').getAttribute('data-run-detail')`) : null;
  return { clicked, id };
}
const chip = () => b.eval(`(document.querySelector('.lab-detail-head')||{}).innerText||null`);
const stream = () => b.eval(`(document.querySelector('[data-stream-status]')||{}).getAttribute?.('data-stream-status')||null`);
const progress = () => txt('[data-progress]');
const isTerminal = (t) => t && /(completed|failed|cancelled|interrupted)/i.test(t);

async function watch(maxMs, label, every = 3000, stopWhen = isTerminal) {
  const seen = []; const t0 = Date.now(); let last = null;
  while (Date.now() - t0 < maxMs) {
    const c = await chip().catch(() => null); const s = await stream().catch(() => null); const p = await progress().catch(() => null);
    const row = `${((Date.now() - t0) / 1000).toFixed(0)}s chip=${(c || '').replace(/\s+/g, ' ').slice(0, 60)} | stream=${s} | prog=${(p || '').replace(/\s+/g, ' ').slice(0, 90)}`;
    if (row.slice(row.indexOf('chip')) !== last) { seen.push(row); last = row.slice(row.indexOf('chip')); }
    if (stopWhen(c)) break;
    await sleep(every);
  }
  return seen;
}

try {
  await b.viewport(1280, 800, false);
  await b.media([{ name: 'prefers-color-scheme', value: 'light' }]);
  if (phase === 'anon') {
    // ---- 1. Lab landing, capabilities banner
    await b.goto(page('#/lab'), 2500);
    R.add('lab.landing', 'OBSERVED', (await body()).slice(0, 1800), await shot('lab_landing'));
    const bannerTxt = await b.eval(`[...document.querySelectorAll('[role=status],.banner,.lab-connection,[data-connection]')].map(e=>e.innerText.replace(/\\s+/g,' ')).slice(0,6)`);
    R.add('lab.capabilities_banner', bannerTxt.length ? 'OBSERVED' : 'FAIL', bannerTxt);
    // ---- 2. Evidence list and details
    await b.clickText('Evidence', 'a'); await sleep(1500);
    R.add('lab.evidence_list', 'OBSERVED', (await body()).slice(0, 2500), await shot('evidence_list'));
    for (const id of ['m6_sealed_2026-10-29T234845Z', 'scenario_b.20261030T003932Z']) {
      await b.goto(page(`#/lab/evidence/${id}`), 2500);
      const t = await body();
      await shot('evidence_' + id.slice(0, 8)); await b.shot(`${phase}_evidence_${id.slice(0, 8)}_full.png`, 'shots', true);
      const checks = id.startsWith('m6')
        ? { NOT_SUPPORTED: /not[_ -]supported/i.test(t), tierBadge: /RETAINED|SIMULATED|MEASURED/.test(t) }
        : { TRADE_OFF_FAILURE: /trade[_ -]off[_ -]failure/i.test(t), interpretationNotice: /interpretation/i.test(t), limitations: /limitation/i.test(t), tierBadge: /RETAINED|MEASURED/.test(t) };
      R.add('lab.evidence_detail.' + id, Object.values(checks).every(Boolean) ? 'PASS' : 'FAIL', checks);
      const chartInfo = await b.eval(`({ svgs: document.querySelectorAll('svg[role=img],svg[aria-label]').length, viewAsTable: [...document.querySelectorAll('button,summary,a')].filter(e=>/table/i.test(e.innerText)).map(e=>e.innerText.trim()).slice(0,5) })`);
      R.add('lab.evidence_charts.' + id, 'OBSERVED', chartInfo);
    }
    // ---- 3. Run form, invalid state first
    await openForm('scenario_b');
    await b.setField('Repetitions per arm', '0'); await sleep(300);
    R.add('run.form_invalid', 'OBSERVED', { submitDisabled: await b.eval(`document.querySelector('button[type=submit]').disabled`), errors: await b.eval(`[...document.querySelectorAll('[data-field-error]')].map(e=>e.innerText)`) }, await shot('run_form_invalid'));
    // ---- 4. Small Scenario B run to completion
    const fill = await fillSb({ reps: 1, warm: 1, healthy: 2, degraded: 2, recovery: 1, rate: 100, keys: 100 });
    R.add('run.form_filled', fill.every((x) => x === 'ok') ? 'PASS' : 'FAIL', { fill, spec: await txt('[data-runspec]') }, await shot('run_form_filled'));
    const s1 = await startRun('run1');
    R.add('run1.started', s1.id ? 'PASS' : 'FAIL', s1, await shot('run1_started'));
    if (s1.id) {
      const seen = await watch(240000, 'run1', 3000);
      R.add('run1.progress_trail', 'OBSERVED', seen);
      await sleep(1500);
      const finalChip = await chip();
      R.add('run1.terminal_state', /completed/i.test(finalChip || '') ? 'PASS' : 'FAIL', finalChip, await shot('run1_final'));
      await b.shot(`${phase}_run1_final_full.png`, 'shots', true);
      const results = await body();
      R.add('run1.result_labels', 'OBSERVED', { exploratory: /EXPLORATORY/.test(results), debugBuild: /debug/i.test(results), tradeOff: /TRADE_OFF|SUPPORTED|NOT_SUPPORTED|outcome/i.test(results), notice: /interpretation|notice/i.test(results), tierBadge: /MEASURED_LOOPBACK|Measured/i.test(results) });
      const arts = await b.eval(`[...document.querySelectorAll('[data-artifact]')].map(r=>r.innerText.replace(/\\s+/g,' '))`);
      R.add('run1.artifacts_listed', arts.length >= 2 ? 'PASS' : 'FAIL', arts);
      // download: intercept anchor click by wrapping URL.createObjectURL to capture blob size
      await b.eval(`window.__dl = []; const orig = URL.createObjectURL; URL.createObjectURL = (bl) => { window.__dl.push({size: bl.size, type: bl.type}); return orig.call(URL, bl); };`);
      await b.click(`document.querySelector('[data-artifact] button')`);
      await sleep(1500);
      R.add('run1.artifact_download', (await b.eval('window.__dl')).length ? 'PASS' : 'FAIL', await b.eval('window.__dl'), await shot('run1_artifacts'));
      // runs list
      await b.goto(page('#/lab/runs'), 2000);
      R.add('runs.list', (await body()).includes(s1.id) ? 'PASS' : 'FAIL', (await txt('[data-runs-table]'))?.slice(0, 500), await shot('runs_list'));
    }
    // ---- 5. Run 2: reload mid-run, then cancel with confirmation
    await openForm('scenario_b');
    await fillSb({ reps: 2, warm: 2, healthy: 5, degraded: 5, recovery: 3, rate: 100, keys: 100 });
    const s2 = await startRun('run2');
    R.add('run2.started', s2.id ? 'PASS' : 'FAIL', s2, await shot('run2_started'));
    if (s2.id) {
      await sleep(9000);
      R.add('run2.before_reload', 'OBSERVED', { chip: await chip(), stream: await stream(), prog: await progress() }, await shot('run2_before_reload'));
      await b.send('Page.reload'); await sleep(3500);
      await b.waitFor(`document.querySelector('[data-run-detail]')`, 15000);
      await sleep(2500);
      const afterReload = { url: await b.eval('location.href'), chip: await chip(), stream: await stream(), prog: await progress(), events: await b.eval(`(document.body.innerText.match(/Events received\\s*(\\d+)/)||[])[1]`) };
      R.add('run2.reload_resume', afterReload.stream === 'live' || afterReload.stream === 'connecting' ? 'PASS' : 'OBSERVED', afterReload, await shot('run2_after_reload'));
      // Cancel flow: click Cancel run -> confirmation -> Escape (observe) -> Keep running -> Cancel again -> confirm
      await b.clickText('Cancel run'); await sleep(500);
      R.add('run2.cancel_confirm_shown', (await body()).includes('Cancel this run?') ? 'PASS' : 'FAIL', null, await shot('run2_cancel_confirm'));
      await b.key('Escape', { code: 'Escape', vk: 27 }); await sleep(500);
      R.add('run2.escape_on_confirm', (await body()).includes('Cancel this run?') ? 'OBSERVED' : 'OBSERVED', (await body()).includes('Cancel this run?') ? 'Escape did NOT dismiss the confirmation (it stays open)' : 'Escape dismissed the confirmation');
      if (!(await body()).includes('Cancel this run?')) await b.clickText('Cancel run');
      await b.clickText('Keep running'); await sleep(500);
      R.add('run2.keep_running', (await chip() || '').match(/running|queued/i) ? 'PASS' : 'FAIL', await chip());
      await b.clickText('Cancel run'); await sleep(300);
      await b.clickText('Yes, cancel'); await sleep(1000);
      R.add('run2.cancel_sent', 'OBSERVED', { chip: await chip(), body: (await body()).match(/Cancel requested[^\n]*/)?.[0] }, await shot('run2_cancel_requested'));
      const seen = await watch(90000, 'run2cancel', 2000);
      R.add('run2.cancel_trail', 'OBSERVED', seen);
      R.add('run2.terminal_cancelled', /cancelled/i.test(await chip() || '') ? 'PASS' : 'FAIL', await chip(), await shot('run2_cancelled'));
    }
    // ---- 6. Run 3: kill the gateway mid-run, observe, restart
    await openForm('scenario_b');
    await fillSb({ reps: 2, warm: 2, healthy: 5, degraded: 5, recovery: 3, rate: 100, keys: 100 });
    const s3 = await startRun('run3');
    R.add('run3.started', s3.id ? 'PASS' : 'FAIL', s3, await shot('run3_started'));
    if (s3.id) {
      await sleep(8000);
      R.add('run3.before_kill', 'OBSERVED', { chip: await chip(), stream: await stream(), prog: await progress() });
      R.add('gateway.kill', 'OBSERVED', ctlRun('stop gw_anon'));
      const seen = await watch(40000, 'killed', 2000, () => false);
      R.add('run3.after_kill_trail', 'OBSERVED', seen, await shot('run3_after_kill'));
      const killedBody = await body();
      R.add('run3.disconnect_state_shown', /disconnect|reconnect|stale|lost|unavailable/i.test(killedBody) ? 'PASS' : 'FAIL', { stream: await stream(), chip: await chip() });
      await b.shot(`${phase}_run3_killed_full.png`, 'shots', true);
      R.add('gateway.restart', 'OBSERVED', ctlRun('start anon'));
      await sleep(12000);
      const seen2 = await watch(60000, 'restarted', 3000, (c) => /interrupted/i.test(c || ''));
      R.add('run3.after_restart_trail', 'OBSERVED', seen2, await shot('run3_after_restart'));
      await b.send('Page.reload'); await sleep(4000);
      R.add('run3.final_after_reload', /interrupted/i.test(await chip() || '') ? 'PASS' : 'FAIL', { chip: await chip(), stream: await stream(), body: (await body()).match(/Interrupted by a gateway restart[^\n]*/)?.[0] }, await shot('run3_interrupted'));
      await b.shot(`${phase}_run3_interrupted_full.png`, 'shots', true);
    }
    const st = await b.storageAndCookies();
    R.add('anon.storage_after_journey', 'OBSERVED', st);
  }

  if (phase === 'jwt') {
    const T = JSON.parse(readFileSync(tokens, 'utf8'));
    const submitToken = async (tok) => {
      await b.eval(`(() => { const i=document.querySelector('.lab-token input'); const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set; set.call(i, ${JSON.stringify(tok)}); i.dispatchEvent(new Event('input',{bubbles:true})); })()`);
      await sleep(200); await b.clickText('Use token'); await sleep(2500);
    };
    // no token
    await b.goto(page('#/lab'), 2500);
    R.add('jwt.no_token', /token is required|A token/i.test(await body()) ? 'PASS' : 'FAIL', (await body()).slice(0, 900), await shot('no_token'));
    for (const who of ['observer', 'operator', 'expired', 'wrongaud', 'admin']) {
      await b.goto(page('#/lab'), 2000);
      await b.waitFor(`document.querySelector('.lab-token input')`, 8000);
      await submitToken(T[who]);
      const t = await body();
      R.add(`jwt.${who}.lab_landing`, 'OBSERVED', t.slice(0, 900), await shot(`${who}_landing`));
      await b.eval(`window.location.hash='#/lab/evidence'`); await sleep(2000);
      R.add(`jwt.${who}.evidence_read`, 'OBSERVED', { items: await b.eval(`document.querySelectorAll('a[href*="#/lab/evidence/"]').length`), text: (await body()).slice(0, 500) });
    }
    // observer: start disabled/403?  operator-only: 403 role message on start; admin: full run
    for (const who of ['observer', 'operator']) {
      await b.goto(page('#/lab'), 2000); await b.waitFor(`document.querySelector('.lab-token input')`, 8000); await submitToken(T[who]);
      await b.eval(`window.location.hash='#/lab/run'`); await sleep(1500);
      await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
      await fillSb({ reps: 1, warm: 1, healthy: 1, degraded: 1, recovery: 1, rate: 50, keys: 100 });
      const disabled = await b.eval(`document.querySelector('button[type=submit]').disabled`);
      if (!disabled) { await b.clickText('Start run'); await sleep(2500); }
      R.add(`jwt.${who}.start`, 'OBSERVED', { startDisabledInUi: disabled, body: (await body()).match(/(role|permission|forbidden|Observer|Operator)[^\n]*/gi)?.slice(0, 4) }, await shot(`${who}_start`));
    }
    await b.goto(page('#/lab'), 2000); await b.waitFor(`document.querySelector('.lab-token input')`, 8000); await submitToken(T.admin);
    await b.eval(`window.location.hash='#/lab/run'`); await sleep(1500);
    await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
    await fillSb({ reps: 1, warm: 1, healthy: 2, degraded: 2, recovery: 1, rate: 100, keys: 100 });
    const s = await startRun('admin');
    R.add('jwt.admin.run_started', s.id ? 'PASS' : 'FAIL', s, await shot('admin_run_started'));
    if (s.id) {
      const seen = await watch(240000, 'admin', 3000);
      R.add('jwt.admin.trail', 'OBSERVED', seen);
      R.add('jwt.admin.terminal', /completed/i.test(await chip() || '') ? 'PASS' : 'FAIL', await chip(), await shot('admin_final'));
      await b.eval(`window.__dl = []; const orig = URL.createObjectURL; URL.createObjectURL = (bl) => { window.__dl.push({size: bl.size}); return orig.call(URL, bl); };`);
      await b.click(`document.querySelector('[data-artifact] button')`); await sleep(1500);
      R.add('jwt.admin.artifact_download_with_header', (await b.eval('window.__dl')).length ? 'PASS' : 'FAIL', await b.eval('window.__dl'));
    }
    // token leak inspection
    const st = await b.storageAndCookies();
    const needles = Object.values(T).map((t) => t.slice(-24));
    const leaks = [];
    const hay = JSON.stringify(st);
    for (const [k, t] of Object.entries(T)) if (hay.includes(t.slice(-24))) leaks.push('storage/cookies/url contains ' + k);
    for (const r of b.reqById.values()) for (const [k, t] of Object.entries(T)) if ((r.url || '').includes(t.slice(-24))) leaks.push(`request URL contains ${k}: ${r.url.slice(0, 80)}`);
    for (const c of b.consoleLog) for (const [k, t] of Object.entries(T)) if (c.text.includes(t.slice(-24))) leaks.push('console contains ' + k);
    const authedNonGateway = [...b.reqById.values()].filter((r) => r.hasAuth && !r.url.startsWith(gateway));
    R.add('jwt.token_leak_check', leaks.length === 0 && authedNonGateway.length === 0 ? 'PASS' : 'FAIL', { leaks, authedNonGateway: authedNonGateway.map((r) => r.url), storage: st });
  }
} catch (e) {
  R.add('journey.exception', 'FAIL', String(e && e.stack || e));
} finally {
  R.save(); b.saveLogs(`journey_${phase}`);
  const gone = await b.close();
  console.log(JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version }));
  writeFileSync(join(evidence, `journey_${phase}.browser.json`), JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version, port: b.port }));
}

// D-7 round 2 scripted journeys (observe-only; never edits the repo). Dependency-free, real Chrome over CDP.
//   node ui/scripts/browser-r2-journey.mjs --phase anon --evidence <dir> --gateway http://localhost:8085 --ctl D:/Temp/d7b/d7ctl.mjs
//   node ui/scripts/browser-r2-journey.mjs --phase jwt  --evidence <dir> --gateway http://localhost:18086 --tokens D:/Temp/d7b/tokens.json
// Differences from browser-journey.mjs: T1 matrix run through the UI, focus/scroll checks after Start run (F-06),
// Escape + focus return on the cancel confirmation (F-07), state chip while disconnected (F-08), role warning and 403 alert (F-09).
import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { Browser, Results, sleep } from './cdp-lib.mjs';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, x, i, arr) => (x.startsWith('--') ? [...a, [x.slice(2), arr[i + 1]]] : a), []));
const { phase, evidence, ui = 'http://localhost:3000', gateway, ctl, tokens } = args;
if (!phase || !evidence || !gateway) { console.error('required: --phase --evidence --gateway'); process.exit(2); }
const R = new Results(join(evidence, `r2_journey_${phase}.results.json`));
const b = await new Browser({ evidenceDir: evidence, label: `r2j${phase}` }).launch();
const gwQuery = gateway === 'http://localhost:8085' ? '' : `?gateway=${encodeURIComponent(gateway)}`;
const page = (hash) => `${ui}/${gwQuery}${hash}`;
const txt = (sel) => b.eval(`(document.querySelector(${JSON.stringify(sel)})||{}).innerText||null`);
const body = () => b.eval('document.body.innerText');
const ctlRun = (a) => execSync(`node ${ctl} ${a}`).toString().trim();
let n = 0;
const shot = (name, full = false) => b.shot(`r2_${phase}_${String(++n).padStart(2, '0')}_${name}.png`, 'shots/journey', full);
const activeInfo = () => b.eval(`(() => { const a = document.activeElement; return { tag: a.tagName, id: a.id || null, text: (a.innerText || a.getAttribute('aria-label') || '').trim().slice(0, 50), scrollY: Math.round(scrollY) }; })()`);

async function openForm(kind) {
  await b.goto(page('#/lab/run'), 1500);
  await b.waitFor(`document.querySelector('form[aria-label="Run specification"]')`, 15000);
  const label = kind === 'scenario_b' ? 'Scenario B' : 'Simulation matrix';
  await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes(${JSON.stringify(label)})).querySelector('input')`);
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
/** Set a checkbox group (fieldset by legend) to exactly `values`. */
async function setChecks(legend, values) {
  return b.eval(`(() => { const fs = [...document.querySelectorAll('fieldset')].find(f => f.querySelector('legend') && f.querySelector('legend').textContent.trim().startsWith(${JSON.stringify(legend)}));
    if (!fs) return 'no fieldset'; const want = ${JSON.stringify(values)};
    for (const l of fs.querySelectorAll('label.lab-check')) { const i = l.querySelector('input'); const name = l.querySelector('span').childNodes[0].textContent.trim(); const should = want.includes(name); if (i.checked !== should) i.click(); }
    return [...fs.querySelectorAll('input:checked')].map(i => i.closest('label').querySelector('span').childNodes[0].textContent.trim()); })()`);
}
async function startRun() {
  const clicked = await b.clickText('Start run');
  const ok = await b.waitFor(`document.querySelector('[data-run-detail]')`, 20000);
  const id = ok ? await b.eval(`document.querySelector('[data-run-detail]').getAttribute('data-run-detail')`) : null;
  await sleep(600);
  return { clicked, id, focusAfterStart: await activeInfo() };
}
const chip = () => b.eval(`(document.querySelector('.lab-detail-head')||{}).innerText||null`);
const stream = () => b.eval(`(document.querySelector('[data-stream-status]')||{}).getAttribute?.('data-stream-status')||null`);
const progress = () => txt('[data-progress]');
const isTerminal = (t) => t && /(completed|failed|cancelled|interrupted)/i.test(t) && !/last known/i.test(t);
async function watch(maxMs, every = 3000, stopWhen = isTerminal) {
  const seen = []; const t0 = Date.now(); let last = null;
  while (Date.now() - t0 < maxMs) {
    const c = await chip().catch(() => null); const s = await stream().catch(() => null); const p = await progress().catch(() => null);
    const row = `${((Date.now() - t0) / 1000).toFixed(0)}s chip=${(c || '').replace(/\s+/g, ' ').slice(0, 110)} | stream=${s} | prog=${(p || '').replace(/\s+/g, ' ').slice(0, 90)}`;
    if (row.slice(row.indexOf('chip')) !== last) { seen.push(row); last = row.slice(row.indexOf('chip')); }
    if (stopWhen(c)) break;
    await sleep(every);
  }
  return seen;
}
async function downloads() {
  await b.eval(`window.__dl = []; if (!window.__dlHooked) { window.__dlHooked = true; const orig = URL.createObjectURL; URL.createObjectURL = (bl) => { window.__dl.push({size: bl.size, type: bl.type}); return orig.call(URL, bl); }; }`);
  const names = await b.eval(`[...document.querySelectorAll('[data-artifact]')].map(r => r.innerText.replace(/\\s+/g,' ').slice(0, 160))`);
  const got = [];
  const count = await b.eval(`document.querySelectorAll('[data-artifact] button').length`);
  for (let i = 0; i < count; i++) {
    await b.eval(`window.__dl = []`);
    await b.click(`document.querySelectorAll('[data-artifact] button')[${i}]`); await sleep(1200);
    got.push(await b.eval('window.__dl'));
  }
  return { names, got };
}

try {
  await b.viewport(1280, 800, false);
  await b.media([{ name: 'prefers-color-scheme', value: 'light' }]);
  if (phase === 'anon') {
    await b.goto(page('#/lab'), 2500);
    R.add('lab.landing', 'OBSERVED', (await body()).slice(0, 1500), await shot('lab_landing'));
    // ---- Scenario B small run
    await openForm('scenario_b');
    await b.setField('Repetitions per arm', '0'); await sleep(300);
    R.add('run.form_invalid', 'OBSERVED', { submitDisabled: await b.eval(`document.querySelector('button[type=submit]').disabled`), errors: await b.eval(`[...document.querySelectorAll('[data-field-error]')].map(e=>e.innerText)`) }, await shot('run_form_invalid'));
    const fill = await fillSb({ reps: 1, warm: 1, healthy: 2, degraded: 2, recovery: 1, rate: 100, keys: 100 });
    R.add('run.form_filled', fill.every((x) => x === 'ok') ? 'PASS' : 'FAIL', { fill, spec: await txt('[data-runspec]') }, await shot('run_form_filled'));
    await b.eval('window.scrollTo(0, document.body.scrollHeight)'); await sleep(200);
    const pre = await b.eval('Math.round(scrollY)');
    const s1 = await startRun();
    R.add('run1.started', s1.id ? 'PASS' : 'FAIL', { ...s1, scrollYBeforeStart: pre }, await shot('run1_started'));
    R.add('F06.focus_and_scroll_after_start', s1.focusAfterStart.scrollY === 0 && /H1|H2/.test(s1.focusAfterStart.tag) ? 'PASS' : 'OBSERVED', s1.focusAfterStart);
    var sb1 = s1.id;
    if (s1.id) {
      const seen = await watch(240000, 3000);
      R.add('run1.progress_trail', 'OBSERVED', seen);
      await sleep(1500);
      R.add('run1.terminal_state', /completed/i.test(await chip() || '') ? 'PASS' : 'FAIL', await chip(), await shot('run1_final'));
      await b.shot('r2_anon_run1_final_full.png', 'shots/journey', true);
      const results = await body();
      R.add('run1.result_labels', 'OBSERVED', { exploratory: /EXPLORATORY/.test(results), measuredLoopback: /MEASURED.?LOOPBACK|measured/i.test(results), validNarrow: results.match(/Run class[^\n]*/g) });
      R.add('run1.artifacts_and_downloads', 'OBSERVED', await downloads(), await shot('run1_artifacts'));
    }
    // ---- T1 matrix run through the UI (small): dev seed set D, 2 seeds, 1 scenario, 1 family, 2 arms
    await openForm('t1_matrix');
    const caps = JSON.parse(readFileSync(join(evidence, 'logs', 'capabilities_anon.json'), 'utf8')).kinds.t1_matrix;
    const f = [];
    f.push(await setChecks('Scenarios', ['O-NONE']));
    f.push(await setChecks('Workload families', ['W-Z']));
    f.push(await setChecks('Arms', ['BL-0', 'BL-1']));
    await b.click(`[...document.querySelectorAll('fieldset')].find(f => f.querySelector('legend').textContent.startsWith('Seed set')).querySelector('input[value="D"]')`);
    f.push(await b.setField('Number of seeds', '2'));
    await b.click(`[...document.querySelectorAll('fieldset')].find(f => f.querySelector('legend').textContent.startsWith('Profile')).querySelector('input[value="smoke"]')`);
    await sleep(500);
    R.add('t1.form_filled', 'OBSERVED', { f, capsScenarios: caps.scenarios.length, spec: await txt('[data-runspec]'), cells: await txt('[data-cell-estimate]'), submitDisabled: await b.eval(`document.querySelector('button[type=submit]').disabled`) }, await shot('t1_form_filled', true));
    const t1 = await startRun();
    R.add('t1.started', t1.id ? 'PASS' : 'FAIL', t1, await shot('t1_started'));
    var t1id = t1.id;
    if (t1.id) {
      const seen = await watch(240000, 2000);
      R.add('t1.progress_trail', 'OBSERVED', seen);
      await sleep(1500);
      R.add('t1.terminal_state', /completed/i.test(await chip() || '') ? 'PASS' : 'FAIL', await chip(), await shot('t1_final'));
      await b.shot('r2_anon_t1_final_full.png', 'shots/journey', true);
      const rb = await body();
      R.add('t1.result_labels', 'OBSERVED', { simulated: /SIMULATED/.test(rb), measuredMention: /MEASURED/.test(rb), text: rb.slice(0, 2500) });
      R.add('t1.artifacts_and_downloads', 'OBSERVED', await downloads(), await shot('t1_artifacts'));
    }
    R.add('runs.ids', 'OBSERVED', { sb1, t1id });
    writeFileSync(join(evidence, 'r2_runids.json'), JSON.stringify({ sb1, t1id }));
    await b.goto(page('#/lab/runs'), 2000);
    R.add('runs.list', (await body()).includes(sb1 || 'x') ? 'PASS' : 'FAIL', (await txt('[data-runs-table]'))?.slice(0, 600), await shot('runs_list'));

    // ---- Run 2: reload mid-run, cancel (F-07)
    await openForm('scenario_b');
    await fillSb({ reps: 2, warm: 2, healthy: 5, degraded: 5, recovery: 3, rate: 100, keys: 100 });
    const s2 = await startRun();
    R.add('run2.started', s2.id ? 'PASS' : 'FAIL', s2, await shot('run2_started'));
    if (s2.id) {
      await sleep(9000);
      await b.send('Page.reload'); await sleep(3500);
      await b.waitFor(`document.querySelector('[data-run-detail]')`, 15000); await sleep(2500);
      R.add('run2.reload_resume', ['live', 'connecting'].includes(await stream()) ? 'PASS' : 'OBSERVED', { url: await b.eval('location.href'), chip: await chip(), stream: await stream(), prog: await progress() }, await shot('run2_after_reload'));
      await b.clickText('Cancel run'); await sleep(500);
      R.add('run2.cancel_confirm_shown', (await body()).includes('Cancel this run?') ? 'PASS' : 'FAIL', { focusInside: await activeInfo() }, await shot('run2_cancel_confirm'));
      await b.key('Escape', { code: 'Escape', vk: 27 }); await sleep(500);
      const dismissed = !(await body()).includes('Cancel this run?');
      const act = await activeInfo();
      R.add('F07.escape_dismisses_and_returns_focus', dismissed && /^Cancel run/.test(act.text) ? 'PASS' : 'FAIL', { dismissed, active: act }, await shot('run2_after_escape'));
      if (dismissed) await b.clickText('Cancel run');
      await sleep(300);
      await b.clickText('Keep running'); await sleep(500);
      R.add('run2.keep_running', (await chip() || '').match(/running|queued/i) ? 'PASS' : 'FAIL', { chip: await chip(), active: await activeInfo() });
      await b.clickText('Cancel run'); await sleep(300);
      await b.clickText('Yes, cancel'); await sleep(1000);
      R.add('run2.cancel_sent', 'OBSERVED', { chip: await chip(), body: (await body()).match(/Cancel requested[^\n]*/)?.[0] }, await shot('run2_cancel_requested'));
      R.add('run2.cancel_trail', 'OBSERVED', await watch(90000, 2000));
      R.add('run2.terminal_cancelled', /cancelled/i.test(await chip() || '') ? 'PASS' : 'FAIL', await chip(), await shot('run2_cancelled'));
    }
    // ---- Run 3: kill gateway mid-run (F-08)
    await openForm('scenario_b');
    await fillSb({ reps: 2, warm: 2, healthy: 5, degraded: 5, recovery: 3, rate: 100, keys: 100 });
    const s3 = await startRun();
    R.add('run3.started', s3.id ? 'PASS' : 'FAIL', s3, await shot('run3_started'));
    if (s3.id) {
      await sleep(8000);
      R.add('run3.before_kill', 'OBSERVED', { chip: await chip(), stream: await stream(), prog: await progress() });
      R.add('gateway.kill', 'OBSERVED', ctlRun('stop gw_anon'));
      const seen = await watch(45000, 2000, () => false);
      R.add('run3.after_kill_trail', 'OBSERVED', seen, await shot('run3_after_kill'));
      const chipK = await chip();
      R.add('F08.chip_while_disconnected', /last known|disconnected/i.test(chipK || '') ? 'PASS' : 'FAIL', { chip: chipK, stream: await stream() });
      await b.shot('r2_anon_run3_killed_full.png', 'shots/journey', true);
      R.add('gateway.restart', 'OBSERVED', ctlRun('start anon'));
      await sleep(12000);
      R.add('run3.after_restart_trail', 'OBSERVED', await watch(60000, 3000, (c) => /interrupted/i.test(c || '')), await shot('run3_after_restart'));
      await b.send('Page.reload'); await sleep(4000);
      R.add('run3.final_after_reload', /interrupted/i.test(await chip() || '') ? 'PASS' : 'FAIL', { chip: await chip(), body: (await body()).match(/Interrupted by a gateway restart[^\n]*/)?.[0] }, await shot('run3_interrupted'));
      await b.shot('r2_anon_run3_interrupted_full.png', 'shots/journey', true);
    }
    // guard still allows the UI after restart
    await b.goto(page('#/lab'), 2500);
    R.add('ui_works_with_host_origin_guard', /Lab API enabled/i.test(await body()) ? 'PASS' : 'FAIL', (await body()).slice(0, 300));
    R.add('anon.storage_after_journey', 'OBSERVED', await b.storageAndCookies());
  }

  if (phase === 'jwt') {
    const T = JSON.parse(readFileSync(tokens, 'utf8'));
    T.nonjwt = 'this-is-not-a-jwt-just-some-text';
    const submitToken = async (tok) => {
      await b.eval(`(() => { const i=document.querySelector('.lab-token input'); const set=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set; set.call(i, ${JSON.stringify(tok)}); i.dispatchEvent(new Event('input',{bubbles:true})); })()`);
      await sleep(200); await b.clickText('Use token'); await sleep(2500);
    };
    await b.goto(page('#/lab'), 2500);
    R.add('jwt.no_token', /token is required|A token/i.test(await body()) ? 'PASS' : 'FAIL', (await body()).slice(0, 700), await shot('no_token'));
    for (const who of ['observer', 'operator', 'expired', 'wrongaud', 'admin', 'nonjwt']) {
      await b.goto(page('#/lab'), 2000);
      await b.waitFor(`document.querySelector('.lab-token input')`, 8000);
      await submitToken(T[who]);
      R.add(`jwt.${who}.lab_landing`, 'OBSERVED', (await body()).slice(0, 700), await shot(`${who}_landing`));
      await b.eval(`window.location.hash='#/lab/evidence'`); await sleep(2000);
      R.add(`jwt.${who}.evidence_read`, 'OBSERVED', { items: await b.eval(`document.querySelectorAll('a[href*="#/lab/evidence/"]').length`), text: (await body()).slice(0, 300) });
    }
    // F-09: token pasted (via the Lab connection form), then #/lab/run
    for (const who of ['observer', 'operator', 'admin', 'nonjwt']) {
      await b.goto(page('#/lab'), 2000); await b.waitFor(`document.querySelector('.lab-token input')`, 8000); await submitToken(T[who]);
      await b.eval(`window.location.hash='#/lab/run'`); await sleep(1800);
      const formThere = await b.eval(`!!document.querySelector('form[aria-label="Run specification"]')`);
      let warn = null, submitErr = null, disabled = null, tokenInputOnRun = await b.eval(`!!document.querySelector('.lab-token input')`);
      if (formThere) {
        await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
        await fillSb({ reps: 1, warm: 1, healthy: 1, degraded: 1, recovery: 1, rate: 50, keys: 100 });
        warn = await b.eval(`(() => { const e=[...document.querySelectorAll('[role=status]')].find(x => /cannot start runs/i.test(x.innerText)); return e ? e.innerText.replace(/\\s+/g,' ') : null; })()`);
        disabled = await b.eval(`document.querySelector('button[type=submit]').disabled`);
        await shot(`${who}_run_form_warning`, true);
        if (who === 'observer' && !disabled) {
          await b.clickText('Start run'); await sleep(2500);
          submitErr = await b.eval(`(() => { const e=document.querySelector('[role=alert]:not([data-field-error])'); return e ? e.innerText.replace(/\\s+/g,' ') : null; })()`);
          await shot(`${who}_403_alert`);
        }
      }
      R.add(`F09.${who}`, 'OBSERVED', { formThere, tokenInputOnRun, roleWarning: warn, submitDisabled: disabled, alertAfterSubmit: submitErr, expectWarning: who === 'observer' });
    }
    await b.goto(page('#/lab'), 2000); await b.waitFor(`document.querySelector('.lab-token input')`, 8000); await submitToken(T.admin);
    await b.eval(`window.location.hash='#/lab/run'`); await sleep(1500);
    await b.click(`[...document.querySelectorAll('label.lab-check')].find(l => l.innerText.includes('Scenario B')).querySelector('input')`);
    await fillSb({ reps: 1, warm: 1, healthy: 2, degraded: 2, recovery: 1, rate: 100, keys: 100 });
    const s = await startRun();
    R.add('jwt.admin.run_started', s.id ? 'PASS' : 'FAIL', s, await shot('admin_run_started'));
    if (s.id) {
      R.add('jwt.admin.trail', 'OBSERVED', await watch(240000, 3000));
      R.add('jwt.admin.terminal', /completed/i.test(await chip() || '') ? 'PASS' : 'FAIL', await chip(), await shot('admin_final'));
      R.add('jwt.admin.artifact_download_with_header', 'OBSERVED', await downloads());
    }
    const st = await b.storageAndCookies();
    const leaks = []; const hay = JSON.stringify(st);
    for (const [k, t] of Object.entries(T)) if (t.length > 40 && hay.includes(t.slice(-24))) leaks.push('storage/cookies/url contains ' + k);
    for (const r of b.reqById.values()) for (const [k, t] of Object.entries(T)) if (t.length > 40 && (r.url || '').includes(t.slice(-24))) leaks.push(`request URL contains ${k}`);
    for (const c of b.consoleLog) for (const [k, t] of Object.entries(T)) if (t.length > 40 && c.text.includes(t.slice(-24))) leaks.push('console contains ' + k);
    const authedNonGateway = [...b.reqById.values()].filter((r) => r.hasAuth && !r.url.startsWith(gateway));
    R.add('jwt.token_leak_check', leaks.length === 0 && authedNonGateway.length === 0 ? 'PASS' : 'FAIL', { leaks, authedNonGateway: authedNonGateway.map((r) => r.url), storage: st });
  }
} catch (e) {
  R.add('journey.exception', 'FAIL', String((e && e.stack) || e));
} finally {
  R.save(); b.saveLogs(`r2_journey_${phase}`);
  const gone = await b.close();
  console.log(JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version }));
  writeFileSync(join(evidence, `r2_journey_${phase}.browser.json`), JSON.stringify({ chromePid: b.pid, chromeGone: gone, version: b.version, port: b.port }));
}

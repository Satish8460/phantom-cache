#!/usr/bin/env node
// scripts/mvp_iteration.mjs
//
// One live end-to-end validation iteration of the PhantomCache demo. It runs, in order, everything the demo shows and writes an
// evidence folder with one result per step and a summary:
//   1. secure cluster from an empty directory + security scenario + real-requests scenario (scripts/demo_flow.mjs)
//   2. cluster guarantees on gateways it starts itself: burst, dead-peer storm, outcomes, peer-fill staleness
//   3. single-gateway scenarios: freshness, eviction, operability
//   4. the Control Center test suite and its static checks (contrast, forbidden constants, accessibility)
// Usage: node scripts/mvp_iteration.mjs --exe <gateway exe> --out <evidence dir> [--n 1] [--quick] [--skip ui,single]
// Everything is measured on this machine (loopback); the script claims nothing beyond the steps' own results. Exit code 0 only if
// every step passed. No token, secret or key is printed or written to the summary.
import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync, existsSync, readFileSync, rmSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..');
const a = process.argv.slice(2);
const opt = (n, d) => { const i = a.indexOf(`--${n}`); return i !== -1 && a[i + 1] !== undefined ? a[i + 1] : d; };
const EXE = opt('exe'), OUT = opt('out');
if (!EXE || !OUT) { console.error('usage: node scripts/mvp_iteration.mjs --exe <gateway exe> --out <evidence dir> [--n 1] [--quick] [--skip ui,single]'); process.exit(1); }
const N = opt('n', '1');
const QUICK = a.includes('--quick');
const SKIP = new Set((opt('skip', '') || '').split(',').filter(Boolean));
const out = resolve(OUT, `ITER-${N}`);
const work = join(out, 'work');
rmSync(out, { recursive: true, force: true });
mkdirSync(work, { recursive: true });

const steps = [];
const node = (script, args, name, timeoutMs) => {
  const t0 = Date.now();
  const log = join(out, `${name}.log`);
  const r = spawnSync(process.execPath, [join(HERE, script), ...args], { encoding: 'utf8', timeout: timeoutMs, maxBuffer: 256 * 1024 * 1024 });
  const text = (r.stdout || '') + (r.stderr || '');
  writeFileSync(log, text);
  const ok = r.status === 0;
  const last = text.trim().split('\n').filter(Boolean).slice(-1)[0] || '';
  steps.push({ name, ok, seconds: Math.round((Date.now() - t0) / 100) / 10, exit: r.status ?? `signal ${r.signal}`, last: last.slice(0, 220) });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}  (${steps[steps.length - 1].seconds}s)  ${last.slice(0, 120)}`);
  return ok;
};
const cmd = (file, args, cwd, name, timeoutMs) => {
  const t0 = Date.now();
  const r = spawnSync(file, args, { cwd, encoding: 'utf8', timeout: timeoutMs, shell: false, maxBuffer: 256 * 1024 * 1024 });
  const text = (r.stdout || '') + (r.stderr || '');
  writeFileSync(join(out, `${name}.log`), text);
  const ok = r.status === 0;
  const m = /ℹ pass (\d+)/.exec(text);
  const last = (m ? `pass ${m[1]}` : text.trim().split('\n').filter(Boolean).slice(-1)[0] || '').slice(0, 200);
  steps.push({ name, ok, seconds: Math.round((Date.now() - t0) / 100) / 10, exit: r.status ?? `signal ${r.signal}`, last });
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}  (${steps[steps.length - 1].seconds}s)  ${last}`);
  return ok;
};

console.log(`PhantomCache MVP iteration ${N}: ${new Date().toISOString()}  exe=${EXE}  evidence=${out}`);

// 1. secure cluster + security + real traffic (the launcher leaves the cluster running; stop it afterwards)
const sizes = QUICK ? '1000' : '1000,5000,10000';
node('demo_flow.mjs', ['--exe', EXE, '--dir', work, '--scenarios', 'security,traffic', '--reps', QUICK ? '1' : '3', '--sizes', sizes, '--evidence', join(out, 'evidence')], '1_demo_flow_security_and_traffic', 15 * 60 * 1000);
node('cluster_launcher.mjs', ['down', '--dir', work], '1b_cluster_down', 60 * 1000);

// 2. cluster guarantees (they start their own gateways, using the certificates the launcher created)
const certs = join(work, 'certs');
if (!existsSync(join(certs, 'members.json'))) {
  steps.push({ name: '2_cluster_guarantees', ok: false, seconds: 0, exit: 'n/a', last: 'certificates not found (step 1 failed)' });
  console.log('FAIL  2_cluster_guarantees  certificates not found');
} else {
  node('cluster_burst_check.mjs', ['--exe', EXE, '--certs', certs, '--out-dir', join(out, 'burst'), ...(QUICK ? ['--cycles', '5'] : [])], '2a_cluster_burst_check', 10 * 60 * 1000);
  node('cluster_deadpeer_storm_check.mjs', ['--exe', EXE, '--certs', certs, '--out-dir', join(out, 'storm')], '2b_cluster_deadpeer_storm_check', 10 * 60 * 1000);
  node('cluster_outcomes_check.mjs', ['--exe', EXE, '--certs', certs, '--out-dir', join(out, 'outcomes')], '2c_cluster_outcomes_check', 10 * 60 * 1000);
  node('cluster_peer_fill_staleness.mjs', ['--exe', EXE, '--certs', certs, '--out-dir', join(out, 'peerfill')], '2d_cluster_peer_fill_staleness', 10 * 60 * 1000);
}

// 3. single-gateway scenarios
if (!SKIP.has('single')) {
  node('freshness_scenario.mjs', ['--exe', EXE, '--out-dir', join(out, 'freshness')], '3a_freshness_scenario', 10 * 60 * 1000);
  node('eviction_scenario.mjs', ['--exe', EXE, '--out-dir', join(out, 'eviction')], '3b_eviction_scenario', 10 * 60 * 1000);
  node('operability_scenario.mjs', ['--exe', EXE, '--out-dir', join(out, 'operability')], '3c_operability_scenario', 10 * 60 * 1000);
}

// 4. the Control Center
if (!SKIP.has('ui')) {
  const ui = join(ROOT, 'ui');
  cmd(process.execPath, ['--test', 'test'], ui, '4a_ui_tests', 10 * 60 * 1000);
  cmd(process.execPath, ['scripts/check-contrast.mjs'], ui, '4b_ui_contrast', 120 * 1000);
  cmd(process.execPath, ['scripts/check-forbidden.mjs'], ui, '4c_ui_forbidden_constants', 120 * 1000);
  cmd(process.execPath, ['scripts/check-a11y.mjs'], ui, '4d_ui_accessibility', 120 * 1000);
}

const passed = steps.filter((s) => s.ok).length;
const summary = { iteration: N, finished: new Date().toISOString(), exe: EXE, quick: QUICK, host: 'this machine, loopback', steps, passed, total: steps.length, allPassed: passed === steps.length };
writeFileSync(join(out, 'summary.json'), JSON.stringify(summary, null, 1));
const md = [`# MVP iteration ${N}`, '', `Finished ${summary.finished}. Gateway: \`${EXE}\`. Everything measured on this machine over loopback.`, '', '| Step | Result | Seconds | Last line |', '|---|---|---|---|', ...steps.map((s) => `| ${s.name} | ${s.ok ? 'PASS' : 'FAIL'} | ${s.seconds} | ${String(s.last).replace(/\|/g, '/')} |`), '', `**${passed} of ${steps.length} steps passed.**`, ''].join('\n');
writeFileSync(join(out, 'summary.md'), md);
console.log(`\n${passed} of ${steps.length} steps passed. Evidence: ${out}`);
process.exit(summary.allPassed ? 0 : 1);

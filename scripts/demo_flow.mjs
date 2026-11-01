#!/usr/bin/env node
// scripts/demo_flow.mjs
//
// One command from an EMPTY directory to a live, secure, real-content demo.
//   node scripts/demo_flow.mjs --exe <gateway exe> --dir <work dir> [--scenarios security,traffic] [--reps 3] [--sizes 1000,5000,10000]
//        [--console] [--console-port 3201] [--ui-base http://localhost:3000] [--evidence <dir>]
//        [--http-port-base 18185] [--cluster-port-base 19501] [--origin-port 19099]
//   1. stops a cluster left in <dir>, 2. `cluster_launcher up --secure` (certificates, random JWT secret kept in <dir>, demo tokens,
//   real-content origin, 3 gateways), 3. prints the Control Center URLs, 4. optionally runs the security and real-traffic
//   scenarios (result JSON files in <dir>), 5. with --console serves the one-page demo console and keeps running until Ctrl-C
//   (then stops everything). Without --console the cluster is left running: stop it with
//   `node scripts/cluster_launcher.mjs down --dir <dir>`.
// No token, secret or key is printed.

import { spawnSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const a = process.argv.slice(2);
const opt = (n, d) => { const i = a.indexOf(`--${n}`); return i !== -1 && a[i + 1] !== undefined ? a[i + 1] : d; };
const flag = (n) => a.includes(`--${n}`);
const EXE = opt('exe'), DIR = opt('dir') && resolve(opt('dir'));
if (!EXE || !DIR) { console.error('usage: node scripts/demo_flow.mjs --exe <gateway exe> --dir <work dir> [--scenarios security,traffic] [--console] (see the header of this file)'); process.exit(1); }
const say = (m) => console.log(`\n=== ${m}`);
const node = (script, args, inherit = true) => spawnSync(process.execPath, [join(HERE, script), ...args], { stdio: inherit ? 'inherit' : 'pipe', encoding: 'utf8' });
const ports = ['--http-port-base', opt('http-port-base', '18185'), '--cluster-port-base', opt('cluster-port-base', '19501'), '--origin-port', opt('origin-port', '19099')];
const ui = ['--ui-base', opt('ui-base', 'http://localhost:3000')];
let exit = 0;

mkdirSync(DIR, { recursive: true });
if (existsSync(join(DIR, 'cluster.json'))) { say('stopping a cluster left in the directory'); node('cluster_launcher.mjs', ['down', '--dir', DIR]); }

say('1. start the secure cluster (3 gateways with mTLS, JWT required, real-content origin)');
let r = node('cluster_launcher.mjs', ['up', '--secure', '--exe', EXE, '--dir', DIR, ...ports, ...ui]);
if (r.status !== 0) { console.error('cluster did not start'); process.exit(1); }

const scenarios = (opt('scenarios', '') || '').split(',').filter(Boolean);
if (scenarios.includes('security')) {
  say('2. security scenario');
  const ev = opt('evidence');
  r = node('security_scenario.mjs', ['--dir', DIR, '--exe', EXE, ...(ev ? ['--evidence', join(ev, 'security')] : [])]);
  if (r.status !== 0) exit = 1;
}
if (scenarios.includes('traffic')) {
  say('3. real-requests scenario (sha256 integrity on every response; origin counts its own requests)');
  r = node('real_requests_scenario.mjs', ['--dir', DIR, '--reps', opt('reps', '3'), '--sizes', opt('sizes', '1000,5000,10000')]);
  if (r.status !== 0) exit = 1;
}

if (flag('console')) {
  const port = opt('console-port', '3201');
  say(`4. demo console: http://127.0.0.1:${port}/   (Ctrl-C stops the console and the cluster)`);
  const c = spawn(process.execPath, [join(HERE, 'demo_console', 'server.mjs'), '--dir', DIR, '--port', port, '--exe', EXE, ...ui, ...ports], { stdio: 'inherit' });
  const stop = () => { try { c.kill(); } catch { /* ignore */ } node('cluster_launcher.mjs', ['down', '--dir', DIR]); process.exit(exit); };
  process.on('SIGINT', stop); process.on('SIGTERM', stop);
} else {
  say(`cluster left running. Stop it with: node scripts/cluster_launcher.mjs down --dir ${DIR}`);
  process.exit(exit);
}

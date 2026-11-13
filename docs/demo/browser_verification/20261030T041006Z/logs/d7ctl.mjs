// Lifecycle controller for D-7: starts/stops ONLY processes it started; records PIDs in pids.json
import { spawn, execSync } from 'node:child_process';
import { readFileSync, writeFileSync, existsSync, openSync } from 'node:fs';
const EXE = 'D:/Personal/_m7_target/debug/phantom-cache-demo.exe';
const ts = readFileSync('D:/Temp/d7_ts.txt', 'utf8').trim();
const LOGS = `D:/Personal/phantom-cache/docs/demo/browser_verification/${ts}/logs`;
const PF = 'D:/Temp/d7/pids.json';
const pids = () => (existsSync(PF) ? JSON.parse(readFileSync(PF, 'utf8')) : {});
const save = (p) => writeFileSync(PF, JSON.stringify(p, null, 1));
const alive = (pid) => { try { return execSync(`tasklist /FI "PID eq ${pid}" /NH`).toString().includes(String(pid)); } catch { return false; } };
const cmd = process.argv[2], name = process.argv[3];
function start(name, file, args, env, cwd) {
  const out = openSync(`${LOGS}/${name}.${Date.now()}.log`, 'a');
  const c = spawn(file, args, { env: { ...process.env, ...env }, cwd, detached: true, stdio: ['ignore', out, out], windowsHide: true });
  c.unref();
  const p = pids(); p[name] = c.pid; save(p);
  console.log(name, 'pid', c.pid);
}
const SECRET = '<redacted test secret>';
const common = { PHANTOM_MODE: 'development', PHANTOM_LAB_ENABLED: 'true', PHANTOM_LAB_DIR: 'D:/Temp/d7_lab' };
delete process.env.PHANTOM_SEALED_SALT_HEX;
if (cmd === 'start') {
  if (name === 'anon') start('gw_anon', EXE, [], { ...common, PHANTOM_BIND_ADDR: '127.0.0.1:8085' }, 'D:/Temp/d7');
  if (name === 'jwt') start('gw_jwt', EXE, [], { ...common, PHANTOM_BIND_ADDR: '127.0.0.1:18086', PHANTOM_JWT_SECRET: SECRET, PHANTOM_CORS_ORIGINS: 'http://localhost:3000' }, 'D:/Temp/d7');
  if (name === 'vite') start('vite', 'node', ['node_modules/vite/bin/vite.js', '--port', '3000', '--strictPort', '--host', 'localhost'], {}, 'D:/Personal/phantom-cache/ui');
} else if (cmd === 'stop') {
  const p = pids(); const pid = p[name];
  if (pid && alive(pid)) { execSync(`taskkill /PID ${pid} /T /F`, { stdio: 'ignore' }); console.log('killed', name, pid); }
  else console.log('not running', name, pid);
  delete p[name]; save(p);
} else if (cmd === 'status') {
  const p = pids(); for (const [k, v] of Object.entries(p)) console.log(k, v, alive(v) ? 'ALIVE' : 'dead');
}

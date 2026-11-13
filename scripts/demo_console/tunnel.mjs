#!/usr/bin/env node
// scripts/demo_console/tunnel.mjs -- Cloudflare Edge Tunnel launcher with self-healing keepalive
import { spawn } from 'node:child_process';
import http from 'node:http';
import { existsSync } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '..', '..');
const CF_EXE = resolve(ROOT, '.local', 'cloudflared.exe');
const PORT = 3201;

console.log('================================================================');
console.log('⚡ PhantomStore Cloudflare Global Edge Tunnel Launcher');
console.log(`Forwarding local port ${PORT} via Cloudflare Quick Tunnel...`);
console.log('================================================================\n');

let activeChild = null;
let currentTunnelUrl = '';
let keepAliveTimer = null;

function checkAndKeepAlive() {
  if (!currentTunnelUrl) return;
  fetch(`${currentTunnelUrl}/api/lan-ip`)
    .then((r) => {
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
    })
    .catch((err) => {
      console.warn(`Tunnel keepalive ping failed: ${err.message}. Rotating tunnel...`);
      if (activeChild) {
        try { activeChild.kill(); } catch {}
      }
    });
}

function startTunnel() {
  const exe = existsSync(CF_EXE) ? CF_EXE : 'cloudflared';
  const cf = spawn(exe, ['tunnel', '--url', `http://127.0.0.1:${PORT}`], {
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe']
  });
  activeChild = cf;

  const handleOutput = (chunk) => {
    const str = chunk.toString();
    const m = str.match(/(https:\/\/[a-zA-Z0-9-]+\.trycloudflare\.com)/);
    if (m) {
      currentTunnelUrl = m[1];
      console.log('\n----------------------------------------------------------------');
      console.log('🎉 ZERO-PASSWORD CLOUDFLARE PUBLIC STOREFRONT READY:');
      console.log(`👉 Storefront: ${currentTunnelUrl}/store`);
      console.log(`👉 Crowd Hub:  ${currentTunnelUrl}/crowd`);
      console.log('----------------------------------------------------------------');
      console.log('🚀 Direct access from any mobile phone (No password / No IP needed)');
      console.log('----------------------------------------------------------------\n');

      // Notify local server of the public tunnel URL
      const req = http.request({
        hostname: '127.0.0.1',
        port: PORT,
        path: '/api/tunnel',
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
      }, () => {});
      req.on('error', () => {});
      req.write(JSON.stringify({ url: currentTunnelUrl }));
      req.end();

      // Start keep-alive ping every 30s
      clearInterval(keepAliveTimer);
      keepAliveTimer = setInterval(checkAndKeepAlive, 30000);
    }
  };

  cf.stdout.on('data', handleOutput);
  cf.stderr.on('data', handleOutput);

  cf.on('error', (err) => {
    console.error(`Cloudflare tunnel process error: ${err.message}. Retrying in 15 seconds...`);
    activeChild = null;
    currentTunnelUrl = '';
    setTimeout(startTunnel, 15000);
  });

  cf.on('close', (code) => {
    console.log(`Cloudflare tunnel closed (code ${code}). Auto-reconnecting in 15 seconds...`);
    activeChild = null;
    currentTunnelUrl = '';
    setTimeout(startTunnel, 15000);
  });
}

startTunnel();

process.on('SIGINT', () => {
  if (activeChild) activeChild.kill();
  process.exit(0);
});
process.on('SIGTERM', () => {
  if (activeChild) activeChild.kill();
  process.exit(0);
});

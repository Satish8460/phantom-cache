// AI-reviewer usability walkthrough driver (NOT human data). Launches its own throwaway Chrome via cdp-lib and
// exposes a tiny local command server so a reviewer can act only as a user can: goto, text, click, type, key, shot.
// Usage: node ui/scripts/browser-usability.mjs [ctlPort=9890]   then POST JSON {op,...} to http://127.0.0.1:<port>/
import { createServer } from 'node:http';
import { Browser, sleep } from './cdp-lib.mjs';

const ctl = Number(process.argv[2] || 9890);
const EV = 'docs/design/remediation/usability_evidence';
const b = await new Browser({ evidenceDir: EV, label: 'usab' }).launch();
console.log('chrome pid', b.pid, 'cdp port', b.port, b.version);
await b.viewport(1280, 800);

const handlers = {
  async goto({ url, settle }) { await b.goto(url, settle ?? 1500); return 'ok'; },
  async viewport({ w, h, mobile }) { await b.viewport(w, h, !!mobile); return 'ok'; },
  async media({ scheme }) { await b.media([{ name: 'prefers-color-scheme', value: scheme }]); return 'ok'; },
  async text() { return b.eval('document.body.innerText'); },
  async shot({ name, full }) { return b.shot(name, 'shots', !!full); },
  async eval({ js }) { return b.eval(js); },
  async clickText({ text, sel }) { return b.clickText(text, sel); },
  async setField({ label, value }) { return b.setField(label, value); },
  async key({ key, code, vk }) { await b.key(key, { code: code || key, vk: vk || 0 }); return 'ok'; },
  async tab({ shift }) { await b.tab(!!shift); return 'ok'; },
  async buttons() {
    return b.eval(`[...document.querySelectorAll('button,a,[role=button],[role=tab],select,input,summary')].filter(e=>e.offsetParent!==null).map(e=>({tag:e.tagName,text:(e.innerText||e.getAttribute('aria-label')||e.value||'').trim().slice(0,70),dis:!!e.disabled,type:e.type||''}))`);
  },
  async sleep({ ms }) { await sleep(ms); return 'ok'; },
  async quit() { const ok = await b.close(); setTimeout(() => process.exit(0), 200); return { pidGone: ok }; },
};

createServer(async (req, res) => {
  let body = ''; for await (const c of req) body += c;
  try {
    const cmd = JSON.parse(body || '{}');
    const out = await handlers[cmd.op](cmd);
    res.end(JSON.stringify({ ok: true, out }));
  } catch (e) { res.end(JSON.stringify({ ok: false, err: String(e.message || e) })); }
}).listen(ctl, '127.0.0.1', () => console.log('control on', ctl));

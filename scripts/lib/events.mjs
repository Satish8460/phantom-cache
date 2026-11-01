// scripts/lib/events.mjs
//
// Structured step events for the Demo Hub. With `--events` a scenario prints, in addition to its normal output, one line per
// event:  @EV {json}   (the hub driver streams them to the page with Server-Sent Events; every other line is kept as raw log).
// Event: { t: ISO time, kind: 'action'|'observation'|'check'|'progress'|'summary', text, means?, request?, response?, pass?, counters?, numbers? }
//   action       something the scenario DID (a request, a DELETE, killing a node)
//   observation  something it SAW
//   check        an assertion with pass true/false
//   progress     { done, total, counters } live counters for bulk traffic (sampled, not one line per request)
//   summary      { verdict: 'passed'|'shows'|'attention', numbers: [{label, value, tag: measured|derived|unavailable}] }
// Secrets never go in an event: callers pass headers by name (a whitelist below), never an Authorization value.


const ON = process.argv.includes('--events');
const SHOW_HEADERS = ['x-cache-status', 'x-cache-source', 'age', 'content-type', 'etag', 'content-length', 'x-auth-error', 'www-authenticate'];
export const eventsOn = ON;
export function emit(kind, text, extra = {}) {
  if (!ON) return;
  process.stdout.write('@EV ' + JSON.stringify({ t: new Date().toISOString(), kind, text, ...extra }) + '\n');
}
export const pickHeaders = (h) => { const o = {}; for (const k of SHOW_HEADERS) if (h && h[k] !== undefined) o[k] = String(h[k]); return o; };
export const ev = {
  action: (text, extra) => emit('action', text, extra),
  observe: (text, extra) => emit('observation', text, extra),
  check: (pass, text, extra) => emit('check', text, { pass: !!pass, ...extra }),
  progress: (done, total, counters) => emit('progress', '', { done, total, counters }),
  summary: (verdict, numbers, extra) => emit('summary', '', { verdict, numbers, ...extra }),
};

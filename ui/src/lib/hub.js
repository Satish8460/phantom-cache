// Demo Hub links and the one-line help of every Control Center screen. Pure: no DOM, no network, no storage.
// The Hub (scripts/demo_console, lane L1) listens on http://127.0.0.1:3201 by default. `?hub=` may point to another LOOPBACK http(s) origin.
// Links carry no token: a window opened from the Hub receives its token by message (lib/gatewayAuth.js hand-off), never in a URL.

export const DEFAULT_HUB = 'http://127.0.0.1:3201';

export function isLoopbackHost(h) {
  return h === 'localhost' || h === '[::1]' || h === '::1' || /^127(\.\d{1,3}){3}$/.test(h);
}

/** { url, source: 'default'|'query', note } . Anything that is not an http(s) loopback origin without credentials is refused with a note. */
export function resolveHub(search = '') {
  const raw = new URLSearchParams(search).get('hub');
  if (!raw) return { url: DEFAULT_HUB, source: 'default', note: null };
  try {
    const u = new URL(raw);
    if ((u.protocol === 'http:' || u.protocol === 'https:') && !u.username && !u.password && isLoopbackHost(u.hostname)) return { url: u.origin, source: 'query', note: null };
  } catch {
    /* falls through */
  }
  return { url: DEFAULT_HUB, source: 'default', note: 'Ignored ?hub=: the Demo Hub address must be an http or https address on this computer (localhost or 127.x.x.x).' };
}

/** The ONE list of Demo Hub scenarios (order = storyline of D:\ClaudeTemp\lanes\REQUIRED_SCENARIOS.md; ids are the Hub route ids #/scenario/<id>). */
export const SCENARIOS = Object.freeze([
  { id: 'tour', group: 'Start', title: "Guided tour (5 minutes)", what: "A few of the scenarios below run in order, each explained in two sentences." },
  { id: 'gallery', group: 'Speed and savings', title: "Cold and warm gallery", what: "Real images load twice: first from the slow server, then from the cache." },
  { id: 'stampede', group: 'Speed and savings', title: "Stampede", what: "300 simultaneous requests for one cold image cost the slow server one request." },
  { id: 'peerfill', group: 'Speed and savings', title: "Peer fill", what: "A second gateway answers from its neighbour without asking the slow server." },
  { id: 'offload', group: 'Speed and savings', title: "Origin offload", what: "How many client requests were sent versus how many the slow server counted itself." },
  { id: 'workloads', group: 'Speed and savings', title: "Workload shapes", what: "Hot keys, all-unique keys, bursts and a slow server, with the hit ratio of each." },
  { id: 'scale', group: 'Speed and savings', title: "Real requests at scale", what: "1,000 to 10,000 requests with zero errors and every response checked byte for byte." },
  { id: 'auth', group: 'Safety', title: "Sign-in checks", what: "No token, expired, wrong audience and forged tokens are refused with 401." },
  { id: 'roles', group: 'Safety', title: "Roles", what: "A read-only token can read but its delete is refused with 403." },
  { id: 'isolation', group: 'Safety', title: "Tenant isolation", what: "Tenant B never sees what tenant A cached, and A's delete purges only A." },
  { id: 'targetsafety', group: 'Safety', title: "Request-target safety", what: "Paths with control characters or dot segments are refused before the slow server." },
  { id: 'mtls', group: 'Safety', title: "Cluster mutual TLS", what: "A member certificate is answered; missing, foreign or wrong-key certificates are refused." },
  { id: 'failclosed', group: 'Safety', title: "Fail-closed start", what: "The gateway refuses to start without a secret when sign-in is required." },
  { id: 'multiuser', group: 'Safety', title: "Two users, two windows", what: "Open the Control Center as tenant A and tenant B; the token is passed by message, never in a link." },
  { id: 'invalidation', group: 'Resilience', title: "Invalidation", what: "Delete one cached item and watch every gateway stop serving it, with milliseconds." },
  { id: 'deadnode', group: 'Resilience', title: "Kill and restart a node", what: "Stop a gateway while traffic runs: no errors, then it returns to Active." },
  { id: 'deletestorm', group: 'Resilience', title: "Delete storm with a dead peer", what: "400 concurrent deletes while a peer is down; all acknowledged, nothing stale served." },
  { id: 'burstdelete', group: 'Resilience', title: "Burst of deletes", what: "A burst of concurrent deletes leaves every member Active." },
  { id: 'peerexpiry', group: 'Resilience', title: "Peer copy expiry", what: "A copy held by a peer expires together with its source." },
  { id: 'freshness', group: 'Freshness and memory', title: "Freshness suite", what: "Stale-while-revalidate, stale-if-error, no-store, ETag revalidation and the staleness cap." },
  { id: 'eviction', group: 'Freshness and memory', title: "Eviction and memory budget", what: "Memory stays under its budget, hot keys survive and accounting returns to zero." },
  { id: 'largeobjects', group: 'Freshness and memory', title: "Large objects", what: "The 1 MiB rule explained live; an object over 2 MiB gets a clean 502." },
  { id: 'originprotect', group: 'Protection', title: "Origin protection", what: "With a slow server and 60 cold keys at once, the server never sees more than its cap." },
  { id: 'operability', group: 'Operability', title: "Operability", what: "Bad configuration fails fast, health endpoints answer, abusive requests are refused." },
  { id: 'restart', group: 'Operability', title: "Restart under load", what: "Kill and restart a gateway under a running client: a clean outage and recovery." },
  { id: 'intelligence', group: 'Intelligence', title: "Policy engine (observe and advise)", what: "Registry, live signals and shadow recommendations; nothing is applied." },
  { id: 'intelcontrol', group: 'Intelligence', title: "Negative control", what: "A constant workload gives no recommendation; a change in workload gives one." },
  { id: 'dashboards', group: 'Observability', title: "Live dashboards", what: "Run traffic while the Control Center screens update; open each in one click." },
  { id: 'benchmark', group: 'Performance', title: "Quick benchmark", what: "A short, honest comparison with a slow server and with a zero-latency one." },
  { id: 'everything', group: 'All', title: "Demonstrate everything", what: "The full storyline above in order, then a summary of the session." },
]);

const ID = new Set(SCENARIOS.map((s) => s.id));

export function scenarioById(id) {
  return SCENARIOS.find((s) => s.id === id) || null;
}

/** Deep link into the Hub. Unknown ids go to the catalogue, never to a made-up address. */
export function hubLink(hubUrl, id) {
  const base = String(hubUrl || DEFAULT_HUB).replace(/\/+$/, '');
  return ID.has(id) ? `${base}/#/scenario/${id}` : `${base}/`;
}

/** "What am I looking at?" for each Control Center screen; `scenarios` are the Hub scenarios that relate to it. */
export const HELP = Object.freeze({
  overview: { text: 'A summary of what this gateway reports right now. Each value says whether it was measured, derived from measurements, or is unavailable.', scenarios: ['gallery'] },
  traffic: { text: 'How many requests the gateway answered and how fast, since it started. Nothing is estimated on this page.', scenarios: ['scale'] },
  cache: { text: 'How much the cache holds, its memory budget and what it removed to stay inside it.', scenarios: ['eviction'] },
  freshness: { text: 'How the gateway decides an answer is still fresh enough, and when it served an older one on purpose.', scenarios: ['freshness'] },
  origin: { text: 'The slow server behind the gateway: how often it was asked and how long it took. Fewer requests here means the cache is working.', scenarios: ['stampede'] },
  cluster: { text: 'How this gateway sees the other gateways, how its requests were answered, and whether delete messages are getting through.', scenarios: ['deadnode', 'peerfill'] },
  intelligence: { text: 'What the policy engine observed and what it would advise. It only advises: nothing on this page changes the gateway.', scenarios: [] },
  lab: { text: 'Experiments run on this machine and their saved results, kept apart from live numbers.', scenarios: ['benchmark'] },
  simulation: { text: 'A picture of how the idea works. It is a model, not a measurement of this gateway.', scenarios: [] },
});

export function helpFor(screenId) {
  return HELP[screenId] || null;
}

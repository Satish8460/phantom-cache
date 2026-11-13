// Screen registry and hash routing. A screen is listed only if it is backed by data that exists today
// ("data"), is an honest placeholder ("placeholder"), or is the always-bannered simulation explainer ("sim").
// Groups are named for their contents (skill S16: specific labels, wayfinding). `tag` is a short visible text label.

export const SCREENS = Object.freeze([
  // The guided page for a first-time user: what this app shows, the connection, and links into the Demo Hub scenarios.
  { id: 'start', label: 'Start here', kind: 'guide', group: 'Start' },
  { id: 'overview', label: 'Overview', kind: 'data', group: 'Live gateway' },
  { id: 'traffic', label: 'Traffic & latency', kind: 'data', group: 'Live gateway' },
  { id: 'cache', label: 'Cache & memory', kind: 'data', group: 'Live gateway' },
  { id: 'freshness', label: 'Freshness', kind: 'data', group: 'Live gateway' },
  { id: 'origin', label: 'Origin', kind: 'data', group: 'Live gateway' },
  // Backed by metrics.cluster and metrics.outcomes (batch B2). On a gateway that reports no cluster block the page is the
  // honest capability statement, not an invented cluster.
  { id: 'cluster', label: 'Cluster', kind: 'data', group: 'Live gateway' },
  { id: 'lab', label: 'Phantom Lab', kind: 'lab', group: 'Experiments' },
  // The simulation page explains a model; it is not a live demo. The group says what it is and the tag keeps it honestly labelled.
  { id: 'simulation', label: 'Simulation', kind: 'sim', group: 'Explainer', tag: 'Simulated' },
  { id: 'intelligence', label: 'Intelligence', kind: 'placeholder', group: 'Not available yet', tag: 'Not available' },
]);

export const DEFAULT_SCREEN = 'overview';

export function screenById(id) {
  return SCREENS.find((s) => s.id === id);
}

export function hrefFor(id) {
  return `#/${id}`;
}

/** Legacy `?view=sim-demo` is still honoured. Unknown routes fall back to the overview and say so. */
export function parseRoute({ hash = '', search = '' } = {}) {
  const view = new URLSearchParams(search).get('view');
  if (view === 'sim-demo' && !hash) return { id: 'simulation', notFound: false };
  const cleanHash = (hash || '').split('?')[0];
  if (!hash || hash === '#' || hash === '#/' || cleanHash === '#/overview') return { id: DEFAULT_SCREEN, notFound: false };
  const m = /^#\/([a-z-]+)\/?$/.exec(cleanHash);
  if (m && screenById(m[1])) return { id: m[1], notFound: false };
  // Sub-routes exist for the Lab only: #/lab/evidence/<id>, #/lab/runs/<id> ...
  const lab = parseLabSub(hash);
  if (lab) return { id: 'lab', notFound: false, sub: lab };
  return { id: DEFAULT_SCREEN, notFound: true };
}

/** Roving focus for the vertical nav: returns the next index for a key, or null if the key is not handled. */
export function nextNavIndex(key, index, length) {
  if (length <= 0) return null;
  switch (key) {
    case 'ArrowDown':
    case 'ArrowRight':
      return (index + 1) % length;
    case 'ArrowUp':
    case 'ArrowLeft':
      return (index - 1 + length) % length;
    case 'Home':
      return 0;
    case 'End':
      return length - 1;
    default:
      return null;
  }
}

// Same order and labels as the tab strip in screens/lab/LabScreen.jsx (LAB_TABS): About, Start a run, Runs, Evidence.
export const LAB_VIEWS = Object.freeze([
  { id: 'story', label: 'About', path: '' },
  { id: 'run', label: 'Start a run', path: 'run' },
  { id: 'runs', label: 'Runs', path: 'runs' },
  { id: 'evidence', label: 'Evidence', path: 'evidence' },
]);

const LAB_ID = /^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$/;

/** '#/lab/evidence/m6_sealed_x' -> { view: 'evidence', id: 'm6_sealed_x' }; null if the hash is not a Lab sub-route. */
export function parseLabSub(hash = '') {
  const m = /^#\/lab\/([a-z]+)(?:\/([^/?#]+))?\/?$/.exec(hash);
  if (!m) return null;
  const view = m[1];
  if (!['evidence', 'run', 'runs'].includes(view)) return null;
  if (m[2] === undefined) return { view, id: null };
  if (view === 'run') return null;
  let id;
  try {
    id = decodeURIComponent(m[2]);
  } catch {
    return null;
  }
  if (!LAB_ID.test(id)) return null;
  return { view, id };
}

export function labHref(view, id) {
  const base = view === 'story' || !view ? '#/lab' : `#/lab/${view}`;
  return id ? `${base}/${encodeURIComponent(id)}` : base;
}

export const SITE_NAME = 'PhantomCache';

/** Distinct document.title per route, most specific first: "Run <id> | Runs | Phantom Lab | PhantomCache". */
export function titleFor(route) {
  const screen = screenById(route && route.id) || screenById(DEFAULT_SCREEN);
  if (screen.id === 'lab' && route.sub) {
    const view = LAB_VIEWS.find((v) => v.id === route.sub.view);
    const parts = [];
    if (route.sub.id) parts.push(`${route.sub.view === 'evidence' ? 'Evidence' : 'Run'} ${route.sub.id}`);
    if (view) parts.push(view.label);
    return [...parts, screen.label, SITE_NAME].join(' | ');
  }
  return `${screen.label} | ${SITE_NAME}`;
}

const FOOTERS = Object.freeze({
  data: 'Telemetry schema 2. Values come from the gateway and carry their provenance; this page computes no metrics.',
  lab: 'Lab data is read from the Lab API, not from live telemetry. Each number carries a label showing where it came from.',
  sim: 'This page explains a model. Its figures are model output, not measurements of PhantomCache.',
  guide: 'This page explains the app and links to the Demo Hub. The connection and identity shown here are what this page itself sees; it measures nothing.',
  placeholder: 'This page shows only what the gateway reports. Where nothing is reported it says so and shows no numbers.',
});

/** Footer sentence that is true for the kind of page being shown. */
export function footerFor(route) {
  const screen = screenById(route && route.id) || screenById(DEFAULT_SCREEN);
  return FOOTERS[screen.kind] || FOOTERS.data;
}

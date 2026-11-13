// The Control Center's bearer-token holder. This is the ONLY module of the live screens that touches browser storage for a credential.
//
// Rules (docs/user/security.md; board D-045 item 2):
//   - The token is held in memory, bound to the gateway ORIGIN it was entered for. It is never sent to another origin.
//   - Optionally ("keep for this tab") it is also written to sessionStorage, which the browser clears when the tab closes. Off by default.
//   - Never in a URL, never in localStorage, cookies or IndexedDB, never logged, never returned by getSnapshot() (only a boolean and a counter).
//   - Every storage access is wrapped: private windows, blocked site data and quota errors must not break the page.

export const TAB_KEY = 'phantomcache.token.tab.v1';
const MAX_ORIGINS = 8;

const defaultGetStorage = () => globalThis.sessionStorage;

export function createTokenSession({ getStorage = defaultGetStorage } = {}) {
  /** @type {Map<string, {token: string, keep: boolean}>} */
  const mem = new Map();
  const listeners = new Set();
  let version = 0;
  let storageProblem = false;

  const withStorage = (fn) => {
    try {
      const s = getStorage();
      if (!s) {
        storageProblem = true;
        return null;
      }
      return fn(s);
    } catch {
      storageProblem = true;
      return null;
    }
  };

  // Restore what a previous load of THIS tab chose to keep.
  withStorage((s) => {
    const raw = s.getItem(TAB_KEY);
    if (!raw) return;
    try {
      const o = JSON.parse(raw);
      if (o && typeof o === 'object' && !Array.isArray(o)) {
        for (const [origin, token] of Object.entries(o).slice(0, MAX_ORIGINS)) {
          if (typeof token === 'string' && token.length > 0 && token.length <= 8192) mem.set(origin, { token, keep: true });
        }
      }
    } catch {
      /* a damaged entry is ignored; the next write replaces it */
    }
  });

  const persist = () => {
    withStorage((s) => {
      const kept = {};
      for (const [origin, v] of mem) if (v.keep) kept[origin] = v.token;
      if (Object.keys(kept).length === 0) s.removeItem(TAB_KEY);
      else s.setItem(TAB_KEY, JSON.stringify(kept));
    });
  };

  const snapshot = () => Object.freeze({ version, storageProblem });
  let snap = snapshot();

  // A change makes a NEW snapshot object (useSyncExternalStore compares by identity) with the new version, then tells subscribers.
  const publish = () => {
    version += 1;
    snap = snapshot();
    for (const l of [...listeners]) l();
  };

  return {
    /** The token for this origin, or null. Callers must hand it only to a request to that same origin. */
    tokenFor(origin) {
      const v = mem.get(origin);
      return v ? v.token : null;
    },
    isKept(origin) {
      const v = mem.get(origin);
      return Boolean(v && v.keep);
    },
    set(origin, token, { keep = false } = {}) {
      if (typeof origin !== 'string' || !origin || typeof token !== 'string' || !token) return false;
      if (!mem.has(origin) && mem.size >= MAX_ORIGINS) mem.delete(mem.keys().next().value);
      mem.set(origin, { token, keep: Boolean(keep) });
      persist();
      publish();
      return true;
    },
    forget(origin) {
      if (!mem.delete(origin)) return false;
      persist();
      publish();
      return true;
    },
    subscribe(l) {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    // Only a counter and a flag: the token is not part of the snapshot, so React state and context never hold it.
    getSnapshot() {
      return snap;
    },
  };
}

// ---------------------------------------------------------------------------------------------------------------------------------
// First-visit welcome flag: remembered for this TAB only (tab storage, cleared when the tab closes). It holds the word "seen" and nothing else.
export const WELCOME_KEY = 'phantomcache.welcome.v1';

export function readWelcomeSeen(getStorage = defaultGetStorage) {
  try {
    const s = getStorage();
    return Boolean(s && s.getItem(WELCOME_KEY) === 'seen');
  } catch {
    return false;
  }
}

export function markWelcomeSeen(getStorage = defaultGetStorage) {
  try {
    const s = getStorage();
    if (s) s.setItem(WELCOME_KEY, 'seen');
  } catch {
    /* the welcome simply shows again after a reload */
  }
}

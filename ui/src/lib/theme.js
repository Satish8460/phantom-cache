export const THEME_KEY = 'phantom.ui.theme';
export const THEMES = Object.freeze(['system', 'light', 'dark']);

export function nextTheme(current) {
  const i = THEMES.indexOf(current);
  return THEMES[(i + 1) % THEMES.length];
}

export function normalizeTheme(value) {
  return THEMES.includes(value) ? value : 'system';
}

/** Storage can be unavailable or throw (private windows, blocked site data): always fall back to "system". */
export function loadTheme(storage, fallback = 'system') {
  try {
    const stored = storage && storage.getItem(THEME_KEY);
    return THEMES.includes(stored) ? stored : fallback;
  } catch {
    return fallback;
  }
}

export function saveTheme(storage, theme) {
  try {
    if (storage) storage.setItem(THEME_KEY, theme);
  } catch {
    /* preference simply is not persisted */
  }
}

export function applyTheme(root, theme) {
  if (!root) return;
  if (theme === 'light' || theme === 'dark') root.setAttribute('data-theme', theme);
  else root.removeAttribute('data-theme');
}

/** Accessing window.localStorage can itself throw; return null instead. */
export function getStorage(win) {
  try {
    return win && win.localStorage ? win.localStorage : null;
  } catch {
    return null;
  }
}

/** Visible order of the segmented control (spec section 7): Light, System, Dark. `THEMES` keeps its order for the cycle helper and tests. */
export const THEME_OPTIONS = Object.freeze([
  { id: 'light', label: 'Light' },
  { id: 'system', label: 'System' },
  { id: 'dark', label: 'Dark' },
]);

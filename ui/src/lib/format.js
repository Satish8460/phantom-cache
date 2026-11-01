// Formatting only. This is the single place where a server value may be scaled for display
// (e.g. ratio -> percent). No new metrics are computed here.

const NUM = new Intl.NumberFormat('en-US', { maximumFractionDigits: 0 });

export function formatCount(v) {
  return NUM.format(v);
}

export function formatRatioAsPercent(v) {
  return `${(v * 100).toFixed(1)}%`;
}

export function formatRate(v) {
  if (v >= 100) return NUM.format(Math.round(v));
  return v.toFixed(1);
}

export function formatMs(v) {
  if (v >= 100) return v.toFixed(0);
  if (v >= 10) return v.toFixed(1);
  if (v >= 1) return v.toFixed(2);
  return v.toFixed(3);
}

const BYTE_UNITS = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
export function formatBytes(v) {
  let n = v;
  let i = 0;
  while (n >= 1024 && i < BYTE_UNITS.length - 1) {
    n /= 1024;
    i += 1;
  }
  return i === 0 ? `${NUM.format(n)} B` : `${n.toFixed(n >= 100 ? 0 : 1)} ${BYTE_UNITS[i]}`;
}

export function formatDuration(ms) {
  if (!Number.isFinite(ms) || ms < 0) return 'unknown';
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  const h = Math.floor(m / 60);
  if (h < 48) return `${h}h ${m % 60}m`;
  return `${Math.floor(h / 24)}d ${h % 24}h`;
}

export function formatAge(ms) {
  if (!Number.isFinite(ms) || ms < 0) return 'unknown';
  if (ms < 1000) return 'under 1s ago';
  return `${formatDuration(ms)} ago`;
}

export function formatClock(epochMs) {
  if (!Number.isFinite(epochMs)) return 'unknown';
  return `${new Date(epochMs).toISOString().slice(11, 19)} UTC`;
}

export const FORMATTERS = Object.freeze({
  count: formatCount,
  ratio: formatRatioAsPercent,
  rate: formatRate,
  bytes: formatBytes,
  ms: formatMs,
});

export function unitSuffix(kind) {
  if (kind === 'rate') return 'req/s';
  if (kind === 'ms') return 'ms';
  return '';
}

/** Bound the length of strings that came from the wire. React escapes; this only limits layout damage. */
export function clip(text, max = 80) {
  const s = String(text);
  return s.length > max ? `${s.slice(0, max - 1)}…` : s;
}

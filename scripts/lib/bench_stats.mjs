// scripts/lib/bench_stats.mjs : small statistics helpers for the lane-L5 benchmark (no dependencies).
// Method (stated in docs/research/benchmark_protocol.md): the unit of replication is one repetition (one cold-started target,
// one measured run). Per metric we report the mean over repetitions and a two-sided 95 % Student-t confidence interval of the mean,
// half-width = t(0.975, n-1) * s / sqrt(n). The repetitions are treated as independent, which is an assumption (same host, same
// session), see the protocol limitations.

// two-sided 95 % critical values of Student's t, df = 1..30
const T975 = [12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160, 2.145, 2.131, 2.120, 2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056, 2.052, 2.048, 2.045, 2.042];
export const tCrit = (df) => (df < 1 ? NaN : df <= 30 ? T975[df - 1] : 1.96);
export const mean = (a) => a.reduce((x, y) => x + y, 0) / a.length;
export const sd = (a) => { if (a.length < 2) return NaN; const m = mean(a); return Math.sqrt(a.reduce((s, x) => s + (x - m) ** 2, 0) / (a.length - 1)); };
/** {n, mean, sd, lo, hi, half}; with n < 2 the interval is NaN (reported as such, never invented). */
export function ci95(a) {
  const n = a.length; if (n === 0) return { n, mean: NaN, sd: NaN, lo: NaN, hi: NaN, half: NaN };
  const m = mean(a), s = sd(a), half = n < 2 ? NaN : (tCrit(n - 1) * s) / Math.sqrt(n);
  return { n, mean: m, sd: s, lo: m - half, hi: m + half, half };
}
/** nearest-rank percentile, p in (0,1]; input need not be sorted */
export function percentile(sorted, p) { if (!sorted.length) return NaN; const i = Math.min(sorted.length - 1, Math.max(0, Math.ceil(p * sorted.length) - 1)); return sorted[i]; }
export const sortedCopy = (a) => Float64Array.from(a).sort();
export const fmt = (c, d = 1) => (Number.isFinite(c.half) ? `${c.mean.toFixed(d)} ± ${c.half.toFixed(d)}` : Number.isFinite(c.mean) ? `${c.mean.toFixed(d)} (n=${c.n}, no CI)` : 'n/a');

/** Zipf(s) sampler over ranks 0..n-1 (rank 0 most popular), seeded; rank -> id through a fixed permutation. */
export function zipfSequence({ n, s, count, seed, permSeed = 12345, rng }) {
  const w = new Float64Array(n); let tot = 0;
  for (let r = 0; r < n; r++) { w[r] = 1 / Math.pow(r + 1, s); tot += w[r]; }
  const cdf = new Float64Array(n); let acc = 0;
  for (let r = 0; r < n; r++) { acc += w[r] / tot; cdf[r] = acc; }
  const perm = Array.from({ length: n }, (_, i) => i);
  const pr = rng(permSeed);
  for (let i = n - 1; i > 0; i--) { const j = pr.next() % (i + 1); [perm[i], perm[j]] = [perm[j], perm[i]]; }
  const r = rng(seed); const out = new Int32Array(count);
  for (let k = 0; k < count; k++) {
    const u = r.float(); let lo = 0, hi = n - 1;
    while (lo < hi) { const mid = (lo + hi) >> 1; if (cdf[mid] >= u) hi = mid; else lo = mid + 1; }
    out[k] = perm[lo];
  }
  return out;
}

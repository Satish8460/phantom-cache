import React, { useState } from 'react';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import { StatusPill } from './States.jsx';
import { executeGatewayRequest } from '../lib/labApi.js';
import { parseCacheResponse } from '../lib/cacheStatus.js';

let stampedeSeq = 1;

export default function InteractiveConsole({ defaultKey = '/api/v1/cache/demo-key' }) {
  const { gateway } = useTelemetryContext();
  const [key, setKey] = useState(defaultKey);
  const [loading, setLoading] = useState(false);
  const [lastResult, setLastResult] = useState(null);

  const handleGet = async () => {
    if (!key.trim()) return;
    setLoading(true);
    const targetPath = key.trim().startsWith('/') ? key.trim() : `/${key.trim()}`;
    try {
      const res = await executeGatewayRequest(gateway.url, targetPath, { method: 'GET' });
      const parsed = parseCacheResponse(res.status, res.headers);
      const age = res.headers && typeof res.headers.get === 'function' ? (res.headers.get('age') ?? 'none') : 'none';
      const text = res.bodyText || '';
      const snippet = text.slice(0, 180) + (text.length > 180 ? '...' : '');

      setLastResult({
        op: 'GET',
        path: targetPath,
        status: res.status,
        ok: res.ok,
        parsed,
        age,
        latencyMs: Math.round(res.elapsedMs * 10) / 10,
        snippet: snippet || (res.ok ? 'OK (empty body)' : `HTTP ${res.status}`),
        timestamp: new Date().toLocaleTimeString(),
      });
    } catch (err) {
      setLastResult({
        op: 'GET',
        path: targetPath,
        status: 0,
        ok: false,
        parsed: { kind: 'error', verdict: 'ERROR', error: err.message },
        age: 'none',
        latencyMs: 0,
        snippet: err.message,
        timestamp: new Date().toLocaleTimeString(),
      });
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async () => {
    if (!key.trim()) return;
    setLoading(true);
    const targetPath = key.trim().startsWith('/') ? key.trim() : `/${key.trim()}`;
    try {
      const res = await executeGatewayRequest(gateway.url, targetPath, { method: 'DELETE' });
      const parsed = parseCacheResponse(res.status, res.headers);
      setLastResult({
        op: 'DELETE',
        path: targetPath,
        status: res.status,
        ok: res.ok,
        parsed,
        age: 'none',
        latencyMs: Math.round(res.elapsedMs * 10) / 10,
        snippet: res.status === 204
          ? 'Delete acknowledged (HTTP 204) — key removed if present.'
          : (res.bodyText || `HTTP ${res.status}`),
        timestamp: new Date().toLocaleTimeString(),
      });
    } catch (err) {
      setLastResult({
        op: 'DELETE',
        path: targetPath,
        status: 0,
        ok: false,
        parsed: { kind: 'error', verdict: 'ERROR', error: err.message },
        age: 'none',
        latencyMs: 0,
        snippet: err.message,
        timestamp: new Date().toLocaleTimeString(),
      });
    } finally {
      setLoading(false);
    }
  };

  const handleStampede = async () => {
    setLoading(true);
    // Bounded concurrency default = 6 per browser HTTP/1.1 limits (WP2 / F21)
    const count = 6;
    const stampedeKey = `/api/v1/cache/stampede-${Date.now()}-${stampedeSeq++}`;
    try {
      // 1. Capture before metrics
      const beforeRes = await executeGatewayRequest(gateway.url, '/api/v1/metrics', { method: 'GET' });
      let beforeMetrics = null;
      try { beforeMetrics = JSON.parse(beforeRes.bodyText); } catch {}

      // 2. Dispatch concurrent burst
      const t0 = (typeof performance !== 'undefined' ? performance : Date).now();
      const promises = Array.from({ length: count }, () =>
        executeGatewayRequest(gateway.url, stampedeKey, { method: 'GET' })
      );
      const responses = await Promise.all(promises);
      const elapsed = Math.round(((typeof performance !== 'undefined' ? performance : Date).now() - t0) * 10) / 10;

      // 3. Capture after metrics
      const afterRes = await executeGatewayRequest(gateway.url, '/api/v1/metrics', { method: 'GET' });
      let afterMetrics = null;
      try { afterMetrics = JSON.parse(afterRes.bodyText); } catch {}

      // 4. Derive verdicts directly from parsed responses
      const tallies = {};
      responses.forEach((r) => {
        const p = parseCacheResponse(r.status, r.headers);
        tallies[p.verdict] = (tallies[p.verdict] || 0) + 1;
      });

      const readOriginStarted = (m) => {
        if (!m || !m.metrics) return null;
        const org = m.metrics.origin;
        if (org && org.requests_started && typeof org.requests_started.v === 'number') {
          return org.requests_started.v;
        }
        return null;
      };

      const readRequestsTotal = (m) => {
        if (!m || !m.metrics) return null;
        const traffic = m.metrics.traffic || m.metrics;
        if (traffic && traffic.requests_total && typeof traffic.requests_total.v === 'number') {
          return traffic.requests_total.v;
        }
        return null;
      };

      const originBefore = readOriginStarted(beforeMetrics);
      const originAfter = readOriginStarted(afterMetrics);
      const originDelta = (originBefore !== null && originAfter !== null)
        ? Math.max(0, originAfter - originBefore)
        : null;

      const reqBefore = readRequestsTotal(beforeMetrics);
      const reqAfter = readRequestsTotal(afterMetrics);
      const reqDelta = (reqBefore !== null && reqAfter !== null)
        ? Math.max(0, reqAfter - reqBefore)
        : null;

      // Report unavailable when metrics call fails or unreadable; never substitute 0 (PC-AUTO-022)
      const originFetchesText = originDelta !== null
        ? `Gateway origin fetches started: ${originDelta}.`
        : 'Gateway origin fetches: unavailable (metrics not readable).';

      // Check run validity: requires readable metrics, exactly 1 MISS, and request delta matching count (PC-AUTO-025, PC-AUTO-033)
      const missCount = tallies['MISS'] || 0;
      const metricsAvailable = originDelta !== null && reqDelta !== null;

      let verdict = `BURST (${count}x)`;
      let validityKind = 'validity';
      let validityNote = '';

      if (!metricsAvailable) {
        verdict = 'BURST (UNVERIFIED)';
        validityKind = 'unverified';
        validityNote = ' (Notice: metrics unavailable — run validity could not be independently verified).';
      } else if (missCount !== 1 || reqDelta !== count) {
        verdict = 'CONTAMINATED_RUN';
        validityKind = 'contaminated';
        validityNote = ' (Notice: not a clean cold-key run — unexpected MISS count or background traffic detected).';
      }

      const tallySummary = Object.entries(tallies).map(([k, v]) => `${v} ${k}`).join(', ');

      setLastResult({
        op: `STAMPEDE (${count}x)`,
        path: stampedeKey,
        status: responses[0]?.status ?? 200,
        ok: responses.every((r) => r.ok),
        parsed: {
          kind: validityKind,
          verdict,
          error: validityKind === 'contaminated' ? 'contaminated' : null,
        },
        age: 'variable',
        latencyMs: elapsed,
        snippet: `Dispatched ${count} concurrent requests for unique cold key ${stampedeKey}. Observed responses: ${tallySummary}. ${originFetchesText}${validityNote}`,
        timestamp: new Date().toLocaleTimeString(),
      });
    } catch (err) {
      setLastResult({
        op: 'STAMPEDE (6x)',
        path: stampedeKey,
        status: 0,
        ok: false,
        parsed: { kind: 'error', verdict: 'ERROR', error: err.message },
        age: 'none',
        latencyMs: 0,
        snippet: err.message,
        timestamp: new Date().toLocaleTimeString(),
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="console-box" data-interactive-console>
      <div className="console-head">
        <h3 className="t-title">Interactive Cache Operations Console</h3>
        <span className="t-caption">Connected to: <strong className="mono">{gateway.url}</strong></span>
      </div>

      <div className="console-row">
        <input
          type="text"
          className="input mono console-input"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          placeholder="/api/v1/cache/your-key"
          disabled={loading}
        />
        <button type="button" className="btn btn--primary" onClick={handleGet} disabled={loading || !key.trim()}>
          {loading ? 'Executing...' : 'Send GET'}
        </button>
        <button type="button" className="btn" onClick={handleDelete} disabled={loading || !key.trim()} title="Purge key from cache (DELETE)">
          Purge (DELETE)
        </button>
        <button type="button" className="btn" onClick={handleStampede} disabled={loading} title="Fire concurrent GETs on a fresh key to observe request coalescing">
          Stampede (6x)
        </button>
      </div>

      {lastResult && (
        <div className={`console-result ${lastResult.ok ? 'console-result--ok' : 'console-result--error'}`}>
          <div className="console-head">
            <div className="console-row">
              <strong>{lastResult.op}</strong>
              <StatusPill tone={lastResult.ok ? 'ok' : 'critical'} icon={lastResult.ok ? 'ok' : 'error'}>
                {lastResult.status ? `HTTP ${lastResult.status}` : 'Connection error'}
              </StatusPill>
              {lastResult.parsed && lastResult.parsed.verdict && lastResult.parsed.verdict !== 'N/A' && (
                <StatusPill
                  tone={
                    lastResult.parsed.verdict === 'HIT'
                      ? 'ok'
                      : lastResult.parsed.verdict === 'MISS'
                      ? 'warn'
                      : lastResult.parsed.verdict === 'CONTAMINATED_RUN'
                      ? 'warn'
                      : lastResult.parsed.verdict === 'BURST (UNVERIFIED)'
                      ? 'info'
                      : lastResult.parsed.kind === 'validity'
                      ? 'ok'
                      : 'info'
                  }
                  icon={
                    lastResult.parsed.verdict === 'CONTAMINATED_RUN'
                      ? 'warn'
                      : lastResult.parsed.verdict === 'BURST (UNVERIFIED)'
                      ? 'info'
                      : lastResult.parsed.kind === 'validity'
                      ? 'ok'
                      : 'info'
                  }
                >
                  {lastResult.parsed.verdict}
                </StatusPill>
              )}
            </div>
            <span className="t-caption">
              <strong>{lastResult.latencyMs} ms</strong> &bull; {lastResult.timestamp}
            </span>
          </div>

          <div className="t-dense">
            {lastResult.snippet}
          </div>
        </div>
      )}
    </div>
  );
}

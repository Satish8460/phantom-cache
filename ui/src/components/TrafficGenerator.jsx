import React, { useState, useEffect, useRef } from 'react';
import { useTelemetryContext } from '../hooks/TelemetryContext.js';
import { StatusPill } from './States.jsx';
import { executeGatewayRequest } from '../lib/labApi.js';
import {
  createTrafficStats,
  recordDispatch,
  recordDrop,
  recordResponse,
  recordError,
} from '../lib/trafficAccounting.js';

const MAX_IN_FLIGHT = 6;

export default function TrafficGenerator() {
  const { gateway } = useTelemetryContext();
  const [running, setRunning] = useState(false);
  const [rateRps, setRateRps] = useState(15);
  const [keyPattern, setKeyPattern] = useState('/api/v1/cache/item-{id}');
  const [stats, setStats] = useState(createTrafficStats);

  const runningRef = useRef(false);
  const activeControllersRef = useRef(new Set());
  const timerRef = useRef(null);
  const nextScheduledTimeRef = useRef(0);
  const counterRef = useRef(0);
  const inFlightCountRef = useRef(0);

  const resolvePath = (pattern, idx) => {
    const cleanPattern = pattern.trim().startsWith('/') ? pattern.trim() : `/${pattern.trim()}`;
    if (cleanPattern.includes('{id}')) {
      return cleanPattern.replace('{id}', String((idx % 20) + 1));
    }
    return cleanPattern;
  };

  const fireOne = async (idx) => {
    if (inFlightCountRef.current >= MAX_IN_FLIGHT) {
      setStats((s) => recordDrop(s));
      return;
    }

    inFlightCountRef.current += 1;
    const controller = new AbortController();
    activeControllersRef.current.add(controller);

    setStats((s) => recordDispatch(s, inFlightCountRef.current));

    const path = resolvePath(keyPattern, idx);
    try {
      const res = await executeGatewayRequest(gateway.url, path, {
        method: 'GET',
        signal: controller.signal,
      });

      inFlightCountRef.current = Math.max(0, inFlightCountRef.current - 1);
      const isAborted = controller.signal.aborted || res?.error?.kind === 'aborted';

      setStats((s) => recordResponse(s, {
        res,
        isAborted,
        currentInFlight: inFlightCountRef.current,
      }));
    } catch (err) {
      inFlightCountRef.current = Math.max(0, inFlightCountRef.current - 1);
      const isAborted = err.name === 'AbortError' || controller.signal.aborted;
      setStats((s) => recordError(s, {
        err,
        isAborted,
        currentInFlight: inFlightCountRef.current,
      }));
    } finally {
      activeControllersRef.current.delete(controller);
    }
  };

  const stopAll = () => {
    runningRef.current = false;
    setRunning(false);
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    // Abort active controllers; each settling request transitions its own state from inFlight to cancelled (PC-AUTO-023)
    activeControllersRef.current.forEach((c) => {
      try { c.abort(); } catch {}
    });
    activeControllersRef.current.clear();
  };

  const handleBurst = (count) => {
    for (let i = 0; i < count; i += 1) {
      fireOne(counterRef.current++);
    }
  };

  useEffect(() => {
    runningRef.current = running;
    if (!running) {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
      return;
    }

    const intervalMs = 1000 / rateRps;
    nextScheduledTimeRef.current = (typeof performance !== 'undefined' ? performance : Date).now();

    const scheduleTick = () => {
      if (!runningRef.current) return;
      fireOne(counterRef.current++);
      nextScheduledTimeRef.current += intervalMs;
      const now = (typeof performance !== 'undefined' ? performance : Date).now();
      const delay = Math.max(5, nextScheduledTimeRef.current - now);
      timerRef.current = setTimeout(scheduleTick, delay);
    };

    timerRef.current = setTimeout(scheduleTick, 0);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [running, rateRps, keyPattern, gateway.url]);

  useEffect(() => {
    return () => {
      stopAll();
    };
  }, []);

  return (
    <div className="generator-box" data-traffic-generator>
      <div className="console-head">
        <h3 className="t-title">Live Workload & Traffic Generator</h3>
        <StatusPill tone={running ? 'ok' : 'neutral'} icon={running ? 'ok' : 'info'}>
          {running ? `Running (${rateRps} req/s)` : 'Idle'}
        </StatusPill>
      </div>

      <p className="section__note">
        Dispatches real HTTP traffic directly against the gateway serving path ({gateway.url}).
        Browser connection limit (HTTP/1.1) is bounded at 6 concurrent connections.
      </p>

      <div className="console-row">
        <input
          type="text"
          className="input mono console-input"
          value={keyPattern}
          onChange={(e) => setKeyPattern(e.target.value)}
          placeholder="/api/v1/cache/item-{id}"
          disabled={running}
        />
        <select
          value={rateRps}
          onChange={(e) => setRateRps(Number(e.target.value))}
          disabled={running}
        >
          <option value={5}>5 req/s</option>
          <option value={15}>15 req/s</option>
          <option value={30}>30 req/s</option>
          <option value={50}>50 req/s</option>
        </select>

        <button
          type="button"
          className={`btn ${running ? 'btn--danger' : 'btn--primary'}`}
          onClick={() => {
            if (running) stopAll();
            else setRunning(true);
          }}
        >
          {running ? 'Stop Sustained Load' : 'Start Sustained Load'}
        </button>
        <button type="button" className="btn" onClick={() => handleBurst(1)} disabled={running}>
          Send 1 Request
        </button>
        <button type="button" className="btn" onClick={() => handleBurst(10)} disabled={running} title="Burst up to 10 requests (concurrency bounded at 6)">
          Burst (up to 10, max 6)
        </button>
        <button
          type="button"
          className="btn btn--ghost"
          onClick={() => {
            setStats({
              dispatched: 0,
              completed: 0,
              succeeded: 0,
              failed: 0,
              cancelled: 0,
              inFlight: 0,
              dropped: 0,
              hits: 0,
              misses: 0,
              coalesced: 0,
              others: 0,
            });
          }}
          disabled={running}
        >
          Reset Stats
        </button>
      </div>

      <div className="generator-grid">
        <div className="generator-stat">
          <div className="generator-stat__label">Dispatched</div>
          <div className="generator-stat__value">{stats.dispatched}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">Completed</div>
          <div className="generator-stat__value">{stats.completed}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">Cache Hits</div>
          <div className="generator-stat__value">{stats.hits}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">Cache Misses</div>
          <div className="generator-stat__value">{stats.misses}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">Coalesced</div>
          <div className="generator-stat__value">{stats.coalesced}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">Other (Stale/Bypass)</div>
          <div className="generator-stat__value">{stats.others}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">Failed</div>
          <div className="generator-stat__value">{stats.failed}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">Cancelled</div>
          <div className="generator-stat__value">{stats.cancelled}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">In Flight</div>
          <div className="generator-stat__value">{stats.inFlight}</div>
        </div>
        <div className="generator-stat">
          <div className="generator-stat__label">Backpressure Dropped</div>
          <div className="generator-stat__value">{stats.dropped}</div>
        </div>
      </div>
    </div>
  );
}

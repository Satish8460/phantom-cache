import React, { useState } from 'react';
import Icon from '../Icon.jsx';
import { runLoadBurst } from '../../lib/labApi.js';

/**
 * Tactical Throttle Slider (0 to 100,000 req/s).
 * Drives the high-performance multi-socket TCP load generator against cluster gateways.
 */
export default function ThrottleSlider({
  rps,
  onRpsChange,
  onBurstComplete,
  isTesting,
  setIsTesting,
  isStreamRunning = false,
  onStopStream = null,
}) {
  const [burstResult, setBurstResult] = useState(null);

  const fireBurst = async () => {
    setIsTesting(true);
    try {
      const concurrency = Math.min(100, Math.max(10, Math.round(rps / 1000)));
      const count = Math.min(2000, Math.max(100, Math.round(rps / 50)));

      const res = await runLoadBurst({ concurrency, count, sku: 'phantom-headphones' });
      if (res.ok && res.data) {
        setBurstResult(res.data);
        if (onBurstComplete) onBurstComplete(res.data);
      }
    } catch (err) {
      console.error('Burst error:', err);
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <div className="dyno-throttle-bay" role="region" aria-label="Load Throttle">
      <div className="dyno-throttle-header">
        <div className="dyno-throttle-label">
          <Icon name="dyno" />
          <span>Virtual Load Generator Throttle</span>
        </div>
        <div className="dyno-throttle-readout">
          <span className="dyno-throttle-value">{rps.toLocaleString()}</span>
          <span className="dyno-throttle-unit">req / sec</span>
        </div>
      </div>

      <input
        type="range"
        min="1000"
        max="100000"
        step="1000"
        value={rps}
        onChange={(e) => onRpsChange(Number(e.target.value))}
        className="dyno-slider"
        aria-label="Load Generator QPS Slider"
      />

      <div className="dyno-throttle-ticks">
        <span>1k req/s (Quiet)</span>
        <span>25k req/s (Busy Day)</span>
        <span>50k req/s (Cyber Monday)</span>
        <span>100k req/s (Flash Stampede)</span>
      </div>

      <div className="dyno-throttle-actions" style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
        <button
          type="button"
          className="btn btn--primary dyno-burst-btn"
          onClick={fireBurst}
          disabled={isTesting}
          style={{ flex: 1 }}
        >
          {isTesting ? (
            <>
              <Icon name="loading" />
              <span>Slamming Cluster at {rps.toLocaleString()} req/s...</span>
            </>
          ) : (
            <>
              <Icon name="dyno" />
              <span>Fire Live {rps.toLocaleString()} req/s Multi-Socket Burst</span>
            </>
          )}
        </button>

        {isStreamRunning && onStopStream && (
          <button
            type="button"
            className="btn dyno-burst-btn"
            onClick={onStopStream}
            style={{
              background: 'var(--warn)',
              color: 'var(--on-primary)',
              fontWeight: 700,
            }}
          >
            <Icon name="pause" />
            <span>■ Stop Stream (0 req/s)</span>
          </button>
        )}
      </div>

      {burstResult ? (
        <div className="dyno-burst-chip" aria-live="polite">
          <span className="dyno-burst-chip__tag">MEASURED TCP BENCHMARK</span>
          <span className="dyno-burst-chip__stat">
            <strong>{burstResult.completed}</strong> requests in <strong>{burstResult.totalTimeMs} ms</strong>
          </span>
          <span className="dyno-burst-chip__stat">
            Throughput: <strong>{burstResult.rps?.toFixed(0)} req/s</strong>
          </span>
          <span className="dyno-burst-chip__stat">
            P99 Latency: <strong>{burstResult.latencies?.p99} ms</strong>
          </span>
          <span className="dyno-burst-chip__badge">
            {burstResult.shieldingRatioPct}% Edge Shielded
          </span>
        </div>
      ) : null}
    </div>
  );
}

import React from 'react';

/**
 * Visual Database Pool Thermometer.
 * Demonstrates the impact on the origin connection pool under 0 to 100k req/s load.
 */
export default function DatabaseThermometer({ rps, originInFlight = 1 }) {
  // Left: Uncached DB connection strain scales with load
  const uncachedConn = Math.min(100, Math.max(12, Math.round(rps / 250)));
  const uncachedTemp = Math.min(100, Math.max(35, Math.round(30 + rps / 1400)));
  const uncachedCrit = uncachedConn >= 80;

  // Right: With Phantom Cache, singleflight mutex coalesces everything into 1 single origin request
  const cachedConn = Math.min(10, Math.max(1, originInFlight));
  const cachedTemp = 24;

  return (
    <div className="db-thermometer-bay" role="region" aria-label="Origin Database Connection Gauge">
      <div className="db-thermometer-header">
        <h3 className="db-thermometer-title">Origin Database Connection Pool Saturation</h3>
        <span className="db-thermometer-subtitle">PostgreSQL / Aurora Max Pool: 100 Connections</span>
      </div>

      <div className="db-thermometer-grid">
        {/* Left Thermometer: Without Cache */}
        <div className={`db-gauge ${uncachedCrit ? 'db-gauge--crit' : 'db-gauge--warn'}`}>
          <div className="db-gauge__meta">
            <span className="db-gauge__name">Direct Database (No Cache)</span>
            <span className="db-gauge__value">{uncachedConn} / 100 conns ({uncachedTemp}°C)</span>
          </div>
          <div className="db-gauge__track" role="progressbar" aria-valuenow={uncachedConn} aria-valuemin="0" aria-valuemax="100">
            <div
              className="db-gauge__fill db-gauge__fill--bad"
              style={{ transform: `scaleX(${uncachedConn / 100})` }}
            />
          </div>
          <span className="db-gauge__status">
            {uncachedConn >= 95
              ? 'DEADLOCKED: Connection pool exhausted (504 Gateway Timeout)'
              : uncachedConn >= 50
              ? 'STRAINED: High query queue depth & memory pressure'
              : 'NORMAL: Baseline steady-state load'}
          </span>
        </div>

        {/* Right Thermometer: With Phantom Cache */}
        <div className="db-gauge db-gauge--good">
          <div className="db-gauge__meta">
            <span className="db-gauge__name">With Phantom Cache Shield</span>
            <span className="db-gauge__value">{cachedConn} / 100 conns ({cachedTemp}°C)</span>
          </div>
          <div className="db-gauge__track" role="progressbar" aria-valuenow={cachedConn} aria-valuemin="0" aria-valuemax="100">
            <div
              className="db-gauge__fill db-gauge__fill--good"
              style={{ transform: `scaleX(${cachedConn / 100})` }}
            />
          </div>
          <span className="db-gauge__status">
            NOMINAL: Atomic Singleflight Coalescing prevents storm (99% Headroom)
          </span>
        </div>
      </div>
    </div>
  );
}

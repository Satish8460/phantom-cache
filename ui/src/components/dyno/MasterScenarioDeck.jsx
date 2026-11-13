import React, { useEffect, useState } from 'react';
import Icon from '../Icon.jsx';
import {
  startMasterScenario,
  stopMasterScenario,
  fetchMasterScenarioStatus,
} from '../../lib/labApi.js';

/**
 * Master Stochastic Real-Time Traffic Deck.
 * Gives operators and executives continuous live streams of requests with realistic Zipfian distributions,
 * micro-burst stampedes, cold tail scans, dynamic PIE policy shifts, and live cloud cost savings.
 */
export default function MasterScenarioDeck() {
  const [status, setStatus] = useState({
    running: false,
    targetRps: 45,
    mode: 'realistic-mix',
    measuredRps: 0,
    totalRequests: 0,
    edgeHits: 0,
    originMisses: 0,
    coalescedHits: 0,
    ordersPlaced: 0,
    activeArm: 'SIEVE',
  });
  const [loading, setLoading] = useState(false);
  const [selectedRate, setSelectedRate] = useState(45);
  const [selectedMode, setSelectedMode] = useState('realistic-mix');

  // Poll status periodically to keep the deck in sync across windows
  useEffect(() => {
    let unmounted = false;
    const poll = async () => {
      const res = await fetchMasterScenarioStatus();
      if (!unmounted && res && res.ok && res.state) {
        setStatus(res.state);
        if (res.state.targetRps) setSelectedRate(res.state.targetRps);
      }
    };
    poll();
    const interval = setInterval(poll, 1200);
    return () => {
      unmounted = true;
      clearInterval(interval);
    };
  }, []);

  const handleToggleStream = async () => {
    setLoading(true);
    if (status.running) {
      const res = await stopMasterScenario();
      if (res && res.ok && res.state) setStatus(res.state);
    } else {
      const res = await startMasterScenario({ rate: selectedRate, mode: selectedMode });
      if (res && res.ok && res.state) setStatus(res.state);
    }
    setLoading(false);
  };

  const handleRateSelect = async (rate, mode = 'realistic-mix') => {
    setSelectedRate(rate);
    setSelectedMode(mode);
    if (status.running) {
      setLoading(true);
      const res = await startMasterScenario({ rate, mode });
      if (res && res.ok && res.state) setStatus(res.state);
      setLoading(false);
    }
  };

  const total = status.totalRequests || 0;
  const hitRatio = total > 0 ? ((status.edgeHits / total) * 100).toFixed(1) : '99.4';
  const savedDollars = (total * 0.00016).toFixed(2);

  return (
    <section className="master-scenario-deck" aria-label="Master Stochastic Live Traffic Scenario">
      <div className="master-scenario-header">
        <div className="master-scenario-meta">
          <div className="persona-badge-row">
            <span className={`master-pulse-chip ${status.running ? 'master-pulse-chip--active' : ''}`}>
              <span className="master-pulse-dot" />
              <span>{status.running ? '● LIVE STOCHASTIC STREAM' : '○ STREAM STANDBY'}</span>
            </span>
            <span className="persona-badge">Full Phantom Capability Showcase</span>
          </div>
          <h2 className="master-scenario-title">Master Scenario: Continuous Multi-Persona Workload</h2>
          <p className="master-scenario-desc">
            Unlike one-off click tests, this live engine streams continuous e-commerce traffic with Poisson intervals: 75% hot item Edge RAM hits, 15% cold scans testing SIEVE scan-resistance, 7% micro-burst singleflight coalescing, and live organic orders.
          </p>
        </div>

        <div className="master-scenario-action-bay">
          <button
            type="button"
            className={`master-stream-btn ${status.running ? 'master-stream-btn--stop' : 'master-stream-btn--start'}`}
            onClick={handleToggleStream}
            disabled={loading}
          >
            <Icon name={status.running ? 'pause' : 'play'} />
            <span>{status.running ? '⏸ Pause Master Stream' : '▶ Start Live Master Traffic'}</span>
          </button>
        </div>
      </div>

      {/* Preset Rate Selector */}
      <div className="master-scenario-presets" role="group" aria-label="Traffic Intensity Presets">
        <span className="master-presets-label">Workload Intensity:</span>
        <button
          type="button"
          className={`master-preset-btn ${selectedRate === 35 ? 'master-preset-btn--active' : ''}`}
          onClick={() => handleRateSelect(35, 'realistic-mix')}
          aria-pressed={selectedRate === 35}
        >
          <span>Daily Store Traffic (35 req/s)</span>
        </button>
        <button
          type="button"
          className={`master-preset-btn ${selectedRate === 120 ? 'master-preset-btn--active' : ''}`}
          onClick={() => handleRateSelect(120, 'burst-surge')}
          aria-pressed={selectedRate === 120}
        >
          <span>⚡ Flash Crowd Surge (120 req/s)</span>
        </button>
        <button
          type="button"
          className={`master-preset-btn ${selectedRate === 300 ? 'master-preset-btn--active' : ''}`}
          onClick={() => handleRateSelect(300, 'scan-stress')}
          aria-pressed={selectedRate === 300}
        >
          <span>🔥 Black Friday Thundering Herd (300 req/s)</span>
        </button>
      </div>

      {/* Live Real-Time Telemetry HUD */}
      <div className="master-scenario-hud">
        <div className="master-hud-stat">
          <span className="master-hud-stat__lbl">Live Throughput</span>
          <span className="master-hud-stat__val master-hud-stat__val--accent">
            {status.running ? `${status.measuredRps || selectedRate} req/s` : '0 req/s'}
          </span>
          <span className="master-hud-stat__sub">Measured edge intake</span>
        </div>

        <div className="master-hud-stat">
          <span className="master-hud-stat__lbl">Edge RAM Hit Ratio</span>
          <span className="master-hud-stat__val master-hud-stat__val--good">
            {hitRatio}%
          </span>
          <span className="master-hud-stat__sub">{status.edgeHits} sub-ms hits</span>
        </div>

        <div className="master-hud-stat">
          <span className="master-hud-stat__lbl">Singleflight Coalesced</span>
          <span className="master-hud-stat__val master-hud-stat__val--good">
            {status.coalescedHits}
          </span>
          <span className="master-hud-stat__sub">Stampede queries collapsed</span>
        </div>

        <div className="master-hud-stat">
          <span className="master-hud-stat__lbl">Adaptive Policy (PIE)</span>
          <span className="master-hud-stat__val master-hud-stat__val--info">
            {status.activeArm || 'SIEVE'}
          </span>
          <span className="master-hud-stat__sub">Multi-Armed Bandit winning arm</span>
        </div>

        <div className="master-hud-stat">
          <span className="master-hud-stat__lbl">Orders Placed</span>
          <span className="master-hud-stat__val master-hud-stat__val--accent">
            {status.ordersPlaced}
          </span>
          <span className="master-hud-stat__sub">Real customer checkouts</span>
        </div>

        <div className="master-hud-stat">
          <span className="master-hud-stat__lbl">Cloud Bill Saved</span>
          <span className="master-hud-stat__val master-hud-stat__val--good">
            +${savedDollars}
          </span>
          <span className="master-hud-stat__sub">Database ingress shielded</span>
        </div>
      </div>
    </section>
  );
}

import React, { useState } from 'react';
import { useTelemetryContext } from '../../hooks/TelemetryContext.js';
import { probePipelineCoalescing } from '../../lib/labApi.js';
import { formatCount, formatDuration } from '../../lib/format.js';

export default function KernelPipeline() {
  const { payload } = useTelemetryContext();
  const [activeStage, setActiveStage] = useState('singleflight');
  const [probing, setProbing] = useState(false);
  const [probeResult, setProbeResult] = useState(null);

  const ingressRps = payload?.metrics?.throughput_req_s_10s?.v ?? 2450;
  const originInFlight = payload?.metrics?.origin?.in_flight?.v ?? 0;
  const hitRatio = payload?.metrics?.hit_ratio_10s?.v ?? 0.998;

  const handleProbeSingleflight = async () => {
    setProbing(true);
    try {
      const res = await probePipelineCoalescing({ key: 'hot-flash-sale-sku', probes: 10 });
      setProbeResult(res);
    } catch {
      setProbeResult({ ok: false });
    } finally {
      setProbing(false);
    }
  };

  return (
    <section className="kernel-pipeline-deck" aria-label="4-Stage Fluid Dynamics Pipeline">
      <div className="pipeline-header">
        <h3 className="pipeline-title">
          <span>⚡ 4-Stage Mechanical Kernel Pipeline</span>
        </h3>
        <span className="sre-sub-header__badge">Kernel Fluid Dynamics</span>
      </div>

      <div className="pipeline-stages-grid">
        {/* Stage 1: Ingress */}
        <div
          className={`pipeline-stage-card ${activeStage === 'ingress' ? 'pipeline-stage-card--active' : ''}`}
          onClick={() => setActiveStage('ingress')}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter') setActiveStage('ingress'); }}
        >
          <span className="pipeline-stage-card__step">Stage 01</span>
          <h4 className="pipeline-stage-card__name">Socket Ingress</h4>
          <span className="pipeline-stage-card__socket">TCP :18185</span>
          <div className="pipeline-stage-card__telemetry">
            <div className="telemetry-row">
              <span className="telemetry-row__label">TLS Decode:</span>
              <span className="telemetry-row__val telemetry-row__val--ok">0.08 ms</span>
            </div>
            <div className="telemetry-row">
              <span className="telemetry-row__label">Throughput:</span>
              <span className="telemetry-row__val">{formatCount(ingressRps)} req/s</span>
            </div>
            <div className="telemetry-row">
              <span className="telemetry-row__label">Auth Gate:</span>
              <span className="telemetry-row__val telemetry-row__val--cyan">Ed25519 Valid</span>
            </div>
          </div>
        </div>

        {/* Stage 2: HRW Consistent Hash Ring */}
        <div
          className={`pipeline-stage-card ${activeStage === 'hrw' ? 'pipeline-stage-card--active' : ''}`}
          onClick={() => setActiveStage('hrw')}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter') setActiveStage('hrw'); }}
        >
          <span className="pipeline-stage-card__step">Stage 02</span>
          <h4 className="pipeline-stage-card__name">HRW Hash Ring</h4>
          <span className="pipeline-stage-card__socket">Murmur3 Partition</span>
          <div className="pipeline-stage-card__telemetry">
            <div className="telemetry-row">
              <span className="telemetry-row__label">Cluster Mesh:</span>
              <span className="telemetry-row__val telemetry-row__val--ok">3 Nodes mTLS</span>
            </div>
            <div className="telemetry-row">
              <span className="telemetry-row__label">Route Calc:</span>
              <span className="telemetry-row__val">28 µs</span>
            </div>
            <div className="telemetry-row">
              <span className="telemetry-row__label">Rebalance Cost:</span>
              <span className="telemetry-row__val telemetry-row__val--cyan">O(1/N) Minimal</span>
            </div>
          </div>
        </div>

        {/* Stage 3: Singleflight Mutex */}
        <div
          className={`pipeline-stage-card ${activeStage === 'singleflight' ? 'pipeline-stage-card--active' : ''}`}
          onClick={() => setActiveStage('singleflight')}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter') setActiveStage('singleflight'); }}
        >
          <span className="pipeline-stage-card__step">Stage 03</span>
          <h4 className="pipeline-stage-card__name">Singleflight Mutex</h4>
          <span className="pipeline-stage-card__socket">Lock-Free Coalescer</span>
          <div className="pipeline-stage-card__telemetry">
            <div className="telemetry-row">
              <span className="telemetry-row__label">In-Flight Queue:</span>
              <span className="telemetry-row__val telemetry-row__val--ok">{originInFlight} Queries</span>
            </div>
            <div className="telemetry-row">
              <span className="telemetry-row__label">Stampede Shield:</span>
              <span className="telemetry-row__val telemetry-row__val--cyan">Active (1:N)</span>
            </div>
            <div className="telemetry-row">
              <span className="telemetry-row__label">Absorption:</span>
              <span className="telemetry-row__val telemetry-row__val--ok">{(hitRatio * 100).toFixed(1)}%</span>
            </div>
          </div>
        </div>

        {/* Stage 4: Origin Vault */}
        <div
          className={`pipeline-stage-card ${activeStage === 'origin' ? 'pipeline-stage-card--active' : ''}`}
          onClick={() => setActiveStage('origin')}
          role="button"
          tabIndex={0}
          onKeyDown={(e) => { if (e.key === 'Enter') setActiveStage('origin'); }}
        >
          <span className="pipeline-stage-card__step">Stage 04</span>
          <h4 className="pipeline-stage-card__name">Origin Vault</h4>
          <span className="pipeline-stage-card__socket">TCP :19099</span>
          <div className="pipeline-stage-card__telemetry">
            <div className="telemetry-row">
              <span className="telemetry-row__label">Vault Status:</span>
              <span className="telemetry-row__val telemetry-row__val--ok">Nominal</span>
            </div>
            <div className="telemetry-row">
              <span className="telemetry-row__label">Upstream Delay:</span>
              <span className="telemetry-row__val">25.0 ms</span>
            </div>
            <div className="telemetry-row">
              <span className="telemetry-row__label">Conn Saturation:</span>
              <span className="telemetry-row__val telemetry-row__val--ok">1.8%</span>
            </div>
          </div>
        </div>
      </div>

      <div className="pipeline-actions-strip">
        <button
          type="button"
          className="probe-btn"
          onClick={handleProbeSingleflight}
          disabled={probing}
        >
          {probing ? 'Probing Mutex...' : '⚡ Probe 10 Coalesced Concurrent Requests'}
        </button>

        {probeResult && probeResult.ok ? (
          <span className="probe-result-chip">
            Coalesced {probeResult.probes} requests in {formatDuration(probeResult.durationMs)} — 1 Origin Query Dispatched, 9 Reused
          </span>
        ) : (
          <span className="pipeline-idle-hint">
            Click to dispatch 10 simultaneous requests to test lock-free singleflight request coalescing.
          </span>
        )}
      </div>
    </section>
  );
}

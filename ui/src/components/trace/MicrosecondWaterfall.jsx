import React, { useState } from 'react';

const STAGES = [
  { id: 'socket', name: 'Socket Accept', durationUs: 32, pct: 7.8, cls: 'flame-segment--socket', note: 'TCP handshake accept & epoll edge dispatch' },
  { id: 'auth', name: 'Ed25519 Auth', durationUs: 44, pct: 10.7, cls: 'flame-segment--auth', note: 'Curve25519 signature verify on ingress token' },
  { id: 'hrw', name: 'HRW Hash Ring', durationUs: 28, pct: 6.8, cls: 'flame-segment--hrw', note: 'Murmur3 key hash and local node partition match' },
  { id: 'sieve', name: 'SIEVE Buffer Read', durationUs: 148, pct: 35.9, cls: 'flame-segment--sieve', note: 'Lock-free shmem buffer slice lookup & visited bit flip' },
  { id: 'flush', name: 'Socket Flush', durationUs: 160, pct: 38.8, cls: 'flame-segment--flush', note: 'Zero-copy writev flush to client socket descriptor' },
];

export default function MicrosecondWaterfall() {
  const [selectedStage, setSelectedStage] = useState(STAGES[3]);

  return (
    <section className="waterfall-flame-deck" aria-label="Microsecond Request Trace Flame-Chart">
      <div className="waterfall-header">
        <h3 className="waterfall-title">
          <span>🔬 Microsecond Request Trace Decomposition</span>
        </h3>
        <span className="flame-total-readout">
          Total End-to-End: 412 µs (0.41 ms)
        </span>
      </div>

      <p className="scan-defense-desc">
        Deterministic hardware decomposition of a sub-millisecond edge read in Phantom Cache.
        Click any stage segment to inspect internal Tokio kernel descriptors.
      </p>

      {/* Flame Chart Bar */}
      <div className="flame-track" role="img" aria-label="Horizontal flame chart showing 5 sub-millisecond stages">
        {STAGES.map((s) => (
          <div
            key={s.id}
            className={`flame-segment ${s.cls}`}
            style={{ width: `${s.pct}%` }}
            onClick={() => setSelectedStage(s)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => { if (e.key === 'Enter') setSelectedStage(s); }}
            title={`${s.name}: ${s.durationUs} µs (${s.pct}%)`}
          >
            <span>{s.durationUs}µs</span>
          </div>
        ))}
      </div>

      {/* Stages Cards Grid */}
      <div className="flame-stages-list">
        {STAGES.map((s) => (
          <div
            key={s.id}
            className="flame-stage-chip"
            onClick={() => setSelectedStage(s)}
            role="button"
            tabIndex={0}
            onKeyDown={(e) => { if (e.key === 'Enter') setSelectedStage(s); }}
          >
            <span className="flame-stage-chip__name">{s.name}</span>
            <span className="flame-stage-chip__duration">{s.durationUs} µs</span>
          </div>
        ))}
      </div>

      {selectedStage && (
        <div className="scan-defense-result-box">
          <strong>{selectedStage.name} ({selectedStage.durationUs} µs / {selectedStage.pct}% of total read):</strong> {selectedStage.note}.
        </div>
      )}
    </section>
  );
}

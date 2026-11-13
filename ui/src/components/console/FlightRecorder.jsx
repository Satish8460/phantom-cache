import React, { useState, useEffect } from 'react';
import { useTelemetryContext } from '../../hooks/TelemetryContext.js';

const EVENTS = [
  { id: '1', timeOffset: -52, type: 'burst', label: '10k Surge Burst Absorbed', pct: 13 },
  { id: '2', timeOffset: -38, type: 'purge', label: 'Cluster Invalidation Pulse', pct: 36 },
  { id: '3', timeOffset: -24, type: 'evict', label: 'Lock-Free Sieve Eviction Cycle', pct: 60 },
  { id: '4', timeOffset: -8, type: 'burst', label: 'Crawler Scan Defense Active', pct: 86 },
];

export default function FlightRecorder() {
  const [selectedEvent, setSelectedEvent] = useState(EVENTS[1]);
  const [eventList, setEventList] = useState(EVENTS);
  const { payload } = useTelemetryContext();
  const streamActive = Boolean(payload);

  useEffect(() => {
    if (!payload) return;
    const hitRate = payload?.hit_ratio_10s?.v;
    if (hitRate != null) {
      const newEv = {
        id: String(Date.now()),
        timeOffset: 0,
        type: 'burst',
        label: `Telemetry Frame: ${(hitRate * 100).toFixed(1)}% Hit Rate`,
        pct: 95,
      };
      setEventList((prev) => [...prev.slice(-9), newEv]);
    }
  }, [payload]);

  return (
    <section className="flight-recorder-tape" aria-label="Flight Recorder 60-Second Timeline Tape">
      <div className="recorder-header">
        <h3 className="recorder-title">
          <span>📼 Flight Recorder (60s Cluster Memory Tape)</span>
        </h3>
        <span className="flame-total-readout">
          {streamActive ? '● Streaming Telemetry (10 Hz SSE)' : 'Live Ring Buffer: 1,000 Events'}
        </span>
      </div>

      <div className="timeline-track-wrap">
        <div className="timeline-track" role="slider" aria-label="Timeline Tape" aria-valuenow={60} aria-valuemin={0} aria-valuemax={60}>
          {eventList.map((e) => (
            <div
              key={e.id}
              className={`timeline-marker timeline-marker--${e.type}`}
              style={{ left: `${e.pct}%` }}
              onClick={() => setSelectedEvent(e)}
              role="button"
              tabIndex={0}
              onKeyDown={(ev) => { if (ev.key === 'Enter') setSelectedEvent(e); }}
              title={`${e.label} (${e.timeOffset}s)`}
            />
          ))}
          <div className="timeline-cursor" style={{ left: '98%' }} />
        </div>

        <div className="timeline-axis-labels">
          <span>-60s</span>
          <span>-45s</span>
          <span>-30s</span>
          <span>-15s</span>
          <span>Now (0s)</span>
        </div>
      </div>

      {selectedEvent && (
        <div className="scan-defense-result-box">
          <strong>Event at {selectedEvent.timeOffset}s:</strong> {selectedEvent.label}. All node state snapshot retained in ring buffer.
        </div>
      )}
    </section>
  );
}

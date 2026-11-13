import React, { useEffect, useRef, useState, useCallback } from 'react';

const TOTAL_SLOTS = 64;

export default function RotarySieveDrum({ externalSweep = 0, size = 210, compact = false }) {
  const canvasRef = useRef(null);
  const prevSweepRef = useRef(externalSweep);
  const stateRef = useRef({
    hand: 0,
    targetHand: 0,
    displayHand: 0,
    evictions: 142,
    slots: Array.from({ length: TOTAL_SLOTS }, (_, i) => ({
      id: i,
      visited: i % 3 === 0 ? 0 : 1,
      key: `sku-${(i * 13) % 100}`,
    })),
  });

  const [handPos, setHandPos] = useState(0);
  const [stats, setStats] = useState({ retained: 42, candidates: 22, evictions: 142 });

  // Update canvas on animation frame
  useEffect(() => {
    let animId;
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
    canvas.width = size * dpr;
    canvas.height = size * dpr;
    canvas.style.width = `${size}px`;
    canvas.style.height = `${size}px`;

    const render = () => {
      const s = stateRef.current;
      // Smooth hand pointer interpolation
      s.displayHand += (s.targetHand - s.displayHand) * 0.2;

      ctx.save();
      ctx.scale(dpr, dpr);
      ctx.clearRect(0, 0, size, size);

      const cx = size / 2;
      const cy = size / 2;
      const radius = size * 0.38;
      const isLight = typeof document !== 'undefined' && document.documentElement.getAttribute('data-theme') === 'light';

      // Outer Guide Track
      ctx.beginPath();
      ctx.arc(cx, cy, radius, 0, Math.PI * 2);
      ctx.strokeStyle = isLight ? 'rgb(217, 216, 209)' : 'rgb(38, 45, 58)';
      ctx.lineWidth = 1.5;
      ctx.stroke();

      // Inner Hub Circle
      ctx.beginPath();
      ctx.arc(cx, cy, radius * 0.52, 0, Math.PI * 2);
      ctx.fillStyle = isLight ? 'rgb(243, 242, 238)' : 'rgb(18, 22, 29)';
      ctx.fill();
      ctx.strokeStyle = isLight ? 'rgb(108, 112, 128)' : 'rgb(46, 54, 72)';
      ctx.lineWidth = 1;
      ctx.stroke();

      // Hub Center Readout Text
      ctx.fillStyle = isLight ? 'rgb(79, 87, 102)' : 'rgb(148, 163, 184)';
      ctx.font = '10px ui-monospace, monospace';
      ctx.textAlign = 'center';
      ctx.fillText('SIEVE CLOCK', cx, cy - 14);

      ctx.fillStyle = isLight ? 'rgb(20, 24, 31)' : 'rgb(248, 250, 252)';
      ctx.font = 'bold 15px ui-monospace, monospace';
      ctx.fillText(`SLOT #${Math.round(s.displayHand) % TOTAL_SLOTS}`, cx, cy + 6);

      ctx.fillStyle = isLight ? 'rgb(11, 95, 209)' : 'rgb(6, 182, 212)';
      ctx.font = '10px ui-monospace, monospace';
      ctx.fillText(`${s.evictions} EVICTIONS`, cx, cy + 22);

      // Draw 64 Rotary Slots
      for (let i = 0; i < TOTAL_SLOTS; i++) {
        const angle = (i / TOTAL_SLOTS) * (Math.PI * 2) - Math.PI / 2;
        const x = cx + Math.cos(angle) * radius;
        const y = cy + Math.sin(angle) * radius;
        const slot = s.slots[i];

        ctx.beginPath();
        ctx.arc(x, y, 4, 0, Math.PI * 2);

        if (slot.visited === 1) {
          ctx.fillStyle = 'rgb(16, 185, 129)'; // Retained (Protected hot)
        } else {
          ctx.fillStyle = 'rgb(245, 158, 11)'; // Candidate (Cold/scan item)
        }
        ctx.fill();

        // Highlight current hand slot
        if (Math.round(s.displayHand) % TOTAL_SLOTS === i) {
          ctx.strokeStyle = 'rgb(6, 182, 212)';
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.arc(x, y, 7, 0, Math.PI * 2);
          ctx.stroke();
        }
      }

      // Draw Hand Pointer Needle
      const handAngle = (s.displayHand / TOTAL_SLOTS) * (Math.PI * 2) - Math.PI / 2;
      ctx.beginPath();
      ctx.moveTo(cx, cy);
      ctx.lineTo(cx + Math.cos(handAngle) * (radius - 12), cy + Math.sin(handAngle) * (radius - 12));
      ctx.strokeStyle = 'rgb(6, 182, 212)';
      ctx.lineWidth = 2.5;
      ctx.stroke();

      // Center pivot point
      ctx.beginPath();
      ctx.arc(cx, cy, 3.5, 0, Math.PI * 2);
      ctx.fillStyle = 'rgb(6, 182, 212)';
      ctx.fill();

      ctx.restore();

      animId = requestAnimationFrame(render);
    };

    animId = requestAnimationFrame(render);
    return () => cancelAnimationFrame(animId);
  }, []);

  // Respond to step advances or external sweep signals
  const advanceHand = useCallback((steps = 1) => {
    const s = stateRef.current;
    for (let step = 0; step < steps; step++) {
      const idx = s.hand % TOTAL_SLOTS;
      const slot = s.slots[idx];

      if (slot.visited === 1) {
        // Bit was 1: Give second chance, reset to 0
        slot.visited = 0;
        s.hand++;
      } else {
        // Bit was 0: Evict!
        slot.key = `new-key-${Date.now() % 1000}`;
        slot.visited = 0; // New items enter with visited = 0 in SIEVE
        s.evictions++;
        s.hand++;
        break; // Eviction found, hand stops
      }
    }

    s.targetHand = s.hand;
    setHandPos(s.hand % TOTAL_SLOTS);

    const retained = s.slots.filter((x) => x.visited === 1).length;
    const candidates = s.slots.filter((x) => x.visited === 0).length;
    setStats({ retained, candidates, evictions: s.evictions });
  }, []);

  useEffect(() => {
    if (externalSweep > 0 && externalSweep !== prevSweepRef.current) {
      const delta = externalSweep > prevSweepRef.current
        ? externalSweep - prevSweepRef.current
        : (externalSweep + TOTAL_SLOTS - prevSweepRef.current);
      prevSweepRef.current = externalSweep;
      if (delta > 0) {
        advanceHand(Math.min(delta, 8));
      }
    }
  }, [externalSweep, advanceHand]);

  if (compact) {
    return (
      <div className="sieve-canvas-container" style={{ display: 'flex', justifyContent: 'center' }}>
        <canvas ref={canvasRef} aria-label="Rotary Sieve Clock Drum showing 64 cache slots and eviction hand" />
      </div>
    );
  }

  return (
    <div className="sieve-drum-card">
      <div className="sieve-drum-card__header">
        <h3 className="sieve-drum-card__title">
          <span>⚙️ Rotary SIEVE Cache Buffer</span>
        </h3>
        <span className="sre-sub-header__badge">Lock-Free O(1) Eviction</span>
      </div>

      <div className="sieve-canvas-container">
        <canvas ref={canvasRef} aria-label="Rotary Sieve Clock Drum showing 64 cache slots and eviction hand" />
      </div>

      <div className="sieve-drum-legend">
        <div className="legend-item">
          <span className="legend-dot legend-dot--retained" />
          <span>Visited=1 ({stats.retained} Retained)</span>
        </div>
        <div className="legend-item">
          <span className="legend-dot legend-dot--candidate" />
          <span>Visited=0 ({stats.candidates} Candidates)</span>
        </div>
        <div className="legend-item">
          <span className="legend-dot legend-dot--hand" />
          <span>Clock Hand (#{handPos})</span>
        </div>
      </div>
    </div>
  );
}

import React, { useEffect, useRef } from 'react';
import Icon from '../Icon.jsx';

/**
 * Cubic Bézier interpolation between 4 control points.
 */
function getBezierPoint(p0, p1, p2, p3, t) {
  const cX = 3 * (p1.x - p0.x);
  const bX = 3 * (p2.x - p1.x) - cX;
  const aX = p3.x - p0.x - cX - bX;

  const cY = 3 * (p1.y - p0.y);
  const bY = 3 * (p2.y - p1.y) - cY;
  const aY = p3.y - p0.y - cY - bY;

  const x = aX * Math.pow(t, 3) + bX * Math.pow(t, 2) + cX * t + p0.x;
  const y = aY * Math.pow(t, 3) + bY * Math.pow(t, 2) + cY * t + p0.y;
  return { x, y };
}

/**
 * PhantomFlowDiagram: Interactive Visual Storytelling Architecture Diagram.
 * Shows the physical traffic flow from Devices -> Phantom 3-Node Edge Mesh -> Origin Database.
 * Runs an HTML5 Canvas Photon Particle Physics Engine with individual fiber optic threads
 * extending from each connected device to Gateway Nodes 1, 2, 3, and upstream to the Origin DB Vault.
 */
export default function PhantomFlowDiagram({
  rps = 0,
  activeAct = 1,
  isTesting = false,
  originInFlight = 0,
  hitRatio = '99.8%',
  coalescedRatio = '1:N',
  latencyMs = '0.42',
  masterRunning = false,
  devices = [],
}) {
  const isStampede = activeAct === 2 || rps >= 50000;
  const isMutexShield = activeAct === 3;

  const containerRef = useRef(null);
  const canvasRef = useRef(null);
  const zone1Ref = useRef(null);
  const zone2Ref = useRef(null);
  const node1Ref = useRef(null);
  const node2Ref = useRef(null);
  const node3Ref = useRef(null);
  const originDbRef = useRef(null);

  // High-performance 60fps Canvas Photon Engine with Per-Device Threads
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const ctx = canvas.getContext('2d');
    let animId;
    let frameCount = 0;

    const particles = [];
    const shockwaves = [];

    const resize = () => {
      const dpr = typeof window !== 'undefined' ? window.devicePixelRatio || 1 : 1;
      const w = container.clientWidth || 900;
      const h = container.clientHeight || 180;
      canvas.width = Math.round(w * dpr);
      canvas.height = Math.round(h * dpr);
      canvas.style.width = `${w}px`;
      canvas.style.height = `${h}px`;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };

    resize();
    window.addEventListener('resize', resize);

    const getCenter = (el) => {
      if (!el || !container) return null;
      const cRect = container.getBoundingClientRect();
      const r = el.getBoundingClientRect();
      return {
        x: r.left - cRect.left + r.width / 2,
        y: r.top - cRect.top + r.height / 2,
      };
    };

    const getEmitters = () => {
      if (!container) return [];
      const emitterEls = container.querySelectorAll('[data-device-emitter="true"]');
      const origins = [];
      const cRect = container.getBoundingClientRect();

      if (emitterEls && emitterEls.length > 0) {
        emitterEls.forEach((el) => {
          const r = el.getBoundingClientRect();
          origins.push({
            x: r.right - cRect.left,
            y: r.top - cRect.top + r.height / 2,
            active: el.getAttribute('data-emitter-active') === 'true',
            name: el.getAttribute('data-emitter-name') || 'Device',
          });
        });
      }

      if (origins.length === 0) {
        const z1 = zone1Ref.current;
        if (z1) {
          const r = z1.getBoundingClientRect();
          origins.push({
            x: r.right - cRect.left,
            y: r.top - cRect.top + r.height / 2,
            active: masterRunning || isTesting || rps > 0,
            name: 'Client Ingress',
          });
        }
      }

      return origins;
    };

    const spawnParticle = (forcedType, forcedEmitter) => {
      if (particles.length > 55) return;
      frameCount++;
      const pseudo = (Date.now() + frameCount * 29) % 100;
      let type = forcedType;
      if (!type) {
        if (pseudo < 84) type = 'hit';
        else if (pseudo < 96) type = 'peer';
        else type = 'miss';
      }

      const emitters = getEmitters();
      const emitterIdx = typeof forcedEmitter === 'number'
        ? forcedEmitter
        : (frameCount % Math.max(1, emitters.length));

      const targetNode = (frameCount % 3) + 1; // 1, 2, or 3
      particles.push({
        type,
        emitterIdx,
        targetNode,
        progress: 0.0,
        speed: type === 'hit' ? 0.032 : (type === 'peer' ? 0.024 : 0.022),
        phase: 0, // 0: client->node, 1: node->origin, 2: origin->node, 3: node->client, 4: node->node peer
        trail: [],
        alive: true,
      });
    };

    const render = () => {
      const w = container.clientWidth || 900;
      const h = container.clientHeight || 180;
      ctx.clearRect(0, 0, w, h);

      // Gateway Nodes and Origin positions
      const n1 = getCenter(node1Ref.current) || { x: w * 0.38, y: h * 0.28 };
      const n2 = getCenter(node2Ref.current) || { x: w * 0.50, y: h * 0.28 };
      const n3 = getCenter(node3Ref.current) || { x: w * 0.62, y: h * 0.28 };
      const nodes = [n1, n2, n3];

      const origin = getCenter(originDbRef.current) || { x: w * 0.87, y: h * 0.45 };
      const emitters = getEmitters();

      // 1. Draw Visible Threads (Fiber Optic Splines) from each individual device to Gateway Nodes
      emitters.forEach((emitter) => {
        nodes.forEach((node) => {
          ctx.beginPath();
          const cp1x = emitter.x + (node.x - emitter.x) * 0.45;
          ctx.moveTo(emitter.x, emitter.y);
          ctx.bezierCurveTo(cp1x, emitter.y, cp1x, node.y, node.x, node.y);

          if (emitter.active && (masterRunning || isTesting || rps > 0)) {
            ctx.strokeStyle = 'rgba(16, 185, 129, 0.45)';
            ctx.lineWidth = 1.6;
          } else {
            ctx.strokeStyle = 'rgba(16, 185, 129, 0.14)';
            ctx.lineWidth = 1.0;
          }
          ctx.stroke();
        });
      });

      // Gossip mesh line between nodes (Node 1 <-> Node 2 <-> Node 3)
      ctx.beginPath();
      ctx.moveTo(n1.x, n1.y);
      ctx.lineTo(n2.x, n2.y);
      ctx.lineTo(n3.x, n3.y);
      ctx.strokeStyle = 'rgba(6, 182, 212, 0.22)';
      ctx.setLineDash([3, 3]);
      ctx.lineWidth = 1.2;
      ctx.stroke();
      ctx.setLineDash([]);

      // Nodes -> Origin DB Fiber Splines
      nodes.forEach((node) => {
        ctx.beginPath();
        const cp1x = node.x + (origin.x - node.x) * 0.5;
        ctx.moveTo(node.x, node.y);
        ctx.bezierCurveTo(cp1x, node.y, cp1x, origin.y, origin.x, origin.y);
        ctx.strokeStyle = 'rgba(245, 158, 11, 0.14)';
        ctx.lineWidth = 1.2;
        ctx.stroke();
      });

      // 2. Spawn photons strictly based on live incoming traffic (True Standby Silence: 0 req/s = 0 photons)
      frameCount++;
      const isLiveTraffic = masterRunning || isTesting || rps > 0;
      if (isLiveTraffic) {
        const spawnCadence = rps > 1000 ? 2 : (rps > 200 ? 4 : 6);
        if (frameCount % spawnCadence === 0) spawnParticle();
      }

      // 3. Render Shockwaves
      for (let s = shockwaves.length - 1; s >= 0; s--) {
        const sw = shockwaves[s];
        sw.radius += 1.4;
        sw.alpha -= 0.038;
        if (sw.alpha <= 0 || sw.radius >= sw.maxRadius) {
          shockwaves.splice(s, 1);
          continue;
        }
        ctx.beginPath();
        ctx.arc(sw.x, sw.y, sw.radius, 0, Math.PI * 2);
        ctx.strokeStyle = sw.color;
        ctx.lineWidth = 1.8;
        ctx.globalAlpha = Math.max(0, sw.alpha);
        ctx.shadowColor = sw.color;
        ctx.shadowBlur = 8;
        ctx.stroke();
        ctx.shadowBlur = 0;
        ctx.globalAlpha = 1.0;
      }

      // 4. Update & Render Photon Particles along their Device Threads
      for (let i = particles.length - 1; i >= 0; i--) {
        const p = particles[i];
        p.progress += p.speed;

        const currentEmitter = emitters[p.emitterIdx % Math.max(1, emitters.length)] || { x: w * 0.16, y: h * 0.5 };
        const targetNodePos = nodes[(p.targetNode - 1) % 3] || n1;
        let pos = null;

        if (p.phase === 0) {
          // Device Thread -> Gateway Node
          const cp1x = currentEmitter.x + (targetNodePos.x - currentEmitter.x) * 0.45;
          pos = getBezierPoint(
            currentEmitter,
            { x: cp1x, y: currentEmitter.y },
            { x: cp1x, y: targetNodePos.y },
            targetNodePos,
            Math.min(1.0, p.progress)
          );

          if (p.progress >= 1.0) {
            // Arrived at Gateway Node!
            shockwaves.push({
              x: targetNodePos.x,
              y: targetNodePos.y,
              radius: 6,
              maxRadius: 28,
              alpha: 0.9,
              color: p.type === 'hit' ? 'rgb(16, 185, 129)' : (p.type === 'peer' ? 'rgb(6, 182, 212)' : 'rgb(245, 158, 11)'),
            });

            if (p.type === 'hit') {
              p.phase = 3;
              p.progress = 0.0;
              p.speed = 0.036;
            } else if (p.type === 'peer') {
              p.phase = 4;
              p.progress = 0.0;
              p.speed = 0.026;
            } else {
              p.phase = 1;
              p.progress = 0.0;
              p.speed = 0.024;
            }
          }
        } else if (p.phase === 1) {
          // Node -> Origin Database Vault
          const cp1x = targetNodePos.x + (origin.x - targetNodePos.x) * 0.5;
          pos = getBezierPoint(
            targetNodePos,
            { x: cp1x, y: targetNodePos.y },
            { x: cp1x, y: origin.y },
            origin,
            Math.min(1.0, p.progress)
          );

          if (p.progress >= 1.0) {
            shockwaves.push({
              x: origin.x,
              y: origin.y,
              radius: 8,
              maxRadius: 36,
              alpha: 0.84,
              color: 'rgb(245, 158, 11)',
            });
            p.phase = 2;
            p.progress = 0.0;
            p.speed = 0.030;
          }
        } else if (p.phase === 2) {
          // Origin -> Node return
          const cp1x = origin.x - (origin.x - targetNodePos.x) * 0.5;
          pos = getBezierPoint(
            origin,
            { x: cp1x, y: origin.y },
            { x: cp1x, y: targetNodePos.y },
            targetNodePos,
            Math.min(1.0, p.progress)
          );

          if (p.progress >= 1.0) {
            p.phase = 3;
            p.progress = 0.0;
            p.speed = 0.038;
          }
        } else if (p.phase === 3) {
          // Node -> Return along thread to originating Device
          const cp1x = targetNodePos.x - (targetNodePos.x - currentEmitter.x) * 0.45;
          pos = getBezierPoint(
            targetNodePos,
            { x: cp1x, y: targetNodePos.y },
            { x: cp1x, y: currentEmitter.y },
            currentEmitter,
            Math.min(1.0, p.progress)
          );

          if (p.progress >= 1.0) {
            p.alive = false;
          }
        } else if (p.phase === 4) {
          // Peer gossip between nodes
          const peerTarget = p.targetNode === 1 ? n2 : (p.targetNode === 2 ? n3 : n1);
          pos = {
            x: targetNodePos.x + (peerTarget.x - targetNodePos.x) * Math.min(1.0, p.progress),
            y: targetNodePos.y + (peerTarget.y - targetNodePos.y) * Math.min(1.0, p.progress),
          };
          if (p.progress >= 1.0) {
            shockwaves.push({
              x: peerTarget.x,
              y: peerTarget.y,
              radius: 6,
              maxRadius: 22,
              alpha: 0.84,
              color: 'rgb(6, 182, 212)',
            });
            p.alive = false;
          }
        }

        if (pos) {
          p.trail.unshift({ x: pos.x, y: pos.y });
          if (p.trail.length > 7) p.trail.pop();

          const color = p.type === 'hit' ? 'rgb(16, 185, 129)' : (p.type === 'peer' ? 'rgb(6, 182, 212)' : 'rgb(245, 158, 11)');

          // Draw Comet Tail
          for (let t = p.trail.length - 1; t >= 0; t--) {
            const pt = p.trail[t];
            const tailAlpha = (1 - t / p.trail.length) * 0.65;
            ctx.beginPath();
            ctx.arc(pt.x, pt.y, Math.max(1.5, 3.8 - t * 0.4), 0, Math.PI * 2);
            ctx.fillStyle = color;
            ctx.globalAlpha = tailAlpha;
            ctx.fill();
            ctx.globalAlpha = 1.0;
          }

          // Main Particle Head
          ctx.beginPath();
          ctx.fillStyle = 'rgb(255, 255, 255)';
          ctx.shadowColor = color;
          ctx.shadowBlur = 10;
          ctx.arc(pos.x, pos.y, 3.8, 0, Math.PI * 2);
          ctx.fill();
          ctx.shadowBlur = 0;
        }

        if (!p.alive) {
          particles.splice(i, 1);
        }
      }

      animId = requestAnimationFrame(render);
    };

    animId = requestAnimationFrame(render);

    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener('resize', resize);
    };
  }, [rps, masterRunning, isTesting, devices]);

  return (
    <div
      className={`phantom-flow-deck ${isTesting ? 'phantom-flow-deck--bursting' : ''}`}
      role="region"
      aria-label="Phantom Cache Architecture Flow Diagram"
    >
      <div className="flow-stage-container" ref={containerRef}>
        {/* Silky 60fps Canvas Photon Particle & Fiber Thread Overlay */}
        <canvas ref={canvasRef} className="flow-photon-canvas" aria-hidden="true" />

        {/* Zone 1: Inbound Traffic & Clients */}
        <div className="flow-zone flow-zone--clients" ref={zone1Ref}>
          <div className="flow-zone__header">
            <span className="flow-zone__tag">ZONE 01</span>
            <h4 className="flow-zone__title">Inbound Traffic &amp; Clients</h4>
          </div>

          <div className="flow-clients-box">
            {/* Live External Devices Fleet Emitter Badges with Individual Thread Anchors */}
            {devices && devices.length > 0 ? (
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '3px',
                  maxHeight: '52px',
                  overflowY: 'auto',
                  marginBottom: '4px',
                }}
              >
                {devices.map((d, i) => (
                  <div
                    key={d.id || i}
                    data-device-emitter="true"
                    data-emitter-active={d.running ? 'true' : 'false'}
                    data-emitter-name={d.type || 'Phone'}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'space-between',
                      background: 'var(--surface-raised)',
                      border: d.running ? '1px solid var(--ok)' : '1px solid var(--line)',
                      borderRadius: '4px',
                      padding: '2px 5px',
                      fontSize: '9px',
                      fontFamily: 'var(--font-mono)',
                    }}
                  >
                    <span style={{ display: 'flex', alignItems: 'center', gap: '4px', fontWeight: 600 }}>
                      <span
                        style={{
                          width: '5px',
                          height: '5px',
                          borderRadius: '50%',
                          background: d.running ? 'var(--ok)' : 'var(--warn)',
                          boxShadow: d.running ? '0 0 5px var(--ok)' : 'none',
                        }}
                      />
                      <span style={{ color: 'var(--text)' }}>{d.type || 'Phone'}</span>
                      <span style={{ color: 'var(--text-muted)', fontSize: '8px' }}>
                        ({d.network?.includes('Cellular') ? '5G' : 'Wi-Fi'})
                      </span>
                    </span>
                    <span style={{ color: 'var(--accent)', fontWeight: 700 }}>
                      {d.vusers || 500} v-clients
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <div
                data-device-emitter="true"
                data-emitter-active="false"
                data-emitter-name="Standby Device"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '5px',
                  background: 'var(--surface-raised)',
                  border: '1px dashed var(--line)',
                  borderRadius: '4px',
                  padding: '2px 6px',
                  fontSize: '9px',
                  color: 'var(--text-muted)',
                  fontFamily: 'var(--font-mono)',
                  marginBottom: '4px',
                }}
              >
                <span>📱</span>
                <span>Standby · Scan QR Below to Pair Phone</span>
              </div>
            )}

            <div className="flow-client-types">
              <span
                className="flow-client-pill"
                title="Mobile Shoppers"
                data-device-emitter="true"
                data-emitter-active={rps > 0 ? 'true' : 'false'}
                data-emitter-name="Mobile Shoppers"
              >
                <Icon name="buyer" />
                <span>Mobile (75%)</span>
              </span>
              <span
                className="flow-client-pill"
                title="Crawler Scans"
                data-device-emitter="true"
                data-emitter-active={rps > 0 ? 'true' : 'false'}
                data-emitter-name="Crawler Scans"
              >
                <Icon name="trace" />
                <span>Scans (15%)</span>
              </span>
              <span
                className="flow-client-pill flow-client-pill--surge"
                title="Flash Sale Crowd"
                data-device-emitter="true"
                data-emitter-active={rps > 0 ? 'true' : 'false'}
                data-emitter-name="Flash Crowd"
              >
                <Icon name="dyno" />
                <span>Crowd (10%)</span>
              </span>
            </div>

            <div className="flow-rps-counter">
              <span className="flow-rps-number">
                {rps >= 1000 ? `${(rps / 1000).toFixed(rps >= 10000 ? 0 : 1)}k` : rps}
              </span>
              <span className="flow-rps-label">req / sec</span>
            </div>

            <div className="flow-stream-status">
              <span className={`flow-pulse-indicator ${masterRunning || isTesting || rps > 0 ? 'flow-pulse-indicator--active' : ''}`} />
              <span>
                {isTesting
                  ? 'Slamming Cluster'
                  : masterRunning || rps > 0
                  ? 'Continuous Live Stream'
                  : 'Fleet Standby'}
              </span>
            </div>
          </div>
        </div>

        {/* Connector 1: Open Fiber Optic Ingress Conduit (Clean Floating Pill, No Static Grey Bar) */}
        <div className="flow-connector">
          <span className="flow-conduit-pill">
            <span className="flow-conduit-dot" />
            TCP / TLS Ingress
          </span>
        </div>

        {/* Zone 2: Phantom 3-Node Edge Mesh (The Core Magic) */}
        <div className="flow-zone flow-zone--edge" ref={zone2Ref}>
          <div className="flow-zone__header">
            <div className="flow-zone__badge-row">
              <span className="flow-zone__tag flow-zone__tag--edge">ZONE 02</span>
              <span className="flow-shield-badge">SHIELD ACTIVE</span>
            </div>
            <h4 className="flow-zone__title">Phantom 3-Node Edge RAM Mesh</h4>
          </div>

          {/* 3 Distributed Gateway Nodes */}
          <div className="flow-mesh-nodes">
            <div className="flow-node-card flow-node-card--primary" ref={node1Ref}>
              <div className="flow-node-card__top">
                <span className="flow-node-name">Node 1</span>
                <span className="flow-node-port">:18185</span>
              </div>
              <span className="flow-node-badge">Primary RAM</span>
              <span className="flow-node-lat">{latencyMs} ms HIT</span>
            </div>

            <div className="flow-node-card" ref={node2Ref}>
              <div className="flow-node-card__top">
                <span className="flow-node-name">Node 2</span>
                <span className="flow-node-port">:18186</span>
              </div>
              <span className="flow-node-badge">Peer Gossip</span>
              <span className="flow-node-lat">0.18 ms Synced</span>
            </div>

            <div className="flow-node-card" ref={node3Ref}>
              <div className="flow-node-card__top">
                <span className="flow-node-name">Node 3</span>
                <span className="flow-node-port">:18187</span>
              </div>
              <span className="flow-node-badge">Peer Gossip</span>
              <span className="flow-node-lat">0.18 ms Synced</span>
            </div>
          </div>

          {/* Core Hardware & Algorithmic Mechanics */}
          <div className="flow-mechanics-grid">
            <div className={`flow-mechanic-bay ${isMutexShield ? 'flow-mechanic-bay--highlight' : ''}`}>
              <div className="flow-mechanic-bay__title">
                <Icon name="lock" />
                <span>Singleflight Mutex Lock</span>
              </div>
              <p className="flow-mechanic-bay__desc">
                {isMutexShield
                  ? 'Coalescing 50,000 concurrent stampeding requests into 1 single upstream query.'
                  : 'Atomic deduplication: locks identical keys in flight; 1 query dispatched, N-1 wait and reuse.'}
              </p>
              <span className="flow-mechanic-tag">Coalesce: {coalescedRatio}</span>
            </div>

            <div className={`flow-mechanic-bay ${isStampede ? 'flow-mechanic-bay--highlight' : ''}`}>
              <div className="flow-mechanic-bay__title">
                <Icon name="clock" />
                <span>Lock-Free SIEVE RAM</span>
              </div>
              <p className="flow-mechanic-bay__desc">
                {isStampede
                  ? 'Absorbing 99.8% of read traffic directly from memory at sub-millisecond latency.'
                  : 'Rotary clock eviction with 1-bit visited flag; scan-resistant against crawler pollution.'}
              </p>
              <span className="flow-mechanic-tag">Absorption: {hitRatio}</span>
            </div>
          </div>
        </div>

        {/* Connector 2: Open Fiber Optic Origin Conduit (Clean Floating Pill, No Static Grey Bar) */}
        <div className="flow-connector flow-connector--filtered">
          <span className="flow-conduit-pill flow-conduit-pill--filtered">
            <span className="flow-conduit-dot flow-conduit-dot--filtered" />
            0.2% Filtered
          </span>
        </div>

        {/* Zone 3: Protected Origin Database */}
        <div className="flow-zone flow-zone--origin">
          <div className="flow-zone__header">
            <span className="flow-zone__tag">ZONE 03</span>
            <h4 className="flow-zone__title">Origin Database</h4>
          </div>

          <div className="flow-origin-box">
            <div className="flow-db-cylinder" ref={originDbRef}>
              <Icon name="database" />
              <span>PostgreSQL / Aurora</span>
              <span className="flow-db-port">:19099 Vault</span>
            </div>

            <div className="flow-db-status-bay">
              <span className="flow-db-stat-lbl">Active Connection Pool:</span>
              <span className="flow-db-stat-val flow-db-stat-val--safe">
                {originInFlight} conn ({originInFlight > 0 ? '1.2%' : '0.0%'} Saturation)
              </span>
            </div>

            <div className="flow-db-contrast-box">
              <div className="flow-contrast-item flow-contrast-item--good">
                <span className="flow-contrast-tag">WITH PHANTOM</span>
                <span className="flow-contrast-text">Calm &amp; Protected ({originInFlight > 0 ? '1.2%' : '0.0%'} Pool)</span>
              </div>
              <div className="flow-contrast-item flow-contrast-item--bad">
                <span className="flow-contrast-tag">WITHOUT PHANTOM</span>
                <span className="flow-contrast-text">
                  {isStampede ? '100% DEADLOCKED (504 Timeout)' : 'Strained under traffic'}
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Storytelling Narrative Summary Strip */}
      <div className="flow-story-bar">
        <span className="flow-story-step">
          <strong>Step 1:</strong> {rps > 0 ? `${rps.toLocaleString()} req/s enters Node 1` : 'Standby for External Swarm'}
        </span>
        <span className="flow-story-divider">→</span>
        <span className="flow-story-step">
          <strong>Step 2:</strong> SIEVE &amp; Singleflight absorb <strong>{hitRatio}</strong> in RAM
        </span>
        <span className="flow-story-divider">→</span>
        <span className="flow-story-step">
          <strong>Step 3:</strong> Origin handles only <strong>{originInFlight} query</strong> (Zero crash)
        </span>
      </div>
    </div>
  );
}

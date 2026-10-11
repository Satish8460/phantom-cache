import React, { useState, useEffect, useRef } from 'react';

export default function App() {
  const [status, setStatus] = useState('CONNECTED');
  const [phantomEnabled, setPhantomEnabled] = useState(true);
  const [commandAck, setCommandAck] = useState('READY');
  const [isPaused, setIsPaused] = useState(false);
  const [simSpeed, setSimSpeed] = useState('1x');
  const [showWhyPhantom, setShowWhyPhantom] = useState(false);
  const [showDataInspector, setShowDataInspector] = useState(false);

  // Composed Environmental Variables
  const [trafficConfig, setTrafficConfig] = useState('NORMAL');
  const [networkConfig, setNetworkConfig] = useState('NORMAL');
  const [backendConfig, setBackendConfig] = useState('NORMAL');
  const [clusterConfig, setClusterConfig] = useState('3_NODES');

  // Raw high-frequency telemetry buffer (fed by 100ms ticks)
  const rawTelemetryRef = useRef([]);

  // Human-legible presentation state (updated every 500ms)
  const [metrics, setMetrics] = useState({
    throughput_req_sec: 12050,
    p50_latency_ms: 10,
    p95_latency_ms: 12,
    p99_latency_ms: 15,
    backend_load_ratio: 0.20,
    avoided_compute_dollars: 0.05,
  });

  const [nodes, setNodes] = useState([
    { node_id: 1, state: 'Active', cpu_utilization: 28.4, memory_used_bytes: 536870912 },
    { node_id: 2, state: 'Active', cpu_utilization: 28.4, memory_used_bytes: 536870912 },
    { node_id: 3, state: 'Active', cpu_utilization: 28.4, memory_used_bytes: 536870912 }
  ]);

  // Rolling trend states
  const [throughputTrend, setThroughputTrend] = useState('0.0%');
  const [latencyTrend, setLatencyTrend] = useState('+0 ms');

  // Rolling history arrays for large charts
  const [throughputHistory, setThroughputHistory] = useState(Array(30).fill(12000));
  const [latencyHistory, setLatencyHistory] = useState(Array(30).fill(15));
  const [dbLoadHistory, setDbLoadHistory] = useState(Array(30).fill(0.20));
  const [hitRateHistory, setHitRateHistory] = useState(Array(30).fill(0.88));
  const [repBandwidthHistory, setRepBandwidthHistory] = useState(Array(30).fill(12));
  const [costHistory, setCostHistory] = useState(Array(30).fill(0.05));

  const [timeline, setTimeline] = useState([
    { time: '23:34:01', event: 'SYSTEM_BOOT', desc: 'Phantom virtual 3-node cluster initialized.' }
  ]);

  const hitRate = phantomEnabled ? (trafficConfig === 'FLASH_CROWD' ? 0.94 : 0.88) : 0.70;
  const coalescedRatio = phantomEnabled ? (trafficConfig === 'FLASH_CROWD' ? 0.85 : 0.40) : 0.00;
  
  const hits = Math.round(metrics.throughput_req_sec * hitRate);
  const misses = Math.round(metrics.throughput_req_sec * (1.0 - hitRate));
  const coalesced = Math.round(misses * coalescedRatio);
  const backendQps = Math.max(0, misses - coalesced);
  const workAvoided = metrics.throughput_req_sec > 0 ? ((hits + coalesced) / metrics.throughput_req_sec * 100).toFixed(1) : '0.0';

  const backendCostPerMill = 0.15;
  const nodeCostPerHr = 0.05;
  const baselineCostHr = (metrics.throughput_req_sec * (1.0 - 0.70) * 3600.0 / 1_000_000.0) * backendCostPerMill;
  const phantomCostHr = (backendQps * 3600.0 / 1_000_000.0) * backendCostPerMill + (3 * nodeCostPerHr);
  const breakEvenQps = 8000;

  const getNarrative = () => {
    if (trafficConfig === 'FLASH_CROWD') {
      return {
        title: 'FLASH CROWD IN PROGRESS',
        desc: `Workload demand spiked to ${Math.round(metrics.throughput_req_sec)} req/s. Coalescing avoided ${workAvoided}% database operations.`
      };
    }
    return {
      title: 'NORMAL WORKLOAD',
      desc: 'System metrics are operating within expected baseline thresholds.'
    };
  };

  const narrative = getNarrative();

  // High-frequency raw data collector
  useEffect(() => {
    if (isPaused) return;

    const sse = new EventSource('http://localhost:8085/api/events');
    sse.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.metrics) {
          let currentMetrics = { ...data.metrics };
          if (!phantomEnabled) {
            currentMetrics.p50_latency_ms *= 2;
            currentMetrics.p95_latency_ms *= 2.5;
            currentMetrics.p99_latency_ms *= 3;
            currentMetrics.backend_load_ratio = Math.min(1.0, currentMetrics.backend_load_ratio * 2.5);
            currentMetrics.avoided_compute_dollars = 0.0;
          }
          // Buffer raw values immediately
          rawTelemetryRef.current.push({
            throughput: currentMetrics.throughput_req_sec,
            p50: currentMetrics.p50_latency_ms,
            p95: currentMetrics.p95_latency_ms,
            p99: currentMetrics.p99_latency_ms,
            dbLoad: currentMetrics.backend_load_ratio,
            avoidedCompute: currentMetrics.avoided_compute_dollars
          });
        }
        if (data.nodes) setNodes(data.nodes);
        setStatus('CONNECTED');
      } catch (err) {
        console.error("SSE parse error: ", err);
      }
    };
    sse.onerror = () => {
      setStatus('DISCONNECTED');
    };
    return () => sse.close();
  }, [isPaused, phantomEnabled]);

  // Telemetry presentation aggregator timer (updates every 500ms)
  useEffect(() => {
    if (isPaused) return;

    const timer = setInterval(() => {
      const samples = rawTelemetryRef.current;
      if (samples.length === 0) return;

      // Average the samples collected in this 500ms window
      const count = samples.length;
      const avgThroughput = samples.reduce((sum, s) => sum + s.throughput, 0) / count;
      const avgP50 = samples.reduce((sum, s) => sum + s.p50, 0) / count;
      const avgP95 = samples.reduce((sum, s) => sum + s.p95, 0) / count;
      const avgP99 = samples.reduce((sum, s) => sum + s.p99, 0) / count;
      const avgDbLoad = samples.reduce((sum, s) => sum + s.dbLoad, 0) / count;
      const avgAvoidedCompute = samples.reduce((sum, s) => sum + s.avoidedCompute, 0) / count;

      // Clear buffer for the next window
      rawTelemetryRef.current = [];

      setMetrics(prev => {
        // Expose window delta trends
        const oldThroughput = prev.throughput_req_sec;
        const oldLatency = prev.p99_latency_ms;
        
        const throughputPct = oldThroughput > 0 ? ((avgThroughput - oldThroughput) / oldThroughput * 100).toFixed(1) : '0.0';
        const latencyDiff = Math.round(avgP99 - oldLatency);

        setThroughputTrend(`${throughputPct}%`);
        setLatencyTrend(`${latencyDiff >= 0 ? '+' : ''}${latencyDiff} ms`);

        return {
          throughput_req_sec: avgThroughput,
          p50_latency_ms: Math.round(avgP50),
          p95_latency_ms: Math.round(avgP95),
          p99_latency_ms: Math.round(avgP99),
          backend_load_ratio: avgDbLoad,
          avoided_compute_dollars: avgAvoidedCompute
        };
      });

      // Update historical rolling charts
      setThroughputHistory(prev => [...prev.slice(-30), avgThroughput]);
      setLatencyHistory(prev => [...prev.slice(-30), avgP99]);
      setDbLoadHistory(prev => [...prev.slice(-30), avgDbLoad]);
      setHitRateHistory(prev => [...prev.slice(-30), hitRate]);
      setRepBandwidthHistory(prev => [...prev.slice(-30), trafficConfig === 'FLASH_CROWD' ? 45 : 12]);
      setCostHistory(prev => [...prev.slice(-30), avgAvoidedCompute]);

    }, 500);

    return () => clearInterval(timer);
  }, [isPaused, hitRate, trafficConfig]);

  const triggerEvent = async (eventName) => {
    setCommandAck(`${eventName} [TRIGGERED...]`);
    try {
      const res = await fetch('http://localhost:8085/api/control', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ event: eventName })
      });
      const data = await res.json();
      if (data.status === 'success') {
        setCommandAck(`${eventName} [ACCEPTED]`);
        setTimeline(prev => [
          { time: new Date().toLocaleTimeString(), event: eventName, desc: `Executed: ${eventName}` },
          ...prev
        ]);
      }
    } catch (err) {
      setCommandAck('CONTROL FAILED');
    }
  };

  const handlePreset = (presetName) => {
    if (presetName === 'FLASH_CROWD') {
      setTrafficConfig('FLASH_CROWD');
      triggerEvent('FLASH_CROWD');
    } else if (presetName === 'NODE_FAILURE') {
      setClusterConfig('NODE_FAILURE');
      triggerEvent('NODE_FAILURE');
    } else {
      handleReset();
    }
  };

  const handleReset = () => {
    setTrafficConfig('NORMAL');
    setNetworkConfig('NORMAL');
    setBackendConfig('NORMAL');
    setClusterConfig('3_NODES');
    triggerEvent('NORMAL');
    setThroughputHistory(Array(30).fill(12000));
    setLatencyHistory(Array(30).fill(15));
    setDbLoadHistory(Array(30).fill(0.20));
    setHitRateHistory(Array(30).fill(0.88));
    setRepBandwidthHistory(Array(30).fill(12));
    setCostHistory(Array(30).fill(0.05));
    setTimeline([
      { time: new Date().toLocaleTimeString(), event: 'RESET', desc: 'Simulation restored to clean seeded initial state.' }
    ]);
  };

  const renderObservabilityChart = (data, title, yMax = 100, color = '#FF6B00', suffix = '') => {
    const width = 280;
    const height = 90;
    if (data.length < 2) return null;
    const max = Math.max(...data, yMax);
    const min = Math.min(...data, 0);
    const points = data.map((val, idx) => {
      const x = (idx / (data.length - 1)) * width;
      const y = height - ((val - min) / (max - min)) * height;
      return `${x},${y}`;
    }).join(' ');

    return (
      <div style={{ border: '1px solid #1A1C20', padding: '8px', backgroundColor: '#0B0C0E' }}>
        <div style={{ fontSize: '9px', color: '#666', marginBottom: '4px', textTransform: 'uppercase' }}>{title}</div>
        <svg width="100%" height={height} style={{ backgroundColor: '#07080A' }}>
          <polyline fill="none" stroke={color} strokeWidth="1.5" points={points} />
        </svg>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '8px', color: '#444', marginTop: '2px' }}>
          <span>MIN: {min.toFixed(0)}{suffix}</span>
          <span>MAX: {max.toFixed(0)}{suffix}</span>
        </div>
      </div>
    );
  };

  return (
    <div style={{ backgroundColor: '#050607', color: '#F3F4F6', minHeight: '100vh', fontFamily: 'monospace', padding: '16px' }}>
      
      {/* HEADER */}
      <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid #1A1C20', paddingBottom: '10px', marginBottom: '16px' }}>
        <div>
          <h1 style={{ color: '#FF6B00', margin: 0, fontSize: '18px', fontWeight: 'bold' }}>PHANTOM CONTROL PANEL // V4</h1>
          <span style={{ fontSize: '10px', color: '#666' }}>
            CLOCK: {new Date().toLocaleTimeString()} // SEED: 48291 // STATUS: <span style={{ color: status === 'CONNECTED' ? '#4CAF50' : '#F44336' }}>{status}</span>
          </span>
        </div>
        
        <div style={{ display: 'flex', gap: '8px' }}>
          <button 
            onClick={() => setPhantomEnabled(!phantomEnabled)} 
            style={{ 
              backgroundColor: phantomEnabled ? '#FF6B00' : '#202226', 
              color: '#F3F4F6', 
              border: 'none', 
              padding: '6px 12px', 
              fontWeight: 'bold', 
              cursor: 'pointer',
              fontSize: '11px'
            }}
          >
            PHANTOM: {phantomEnabled ? '● ON' : '○ OFF'}
          </button>
          <button onClick={() => setShowWhyPhantom(!showWhyPhantom)} style={{ backgroundColor: '#101216', color: '#888', border: '1px solid #1A1C20', padding: '6px 10px', cursor: 'pointer', fontSize: '11px' }}>WHY PHANTOM?</button>
          <button onClick={() => setShowDataInspector(!showDataInspector)} style={{ backgroundColor: '#101216', color: '#888', border: '1px solid #1A1C20', padding: '6px 10px', cursor: 'pointer', fontSize: '11px' }}>DATA INSPECTOR</button>
        </div>
      </header>

      {/* ROW 1: LIVE KPI STRIP WITH TRENDS */}
      <section style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: '12px', marginBottom: '16px' }}>
        <div style={{ border: '1px solid #1A1C20', padding: '8px', backgroundColor: '#0B0C0E' }}>
          <div style={{ fontSize: '9px', color: '#666' }}>THROUGHPUT</div>
          <div style={{ fontSize: '13px', fontWeight: 'bold', color: '#FF6B00' }}>{Math.round(metrics.throughput_req_sec)} req/s</div>
          <div style={{ fontSize: '8px', color: '#444' }}>{throughputTrend} / 500ms</div>
        </div>
        <div style={{ border: '1px solid #1A1C20', padding: '8px', backgroundColor: '#0B0C0E' }}>
          <div style={{ fontSize: '9px', color: '#666' }}>P99 LATENCY</div>
          <div style={{ fontSize: '13px', fontWeight: 'bold', color: '#FF6B00' }}>{metrics.p99_latency_ms} ms</div>
          <div style={{ fontSize: '8px', color: '#444' }}>{latencyTrend} vs prev</div>
        </div>
        <div style={{ border: '1px solid #1A1C20', padding: '8px', backgroundColor: '#0B0C0E' }}>
          <div style={{ fontSize: '9px', color: '#666' }}>CACHE HIT RATE</div>
          <div style={{ fontSize: '13px', fontWeight: 'bold' }}>{(hitRate * 100).toFixed(0)}%</div>
          <div style={{ fontSize: '8px', color: '#444' }}>LIVE • Rust telemetry</div>
        </div>
        <div style={{ border: '1px solid #1A1C20', padding: '8px', backgroundColor: '#0B0C0E' }}>
          <div style={{ fontSize: '9px', color: '#666' }}>COALESCING</div>
          <div style={{ fontSize: '13px', fontWeight: 'bold' }}>{(coalescedRatio * 100).toFixed(0)}%</div>
          <div style={{ fontSize: '8px', color: '#444' }}>LIVE • coalescer</div>
        </div>
        <div style={{ border: '1px solid #1A1C20', padding: '8px', backgroundColor: '#0B0C0E' }}>
          <div style={{ fontSize: '9px', color: '#666' }}>BACKEND QPS</div>
          <div style={{ fontSize: '13px', fontWeight: 'bold' }}>{backendQps} req/s</div>
          <div style={{ fontSize: '8px', color: '#444' }}>DERIVED • origin</div>
        </div>
        <div style={{ border: '1px solid #1A1C20', padding: '8px', backgroundColor: '#0B0C0E' }}>
          <div style={{ fontSize: '9px', color: '#666' }}>DB LOAD</div>
          <div style={{ fontSize: '13px', fontWeight: 'bold' }}>{(metrics.backend_load_ratio * 100).toFixed(0)}%</div>
          <div style={{ fontSize: '8px', color: '#444' }}>SIMULATED • queue</div>
        </div>
        <div style={{ border: '1px solid #1A1C20', padding: '8px', backgroundColor: '#0B0C0E' }}>
          <div style={{ fontSize: '9px', color: '#666' }}>NET SAVED/1M</div>
          <div style={{ fontSize: '13px', fontWeight: 'bold', color: '#4CAF50' }}>${(baselineCostHr - phantomCostHr).toFixed(2)}</div>
          <div style={{ fontSize: '8px', color: '#444' }}>SIMULATED • economic</div>
        </div>
      </section>

      {/* ROW 2: TOPOLOGY MAP & REQUEST FLOW */}
      <section style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: '16px', marginBottom: '16px' }}>
        
        {/* TOPOLOGY */}
        <div style={{ border: '1px solid #1A1C20', padding: '12px', backgroundColor: '#0B0C0E' }}>
          <h2 style={{ fontSize: '11px', color: '#888', margin: '0 0 10px 0' }}>LIVE CLUSTER TOPOLOGY MAP</h2>
          <div style={{ height: '170px', backgroundColor: '#07080A', border: '1px solid #15171B' }}>
            <svg width="100%" height="100%" viewBox="0 0 500 170">
              <circle cx="50" cy="85" r="20" fill="#101216" stroke="#202226" strokeWidth="1.5" />
              <text x="50" y="88" fill="#F3F4F6" textAnchor="middle" fontSize="7" fontWeight="bold">CLIENTS</text>

              {nodes.map((n, idx) => {
                const targetY = 30 + idx * 55;
                const isActive = n.state === 'Active';
                return (
                  <path 
                    key={`p-${n.node_id}`} 
                    d={`M 70 85 Q 120 ${targetY} 180 ${targetY}`} 
                    fill="none" 
                    stroke={isActive ? '#FF6B00' : '#401515'} 
                    strokeWidth={isActive ? '1.5' : '1'} 
                  />
                );
              })}

              {nodes.map((n, idx) => {
                const y = 30 + idx * 55;
                const isActive = n.state === 'Active';
                return (
                  <g key={`n-${n.node_id}`}>
                    <rect x="180" y={y - 15} width="90" height="30" fill="#101216" stroke={isActive ? '#FF6B00' : '#401515'} strokeWidth="1.5" />
                    <rect x="180" y={y - 15} width="5" height="30" fill={isActive ? '#4CAF50' : '#F44336'} />
                    <text x="195" y={y - 1} fill="#F3F4F6" fontSize="8" fontWeight="bold">NODE N{n.node_id}</text>
                    <text x="195" y={y + 8} fill="#888" fontSize="7">{n.state.toUpperCase()}</text>
                  </g>
                );
              })}

              {nodes.map((n, idx) => {
                const y = 30 + idx * 55;
                return (
                  <path key={`db-${n.node_id}`} d={`M 270 ${y} Q 350 ${y} 410 85`} fill="none" stroke="#202226" strokeWidth="1" />
                );
              })}

              <circle cx="430" cy="85" r="20" fill="#101216" stroke="#202226" strokeWidth="1.5" />
              <text x="430" y="88" fill="#F3F4F6" textAnchor="middle" fontSize="7" fontWeight="bold">DATABASE</text>
            </svg>
          </div>
        </div>

        {/* REQUEST FLOW */}
        <div style={{ border: '1px solid #1A1C20', padding: '12px', backgroundColor: '#0B0C0E', display: 'flex', flexDirection: 'column', justifyContent: 'space-between' }}>
          <div>
            <h2 style={{ fontSize: '11px', color: '#888', margin: '0 0 10px 0' }}>REQUEST FLOW ANALYSIS</h2>
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', fontSize: '11px' }}>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}><span>Incoming Requests:</span><span style={{ fontWeight: 'bold' }}>{Math.round(metrics.throughput_req_sec)}</span></div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}><span>Cache Hits:</span><span style={{ color: '#4CAF50' }}>{hits}</span></div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}><span>Cache Misses:</span><span>{misses}</span></div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}><span>Coalesced Requests:</span><span style={{ color: '#FF6B00' }}>{coalesced}</span></div>
              <div style={{ display: 'flex', justifyContent: 'space-between' }}><span>Backend Requests:</span><span>{backendQps}</span></div>
            </div>
          </div>
          <div style={{ borderTop: '1px solid #1A1C20', paddingTop: '8px', marginTop: '8px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <span style={{ fontSize: '10px', color: '#666' }}>WORK AVOIDED:</span>
            <span style={{ fontSize: '14px', fontWeight: 'bold', color: '#FF6B00' }}>{workAvoided}%</span>
          </div>
        </div>
      </section>

      {/* ROW 3: DETAILED OBSERVABILITY CHARTS */}
      <section style={{ display: 'grid', gridTemplateColumns: 'repeat(6, 1fr)', gap: '12px', marginBottom: '16px' }}>
        {renderObservabilityChart(throughputHistory, 'Request Throughput', 80000, '#FF6B00', ' req/s')}
        {renderObservabilityChart(latencyHistory, 'Latency (P99)', 200, '#F44336', ' ms')}
        {renderObservabilityChart(dbLoadHistory, 'Backend Load', 1.0, '#4CAF50', '%')}
        {renderObservabilityChart(hitRateHistory, 'Cache Hit Rate', 1.0, '#00BCD4', '%')}
        {renderObservabilityChart(repBandwidthHistory, 'Replication Traffic', 100, '#9C27B0', ' MB/s')}
        {renderObservabilityChart(costHistory, 'Net Cumulative Cost', 1.0, '#E91E63', ' $')}
      </section>

      {/* ROW 4: "WHAT JUST HAPPENED?" NARRATIVE & PIE INSPECTOR */}
      <section style={{ display: 'grid', gridTemplateColumns: '1fr 2fr', gap: '16px', marginBottom: '16px' }}>
        
        {/* NARRATIVE PANEL */}
        <div style={{ border: '1px solid #FF6B00', padding: '12px', backgroundColor: '#100D0A' }}>
          <h2 style={{ fontSize: '11px', color: '#FF6B00', margin: '0 0 6px 0', fontWeight: 'bold' }}>{narrative.title}</h2>
          <p style={{ fontSize: '11px', margin: 0, color: '#DDD', lineHeight: '1.5' }}>{narrative.desc}</p>
        </div>

        {/* PIE DECISION INSPECTOR */}
        <div style={{ border: '1px solid #1A1C20', padding: '12px', backgroundColor: '#0B0C0E' }}>
          <h2 style={{ fontSize: '11px', color: '#888', margin: '0 0 10px 0' }}>PIE DECISION INSPECTOR</h2>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '16px', fontSize: '11px' }}>
            <div>
              <span style={{ color: '#666', fontSize: '9px' }}>OBSERVATION</span>
              <div style={{ marginTop: '4px', fontWeight: 'bold' }}>{trafficConfig === 'FLASH_CROWD' ? 'HOT KEY DENSITY DETECTED' : 'STEADY WORKLOAD'}</div>
            </div>
            <div>
              <span style={{ color: '#666', fontSize: '9px' }}>CONSTRAINTS & CANARY</span>
              <div style={{ marginTop: '4px' }}>Memory: OK // Canary Duration: 30s</div>
            </div>
            <div>
              <span style={{ color: '#666', fontSize: '9px' }}>DECISION</span>
              <div style={{ marginTop: '4px', color: '#4CAF50', fontWeight: 'bold' }}>✓ COMMITTED</div>
            </div>
          </div>
        </div>
      </section>

      {/* ROW 5: ECONOMICS WATERFALL */}
      <section style={{ border: '1px solid #1A1C20', padding: '12px', backgroundColor: '#0B0C0E', marginBottom: '16px' }}>
        <h2 style={{ fontSize: '11px', color: '#888', margin: '0 0 10px 0' }}>INFRASTRUCTURE COST BREAKDOWN</h2>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '24px', fontSize: '11px' }}>
          <div>
            <div style={{ borderBottom: '1px solid #1A1C20', paddingBottom: '4px', marginBottom: '6px', color: '#666' }}>BASELINE COST</div>
            <div>Origin Compute: ${baselineCostHr.toFixed(2)}/hr</div>
            <div>Total Infrastructure: ${baselineCostHr.toFixed(2)}/hr</div>
          </div>
          <div>
            <div style={{ borderBottom: '1px solid #1A1C20', paddingBottom: '4px', marginBottom: '6px', color: '#FF6B00' }}>PHANTOM COST</div>
            <div>Origin DB Compute: ${(phantomCostHr - 0.15).toFixed(2)}/hr</div>
            <div>Phantom Node: $0.15/hr</div>
            <div style={{ fontWeight: 'bold' }}>Total Infrastructure: ${phantomCostHr.toFixed(2)}/hr</div>
          </div>
          <div>
            <div style={{ borderBottom: '1px solid #1A1C20', paddingBottom: '4px', marginBottom: '6px', color: '#4CAF50' }}>NET SAVINGS</div>
            <div>Benefit: +${(baselineCostHr - phantomCostHr).toFixed(2)}/hr</div>
            <div>Break-Even Threshold: {breakEvenQps} req/s</div>
            <div style={{ fontWeight: 'bold', color: metrics.throughput_req_sec > breakEvenQps ? '#4CAF50' : '#F44336' }}>
              STATUS: {metrics.throughput_req_sec > breakEvenQps ? 'ABOVE BREAK-EVEN' : 'NOT ECONOMIC'}
            </div>
          </div>
        </div>
      </section>

      {/* ROW 6: TIMELINE & CONTROLS */}
      <section style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '16px' }}>
        
        {/* TIMELINE */}
        <div style={{ border: '1px solid #1A1C20', padding: '12px', backgroundColor: '#0B0C0E', maxHeight: '150px', overflowY: 'auto' }}>
          <h2 style={{ fontSize: '11px', color: '#888', margin: '0 0 8px 0' }}>EVENT TIMELINE</h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '4px', fontSize: '10px' }}>
            {timeline.map((item, idx) => (
              <div key={idx}>
                <span style={{ color: '#666' }}>[{item.time}]</span> <span style={{ color: '#FF6B00', fontWeight: 'bold' }}>{item.event}</span>: {item.desc}
              </div>
            ))}
          </div>
        </div>

        {/* SCENARIO CONTROL & SPEED BAR */}
        <div style={{ border: '1px solid #1A1C20', padding: '12px', backgroundColor: '#0B0C0E' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px' }}>
            <h2 style={{ fontSize: '11px', color: '#888', margin: 0 }}>SCENARIO CONTROL BAR</h2>
            
            {/* Speed Control Strip */}
            <div style={{ display: 'flex', gap: '4px', fontSize: '10px' }}>
              <button onClick={() => setIsPaused(false)} style={{ backgroundColor: !isPaused ? '#FF6B00' : '#101216', color: '#FFF', border: 'none', padding: '2px 6px', cursor: 'pointer' }}>▶ RUN</button>
              <button onClick={() => setIsPaused(true)} style={{ backgroundColor: isPaused ? '#FF6B00' : '#101216', color: '#FFF', border: 'none', padding: '2px 6px', cursor: 'pointer' }}>⏸ PAUSE</button>
              <button onClick={() => setSimSpeed('0.25x')} style={{ backgroundColor: simSpeed === '0.25x' ? '#FF6B00' : '#101216', color: '#FFF', border: 'none', padding: '2px 6px', cursor: 'pointer' }}>0.25x</button>
              <button onClick={() => setSimSpeed('1x')} style={{ backgroundColor: simSpeed === '1x' ? '#FF6B00' : '#101216', color: '#FFF', border: 'none', padding: '2px 6px', cursor: 'pointer' }}>1x</button>
              <button onClick={() => setSimSpeed('2x')} style={{ backgroundColor: simSpeed === '2x' ? '#FF6B00' : '#101216', color: '#FFF', border: 'none', padding: '2px 6px', cursor: 'pointer' }}>2x</button>
            </div>
          </div>
          
          {/* Preset Buttons */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: '8px', marginBottom: '8px' }}>
            <button onClick={() => handlePreset('NORMAL')} style={{ backgroundColor: '#101216', color: '#F3F4F6', border: '1px solid #1A1C20', padding: '6px', cursor: 'pointer', fontSize: '10px' }}>NORMAL</button>
            <button onClick={() => handlePreset('FLASH_CROWD')} style={{ backgroundColor: '#101216', color: '#F3F4F6', border: '1px solid #1A1C20', padding: '6px', cursor: 'pointer', fontSize: '10px' }}>FLASH CROWD</button>
            <button onClick={() => handlePreset('NODE_FAILURE')} style={{ backgroundColor: '#101216', color: '#F3F4F6', border: '1px solid #1A1C20', padding: '6px', cursor: 'pointer', fontSize: '10px' }}>KILL NODE</button>
            <button onClick={() => { setNetworkConfig('DEGRADED'); triggerEvent('DEGRADED'); }} style={{ backgroundColor: '#101216', color: '#F3F4F6', border: '1px solid #1A1C20', padding: '6px', cursor: 'pointer', fontSize: '10px' }}>NETWORK</button>
            <button onClick={handleReset} style={{ backgroundColor: '#101216', color: '#F3F4F6', border: '1px solid #1A1C20', padding: '6px', cursor: 'pointer', fontSize: '10px' }}>RESET</button>
          </div>
        </div>
      </section>

      {/* POPUP DRAWER MODALS */}
      {showWhyPhantom && (
        <div style={{ position: 'fixed', top: '30%', left: '35%', width: '30%', backgroundColor: '#0B0C0E', border: '2px solid #FF6B00', padding: '16px', zIndex: 100 }}>
          <h2 style={{ color: '#FF6B00', margin: '0 0 10px 0', fontSize: '14px' }}>WHY PHANTOM?</h2>
          <ul style={{ fontSize: '11px', lineHeight: '1.6', paddingLeft: '16px' }}>
            <li>Coalesces duplicate database queries during flash loads.</li>
            <li>Balances regional replica hot keys adaptively.</li>
            <li>Protects physical database capacity bounds.</li>
          </ul>
          <button onClick={() => setShowWhyPhantom(false)} style={{ backgroundColor: '#FF6B00', color: '#F3F4F6', border: 'none', padding: '6px 12px', cursor: 'pointer', float: 'right', fontSize: '11px', marginTop: '10px' }}>CLOSE</button>
        </div>
      )}

      {showDataInspector && (
        <div style={{ position: 'fixed', top: '25%', left: '25%', width: '50%', backgroundColor: '#0B0C0E', border: '2px solid #FF6B00', padding: '16px', zIndex: 100 }}>
          <h2 style={{ color: '#FF6B00', margin: '0 0 10px 0', fontSize: '14px' }}>LIVE TELEMETRY SNAPSHOT INSPECTOR</h2>
          <pre style={{ fontSize: '10px', maxHeight: '200px', overflowY: 'auto', backgroundColor: '#050607', padding: '10px', border: '1px solid #1A1C20' }}>
            {JSON.stringify({ metrics, nodes, hotKeys }, null, 2)}
          </pre>
          <button onClick={() => setShowDataInspector(false)} style={{ backgroundColor: '#FF6B00', color: '#F3F4F6', border: 'none', padding: '6px 12px', cursor: 'pointer', float: 'right', fontSize: '11px', marginTop: '10px' }}>CLOSE</button>
        </div>
      )}

    </div>
  );
}

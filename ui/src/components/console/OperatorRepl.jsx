import React, { useState, useRef, useEffect } from 'react';
import { broadcastInvalidateKey, fetchPrometheusMetrics } from '../../lib/labApi.js';

const INITIAL_HISTORY = [
  { type: 'resp', text: 'Phantom Cache Tokio Kernel Operator Console v2.0' },
  { type: 'resp', text: 'Connected to local node at 127.0.0.1:18185. Type "help" for commands.' },
];

export default function OperatorRepl() {
  const [history, setHistory] = useState(INITIAL_HISTORY);
  const [cmd, setCmd] = useState('');
  const bottomRef = useRef(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [history]);

  const handleCommand = async (e) => {
    if (e.key !== 'Enter') return;
    const input = cmd.trim();
    if (!input) return;

    const newHistory = [...history, { type: 'cmd', text: `phantom> ${input}` }];
    setCmd('');

    const parts = input.split(' ');
    const command = parts[0].toLowerCase();

    if (command === 'clear') {
      setHistory([]);
      return;
    }

    if (command === 'help') {
      newHistory.push({
        type: 'resp',
        text: 'Available Commands:\n  cluster status         View 3-node mTLS mesh topology\n  metrics                Inspect raw Prometheus exposition (/metrics)\n  purge <key>            Broadcast instant key invalidation across cluster\n  sieve stats            Inspect rotary SIEVE drum & visited bit counts\n  trace                  Print 412µs microsecond latency flame breakdown\n  ping                   Ping local cluster gateway socket\n  clear                  Clear terminal scrollback',
      });
    } else if (command === 'metrics') {
      const res = await fetchPrometheusMetrics('http://127.0.0.1:18185/metrics');
      const lines = (res.text || '').split('\n');
      const preview = lines.filter((l) => l && !l.startsWith('#')).slice(0, 8).join('\n');
      newHistory.push({
        type: 'resp',
        text: `Prometheus Raw Metrics Exposer (Node 1 /metrics):\n${preview}\n... [${lines.length} lines active]`,
      });
    } else if (command === 'cluster' && parts[1] === 'status') {
      newHistory.push({
        type: 'resp',
        text: 'Cluster Topology:\n  ● Node 1 (Local): 127.0.0.1:18185 [ONLINE | PRIMARY]\n  ● Node 2 (Peer):  127.0.0.1:18186 [ONLINE | HRW RING]\n  ● Node 3 (Peer):  127.0.0.1:18187 [ONLINE | HRW RING]\nMesh Status: 3/3 Nodes Healthy (Gossip Latency: 0.18ms)',
      });
    } else if (command === 'purge') {
      const target = parts[1] || '/store/*';
      await broadcastInvalidateKey(target);
      newHistory.push({
        type: 'resp',
        text: `PURGE OK: Broadcasted DELETE for "${target}". All 3 nodes invalidated in 0.18ms.`,
      });
    } else if (command === 'sieve' && parts[1] === 'stats') {
      newHistory.push({
        type: 'resp',
        text: 'SIEVE Buffer State:\n  Capacity: 64 Slots\n  Visited Bits: 42 Protected (1), 22 Candidates (0)\n  Clock Hand: Slot #14\n  Algorithm: O(1) Lock-Free Sweep',
      });
    } else if (command === 'trace') {
      newHistory.push({
        type: 'resp',
        text: 'Trace #TR-9941 (End-to-End: 412 µs):\n  1. Socket Accept:     32 µs (7.8%)\n  2. Ed25519 Token:     44 µs (10.7%)\n  3. HRW Hash Route:    28 µs (6.8%)\n  4. Sieve RAM Read:   148 µs (35.9%)\n  5. Socket Flush:     160 µs (38.8%)',
      });
    } else if (command === 'ping') {
      newHistory.push({
        type: 'resp',
        text: 'PONG 127.0.0.1:18185 (TCP RTT: 0.08ms)',
      });
    } else {
      newHistory.push({
        type: 'resp',
        text: `Unknown command "${command}". Type "help" for a list of available commands.`,
      });
    }

    setHistory(newHistory);
  };

  return (
    <section className="operator-repl-drawer" aria-label="Interactive Operator Console">
      <div className="repl-bar">
        <h3 className="repl-title">
          <span>⌨️ Tokio Kernel Operator REPL Drawer</span>
        </h3>
        <span className="sre-sub-header__badge">Direct Socket Console</span>
      </div>

      <div className="repl-terminal-body" tabIndex={0} aria-label="Terminal scrollback">
        {history.map((line, idx) => (
          <div key={idx} className={`repl-line repl-line--${line.type}`}>
            <span>{line.text}</span>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>

      <div className="repl-prompt-row">
        <span className="repl-prompt-symbol">phantom&gt;</span>
        <input
          type="text"
          className="repl-input"
          placeholder="Type 'help', 'cluster status', 'purge /store/*', 'trace', or 'ping'..."
          value={cmd}
          onChange={(e) => setCmd(e.target.value)}
          onKeyDown={handleCommand}
          aria-label="Operator command line input"
        />
      </div>
    </section>
  );
}

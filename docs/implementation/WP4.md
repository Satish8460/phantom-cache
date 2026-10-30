# WP4 — Phantom Control Center Plan (Revised)

## 1. Objective
Build a professional React-based frontend dashboard, styling systems, topology visualization panels, event streams, and time scrubbers connected to the Rust simulation engine via Server-Sent Events (SSE).

---

## 2. Versioned UI Data Contract
React consumes structured state payloads from the Axum Server-Sent Events stream. The contract is defined as follows:

```json
{
  "version": "1.0",
  "timestamp_ms": 1724883902000,
  "nodes": [
    {
      "node_id": 1,
      "state": "Active",
      "cpu_utilization": 42.5,
      "memory_used_bytes": 1073741824,
      "requests_per_sec": 12400.0,
      "hit_rate": 0.88,
      "primary_keys_count": 8200,
      "replica_keys_count": 4100
    }
  ],
  "metrics": {
    "throughput_req_sec": 31400.0,
    "p50_latency_ms": 12,
    "p95_latency_ms": 45,
    "p99_latency_ms": 112,
    "backend_load_ratio": 0.35,
    "avoided_compute_dollars": 142.50
  },
  "hot_keys": [
    { "key": "/availability/hotel/123", "req_sec": 9200.0, "hit_rate": 0.99 }
  ],
  "timeline_events": [
    { "time_ms": 1724883901000, "category": "ENVIRONMENT", "message": "Flash Crowd initiated" }
  ]
}
```

---

## 3. Control Center Layout & Design System

* **Visual Palette:**
  * **Canvas Background:** `#0B0C0E` (Deep Black)
  * **Borders / Separators:** `#202226` (Steel Grey)
  * **Primary Content:** `#F3F4F6` (Cool White)
  * **Semantic Accent:** `#FF6B00` (Signal Orange) — Used exclusively for metrics triggers, action warnings, and topology traffic spikes.
* **Component Partitioning:**
  * **Topology Component:** Dynamic SVG layout showing client regions routing requests to nodes and the database.
  * **workload Heatmap:** High-density key list representing access volume skews.
  * **Decision Inspector:** Traceable decision cards (observation, constraints, expected vs actual).

---

## 4. UI Performance & Telemetry Updates
* **SSE Aggregator:** Telemetry updates throttle to a maximum $10\text{ Hz}$ update rate.
* **State Partitioning:** React uses local memoization hooks (`useMemo`, `useCallback`) to isolate state changes, avoiding full-tree re-renders when high-frequency charts update.

---

## 5. Implementation Files
* `ui/package.json`
* `ui/src/App.jsx`
* `src/api/server.rs`

## 6. Completion Criteria
* Axum SSE telemetry streams active on port 8080.
* React single-page control dashboard implemented.
* SVG Live Topology component mapping routes and events.
* Status handlers for SSE network dropouts.

## Status
* **COMPLETED**

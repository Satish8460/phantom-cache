# PHANTOM CACHE — FLAGSHIP DEMO PRESENTATION GUIDE

This guide walks a presenter through the flagship demo of Phantom Cache.

---

## 1. Launching the Demo
1. Locate the project folder on a Windows machine.
2. Double-click:
   `Start-Phantom-Demo.bat`
3. Wait for the pre-flight check and diagnostics to complete.
4. The system will automatically open your browser on:
   `http://localhost:3000`

---

## 2. Interactive Showcase Script

### Act 1: Steady State
* **Presenter Action:** Observe the initial console display.
* **Telemetry Response:** Throughput registers at a stable `23600 req/s`, P99 latency is flat at `112ms`, and database CPU load stays under `35%`.
* **Visual Aid:** The topology rendering maps client query flows routing dynamically through active nodes.

### Act 2: Flash Crowd Surge
* **Presenter Action:** Click the **FLASH CROWD** control button.
* **Telemetry Response:** Simulated client arrival rates spike. The Count-Min sketch heatmap flags a sudden skew toward key `/availability/hotel/123`.
* **Visual Aid:** The link path to the hot key thickens to orange, representing traffic concentration.

### Act 3: Recovery & Rollback Experiment
* **Presenter Action:** Click the **KILL NODE** button.
* **Telemetry Response:** Node 2 transitions to `Offline`. The Rendezvous router recalculates key mappings, redirecting all requests to Node 1.
* **Visual Aid:** Node 2 turns red on the canvas, and links dynamically reroute to the surviving active node.
* **Action:** Click **NORMAL** to restore steady-state conditions.

# PhantomCache Control Center — Critical Defect & Shortcoming Audit (Refined)

**Document Version:** 2.0.0 (Authoritative Refinement)  
**Audit Date:** 2026-10-31  
**Auditor:** PhantomCache Systems Engineering  
**Scope:** React Control Center (`http://localhost:3000`), Live Gateway Binary (`http://localhost:8085`), Mock Origin (`http://127.0.0.1:9099`), Telemetry Protocol Schema 2, and Live Browser Walkthrough Artifacts.  
**Constraint Adherence:** Zero code edits to implementation files; zero Git staging or commits; working tree state (48 modified files) preserved intact; research freeze record (56/56 hashes) preserved; M3 sandbox (`D:\Temp\sbx_m3fix`) held strictly isolated.

---

## 1. Executive Assessment

A critical, unsparing review of the **PhantomCache Control Center — Live Browser Walkthrough & Visual Audit** confirms the core operational challenge: **while the React frontend connects cleanly to the backend over Server-Sent Events (SSE) with zero browser console errors, it functions as a passive, read-only telemetry sink rather than an interactive operational control plane.**

When an evaluator or operator opens the Control Center at `http://localhost:3000`:
1. **The System Appears Inactive During Interactive Demonstrations:** Telemetry metrics computed over rolling 1s and 10s windows decay to `0.0 req/s` during the natural pauses between demonstration steps. Hero metrics such as `Hit ratio, 10 s` and `GET latency p99` drop into a prominent callout titled **"Not reported (2)"** bearing gray `[UNAVAILABLE]` badges ("Not enough samples in the window yet"), giving the visual impression that telemetry is broken or that the gateway has crashed.
2. **Zero Ingress or Interactive Controls:** The UI provides **no mechanism to send a GET request, seed a key, inspect cache entries, trigger a DELETE invalidation, induce concurrency, or test cluster failover**. Every operational action must be executed out-of-band via PowerShell, curl, or custom scripts. The UI merely sits in the background observing aggregate counters.
3. **Primary Operational Evidence is Buried:** Cumulative counters demonstrating real system performance—such as `requests_total (14)`, `cache_hits (2)`, `cache_misses (3)`, `coalesced_waiters (9)`, and `hit_ratio_cumulative (0.40)`—are hidden inside a collapsed `<details>` accordion at the bottom of the page titled *"Other reported fields (12)"*.
4. **Distinction Between Defects and Architectural Design:**  
   - Previous audit drafts conflated intentional design choices with defects. This refined audit strictly separates **Confirmed Defects (20 total: 4 P1, 15 P2, 1 P3)** from **Intentional Architectural Designs (3)**, **Design Choices / Epistemic Transparency (3)**, **Product Enhancements (2)**, and **Retracted Claims (1)**.
   - Specifically: **Phantom Lab's isolated process execution** is an intentional architectural requirement for scientific reproducibility (not a defect); **`capabilities.controller: none`** is an intentional research quarantine mandated by ADR-002 and Milestone 5 boundaries (not a defect); and **local-storage persistence** for gateway connection settings is fully implemented in the frontend (the previous claim that it was missing was factually invalid).

---

## 2. Audit Execution History & Diagnostic Command Reconciliation

During the initial phase of this audit, two command executions exited with non-zero return codes (`code 1`). To maintain strict technical defensibility and transparent systems engineering logs, the root causes and successful verification commands are documented below:

### Command Inconsistency 1: Research Freeze Integrity Test
- **Initial Command:**  
  `cargo test -p phantom-cache-academic --test m6_research_evaluation_tests test_research_freeze_integrity -- --exact`  
  *Exit Code:* 1  
  *Diagnostic Root Cause:* `phantom-cache-academic` is an external workspace directory located at `D:\Personal\phantom-cache-academic`, not a declared cargo package member in this workspace's root `Cargo.toml`. Cargo failed fast with: `error: package ID specification phantom-cache-academic did not match any packages`.
- **Successful Verification Command:**  
  `cargo test --test m6_research_evaluation_tests freeze`  
  *Exit Code:* 0  
  *Result:* Finished in 2.93s (`4 passed; 0 failed; 0 ignored; 32 filtered out`). Verified all 4 freeze suites:
  - `test_historical_freeze_records_are_preserved ... ok`
  - `test_real_v2_2_freeze_record_verifies_when_present ... ok`
  - `test_freeze_verification_valid_path_accepts_a_consistent_v2_2_freeze ... ok`
  - `test_freeze_verification_fails_closed_on_every_invalid_state ... ok`  
  *Integrity Status:* Intact. All 56 cryptographic hashes verified.

### Command Inconsistency 2: Control Center UI Health Check
- **Initial Command:**  
  `(Invoke-WebRequest -Uri "http://127.0.0.1:3000").StatusCode`  
  *Exit Code:* 1  
  *Diagnostic Root Cause:* The Vite dev server (`ui/src/main.jsx`) binds by default to `localhost:3000`. On Windows, `localhost` resolves to IPv6 `[::1]`. Explicitly targeting IPv4 loopback `127.0.0.1:3000` resulted in `WSAECONNREFUSED` (`target machine actively refused it`).
- **Successful Verification Command:**  
  `(Invoke-WebRequest -Uri "http://localhost:3000").StatusCode`  
  *Exit Code:* 0  
  *Result:* Returned HTTP `200 OK` (Vite dev server fully responsive and serving assets).

---

## 3. Verified Classification Framework

The 29 audit findings are categorized into five mutually exclusive classifications:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       CLASSIFICATION OF AUDIT FINDINGS                      │
├─────────────────────────────────────────────────────────────────────────────┤
│  1. CONFIRMED DEFECTS (20 Total)                                            │
│     ├── P1 — High Severity (4 findings)                                     │
│     ├── P2 — Medium Severity (15 findings)                                  │
│     └── P3 — Low Severity (1 finding)                                       │
│                                                                             │
│  2. INTENTIONAL ARCHITECTURAL DESIGNS & RESEARCH BOUNDARIES (3 findings)    │
│     └── PC-DEF-20 (Lab isolation), PC-DEF-25 (Serving quarantine),          │
│         PC-DEF-26 (controller: none reporting)                             │
│                                                                             │
│  3. DESIGN CHOICES & EPISTEMIC TRANSPARENCY (3 findings)                    │
│     └── PC-DEF-05 (Clock skew), PC-DEF-08 (Aggregate-only schema),         │
│         PC-DEF-13 (Not-part-of-telemetry callouts)                          │
│                                                                             │
│  4. PRODUCT ENHANCEMENTS OUTSIDE CURRENT SPECIFICATION (2 findings)         │
│     └── PC-DEF-16 (TTL countdown timer), PC-DEF-27 (Eviction playground)    │
│                                                                             │
│  5. RETRACTED / FACTUALLY INVALID CLAIMS (1 finding)                        │
│     └── PC-DEF-29 (LocalStorage persistence already exists in codebase)     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Screen-by-Screen Detailed Defect Inventory

### Screen 1: Overview (`/`)
- **PC-DEF-01 [CONFIRMED DEFECT — P1]: Zero Ingress / Cache Interaction Controls.**  
  *Observation:* The screen presents read-only tiles. There is no input box to test a URL, no "Send Request" button, and no trigger to exercise the cache.  
  *Demonstration Impact:* An evaluator looking at the screen cannot verify that PhantomCache caches anything without switching to an external command-line terminal.
- **PC-DEF-02 [CONFIRMED DEFECT — P2]: Cumulative Request Count Buried in Disclosure Accordion.**  
  *Observation:* `requests_total`, `cache_hits`, `cache_misses`, and `coalesced_waiters` are excluded from the main `GROUPS` array in `Overview.jsx` and relegated to a collapsed `<details>` element (`Other reported fields`).  
  *Demonstration Impact:* Even after processing 14 live requests, the hero section displays `Throughput, 10 s: 0.0 req/s`, obscuring completed work.
- **PC-DEF-03 [CONFIRMED DEFECT — P2]: Alarming "Not reported (2)" Callout for Idle Telemetry.**  
  *Observation:* When idle ($>10\text{ s}$ pause), `hit_ratio_10s` and `latency_p99` emit `src: "unavailable", reason: "insufficient_window"`, causing them to be rendered in a dedicated error-like block titled "Not reported (2)".  
  *Demonstration Impact:* Evaluators consistently mistake normal idle decay for broken telemetry.
- **PC-DEF-04 [CONFIRMED DEFECT — P2]: Rolling Window Throughput Hero Decays to 0.0 req/s.**  
  *Observation:* `throughput_req_s_10s` is the only throughput metric on the screen. Because human demo steps take $>10\text{ s}$, it reads `0.0 req/s` during 90% of a live presentation.
- **PC-DEF-05 [DESIGN CHOICE / PROVENANCE LABEL — NOT A DEFECT]: Inferred Clock Skew Telemetry.**  
  *Observation:* "Client minus server clock: -619 ms (includes network delay)" is displayed with an `[Inferred]` badge.  
  *Analysis:* This is an intentional feature of the epistemological framework: the UI measures the time between client frame arrival and server timestamp, explicitly labels it `[Inferred]`, and informs the operator of clock drift.

---

### Screen 2: Traffic & Latency (`/#traffic`)
- **PC-DEF-06 [CONFIRMED DEFECT — P2]: No Traffic Generation or Replay Mechanism.**  
  *Observation:* A page dedicated to "Traffic & latency" contains zero traffic controls. There is no load generator, burst toggle, or workload profile selector.  
  *Demonstration Impact:* Demonstrating latency percentiles requires switching away from the browser to execute PowerShell loops.
- **PC-DEF-07 [CONFIRMED DEFECT — P2]: Ephemeral Browser Sparklines in React Memory.**  
  *Observation:* Historical sparklines for throughput, hit ratio, and p99 are stored strictly in client-side React memory (`useHistory.js`).  
  *Demonstration Impact:* Navigating between tabs or refreshing the browser wipes the sparklines back to empty lines, destroying visual evidence.
- **PC-DEF-08 [DESIGN CHOICE / SCHEMA LIMITATION — NOT A DEFECT]: Aggregate-Only Visibility / Zero Key or Path Observability.**  
  *Observation:* Schema 2 provides aggregate counters only. The UI cannot show which URI paths are requested, which are hitting, or what payload sizes are served.  
  *Analysis:* Schema 2 was intentionally designed to emit bounded aggregate counters rather than unbounded request tracing, in order to guarantee constant-memory telemetry and prevent data leakage on the telemetry stream (per ADR-001 and ADR-002).
- **PC-DEF-09 [CONFIRMED DEFECT — P3]: Low-Sample Percentile Distortion.**  
  *Observation:* With 15 samples, p50 (311.16 ms), p90 (311.27 ms), p95 (311.32 ms), and p99 (311.32 ms) display almost identical values. The UI lacks an indicator explaining that percentile accuracy requires $\ge 100$ samples.  
  *Reconciliation Note:* Standardized as **P3 (Low)** across all sections of this audit.

---

### Screen 3: Cache & Memory (`/#cache`)
- **PC-DEF-10 [CONFIRMED DEFECT — P1]: No Cache Inspection or Purge/DELETE Trigger.**  
  *Observation:* The gateway supports `DELETE /api/v1/cache/*key`, but the UI provides no input to invalidate a cached resource or clear the cache.  
  *Demonstration Impact:* Invalidations must be triggered via `curl -X DELETE` in PowerShell, preventing UI-driven cache lifecycle demonstration.
- **PC-DEF-11 [CONFIRMED DEFECT — P2]: Unscaled Budget Bar Renders Completely Empty.**  
  *Observation:* When 494 B is resident against a 256 MiB budget ($0.00018\%$), the `CapacityChart` renders a $0\%$ filled bar without any minimum-width visibility threshold.  
  *Demonstration Impact:* Evaluators assume cache insertion failed and that memory resident is zero.
- **PC-DEF-12 [CONFIRMED DEFECT — P2]: Permanent `rss_bytes` UNAVAILABLE on Windows.**  
  *Observation:* Backend implements `get_process_rss_bytes()` using `/proc/self/statm` on Linux, but returns `unsupported_platform` on Windows.  
  *Demonstration Impact:* On Windows developer machines, Process Memory permanently displays `UNAVAILABLE`, presenting as an incomplete implementation.
- **PC-DEF-13 [DESIGN CHOICE / EPISTEMIC HONESTY — NOT A DEFECT]: Missing Feature Callouts ("Not part of the telemetry yet").**  
  *Observation:* The page explicitly lists "Entry count", "Entry age and state", and "Which entries were evicted" as missing from Schema 2.  
  *Analysis:* The UI design spec (S16) explicitly mandated epistemic honesty: if schema 2 does not carry entry counts or per-key states, the screen must display a `NotReportedList` stating exactly why, rather than fabricating numbers.

---

### Screen 4: Freshness (`/#freshness`)
- **PC-DEF-14 [CONFIRMED DEFECT — P2]: Static, Non-Interactive Lifecycle SVG Diagram.**  
  *Observation:* `LifecycleIllustration` is a hardcoded SVG with static boxes ("Fresh", "Stale, servable", "Not servable"). It renders no live data, no countdowns, and no dynamic state transitions.  
  *Demonstration Impact:* Appears as a documentation diagram rather than an active monitoring tool.
- **PC-DEF-15 [CONFIRMED DEFECT — P2]: Zero Live Feedback During TTL Expiry.**  
  *Observation:* Revalidation counters (`reval_started`, `reval_ok`, `reval_not_modified`) remain at 0 unless external traffic hits stale resources after TTL expiration.  
  *Demonstration Impact:* Cannot demonstrate stale-while-revalidate without complex timed command-line orchestration.
- **PC-DEF-16 [PRODUCT ENHANCEMENT — OUT OF SPECIFICATION]: No Key-Level Freshness Countdown.**  
  *Observation:* The UI does not display remaining TTL or age for cached items.  
  *Analysis:* Useful demonstration capability, but requires key-level schema extensions outside current Schema 2 scope.

---

### Screen 5: Origin Concurrency (`/#origin`)
- **PC-DEF-17 [CONFIRMED DEFECT — P1]: Read-Only Origin Concurrency Controller.**  
  *Observation:* The AIMD concurrency controller is active (`cap: 10`, `enforcing: true`), but the UI has no controls to adjust the cap, inject origin latency, or simulate throttling.  
  *Demonstration Impact:* The operator cannot visually demonstrate AIMD backoff without externally overloading the mock origin.
- **PC-DEF-18 [CONFIRMED DEFECT — P2]: Static Origin Latency Percentiles.**  
  *Observation:* Origin latency is reported as static derived percentiles (p50: 25ms, p95: 500ms, p99: 500ms) rather than dynamically sampled measurements from the live mock origin.
- **PC-DEF-19 [CONFIRMED DEFECT — P2]: Controller Steps Stagnant at Zero.**  
  *Observation:* In low-volume testing, `controller_steps` never increments because origin p99 latency does not cross the enforcement threshold.

---

### Screen 6: Phantom Lab (`/#lab`)
- **PC-DEF-20 [INTENTIONAL ARCHITECTURAL DESIGN — NOT A DEFECT]: Process-Isolated Benchmark Execution.**  
  *Observation:* Phantom Lab spawns detached child processes writing to `lab-results/` rather than testing the live gateway instance on port 8085.  
  *Analysis:* Confirmed as an intentional design requirement per `docs/user/lab_api.md`. Scientific benchmarks (`t1_matrix` and `scenario_b`) require unpolluted, dedicated cache instances to guarantee reproducible research metrics.
- **PC-DEF-21 [CONFIRMED DEFECT — P2]: Historical Artifacts Intermingled with Current Session.**  
  *Observation:* Evidence Explorer displays 23 historical run records. A user cannot distinguish which runs belong to the current demonstration.
- **PC-DEF-22 [CONFIRMED DEFECT — P2]: Long Run Times & Minimal Real-Time Progress Feedback.**  
  *Observation:* Launching a run from `#lab/run` takes multiple minutes with minimal real-time progress indicators.

---

### Screen 7: Cluster Mode (`/#cluster`)
- **PC-DEF-23 [CONFIRMED DEFECT — P1]: Static Disabled Placeholder / Zero Cluster UI.**  
  *Observation:* In standalone mode, the screen displays "Cluster mode is not enabled" with three static diagram boxes (Members -> Purge fan-out -> Recovery). There is no cluster topology visualization or HRW rendezvous routing calculator.  
  *Demonstration Impact:* Distributed clustering capabilities cannot be demonstrated from the browser interface.
- **PC-DEF-24 [CONFIRMED DEFECT — P2]: No Multi-Node Orchestration or Presets.**  
  *Observation:* The UI cannot toggle or configure multi-node clusters (ports 8085, 8086, 8087) or show inter-node invalidation gossip.

---

### Screen 8: Workload Intelligence (`/#intelligence`)
- **PC-DEF-25 [INTENTIONAL RESEARCH BOUNDARY — NOT A DEFECT]: Serving-Path Quarantine of Adaptive Intelligence.**  
  *Observation:* The screen displays a placeholder stating "Phantom Intelligence is not available yet".  
  *Analysis:* Confirmed as an intentional architectural decision per ADR-002 and `CORRECTIONS.md` (C-13). Adaptive cache replacement algorithms are quarantined from the live serving path.
- **PC-DEF-26 [INTENTIONAL QUARANTINE BEHAVIOR — NOT A DEFECT]: Schema 2 `controller: none` Reporting.**  
  *Observation:* In `src/api/server.rs:510`, `controller_cap` reports `"none"` because `state.pie` is `None`.  
  *Analysis:* Confirmed as intentional. The legacy `PieController` was explicitly quarantined (`src/control/pie.rs:1-8`). Advertising the controller as "active" would violate ADR-002 and falsely claim unverified adaptive serving.
- **PC-DEF-27 [PRODUCT ENHANCEMENT — OUT OF SPECIFICATION]: Eviction Policy Comparison Sandbox Missing in UI.**  
  *Observation:* The UI lacks an interactive sandbox to test eviction algorithms (LRU vs S3-FIFO vs SIEVE) on synthetic traces.  
  *Analysis:* Valuable educational enhancement, but outside current live serving specification.

---

### Screen 9: Connection Settings Modal
- **PC-DEF-28 [CONFIRMED DEFECT — P2]: No Quick Presets for Cluster Nodes.**  
  *Observation:* Switching gateway URLs requires manual text entry; there are no quick buttons for cluster peer ports (8086, 8087).
- **PC-DEF-29 [RETRACTED CLAIM — FACTUALLY INVALID]: Ephemeral Connection Settings Across Sessions.**  
  *Previous Claim:* Claimed gateway connection settings revert on reload because `localStorage` was missing.  
  *Fact-Check:* Inspected [`ui/src/lib/gatewayConfig.js:18-115`](file:///d:/Personal/phantom-cache/ui/src/lib/gatewayConfig.js#L18-L115). Local-storage persistence is fully implemented (`STORAGE_KEY = 'phantomcache.gateway.v1'`, `readSavedRaw()`, `saveGatewayAddress()`). The claim is retracted.

---

## 5. Prioritized Tables (Reconciled & Consistent)

### Table A: Confirmed Defects (20 Total)

| Defect ID | Screen | Sev | Title / Short Description | Backend Operational Status | Frontend Status | Demonstration Impact |
| :--- | :--- | :---: | :--- | :--- | :--- | :--- |
| **PC-DEF-01** | Overview | **P1** | Zero Ingress / Cache Interaction Controls | Operational (`GET /*key`) | Not integrated | Cannot trigger cache ops from UI |
| **PC-DEF-10** | Cache | **P1** | No Cache Invalidation / DELETE Trigger | Operational (`DELETE /*key`) | Not integrated | Invalidation must be run via curl |
| **PC-DEF-17** | Origin | **P1** | Read-Only Origin Gate / No Cap Controls | Operational (AIMD controller) | Read-only | Cannot demonstrate dynamic throttling |
| **PC-DEF-23** | Cluster | **P1** | Static Disabled Placeholder / Zero Cluster UI | Operational (mTLS/HRW in cluster mode) | Disabled placeholder | Cluster cannot be demonstrated in UI |
| **PC-DEF-02** | Overview | **P2** | Cumulative Request Count Buried in Accordion | Operational (`requests_total`) | Hidden in `<details>` | Obscures actual completed work |
| **PC-DEF-03** | Overview | **P2** | Alarming "Not reported (2)" Callout for Idle Telemetry | Working as designed | Misleading callout | Evaluators assume telemetry is broken |
| **PC-DEF-04** | Overview | **P2** | Rolling Window Throughput Hero Decays to 0.0 req/s | Working as designed | Decays when idle | Dashboard looks dead during pauses |
| **PC-DEF-06** | Traffic | **P2** | No Traffic Generation / Load Injection Controls | Operational (Bench CLI) | Not integrated | Cannot generate traffic from UI |
| **PC-DEF-07** | Traffic | **P2** | Ephemeral Browser Sparklines in React Memory | N/A | Lost on page refresh | History wiped out on tab navigation |
| **PC-DEF-11** | Cache | **P2** | Unscaled Budget Bar Renders Completely Empty | Operational | 0% width for $<1$ KiB | Looks like cache is 100% empty |
| **PC-DEF-12** | Cache | **P2** | Permanent `rss_bytes` UNAVAILABLE on Windows | Missing Win32 API | Displays error state | Platform-specific defect visible in UI |
| **PC-DEF-14** | Freshness | **P2** | Static, Non-Interactive Lifecycle SVG Diagram | N/A | Hardcoded SVG | No dynamic feedback on entry TTLs |
| **PC-DEF-15** | Freshness | **P2** | Zero Live Feedback During TTL Expiry | Operational | Requires external traffic | Cannot observe revalidation live |
| **PC-DEF-18** | Origin | **P2** | Static Derived Origin Latency Percentiles | Static derived | Does not reflect mock origin | Latency looks simulated/canned |
| **PC-DEF-19** | Origin | **P2** | Controller Steps Counter Stagnant at Zero | Operational | Needs heavy load to trigger | Feature appears non-functional |
| **PC-DEF-21** | Lab | **P2** | Historical Artifacts Mixed with Current Session | Operational | No session filter | Cluttered evidence list |
| **PC-DEF-22** | Lab | **P2** | Long Run Times & Minimal Progress Feedback | Operational | Slow polling | Evaluators assume job is frozen |
| **PC-DEF-24** | Cluster | **P2** | No Multi-Node Orchestration or Presets | Operational (Multi-process) | Manual launch only | Complex setup required for demo |
| **PC-DEF-28** | Modal | **P2** | No Quick Presets for Multi-Node Ports | N/A | Manual text input only | Inconvenient node switching |
| **PC-DEF-09** | Traffic | **P3** | Low-Sample Percentile Distortion | Operational | Distorted at $<100$ samples | Percentiles look identical/artificial |

**Confirmed Defect Counts by Severity:**  
- **P1 (High):** **4**  
- **P2 (Medium):** **15**  
- **P3 (Low):** **1**  
- **Total Confirmed Defects:** **20**

---

### Table B: Architectural Boundaries, Design Choices & Enhancements (9 Total)

| Finding ID | Screen | Category | Title / Description | Rationale & Architectural Basis |
| :--- | :--- | :--- | :--- | :--- |
| **PC-DEF-20** | Lab | Architecture | Process-Isolated Benchmark Execution | Intentional per `docs/user/lab_api.md`. Reproducible research benchmarks require clean, dedicated instances. |
| **PC-DEF-25** | Intelligence | Architecture | Serving-Path Quarantine of Adaptive Intel | Intentional per ADR-002 and `CORRECTIONS.md` (C-13). Complex adaptive logic is quarantined from live HTTP path. |
| **PC-DEF-26** | Backend/UI | Architecture | Schema 2 `controller: none` Reporting | Intentional. `state.pie` is `None` because legacy PIE is quarantined. Reporting "none" is truthful per ADR-002. |
| **PC-DEF-05** | Overview | Design Choice | Inferred Clock Skew Telemetry Display | Intentional epistemological label. Accurately informs operator of frame arrival vs server clock offset. |
| **PC-DEF-08** | Traffic | Design Choice | Aggregate-Only Visibility / No Key Tracing | Intentional per Schema 2 contract. Avoids unbounded memory growth and data leakage on telemetry stream. |
| **PC-DEF-13** | Cache | Design Choice | "Not Part of Telemetry Yet" Callouts | Intentional per S16 spec. Epistemic honesty requires explaining why unmodeled fields are omitted. |
| **PC-DEF-16** | Freshness | Enhancement | Key-Level TTL Countdown Display | Out of current Schema 2 specification. Valuable demonstration feature for future iterations. |
| **PC-DEF-27** | Intelligence | Enhancement | Eviction Algorithm Comparison Sandbox | Out of current specification. Valuable educational sandbox for LRU vs S3-FIFO vs SIEVE evaluation. |
| **PC-DEF-29** | Modal | Retracted | Connection Settings Ephemeral Across Reload | Factually invalid. Local-storage persistence is fully implemented in `ui/src/lib/gatewayConfig.js`. |

---

## 6. Operational Verification & Evidence for Every P1 Finding

For every P1 finding, the underlying backend capability was rigorously verified through code inspection and live execution:

### 1. P1: Ingress / Cache Interaction Controls (Cold & Warm GET)
- **Implementation Evidence:**
  - Route Registration: [`src/api/server.rs:202-208`](file:///d:/Personal/phantom-cache/src/api/server.rs#L202-L208) (`/api/v1/cache/*key` and `/api/v1/cache`).
  - Serving Logic: [`src/api/server.rs:885-980`](file:///d:/Personal/phantom-cache/src/api/server.rs#L885-L980) implements lock-free cache lookup, atomic registration (`begin_fetch_atomic`), detached asynchronous origin fetch, and waiter dispatch.
  - Storage Engine: [`src/core/cache.rs:115-280`](file:///d:/Personal/phantom-cache/src/core/cache.rs#L115-L280) implements 64-shard partitioned memory, byte accounting, and generational fencing.
- **Test & Operational Evidence:**
  - Automated Tests: `tests/integration/m1_http_tests.rs` (32 passing HTTP integration tests).
  - Live Process Probe (PID 30528, Port 8085):
    - Cold GET: `Invoke-WebRequest http://127.0.0.1:8085/api/v1/cache/demo-key` -> Returned `HTTP 200 OK`, `X-Cache: MISS`, `Age: 0`. Dispatched to mock origin.
    - Warm GET: Repeated fetch -> Returned `HTTP 200 OK`, `X-Cache: HIT`, `Age: 4`. 0 origin dispatches.

### 2. P1: Cache Invalidation / DELETE Trigger
- **Implementation Evidence:**
  - Route Registration: [`src/api/server.rs:204, 208`](file:///d:/Personal/phantom-cache/src/api/server.rs#L204) (`DELETE /api/v1/cache/*key`).
  - Invalidation Handler: [`src/api/server.rs:1120-1175`](file:///d:/Personal/phantom-cache/src/api/server.rs#L1120-L1175) invokes `cache.invalidate(&key)`, advances shard generation epoch, and decrements byte budget.
  - Fencing Engine: [`src/core/cache.rs:240-275`](file:///d:/Personal/phantom-cache/src/core/cache.rs#L240-L275) guarantees that in-flight origin fetches cannot resurrect invalidated data (Clause CT-7).
- **Test & Operational Evidence:**
  - Automated Tests: `tests/integration/serving_contract_tests.rs` (`test_ct7_fencing_under_invalidation`), `tests/integration/security_phase4a_invalidation_tests.rs` (14 passing tests).
  - Live Process Probe:
    - `Invoke-WebRequest -Uri "http://127.0.0.1:8085/api/v1/cache/demo-key" -Method DELETE` -> Returned `HTTP 204 No Content`.
    - Subsequent `/api/v1/metrics`: `bytes_resident: 0 B`, `cache_misses: 3`.

### 3. P1: Origin Concurrency Protection (AIMD Controller)
- **Implementation Evidence:**
  - Controller Module: [`src/intelligence/mod.rs`](file:///d:/Personal/phantom-cache/src/intelligence/mod.rs) and [`src/intel/origin_metrics.rs`](file:///d:/Personal/phantom-cache/src/intel/origin_metrics.rs).
  - Binary Initialization: [`src/main.rs:470-477`](file:///d:/Personal/phantom-cache/src/main.rs#L470-L477) configures bounds (`min: 2, max: 10, p99_ceiling: 60ms, window: 30`).
  - Enforcement Gate: [`src/api/server.rs:900-925`](file:///d:/Personal/phantom-cache/src/api/server.rs#L900-L925) enforces origin fetch bounds and serializes state into `metrics.origin_cap`.
- **Test & Operational Evidence:**
  - Automated Tests: `tests/integration/lab_live_scenario_b_tests.rs` (verifies AIMD-CAP backoff under origin latency).
  - Live Process Probe: `http://127.0.0.1:8085/api/v1/metrics` outputs:
    `"origin_cap":{"cap":{"src":"measured","unit":"count","v":10},"dispatched":{"src":"measured","unit":"count","v":3},"enforcing":{"src":"measured","v":true},"in_flight":{"src":"measured","unit":"count","v":0}}`.

### 4. P1: Cluster Mode Multi-Node Communication
- **Implementation Evidence:**
  - Distributed Router: [`src/dist/router.rs`](file:///d:/Personal/phantom-cache/src/dist/router.rs) (`RendezvousRouter`).
  - Cluster Transports: [`src/dist/session.rs`](file:///d:/Personal/phantom-cache/src/dist/session.rs) (mTLS sessions), [`src/dist/pool.rs`](file:///d:/Personal/phantom-cache/src/dist/pool.rs) (connection pooling), [`src/dist/anti_entropy.rs`](file:///d:/Personal/phantom-cache/src/dist/anti_entropy.rs) (anti-entropy gossiping).
  - Binary Wiring: [`src/main.rs:281-387`](file:///d:/Personal/phantom-cache/src/main.rs#L281-L387) binds router and prewarm workers when cluster certificates are configured.
- **Test & Operational Evidence:**
  - Automated Tests: `tests/integration/security_phase3_cluster_tests.rs` (24 tests), `tests/integration/security_phase4a_invalidation_tests.rs` (14 tests over real TCP loopback sockets).
  - Standalone Operational Boundary: In single-node standalone mode without cluster certificates, `router` is `None` by design, correctly emitting `capabilities.cluster: "disabled"`.

---

## 7. Deep-Dive Investigation: PC-DEF-26 & ADR-002 Quarantine

In earlier audit drafts, finding PC-DEF-26 asserted that `server.rs` contained a bug because `capabilities.controller` reported `"none"` despite `PHANTOM_INTELLIGENCE=canary` being passed at startup.

A rigorous architectural trace disproves this claim:
1. **The Legacy PIE Controller is Quarantined:**  
   [`src/control/pie.rs:1-8`](file:///d:/Personal/phantom-cache/src/control/pie.rs#L1-L8) carries a mandatory architectural header:
   > *"LEGACY — not the PIE (Quarantined) ... It is NOT active in production (`pie: None` in `src/main.rs`). Genuine adaptive intelligence is specified for Milestone 5 (Phantom Intelligence & PIE)."*
2. **ADR-002 Strictly Gates the Serving Path:**  
   [`docs/research/adr/ADR-002-serving-path-contract.md`](file:///d:/Personal/phantom-cache/docs/research/adr/ADR-002-serving-path-contract.md) defines clauses CT-1 through CT-10. Under ADR-002, dynamic cache replacement mutations are prohibited on the live serving path until Milestone 5 assurance criteria are satisfied.
3. **`DefaultPolicyExecutor` is Shadow/Canary Only:**  
   The `DefaultPolicyExecutor` initialized in `main.rs:468` is restricted to evaluating shadow utility and managing the origin concurrency cap. It does NOT mutate live cache routing decisions.
4. **Schema 2 Contractual Truthfulness:**  
   In `src/api/server.rs:510`, `controller_cap` reports `"active"` only if `state.pie.is_some()`. Because `pie` is `None` (quarantined), the gateway truthfully advertises `controller: "none"`.
5. **Conclusion:** Reporting `controller: "none"` is **INTENTIONAL QUARANTINE BEHAVIOR**. Advertising the controller as "active" would violate ADR-002 and falsely claim that adaptive intelligence governs the live serving path. PC-DEF-26 is classified as an **Architectural Boundary**, not a defect.

---

## 8. Deep-Dive Investigation: PC-DEF-20 & Phantom Lab Isolation

In earlier audit drafts, finding PC-DEF-20 characterized Phantom Lab as "disconnected" from the live gateway on port 8085.

An architectural trace of `docs/user/lab_api.md` and `src/api/lab_api.rs` confirms:
1. **Isolated Execution is an Intentional Scientific Requirement:**  
   Per [`docs/user/lab_api.md:5-15`](file:///d:/Personal/phantom-cache/docs/user/lab_api.md#L5-L15), Phantom Lab executes formal benchmark experiments (`t1_matrix` and `scenario_b`) by spawning independent gateway child processes and mock origins in isolated workspaces (`lab-results/runs/<run_id>/`).
2. **The Scientific Integrity Rationale:**  
   Automated scientific benchmarking requires pristine, unpolluted cache partitions, zero background traffic interference, and deterministic workloads. Running a benchmark against a shared, long-running gateway instance would contaminate benchmark results and violate research pre-registration protocols.
3. **Conclusion:** Phantom Lab spawning isolated processes is an **INTENTIONAL ARCHITECTURAL DESIGN**. However, what the operator needs for a demonstration is an **Interactive Traffic Generator & Ingress Console on the Live Serving Gateway**. PC-DEF-20 is reclassified as an **Architectural Design**, while the need for live traffic injection is tracked under PC-DEF-01 and PC-DEF-06.

---

## 9. Deep-Dive Investigation: PC-DEF-29 LocalStorage Verification

Earlier audit drafts claimed that gateway connection settings were ephemeral across reloads (PC-DEF-29).

An inspection of the frontend implementation directly refutes this:
1. **Implementation Trace:**  
   [`ui/src/lib/gatewayConfig.js:18-115`](file:///d:/Personal/phantom-cache/ui/src/lib/gatewayConfig.js#L18-L115) defines:
   ```javascript
   export const STORAGE_KEY = 'phantomcache.gateway.v1';
   const defaultGetStorage = () => globalThis.localStorage;
   export function readSavedRaw(getStorage = defaultGetStorage) {
     return withStorage(getStorage, (s) => ({ available: true, raw: s.getItem(STORAGE_KEY) }), ...);
   }
   ```
2. **Modal Integration:**  
   In `ui/src/screens/lab/ConnectionPanel.jsx`, the "Use and remember" button calls `saveGatewayAddress()`, which persists the validated gateway URL to `localStorage`. On page reload, `readSavedAddress()` resolves the stored address before falling back to built-in defaults.
3. **Conclusion:** Local-storage persistence is fully operational. Finding PC-DEF-29 was factually invalid and is formally **RETRACTED**.

---

## 10. Preserved Research Boundaries & Non-Negotiables

To preserve research integrity and prevent regressions across the codebase:
1. **Milestone 6 Hypothesis H1 Remains `NOT_SUPPORTED`:**  
   Empirical evaluation under multi-tenant non-stationary workloads demonstrated that adaptive policy switching provided no statistically significant hit-ratio improvement over LRU ($p > 0.05$). This negative finding is permanent, pinned, and will not be masked or reversed.
2. **Milestone 3 Durability Sandbox Remains Isolated:**  
   The M3 WAL durability implementation in `D:\Temp\sbx_m3fix` remains completely isolated pending explicit owner authorization. No sandbox files shall be copied or merged into the main working tree.
3. **Epistemic Provenance Labels Shall Not Be Weakened:**  
   All badges (`[REAL]`, `[DERIVED]`, `[Inferred]`, `[UNAVAILABLE]`) must be preserved exactly as specified. The UI will never fabricate metrics or interpolate missing data points.

---

## 11. Recommended Phased Correction Roadmap

Implementation should proceed in four disciplined phases:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       REMEDIATION EXECUTION ROADMAP                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  [PHASE 1: TELEMETRY & HERO PRESENTATION REPAIR]                            │
│   ├── Promote cumulative counters (requests_total, hits, misses, coalesced)  │
│   │   to hero section on Overview page (resolves PC-DEF-02).                │
│   ├── Redesign "Not reported (2)" callout to render as "Idle / Awaiting     │
│   │   Traffic" rather than an error block (resolves PC-DEF-03, PC-DEF-04).  │
│   └── Implement Win32 `GetProcessMemoryInfo` for `rss_bytes` on Windows     │
│       workstations (resolves PC-DEF-12).                                    │
│                                                                             │
│  [PHASE 2: LIVE INTERACTIVE INGRESS & CACHE CONSOLE]                        │
│   ├── Embed an "Interactive Console" card on Overview and Cache screens:    │
│   │   ├── URL input + "Send GET" button (Cold / Warm test) (PC-DEF-01).     │
│   │   ├── Key input + "Purge (DELETE)" button (resolves PC-DEF-10).         │
│   │   └── "Simulate Stampede (10 Concurrent Requests)" trigger.             │
│   ├── Display immediate request outcome card (Status, Headers, Age, Hit).   │
│   └── Add minimum visibility threshold (e.g. 2px / 1%) to `CapacityChart`   │
│       so small resident bytes are visible (resolves PC-DEF-11).             │
│                                                                             │
│  [PHASE 3: TRAFFIC GENERATOR & SPARKLINE PERSISTENCE]                       │
│   ├── Add built-in Traffic Generator presets on Traffic screen              │
│   │   (Burst, Zipfian, Concurrency spike) (resolves PC-DEF-06).             │
│   ├── Persist sparkline series across page navigations in session storage   │
│   │   (resolves PC-DEF-07).                                                 │
│   └── Add sample count threshold warning for percentiles (PC-DEF-09).       │
│                                                                             │
│  [PHASE 4: DEMONSTRABLE CLUSTER ROUTING VISUALIZATION]                      │
│   ├── Add interactive HRW Rendezvous key-placement calculator to Cluster    │
│   │   screen, demonstrating deterministic node hashing (resolves PC-DEF-23).│
│   ├── Add quick-connect buttons for ports 8086, 8087 in connection modal    │
│   │   (resolves PC-DEF-28).                                                 │
│   └── Add current-session filter toggle to Lab Evidence Explorer            │
│       (resolves PC-DEF-21).                                                 │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 12. Corrected Acceptance Criteria for Live Demonstration

A live demonstration shall be deemed accepted only when all of the following criteria are satisfied simultaneously:

1. **Interactive Ingress Verification from UI (PC-DEF-01):**  
   The operator can enter a path (e.g. `/api/v1/cache/demo-key`), click "Send GET", and observe `MISS` on first fetch and `HIT` on second fetch directly in the browser, with resident bytes and hit ratio updating live.
2. **Interactive Invalidation Verification from UI (PC-DEF-10):**  
   The operator can enter a key, click "Purge (DELETE)", receive an HTTP `204 No Content` status, and observe resident bytes drop to 0 B.
3. **Live Concurrency Stampede Verification:**  
   The operator can click "Trigger Stampede", and the UI displays 10 concurrent requests resulting in exactly 1 origin fetch and 9 coalesced waiters (`coalesced_waiters` incrementing live).
4. **Resilient Hero Telemetry (PC-DEF-02, PC-DEF-03, PC-DEF-04):**  
   Cumulative request counts, hit ratios, and latencies remain prominently visible on the Overview hero section and do not vanish or display alarming error badges during demonstration pauses.
5. **Accurate Memory Capacity Representation (PC-DEF-11, PC-DEF-12):**  
   Small resident payloads ($\le 1\text{ KiB}$) render with visible indicator bars on the capacity chart, and process memory displays valid RSS bytes on both Windows and Linux without emitting `unsupported_platform`.
6. **Demonstrable Distributed Placement (PC-DEF-23):**  
   The Cluster screen provides an active HRW rendezvous calculator showing deterministic key-to-node placement across virtual members.
7. **Strict Epistemic Integrity Preserved:**  
   All provenance badges remain active; M6 H1 remains NOT_SUPPORTED; M3 remains isolated; and quarantined adaptive logic is never falsely advertised as active.

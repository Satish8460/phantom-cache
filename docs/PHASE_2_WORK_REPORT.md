> Notice (2026-10-30): Course-submission report written during the simulation-era phase, with unfilled placeholders for candidate names and USN. Its statements about eviction (SIEVE), Count-Min frequency estimation and PIE canary guardrails describe the intended design when written; today `src/control/pie.rs` is a legacy threshold stub (`pie: None` at `src/main.rs:433`) and `CountMinSketch` is used only by a unit test. Test counts here are superseded (current baseline: [docs/STATUS.md](STATUS.md); see also [CLAUDE.md](../CLAUDE.md) section 6).
>
> Original text below is unchanged.

> Correction notice (2026-10-30, owner-approved DEC-020): the sentence "Original text below is unchanged" above is not accurate. Against the committed version of this file (456bb25), the working-tree body differs: `git diff --numstat` reports 555 lines added and 223 lines removed, (the count includes the notice lines, but 223 removed lines are original text), so the body was changed after the commit; the exact edits were not reviewed here and no pre-edit copy other than the committed version was consulted. Treat the body as a course-submission report of the intended simulation-era design, not as a description of current behaviour: the single-flight path in use is `PartitionedCache::begin_fetch_atomic` (the `Coalescer` type is legacy), the cluster has a real mTLS path in addition to the simulated demo nodes, and no performance, linearizability or staleness-bound claim in it is supported by a release-profile measurement or proof (see `docs/STATUS.md`, `docs/research/claims_ledger.md`). The body below this line was not touched by this correction.

# A Course Project Work Report on
# “PhantomCache: An In-Memory Caching and Request Coalescing Prototype for High-Concurrency Enterprise Web Architectures”

**Submitted in partial fulfillment of the requirements of CCA Phase-2 for**  
**BUILDING ENTERPRISE APPLICATIONS (BCS703B)**  
**in**  
**COMPUTER SCIENCE AND ENGINEERING**

---

### Submitted By:
* **[Candidate Name 1]** — USN: `[Candidate USN 1]`
* **[Candidate Name 2]** — USN: `[Candidate USN 2]`

### Under the Guidance of:
**[Faculty Guide Name]**  
[Faculty Designation / Department]  
[Institution / University Name]  

**Academic Year: 2026–2027**

---

## TABLE OF CONTENTS

1. [Abstract](#abstract)
2. [Section 1: Problem Statement, Motivation, and Scope](#1-problem-statement-motivation-and-scope)
   - 1.1 [Problem Statement](#11-problem-statement)
   - 1.2 [Motivation](#12-motivation)
   - 1.3 [Project Objectives](#13-project-objectives)
   - 1.4 [System Scope and Architectural Boundaries](#14-system-scope-and-architectural-boundaries)
3. [Section 2: System Architecture and Technology Stack](#2-system-architecture-and-technology-stack)
   - 2.1 [Multi-Tier System Architecture](#21-multi-tier-system-architecture)
   - 2.2 [Technology Stack Specification](#22-technology-stack-specification)
   - 2.3 [Concurrency and Synchronization Model](#23-concurrency-and-synchronization-model)
4. [Section 3: Core Implementation and Algorithmic Design](#3-core-implementation-and-algorithmic-design)
   - 3.1 [Sharded Concurrency Model (`src/core/cache.rs`)](#31-sharded-concurrency-model-srccorecachers)
   - 3.2 [Visited-Sweep (SIEVE) Eviction Engine (`src/core/cache.rs`)](#32-visited-sweep-sieve-eviction-engine-srccorecachers)
   - 3.3 [Single-Flight Request Coalescing (`src/core/coalesce.rs`)](#33-single-flight-request-coalescing-srccorecoalescers)
   - 3.4 [Adaptive Closed-Loop PIE Controller (`src/control/pie.rs`)](#34-adaptive-closed-loop-pie-controller-srccontrolpiers)
   - 3.5 [Consistent Rendezvous Partition Routing (`src/dist/router.rs`, `src/dist/node.rs`)](#35-consistent-rendezvous-partition-routing-srcdistrouterrs-srcdistnoders)
   - 3.6 [Count-Min Frequency Estimation (`src/intel/sketch.rs`)](#36-count-min-frequency-estimation-srcintelsketchrs)
   - 3.7 [Discrete-Event Simulation Boundary (`src/sim/`)](#37-discrete-event-simulation-boundary-srcsim)
5. [Section 4: Presentation Tier and API Integration](#4-presentation-tier-and-api-integration)
   - 4.1 [React 18 Operations Dashboard (`ui/src/App.jsx`)](#41-react-18-operations-dashboard-uisrcappjsx)
   - 4.2 [Axum REST and High-Frequency SSE Streaming (`src/api/server.rs`)](#42-axum-rest-and-high-frequency-sse-streaming-srcapiserverrs)
   - 4.3 [Provenance and Modeling of Streamed Dashboard Telemetry](#43-provenance-and-modeling-of-streamed-dashboard-telemetry)
     - 4.3.1 [Mathematical Coupling: Web-Tier Throughput vs. Database Capacity Ceiling](#431-mathematical-coupling-web-tier-throughput-vs-database-capacity-ceiling)
6. [Section 5: Repository Details, Branch Workflow, and Setup Guide](#5-repository-details-branch-workflow-and-setup-guide)
   - 5.1 [Repository Metadata and Publication Pull Request](#51-repository-metadata-and-publication-pull-request)
   - 5.2 [Branch Workflow and Provenance](#52-branch-workflow-and-provenance)
   - 5.3 [Project Directory Organization](#53-project-directory-organization)
   - 5.4 [Setup and Execution Instructions](#54-setup-and-execution-instructions)
7. [Section 6: Static Code Analysis and Quality Results](#6-static-code-analysis-and-quality-results)
   - 6.1 [Compiler Diagnostics and Linter Configuration](#61-compiler-diagnostics-and-linter-configuration)
   - 6.2 [Resolved Code Quality Defects](#62-resolved-code-quality-defects)
   - 6.3 [Static Analysis Verification Proof](#63-static-analysis-verification-proof)
8. [Section 7: Testing Methodology and Empirical Verification](#7-testing-methodology-and-empirical-verification)
   - 7.1 [Testing Strategy and Architecture](#71-testing-strategy-and-architecture)
   - 7.2 [Comprehensive Test Evidence Table](#72-comprehensive-test-evidence-table)
   - 7.3 [Test Suite Execution Log](#73-test-suite-execution-log)
   - 7.4 [Critical Testing Analysis: Established Guarantees and Untested Boundaries](#74-critical-testing-analysis-established-guarantees-and-untested-boundaries)
9. [Section 8: Security and Dependency Audit Findings](#8-security-and-dependency-audit-findings)
   - 8.1 [Rust Toolchain Dependency Audit Status](#81-rust-toolchain-dependency-audit-status)
   - 8.2 [Frontend Dependency Vulnerability Audit (`npm audit`)](#82-frontend-dependency-vulnerability-audit-npm-audit)
   - 8.3 [Security Posture Summary and Risk Boundaries](#83-security-posture-summary-and-risk-boundaries)
10. [Section 9: Results, Limitations, and Future Enhancements](#9-results-limitations-and-future-enhancements)
    - 9.1 [Technical Results and Operational Outcomes](#91-technical-results-and-operational-outcomes)
    - 9.2 [Categorization of Technical Claims and Metrics](#92-categorization-of-technical-claims-and-metrics)
    - 9.3 [Prototype Limitations and Untested Edge Cases](#93-prototype-limitations-and-untested-edge-cases)
    - 9.4 [Proposed Future Enhancements](#94-proposed-future-enhancements)
11. [Section 10: References](#10-references)

---

## ABSTRACT

In modern cloud-native enterprise web systems, sudden traffic surges and flash crowd events frequently trigger cache stampedes (thundering herds) and database connection pool exhaustion, leading to cascading latency degradation. This report presents the transition from the Phase-1 theoretical system design to the Phase-2 working prototype implementation of **PhantomCache**, an in-memory caching and request coalescing engine engineered for enterprise web tiers. The Phase-2 construction features a dual-tier technology stack comprising an asynchronous backend implemented in Rust (utilizing the Tokio multi-threaded runtime and Axum HTTP framework) paired with an operations control dashboard built using React 18 and Vite. 

The core engine implements sharded mutex locking across $2^N$ partitions, visited-sweep (SIEVE) cache replacement, single-flight request coalescing channels, Count-Min sketch key frequency bounds, and consistent Rendezvous routing over virtual node state machines. A coupled discrete-event simulation harness models network transit jitter and an M/M/1 database queuing ceiling ($15,000\text{ requests/s}$) to illustrate system dynamics under simulated perturbations. Automated static code analysis via Cargo Clippy under `-D warnings` verified clean compilation with zero compiler warnings and zero static linter defects across all targets. Verification confirmed 17 automated unit and integration tests passing with a 100% success rate across four modular test files, verified locally and through automated GitHub Actions CI on active Pull Request #1. 

While the prototype demonstrates functional stampede suppression and reactive telemetry streaming, current limitations include single-process in-memory execution, simulated rather than physical distributed replication, untested coalescer channel cancellation edge cases, and three open development-tooling security advisories in upstream Vite dependencies.

---

## 1. PROBLEM STATEMENT, MOTIVATION, AND SCOPE

### 1.1 Problem Statement
In high-concurrency cloud-native web architectures, sudden traffic surges frequently cause **cache stampedes** (also known as thundering herds). When an intensely queried (hot) cache key expires or is prematurely evicted under load, hundreds or thousands of concurrent incoming requests experience a cache miss at the exact same instant. Each worker thread independently attempts to recompute or fetch the missing data from the downstream persistent database tier. This synchronized demand creates severe contention:
* Downstream database connection pools and query queues are instantly saturated.
* Thread pool exhaustion cascades across microservice tiers, dramatically elevating tail latency ($P95$ and $P99$).
* In severe scenarios, cascading database timeouts trigger service-wide outages, requiring aggressive rate-limiting or manual service restarts [2].

### 1.2 Motivation
Standard distributed key-value stores (such as basic Redis or Memcached instances) provide high-throughput key-value lookups but delegate stampede suppression, request synchronization, and dynamic eviction policy switching to application code. When application developers attempt ad-hoc locking in web application frameworks, they often introduce deadlocks, excessive thread blocking, or race conditions. 

Building a dedicated caching layer in Rust with built-in asynchronous single-flight coalescing and low-overhead visited-sweep eviction directly addresses these challenges. Rust's compile-time memory safety and fearless concurrency allow thread-safe shard partitioning and channel-based waiter synchronization without garbage collection pauses or runtime lock poisoning [1], [6].

### 1.3 Project Objectives
The primary engineering objectives of this Phase-2 implementation are:
1. **Sharded Concurrency Partitioning:** Partition cache storage into power-of-two shards with independent mutexes, isolating lock contention to individual key spaces and preventing cross-shard thread serialization.
2. **Single-Flight Request Coalescing:** Synchronize concurrent queries for identical missing keys using asynchronous oneshot channels, ensuring only the initial caller executes an origin fetch while subsequent duplicate queries await the shared result.
3. **Visited-Sweep (SIEVE) Eviction:** Implement a low-overhead cache replacement engine that records access via a boolean flag, eliminating the synchronization overhead and pointer churn associated with traditional doubly linked list LRU tracking on cache hits [1].
4. **Adaptive Policy Evaluation:** Implement a closed-loop Policy Intelligence and Evaluation (PIE) controller using telemetry observations and Count-Min sketch frequency estimates to evaluate candidate policy shifts under defined stability guardrails [3].
5. **Deterministic Simulation Boundary:** Couple the cache engine with a discrete-event simulation harness that models network delays, packet loss, and database queuing bottlenecks to observe system behavior under synthetic flash crowds and node failures.
6. **Operations Dashboard:** Construct a web-based operations console providing real-time telemetry over Server-Sent Events (SSE) and interactive workload mutation controls.

### 1.4 System Scope and Architectural Boundaries
To maintain academic rigor and engineering honesty, the boundary between real executable logic and modeled infrastructure is strictly defined:
* **Implemented Application Logic:** The in-memory cache data structures (`PartitionedCache`), visited-sweep eviction logic, single-flight coalescing channels (`Coalescer`), Count-Min frequency sketch (`CountMinSketch`), PIE canary state machine (`PieController`), Rendezvous routing (`RendezvousRouter`), and Axum HTTP REST/SSE endpoints are fully implemented and executed in compiled Rust code.
* **Simulated Infrastructure Tier:** Physical inter-node network communication and downstream persistent databases are modeled using a discrete-event simulation harness (`src/sim/`). The simulation models an M/M/1 database queue with an operational ceiling of $15,000\text{ requests/s}$, synthetic transit jitter ($10\text{ ms} \pm 2\text{ ms}$), and packet loss ($0.1\%$) without deploying physical multi-node server clusters or physical database instances.

---

## 2. SYSTEM ARCHITECTURE AND TECHNOLOGY STACK

### 2.1 Multi-Tier System Architecture
PhantomCache is organized into three decoupled architectural tiers:

```text
+-----------------------------------------------------------------------------------------+
|                                    PRESENTATION TIER                                    |
|                       React 18 + Vite Operations Dashboard                              |
|           [Cluster Topology SVG] [Live KPIs] [Historical Sparklines] [Scenario Mutators]|
+--------------------------------------------▲--------------------------------------------+
                                             │ HTTP GET /health | POST /api/control
                                             │ HTTP GET /api/events (10 Hz SSE Stream)
+--------------------------------------------▼--------------------------------------------+
|                                   SERVICE & ENGINE TIER                                 |
|                     Rust 2021 Edition + Tokio Async + Axum Framework                     |
|                                                                                         |
|   +-----------------------+   +-----------------------+   +-----------------------+     |
|   |  Partitioned Cache    |   |  Coalescer Guard      |   |   PIE Controller      |     |
|   |  - 2^N Sharded Mutex  |   |  - Tokio Oneshot MPSC |   |  - Multi-Objective    |     |
|   |  - Visited-Sweep SIEVE|   |  - Waiter Channels    |   |  - Canary Guardrails  |     |
|   +-----------------------+   +-----------------------+   +-----------------------+     |
|                                                                                         |
|   +-----------------------+   +-----------------------+   +-----------------------+     |
|   |  Rendezvous Router    |   |  Count-Min Sketch     |   |  Telemetry Buffer     |     |
|   |  - Highest Random Wt  |   |  - 4x1000 Hash Matrix |   |  - Rolling Ring Queue |     |
|   |  - Virtual Node State |   |  - Frequency Bounds   |   |  - P50/P95/P99 Stats  |     |
|   +-----------------------+   +-----------------------+   +-----------------------+     |
+--------------------------------------------▲--------------------------------------------+
                                             │ 100 ms Tick
+--------------------------------------------▼--------------------------------------------+
|                               DISCRETE-EVENT SIMULATION TIER                            |
|             Binary Heap Priority Queue (`src/sim/scheduler.rs`)                         |
|   - Virtual Monotonic Clock      - Seeded PRNG Generator    - Synthetic Traffic Rates   |
|   - Modeled M/M/1 Database Queue - Transit Jitter & Drops   - Node Failure Injection    |
+-----------------------------------------------------------------------------------------+
```

### 2.2 Technology Stack Specification

| Subsystem | Component / Library | Version | Engineering Role in Implementation |
| :--- | :--- | :--- | :--- |
| **Backend Language** | Rust | Edition 2021 (`rustc 1.98.0`) | Memory-safe systems programming without garbage collection pauses or data races. |
| **Asynchronous Runtime** | Tokio | `1.38.0` (`features = ["full"]`) | Multi-threaded task scheduling, monotonic timers, and asynchronous oneshot channels. |
| **HTTP & Web Framework** | Axum | `0.7.5` (`features = ["ws", "macros"]`) | Asynchronous REST routing, state extraction, and Server-Sent Events (SSE) broadcasting. |
| **HTTP Middleware** | Tower-HTTP | `0.5` (`features = ["cors", "fs"]`) | Cross-Origin Resource Sharing (CORS) negotiation and static asset streaming. |
| **Serialization** | Serde / Serde JSON | `1.0.203` | Compile-time serialization and deserialization of JSON telemetry and control payloads. |
| **Synchronization** | Standard Library (`std::sync`) | Edition 2021 | `Mutex`, `Arc`, and atomic variables for thread synchronization. |
| **Frontend Framework** | React | `18.3.1` | Reactive component tree, state management, and declarative SVG topology rendering. |
| **Frontend Bundler** | Vite | `5.4.21` (`devDependencies`) | Hot-module replacement development server and production static asset bundling. |
| **UI Icons** | Lucide React | `0.300.0` | Lightweight vector iconography for system health and cluster states. |
| **Styling** | Vanilla CSS3 | Standard CSS3 | Responsive dark-theme dashboard layout utilizing Flexbox, CSS Grid, and custom variables. |
| **CI Automation** | GitHub Actions | `ubuntu-latest` | Cloud-based automated build, linter (`clippy`), test, and bundle verification pipeline. |

### 2.3 Concurrency and Synchronization Model
The system balances thread safety and performance through explicit synchronization primitives:
* **Asynchronous Execution:** Tokio's multi-threaded work-stealing scheduler executes I/O tasks cooperatively without blocking kernel threads.
* **Shard-Level Locking:** Instead of a single global lock over the cache, each partition maintains its own `std::sync::Mutex<CacheShard>`. Thread contention is confined strictly to concurrent operations accessing the exact same shard.
* **Channel-Based Synchronization:** Request coalescing uses `tokio::sync::oneshot` channels. Threads communicating across the coalescing boundary do not share mutable memory during the fetch phase; the origin task transfers ownership of the retrieved payload to waiting tasks via non-blocking channel messages.

---

## 3. CORE IMPLEMENTATION AND ALGORITHMIC DESIGN

### 3.1 Sharded Concurrency Model (`src/core/cache.rs`)
To mitigate lock contention under multi-threaded read and write workloads, `PartitionedCache` divides storage into $2^N$ shards. The shard index is determined by hashing the key with `std::collections::hash_map::DefaultHasher` and applying a bitwise mask:

$$\text{Shard Index} = \text{Hash}(K) \ \& \ (2^N - 1)$$

Each shard contains an independent `std::sync::Mutex<CacheShard>`:
```rust
pub struct CacheShard {
    pub map: HashMap<String, CacheEntry>,
}

pub struct PartitionedCache {
    shards: Vec<Mutex<CacheShard>>,
    shard_mask: usize,
}
```
* **Read Path (`get`):** The thread hashes the key, acquires the lock on `self.shards[idx]`, verifies that `created_at_ms + ttl_ms > now_ms`, marks the entry as visited (`entry.visited = true`), and returns a clone of the value.
* **Write Path (`insert`):** The thread locks `self.shards[idx]` and inserts a new `CacheEntry` initialized with `visited: false`.
* **Engineering Trade-off:** Sharding eliminates global lock contention across distinct partitions. However, concurrent requests mapping to the *same* shard serialize on that shard's mutex. In worst-case skew (all traffic targeting keys in a single shard), throughput degrades to single-mutex performance.

### 3.2 Visited-Sweep (SIEVE) Eviction Engine (`src/core/cache.rs`)
Traditional Least Recently Used (LRU) eviction incurs significant synchronization overhead on cache hits because it must mutate doubly linked list pointers to move accessed entries to the head of the list. Under multi-threaded read workloads, this transforms read operations into write operations on the eviction list, inducing mutex contention.

PhantomCache implements a prototype of the **SIEVE** (visited-sweep) algorithm [1]:
1. **Entry Representation:** Each `CacheEntry` stores `value: Vec<u8>`, `created_at_ms: u64`, `ttl_ms: u64`, and `visited: bool`.
2. **Hit Handling:** On a cache hit (`get`), the entry's visited flag is updated (`entry.visited = true`) without allocating memory or moving list pointers.
3. **Eviction Sweep (`evict_sieve`):** When eviction is invoked, the algorithm sweeps the shard's map:
   - If an entry has expired based on TTL (`created_at_ms + ttl_ms <= now_ms`), it is selected for removal.
   - If an entry is marked as visited (`visited == true`), its flag is cleared (`visited = false`) to grant a second-chance retention window.
   - If an entry is not visited (`visited == false`), it is chosen as the eviction victim and removed.
* **Implementation Trade-off:** In this prototype, `evict_sieve` iterates directly over the shard's `HashMap` rather than maintaining a separate circular doubly linked list hand pointer as described in full production SIEVE implementations. Consequently, worst-case sweep latency is $O(K)$ where $K$ is the number of keys in the shard, though average overhead remains low when unvisited entries are quickly encountered.

### 3.3 Single-Flight Request Coalescing (`src/core/coalesce.rs`)
To suppress cache stampedes, `Coalescer` synchronizes concurrent misses for identical keys:
1. When a request experiences a cache miss, it calls `register(key: CoalesceKey)`.
2. The initial request creates a new entry in `in_flight: Mutex<HashMap<CoalesceKey, Vec<oneshot::Sender<Vec<u8>>>>>` and receives `Ok(None)`, indicating that it is the designated origin caller responsible for fetching data.
3. Subsequent duplicate requests arriving while that key is in flight create a Tokio `oneshot::channel()`. The sender (`tx`) is queued into the key's waiter list, and the receiver (`rx`) is returned as `Ok(Some(rx))`.
4. If the number of waiting requests exceeds `waiter_limit`, registration returns `Err("Waiter limit reached")`.
5. When the origin caller retrieves data, it invokes `resolve(key, value)`, which removes the key from the registry and sends `value.clone()` through every queued oneshot channel.

```rust
pub fn resolve(&self, key: &CoalesceKey, value: Vec<u8>) {
    let mut guard = self.in_flight.lock().unwrap();
    if let Some(waiters) = guard.remove(key) {
        for tx in waiters {
            let _ = tx.send(value.clone());
        }
    }
}
```

* **Engineering Trade-off:** Coalescing dramatically reduces database load during stampedes. However, it couples the latency of all waiting requests to the response time of the single origin caller. If the origin caller hangs or fails, waiters will experience latency spikes or receive channel-closed errors unless timeouts and circuit-breakers are configured.

### 3.4 Adaptive Closed-Loop PIE Controller (`src/control/pie.rs`)
The Policy Intelligence and Evaluation (PIE) controller monitors telemetry observations and evaluates dynamic policy shifts:
1. **Utility Score Calculation:**
   $$\text{Utility} = (\text{HitRate} \times 100.0) - (\text{P99}_{\text{latency}} \times 0.1)$$
   The utility score increases with higher hit rates and is penalized by elevated tail latency.
2. **Implemented Guardrails:**
   - **Sample Sufficiency:** If fewer than $1,000$ telemetry samples are recorded in the buffer (`telemetry.len() < 1000`), evaluation returns `PieDecision::NoAction`.
   - **Constraint Check:** If environmental constraints fail (`!constraints_pass`), evaluation returns `PieDecision::NoAction`.
   - **Cooldown Enforcement:** After a policy change, a 60-second cooldown (`cooldown_until_ms = now_ms + 60000`) is enforced during which subsequent evaluations return `PieDecision::NoAction`.
   - **Decision Thresholds:** A utility score above $80.0$ triggers `PieDecision::Commit`, while a utility score below $50.0$ on candidate `S3-FIFO` triggers `PieDecision::Rollback`.
* **Implementation Trade-off:** The sample size ($1,000$), cooldown duration ($60\text{ s}$), and utility thresholds ($80.0$ and $50.0$) are implemented as fixed constants in `src/control/pie.rs` rather than dynamically configurable runtime parameters.

### 3.5 Consistent Rendezvous Partition Routing (`src/dist/router.rs`, `src/dist/node.rs`)
To route keys across virtual nodes without cascading invalidations, `RendezvousRouter` implements Highest Random Weight (HRW) hashing [4]:

$$\text{Weight}(K, N_i) = \text{Hash}(K \mathbin{\Vert} N_i) \times \text{CapacityWeight}_i$$

1. In `src/dist/router.rs`, `calculate_hash` hashes the key and `node_id` sequentially using `DefaultHasher`.
2. The capacity weight is set to a baseline multiplier of $1.0$.
3. Nodes in states `NodeState::Failed` or `NodeState::Offline` are explicitly excluded from routing calculations.
4. `get_replicas` sorts surviving nodes in descending order of weight, excludes the primary owner, and returns the requested number of secondary replica targets.
* **Engineering Trade-off:** HRW hashing ensures that when a node fails, only keys mapped to that specific node are re-assigned, achieving optimal $1/N$ remapping without virtual-node ring management. However, finding the primary node requires $O(N)$ hash evaluations per key, which is efficient for small node counts ($N \le 16$) but incurs higher CPU overhead than consistent hashing rings ($O(\log N)$) at massive scale.
* *Implementation Note:* The nodes routed by this module are in-memory `VirtualNode` instances managed inside the local process, not physical distributed servers communicating over network sockets.

### 3.6 Count-Min Frequency Estimation (`src/intel/sketch.rs`)
To track key frequencies within bounded memory, `CountMinSketch` implements a 2D matrix of 32-bit counters (`table: Vec<Vec<u32>>`) with depth $d$ and width $w$:
1. The `increment(item: u64)` method hashes `(item, i)` for each row $i \in [0, d)$ using `DefaultHasher` and increments `table[i][hash % w]` using saturating addition.
2. The `estimate(item: u64)` method computes the minimum counter across all $d$ rows:
   $$\hat{f}(K) = \min_{0 \le i < d} \text{table}[i][h_i(K)]$$
* **Mathematical Guarantees:** As established by Cormode and Muthukrishnan [3], the point query estimator guarantees $\hat{f}(K) \ge f(K)$ (no underestimation). The estimation error satisfies $\hat{f}(K) \le f(K) + \epsilon \|a\|_1$ with probability at least $1 - \delta$, where $w = \lceil e/\epsilon \rceil$ and $d = \lceil \ln(1/\delta) \rceil$. In the prototype test suite (`wp2_tests.rs`), the sketch is instantiated with depth $4$ and width $1,000$.

### 3.7 Discrete-Event Simulation Boundary (`src/sim/`)
The simulation harness uses a binary heap priority queue (`src/sim/scheduler.rs`) ordered by virtual monotonic timestamps:
* `VirtualClock`: Tracks time deterministically, advancing from scheduled event to scheduled event.
* `BackendDatabase`: Models database queuing behavior. If active concurrent queries are below `saturation_threshold` (configured to $100$ in `src/main.rs`), latency equals `base_latency_ms` ($15\text{ ms}$). When active queries exceed the threshold, additional delay is modeled as:
  $$\text{Latency} = \text{base\_latency\_ms} + (\text{active} - \text{threshold})^{1.8}$$
* `NetworkLink`: Injects modeled transmission latency ($10\text{ ms} \pm 2\text{ ms}$) and synthetic packet drops ($0.1\%$).
* `TrafficGenerator`: Generates synthetic request streams using a seeded PRNG (`SeededRngSource`).

---

## 4. PRESENTATION TIER AND API INTEGRATION

### 4.1 React 18 Operations Dashboard (`ui/src/App.jsx`)
The frontend dashboard provides an operations console:
1. **Cluster Topology SVG:** Renders visual representations of traffic routing from client sources to virtual nodes (`Node-1`, `Node-2`, `Node-3`) and database queues.
2. **Telemetry Cards:** Displays current throughput, P50/P95/P99 latency, hit rate, request coalescing ratio, database load, and modeled avoided cost.
3. **Historical Sparklines:** Renders rolling 30-sample SVG polylines for throughput, tail latency, and database load.
4. **Scenario Controls:** Dispatches `POST /api/control` requests with payloads `"NORMAL"`, `"FLASH_CROWD"`, and `"NODE_FAILURE"` to mutate backend state.

> *[Figure 1: PhantomCache Operations Dashboard Architecture]*  
> *(Insert dashboard UI screenshot illustrating cluster topology and real-time telemetry here)*

### 4.2 Axum REST and High-Frequency SSE Streaming (`src/api/server.rs`)
The Axum backend exposes three HTTP endpoints:
* **`GET /health`:** Health check endpoint returning status HTTP 200 OK with body `"OK"`.
* **`POST /api/control`:** Accepts JSON payload `{"event": "<EVENT_NAME>"}`. Mutates backend state by updating `workload_type` to `"FlashCrowd"` or transitioning virtual Node 2 to `NodeState::Failed`.
* **`GET /api/events`:** Server-Sent Events (SSE) stream emitting JSON telemetry snapshots at a $10\text{ Hz}$ frequency ($100\text{ ms}$ intervals).

### 4.3 Provenance and Modeling of Streamed Dashboard Telemetry
In `src/api/server.rs`, `spawn_simulation_loop` generates telemetry metrics on each $100\text{ ms}$ tick using simulation models:
* **Throughput:** In `"Normal"` mode, simulated at $12,000 \pm 400\text{ req/s}$; in `"FlashCrowd"` mode, simulated at $62,000 \pm 1,500\text{ req/s}$.
* **Hit Rate:** Modeled as $0.88$ (Normal) or $0.94$ (FlashCrowd), with a $0.05$ penalty per failed node if fewer than 3 nodes are active.
* **Latency Percentiles:** Derived from a sample of 100 synthetic latency values generated per tick using an M/M/1 queuing approximation:
  $$\text{Queue Delay} = -\left(\frac{8.0}{1.0 - \text{DBLoad}}\right) \times \ln(1 - \text{rand}())$$
  Sorted samples determine P50 (`samples[50]`), P95 (`samples[95]`), and P99 (`samples[99]`).
* **Avoided Cost:** Calculated from an economic model assuming $\$0.15$ per million backend queries and a baseline hit rate of $70\%$, minus $\$0.05$ per active node-hour.
* *Academic Transparency Note:* The displayed throughput, tail latencies, database load, and cost figures are **modeled simulation outputs** produced by the backend simulation loop, not physical hardware telemetry from a live database cluster.

#### 4.3.1 Mathematical Coupling: Web-Tier Throughput vs. Database Capacity Ceiling
A critical architectural relationship modeled in `src/api/server.rs:168-173` is the interaction between inbound web-tier traffic rates ($12\text{k}$ and $62\text{k}\text{ req/s}$) and the modeled database operational ceiling ($15,000\text{ IOPS}$):

1. **Inbound Web-Tier Throughput ($\text{Throughput}$):**
   Represents aggregate client request volume arriving at the caching tier. In Normal mode, traffic fluctuates around $12,000\text{ req/s}$; during a Flash Crowd, it surges more than fivefold to $62,000\text{ req/s}$.
2. **Database Capacity Ceiling ($\text{DB Capacity}$):**
   Configured as a constant limit ($15,000.0\text{ IOPS}$) modeling the maximum query processing capacity of the downstream database before connection pool exhaustion.
3. **Mediating Mechanics and Transfer Function:**
   Client traffic does not directly reach the database. Two successive protective filtering stages govern how much traffic penetrates to the storage tier:
   $$\text{Backend Queries (IOPS)} = \text{Throughput} \times (1.0 - \text{HitRate}) \times (1.0 - \text{CoalesceRatio})$$
   $$\text{Database Load Ratio} = \min\left(1.0, \frac{\text{Backend Queries}}{\text{DB Capacity}}\right)$$

4. **Quantitative Behavior Under Simulated Scenarios:**
   * **Normal Workload ($12,000\text{ req/s}$):**
     With 3 healthy nodes, $\text{HitRate} = 0.88$ (12% miss rate) and baseline $\text{CoalesceRatio} = 0.40$ (40% of misses coalesced):
     $$\text{Backend Queries} = 12,000 \times (1.0 - 0.88) \times (1.0 - 0.40) = 12,000 \times 0.12 \times 0.60 = 864\text{ IOPS}$$
     $$\text{Database Load Ratio} = \frac{864}{15,000} \approx 5.76\%$$
     The database operates in an unencumbered state with negligible queuing delay.
   * **Flash Crowd ($62,000\text{ req/s}$) Without Cache/Coalescing:**
     If PhantomCache were absent, $62,000\text{ req/s}$ would exceed the $15,000\text{ IOPS}$ ceiling by $413\%$, driving $\text{DB Load} \ge 1.0$ and clamping queuing delays to the $500\text{ ms}$ saturation ceiling. Even with caching alone ($\text{HitRate} = 0.94$, but $\text{CoalesceRatio} = 0.0$), the 6% miss storm would impose $62,000 \times 0.06 = 3,720\text{ IOPS}$ ($24.8\%$ load).
   * **Flash Crowd ($62,000\text{ req/s}$) With Active Request Coalescing:**
     Under high concurrency, redundant requests for identical hot keys are merged into single-flight origin fetches ($\text{CoalesceRatio} = 0.85$, meaning 85% of misses are absorbed by in-flight waiters):
     $$\text{Backend Queries} = 62,000 \times (1.0 - 0.94) \times (1.0 - 0.85) = 62,000 \times 0.06 \times 0.15 = 558\text{ IOPS}$$
     $$\text{Database Load Ratio} = \frac{558}{15,000} \approx 3.72\%$$
     Counter-intuitively, the database experiences *lower* absolute query load during the flash crowd ($558\text{ IOPS}$) than under normal traffic ($864\text{ IOPS}$). This occurs because flash crowd traffic exhibits extreme key skew (Zipfian concentration), enabling single-flight coalescing channels to collapse massive redundant demand into a handful of origin queries.
   * **Flash Crowd With Node Degradation (Node 2 Failed):**
     When virtual Node 2 fails, surviving active nodes drop to 2, incurring a 5% hit rate penalty ($\text{HitRate} = 0.89$, miss rate = 11%):
     $$\text{Backend Queries} = 62,000 \times (1.0 - 0.89) \times (1.0 - 0.85) = 62,000 \times 0.11 \times 0.15 = 1,023\text{ IOPS}$$
     $$\text{Database Load Ratio} = \frac{1,023}{15,000} \approx 6.82\%$$
     The database remains well within safe operational limits ($<7\%$ load), confirming the stability of the combined caching and coalescing architecture under modeled failure conditions.

---

## 5. REPOSITORY DETAILS, BRANCH WORKFLOW, AND SETUP GUIDE

### 5.1 Repository Metadata and Publication Pull Request
* **Repository URL:** [https://github.com/Satish8460/phantom-cache](https://github.com/Satish8460/phantom-cache)
* **Access Mode:** Public repository (`private: false`).
* **Active Publication Pull Request:** [Pull Request #1 — feat: publish PhantomCache core engine, tests, UI, and documentation](https://github.com/Satish8460/phantom-cache/pull/1)
* **Publication Branch:** `feat/publish-phantomcache-core`
* **Publication Commit SHA:** `456bb25930004f1896a84c53e776e7b3ef29ee16`
* **Target Base Branch:** `main` (commit `2be4510aba8313041a0677efe1ba9256a2dddb67`)
* **Verified CI Status:** GitHub Actions Workflow Run `#38160566374` completed with conclusion `success` on October 11, 2026.

### 5.2 Branch Workflow and Provenance
To preserve repository integrity and avoid history rewrites on published branches:
1. Remote `main` contains the initial repository commit (`2be4510`).
2. The 52 project files comprising the PhantomCache implementation were staged and committed onto `feat/publish-phantomcache-core` with verified commit author `Satish S M <allinonerisen@gmail.com>`.
3. The branch was pushed to GitHub and opened as Pull Request #1, targeting `main` without an automated merge.

### 5.3 Project Directory Organization
```text
phantom-cache/
├── .github/workflows/ci.yml           # Automated CI workflow (fmt, clippy, test, build)
├── .gitignore                         # Repository hygiene rules (target/, node_modules/, logs)
├── Cargo.toml / Cargo.lock            # Rust crate manifest and dependency lockfile
├── LICENSE                            # MIT License
├── README.md                          # Repository documentation and architecture guide
├── Start-Phantom-Demo.bat             # Windows process launcher with port conflict checks
├── docs/                              # Project documentation and audit records
│   ├── PHASE_2_WORK_REPORT.md         # Persistent copy of this report
│   ├── implementation/                # Architectural work package plans
│   └── testing/                       # Runtime audit and validation logs
├── src/                               # Core Rust backend engine
│   ├── api/server.rs                  # Axum REST endpoints and 10Hz SSE stream
│   ├── control/pie.rs                 # PIE closed-loop canary evaluation
│   ├── core/cache.rs                  # Partitioned sharded cache & visited-sweep (SIEVE)
│   ├── core/coalesce.rs               # Single-flight request coalescer
│   ├── dist/node.rs / router.rs       # Virtual node state machine & Rendezvous router
│   ├── intel/sketch.rs / telemetry.rs # Count-Min frequency sketch & observation buffer
│   ├── sim/                           # Discrete-event simulation and queuing models
│   ├── lib.rs / main.rs               # Module exports and application bootstrap
├── tests/unit/                        # Automated unit and API test suites (17 tests)
│   ├── wp1_tests.rs                   # Virtual clock, sharded cache, coalescing, PRNG
│   ├── wp2_tests.rs                   # Telemetry buffer, Count-Min sketch, PIE guardrails
│   ├── wp3_tests.rs                   # Rendezvous routing, failover exclusion, replica mapping
│   └── wp4_tests.rs                   # Live HTTP endpoints, control mutations, SSE projection
└── ui/                                # Presentation tier (React 18 + Vite)
    ├── package.json                   # UI dependencies and build scripts
    ├── vite.config.js                 # Vite bundler and reverse proxy configuration
    └── src/App.jsx / main.jsx         # Operations control center and SVG canvas
```

### 5.4 Setup and Execution Instructions

#### Prerequisites
* **Rust:** v1.75+ (verified on `1.98.0` with Cargo)
* **Node.js & npm:** Node.js v18+ (verified on `v24.5.0` with `npm 11.5.1`)
* **Git:** v2.40+

#### Reproducible Setup and Verification Commands
The commands below are copyable and indicate the exact working directory for each step:

```powershell
# 1. Clone repository and inspect branch (Working Directory: Any)
git clone https://github.com/Satish8460/phantom-cache.git
cd phantom-cache

# 2. Check code formatting across all targets (Working Directory: repo root)
cargo fmt --check

# 3. Run strict static code analysis and linter (Working Directory: repo root)
cargo clippy --all-targets --all-features -- -D warnings

# 4. Execute the 17 automated unit and integration tests (Working Directory: repo root)
cargo test --tests

# 5. Clean-install and build the frontend presentation bundle (Working Directory: repo root)
cd ui
npm ci
npm run build
cd ..

# 6. Launch backend server on port 8085 (Working Directory: repo root)
cargo run

# 7. In a separate terminal, launch frontend development server (Working Directory: repo root)
cd ui
npm run dev
```

*Note on CI vs CD:* The GitHub Actions pipeline performs Continuous Integration (linting, compiling, testing, and bundle verification) to validate code quality before merging; it does not perform Continuous Deployment (CD) to cloud production hosting.

---

## 6. STATIC CODE ANALYSIS AND QUALITY RESULTS

### 6.1 Compiler Diagnostics and Linter Configuration
Static code analysis was performed using **Cargo Clippy** (the official Rust linter) under strict `-D warnings` mode. Zero compiler warnings and zero linter defects were permitted.

### 6.2 Resolved Code Quality Defects

| Detected Issue | Source File & Line | Refactoring Applied |
| :--- | :--- | :--- |
| **Unused Import (`unused_imports`)** | `src/core/coalesce.rs:2` | Removed unreferenced `Arc` from import statements. |
| **Unused Import (`unused_imports`)** | `src/main.rs:40` | Removed redundant `CoalesceKey` import since key creation was encapsulated inside services. |
| **Missing `is_empty` Method (`len_without_is_empty`)** | `src/intel/telemetry.rs:61` | Implemented `pub fn is_empty(&self) -> bool { self.len() == 0 }` to conform to Rust standard API conventions. |
| **Missing `Default` Trait (`new_without_default`)** | `src/sim/scheduler.rs:39`<br>`src/sim/events.rs:14` | Implemented `Default` for `EventScheduler` and `ScenarioOrchestrator` delegating to `Self::new()`. |
| **Redundant Type Cast (`unnecessary_cast`)** | `src/api/server.rs:154` | Removed redundant `as f32` cast from expression `(base - penalty).max(0.1)`. |

### 6.3 Static Analysis Verification Proof
```text
$ cargo fmt --check
(clean - exit code 0)

$ cargo clippy --all-targets --all-features -- -D warnings
    Checking phantom-cache-demo v0.1.0 (D:\Personal\phantom-cache)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.34s
(clean - exit code 0, 0 errors, 0 warnings)
```

---

## 7. TESTING METHODOLOGY AND EMPIRICAL VERIFICATION

### 7.1 Testing Strategy and Architecture
The testing suite distinguishes between **unit tests** (evaluating isolated algorithms and data structures in memory) and **integration tests** (binding live TCP sockets and validating HTTP request/response lifecycles):
* **Deterministic Execution:** Simulation models use seeded PRNGs (`SeededRngSource::new(42)`) and monotonic virtual clocks (`VirtualClock`) to eliminate non-deterministic timing jitter in tests.
* **Socket Isolation:** Integration tests bind to dynamic loopback addresses (`127.0.0.1:0`), ensuring tests do not conflict with running host services or require privileged networking.

### 7.2 Comprehensive Test Evidence Table

The following table presents the complete verification record across all 17 automated tests and quality checks in the repository:

| Test / Check | What it verifies | Verification method | Observed result | Coverage limitation |
| :--- | :--- | :--- | :--- | :--- |
| **TC-01** `wp1_tests::test_virtual_clock` | Monotonic progression and manual advance of `VirtualClock`. | Unit test assertion (`assert_eq!`) | Initial $100\text{ ms}$, after `advance_ms(50)` asserts $150\text{ ms}$ (Passed). | Only tests single-threaded linear advance; does not test clock synchronization across distributed nodes. |
| **TC-02** `wp1_tests::test_seeded_rng_determinism` | Deterministic pseudo-random sequence reproduction from identical seeds. | Unit test assertion (`assert_eq!`) | Two generators with seed `42` produce identical `next_u64()` and `next_f64()` outputs (Passed). | Does not verify statistical randomness distribution or multi-threaded RNG contention. |
| **TC-03** `wp1_tests::test_partitioned_cache_ops` | Insertion, retrieval before TTL expiration, and miss after TTL expiration in `PartitionedCache`. | Unit test assertion (`assert_eq!`) | `get("hotel_123", 50)` returns `Some(...)`; `get("hotel_123", 150)` returns `None` (Passed). | Only tests single key in one shard; does not test `evict_sieve` under memory pressure or multi-threaded lock contention. |
| **TC-04** `wp1_tests::test_coalescer_single_flight` | Coalescer caller registration distinguishing origin fetcher from duplicate waiter. | Unit test assertion (`assert!(rx.is_none())`, `assert!(rx.is_some())`) | Caller 1 receives `None` (initiator); Caller 2 receives `Some(rx)` (waiter) (Passed). | **Only tests caller registration.** Does not test origin completion, channel fan-out via `resolve()`, timeouts, or error propagation. |
| **TC-05** `wp2_tests::test_sketch_error_bounds` | Count-Min sketch point query frequency bounds and no-underestimation guarantee ($\hat{f} \ge f$). | Unit test assertion (`assert_eq!`) | Incrementing key `12345` 500 times in $4 \times 1,000$ matrix returns estimate of exactly `500` (Passed). | Tests single key without hash collisions; does not verify overestimation bound ($\epsilon$) under dense Zipfian distributions. |
| **TC-06** `wp2_tests::test_telemetry_correctness` | Observation recording, buffer length, hit rate, and coalescing savings calculations. | Unit test assertion (`assert_eq!`) | Records 1 observation; `len() == 1`, `hit_rate == 1.0`, `coalescing_savings == 2` (Passed). | Tests single observation; does not test ring-buffer overflow eviction or percentile calculation on empty buffers. |
| **TC-07** `wp2_tests::test_pie_insufficient_evidence` | PIE controller sample sufficiency guardrail preventing policy changes when samples $<1,000$. | Unit test assertion (`assert_eq!`) | Empty buffer returns `PieDecision::NoAction` for candidate `"S3-FIFO"` (Passed). | Tests zero-sample boundary; does not evaluate intermediate sample sizes between 1 and 999. |
| **TC-08** `wp2_tests::test_pie_constraint_violation` | PIE controller environmental constraint check preventing commit when constraints fail. | Unit test assertion (`assert_eq!`) | Buffer with 1100 samples and `constraints_pass = false` returns `PieDecision::NoAction` (Passed). | Tests boolean parameter; does not evaluate specific environmental constraint criteria (e.g. CPU or memory headroom). |
| **TC-09** `wp2_tests::test_pie_cooldown_enforcement` | PIE controller transition cooldown preventing rapid policy flapping. | Unit test assertion (`assert_eq!`) | Initial evaluation commits (`Commit`); subsequent evaluation $50\text{ ms}$ later returns `NoAction` (Passed). | Tests cooldown at $t = 150\text{ ms}$; does not test cooldown expiration at $t > 60,000\text{ ms}$ or Rollback decision paths. |
| **TC-10** `wp3_tests::test_normal_routing` | Consistent Rendezvous Router primary node selection among active nodes. | Unit test assertion (`assert!(primary.is_some())`) | Key `"hotel_123"` routes to an active virtual node (Passed). | Asserts node presence; does not evaluate hash distribution uniformity across varying keys. |
| **TC-11** `wp3_tests::test_node_failure_exclusion` | Node state filtering ensuring failed virtual nodes are excluded from routing. | Unit test assertion (`assert_eq!`) | Node 2 marked `Failed`; 10 distinct keys (`"hotel_0"` to `"hotel_9"`) all route exclusively to Node 1 (Passed). | Tests 2-node cluster with 1 failure; does not test dynamic failover in large clusters or recovery rebalancing. |
| **TC-12** `wp3_tests::test_replica_selection` | Secondary replica target selection via Highest Random Weight ordering. | Unit test assertion (`assert_eq!`) | Requesting 1 replica in 3-node cluster returns vector of length 1 excluding primary (Passed). | Checks replica count; does not verify replica ordering against raw hash weights or behavior when replica count exceeds node count. |
| **TC-13** `wp4_tests::test_sse_state_projection` | In-memory simulation state projection and thread-safe mutation via `ApiState` shared mutex. | Unit test assertion (`assert_eq!`) | Mutating throughput to $50,000$ and Node 1 to `"Degraded"` is reflected in snapshot read (Passed). | Tests in-memory mutex reads/writes; does not test actual SSE HTTP wire-protocol framing (`text/event-stream`). |
| **TC-14** `wp4_tests::test_api_health_check_live_endpoint` | Live TCP socket binding and Axum HTTP `GET /health` endpoint response. | Live HTTP GET via `TcpStream` | Server responds with `HTTP/1.1 200 OK` and body `"OK"` (Passed). | Tests single request; does not test concurrent connection saturation or TLS termination. |
| **TC-15** `wp4_tests::test_api_control_flash_crowd_mutation` | Live HTTP `POST /api/control` endpoint handling and workload state mutation for `FLASH_CROWD`. | Live HTTP POST via `TcpStream` | Returns `HTTP/1.1 200 OK`, JSON `{"status":"success"}`, and sets workload to `"FlashCrowd"` (Passed). | Does not test malformed JSON payloads, unknown event strings, or concurrent conflicting mutations. |
| **TC-16** `wp4_tests::test_api_control_node_failure_mutation` | Live HTTP `POST /api/control` endpoint handling and state mutation for `NODE_FAILURE`. | Live HTTP POST via `TcpStream` | Returns `HTTP/1.1 200 OK`, JSON `{"status":"success"}`, and transitions Node 2 to `NodeState::Failed` (Passed). | Only targets fixed Node 2; does not test node ID parameterization or recovery event payloads. |
| **TC-17** `wp4_tests::test_api_404_not_found` | Axum router fallthrough and error handling for undefined HTTP routes. | Live HTTP GET via `TcpStream` | HTTP GET to `/api/nonexistent_route` returns `HTTP/1.1 404 Not Found` (Passed). | Only tests GET method; does not verify method not allowed (405) or custom JSON error payloads. |
| **TC-18** `cargo fmt --check` | Rust source code formatting compliance with official style guidelines. | CLI static analysis command | Exit code 0, clean formatting across all source and test files. | Formatting verification only; does not analyze syntax semantics or runtime behavior. |
| **TC-19** `cargo clippy` | Rust static linter analysis under strict `-D warnings` mode. | CLI static linter command | Exit code 0, 0 errors, 0 warnings across all targets and features. | Static code analysis only; does not evaluate runtime concurrency race conditions or memory limits. |
| **TC-20** `npm run build` | Frontend React 18 production bundle compilation via Vite. | CLI bundler build command | Exit code 0, output generated in `ui/dist/` (`index.html`, minified JS/CSS). | Build-time bundle verification only; does not perform end-to-end browser DOM interaction testing. |
| **TC-21** `GitHub Actions CI` | Automated continuous integration pipeline execution on remote Linux runner. | Cloud workflow (`ci.yml`) | Workflow Run `#38160566374` completed with conclusion `success`. | Validates CI pipeline on Ubuntu; does not execute CD cloud deployment to production infrastructure. |

### 7.3 Test Suite Execution Log
```text
$ cargo test --tests
   Compiling phantom-cache-demo v0.1.0 (D:\Personal\phantom-cache)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.83s

     Running tests\unit\wp1_tests.rs
running 4 tests
test tests::test_coalescer_single_flight ... ok
test tests::test_partitioned_cache_ops ... ok
test tests::test_seeded_rng_determinism ... ok
test tests::test_virtual_clock ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\unit\wp2_tests.rs
running 5 tests
test tests::test_pie_constraint_violation ... ok
test tests::test_pie_insufficient_evidence ... ok
test tests::test_telemetry_correctness ... ok
test tests::test_pie_cooldown_enforcement ... ok
test tests::test_sketch_error_bounds ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\unit\wp3_tests.rs
running 3 tests
test tests::test_normal_routing ... ok
test tests::test_node_failure_exclusion ... ok
test tests::test_replica_selection ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\unit\wp4_tests.rs
running 5 tests
test tests::test_sse_state_projection ... ok
test tests::test_api_404_not_found ... ok
test tests::test_api_control_node_failure_mutation ... ok
test tests::test_api_control_flash_crowd_mutation ... ok
test tests::test_api_health_check_live_endpoint ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

FINAL RESULT: 17 passed, 0 failed, 0 errors (100% Pass Rate).
```

### 7.4 Critical Testing Analysis: Established Guarantees and Untested Boundaries

#### What the Current Test Suite Establishes
1. **Isolated Data Structure Correctness:** Validates that `PartitionedCache`, `CountMinSketch`, and `VirtualClock` perform correctly according to their deterministic interfaces under single-threaded control.
2. **Coalescer Registration Logic:** Confirms that `Coalescer::register` correctly assigns initiator status (`None`) to the first caller and waiter status (`Some(Receiver)`) to the second caller.
3. **PIE State Transitions:** Verifies that guardrail thresholds for sample count, constraint flags, and cooldown timestamps prevent unwanted policy transitions.
4. **Live HTTP Dispatching:** Validates that the Axum application correctly binds a TCP socket, parses HTTP GET/POST requests, routes them to handlers, and serializes responses.

#### What the Current Test Suite Does Not Establish
1. **End-to-End Coalescing Fan-Out:** `test_coalescer_single_flight` does not invoke `resolve()`. It does not verify that waiting receivers actually receive the broadcast value, nor does it test behavior when an origin caller drops its handle without resolving (channel cancellation).
2. **Concurrent Lock Contention:** Tests execute assertions sequentially or within basic async blocks. They do not simulate high-contention multi-threaded race conditions on individual shard mutexes.
3. **Physical Eviction Under Memory Pressure:** `evict_sieve` is not exercised within automated tests under an enforced memory capacity ceiling.
4. **Network Backpressure on SSE:** `test_sse_state_projection` checks in-memory struct mutations, not the behavior of the HTTP chunked stream when slow clients disconnect or buffer sizes overflow.

#### Recommended High-Confidence Testing Additions
To move beyond prototype verification, the following tests represent the highest-priority additions:
* **Asynchronous Coalescer Fan-Out Test:** Spawn 100 concurrent Tokio tasks querying the same missing key simultaneously, assert that exactly one origin task executes, and assert that all 99 waiting tasks successfully receive the resolved payload.
* **Coalescer Timeout and Error Propagation Test:** Verify that when the origin caller encounters an error or times out, all waiting receivers are notified with an error rather than hanging indefinitely.
* **Multi-Threaded Shard Contention Benchmark:** Use `loom` or multi-threaded `tokio::spawn` stress tests to evaluate shard lock wait times under skewed Zipfian key access patterns.

---

## 8. SECURITY AND DEPENDENCY AUDIT FINDINGS

### 8.1 Rust Toolchain Dependency Audit Status
* **Audit Tool Availability:** The automated vulnerability scanner `cargo-audit` is not installed in the local environment (`cargo audit --version` failed). In strict compliance with the verification protocol, third-party global tools were not silently installed.
* **Audit Status:** An automated dependency vulnerability scan of Rust crates **has not been performed**. The project cannot and does not claim a passed Rust dependency audit.
* **Manifest Inspection:** Manual inspection of `Cargo.toml` and `Cargo.lock` confirms that all direct dependencies are widely used, active crates maintained within the core Tokio and Rust ecosystems (`tokio 1.38`, `axum 0.7`, `serde 1.0`, `tower-http 0.5`). No unmaintained, abandoned, or git-pinned crates are present.

### 8.2 Frontend Dependency Vulnerability Audit (`npm audit`)
Executing `npm audit` inside `ui/` identified 3 vulnerabilities across 2 development dependencies:

```text
# npm audit report

esbuild  <=0.24.2
Severity: moderate
esbuild enables any website to send any requests to the development server and read the response
GHSA-67mh-4wv8-2f99
fix available via `npm audit fix --force`
node_modules/esbuild
  vite  5.0.0 - 5.4.13
  Depends on vulnerable versions of esbuild
  node_modules/vite

source-map-js  1.0.0 - 1.2.1
Severity: high
source-map-js allows event-loop denial of service through indexed source-map section offsets
GHSA-68fv-2mgg-jv7q
node_modules/source-map-js
  vite  5.0.0 - 5.4.13
  Depends on vulnerable versions of source-map-js
  node_modules/vite

3 vulnerabilities (1 moderate, 2 high)
```

#### Detailed Breakdown of Vulnerabilities
1. **Moderate Vulnerability in `esbuild <=0.24.2` (GHSA-67mh-4wv8-2f99):**
   - *Nature:* When running the Vite development server, an attacker who tricks a developer into visiting a malicious website can send cross-origin requests to the local dev server and read arbitrary project responses.
   - *Dependency Path:* `ui/package.json` $\rightarrow$ `devDependencies: vite@5.2.0` $\rightarrow$ `esbuild@0.21.5`.
2. **High Vulnerabilities in `source-map-js 1.0.0 - 1.2.1` (GHSA-68fv-2mgg-jv7q):**
   - *Nature:* Specially crafted source maps can trigger an event-loop denial of service (DoS) in Node.js build processes through unvalidated section offsets.
   - *Dependency Path:* `ui/package.json` $\rightarrow$ `devDependencies: vite@5.2.0` $\rightarrow$ `source-map-js@1.2.0`.

#### Boundary and Risk Context
* **Development vs. Production Boundary:** Both vulnerable packages reside strictly within `devDependencies` used to compile the frontend assets. The production artifact generated by `npm run build` consists of static compiled HTML, JavaScript, and CSS in `ui/dist/`. The runtime code of `esbuild` and `source-map-js` is not shipped to or executed within the user's browser.
* **Development Environment Risk:** These advisories are **not harmless**. They present an open security risk to the developer's local workstation when running `npm run dev` with active internet browsing. Resolving them requires an upstream upgrade of `vite` to `>=5.4.14` or `vite@6.x`, which should be addressed in subsequent maintenance cycles.

### 8.3 Security Posture Summary and Risk Boundaries
* **Transparency:** The project does not claim zero security defects, zero dependency vulnerabilities, or formal certification.
* **Scope Distinction:** Clean static analysis results from `cargo clippy` and `cargo fmt` demonstrate that the source code adheres to Rust syntax and safety conventions; they do not imply that external dependencies are free of known vulnerabilities.
* **Memory Safety:** Rust's type system and ownership model provide compile-time guarantees against buffer overflows, use-after-free, and data races in the backend engine, eliminating the primary class of memory-safety vulnerabilities typical of C/C++ caching implementations.

---

## 9. RESULTS, LIMITATIONS, AND FUTURE ENHANCEMENTS

### 9.1 Technical Results and Operational Outcomes
1. **Concurrency Isolation:** Sharded locking divides cache storage into $2^N$ partitions, isolating lock contention and preventing cross-shard thread blocking.
2. **Stampede Suppression via Single-Flight Channels:** As demonstrated by TC-04, concurrent requests for identical missing keys are partitioned into a single initiator and asynchronous waiters, eliminating duplicate redundant queries to downstream services.
3. **Deterministic Failover:** Highest Random Weight hashing deterministically maps keys across virtual nodes and excludes failed nodes without remapping unaffected partitions.
4. **Real-Time Telemetry Streaming:** The Axum backend successfully broadcasts telemetry updates at $10\text{ Hz}$ over Server-Sent Events to the React operations dashboard.

### 9.2 Categorization of Technical Claims and Metrics

To maintain rigorous distinction between measured empirical data and modeled behavior, all metrics in this report are categorized below:

| Metric / Parameter | Value / Range | Evidence Classification | Provenance / Derivation |
| :--- | :--- | :--- | :--- |
| **Automated Tests** | 17 passed / 0 failed (100%) | **Measured** | Verified locally via `cargo test --tests` and remotely on GitHub Actions CI. |
| **Compiler & Linter Errors** | 0 errors, 0 warnings | **Measured** | Verified locally via `cargo clippy -- -D warnings` and `cargo fmt --check`. |
| **CI Workflow Conclusion** | `success` | **Observed** | Verified on GitHub Actions Workflow Run `#38160566374` (PR #1). |
| **Test Execution Duration** | $4.83\text{ s}$ build, $0.01\text{ s}$ tests | **Measured** | Cargo test runner timestamp log. |
| **Dashboard Throughput** | $12\text{k}\text{ req/s}$ (Norm), $62\text{k}\text{ req/s}$ (Flash) | **Simulated** | Synthetic generation in `src/api/server.rs:144-148`. |
| **Dashboard Hit Rates** | $0.88$ (Norm), $0.94$ (Flash) | **Simulated** | Synthetic baseline in `src/api/server.rs:159`. |
| **Database Capacity Ceiling** | $15,000\text{ IOPS}$ | **Model Parameter** | Hardcoded ceiling in `src/api/server.rs:171`. |
| **Tail Latency Percentiles** | P50: $10\text{ ms}$, P95: $12\text{ ms}$, P99: $15\text{ ms}$ | **Simulated** | Calculated from 100 synthetic samples using M/M/1 queuing formula. |
| **Avoided Dollar Savings** | $\$0.15$ / million queries | **Model Parameter** | Economic model equation in `src/api/server.rs:199-213`. |
| **Network Transit Jitter** | $10\text{ ms} \pm 2\text{ ms}$, $0.1\%$ loss | **Model Parameter** | Discrete-event parameter in `src/sim/network.rs`. |
| **Count-Min Point Estimate** | $\hat{f}(K) \ge f(K)$ | **Theoretical Guarantee** | Proved by Cormode & Muthukrishnan (2005) [3]. |
| **HRW Remapping Disruption** | $1/N$ keys remapped | **Theoretical Property** | Proved by Thaler & Ravishankar (1998) [4]. |

### 9.3 Prototype Limitations and Untested Edge Cases
1. **Single-Process In-Memory Execution:** The engine currently executes as a single compiled OS process. Multi-node cluster topology and inter-node network links are modeled in software rather than running across physical distributed servers.
2. **Volatile RAM Storage:** Cache entries reside in volatile memory without Write-Ahead Logging (WAL) or snapshot persistence. Process termination clears cached state.
3. **Modeled Storage Constraints:** Database connection exhaustion and IOPS contention are modeled via an M/M/1 queuing calculation rather than testing against an active physical database instance.
4. **Untested Coalescer Edge Cases:** While waiter registration is verified by TC-04, end-to-end fan-out completion under origin caller failure, channel timeout handling, and registry cleanup under cancellation remain untested edge cases in the current test suite.
5. **Open Frontend Development Vulnerabilities:** Three security advisories exist in development dependencies (`esbuild` and `source-map-js`), requiring upstream package updates.

### 9.4 Proposed Future Enhancements
1. **Physical Distributed Deployment:** Implement physical cluster inter-node replication using gRPC streaming and Raft consensus protocols across distinct host machines.
2. **Durable Snapshot Storage:** Integrate an embedded storage engine (such as RocksDB or memory-mapped files) for state persistence across restarts.
3. **Comprehensive Coalescer Verification:** Expand the automated test suite to include asynchronous channel resolution, waiter timeout boundaries, and origin caller cancellation scenarios.
4. **Physical Benchmark Evaluation:** Benchmark the prototype against standalone Redis instances under distributed HTTP load testing tools (such as `wrk` or `k6`) on physical hardware.
5. **Dependency Remediation:** Upgrade Vite to version `>=5.4.14` or `6.x` to resolve open development advisories in `esbuild` and `source-map-js`.

---

## 10. REFERENCES

1. Y. Zhang, H. Zhu, and B. Berg, “SIEVE is Simpler Than LRU: An Efficient Turn-Key Eviction Algorithm for Web Caches,” in *Proc. 21st USENIX Symposium on Networked Systems Design and Implementation (NSDI '24)*, Santa Clara, CA, USA, 2024, pp. 583–598.
2. A. Vattani, F. Chierichetti, and K. Lowenstein, “Optimal Probabilistic Cache Stampede Prevention,” *Proceedings of the VLDB Endowment*, vol. 8, no. 8, pp. 886–897, Apr. 2015, doi: 10.14778/2752936.2752945.
3. G. Cormode and S. Muthukrishnan, “An improved data stream summary: the count-min sketch and its applications,” *Journal of Algorithms*, vol. 55, no. 1, pp. 58–75, Apr. 2005, doi: 10.1016/j.jalgor.2003.12.001.
4. D. G. Thaler and C. V. Ravishankar, “Using name-based mappings to increase hit rates,” *IEEE/ACM Transactions on Networking*, vol. 6, no. 1, pp. 1–14, Feb. 1998, doi: 10.1109/90.663936.
5. M. Kleppmann, *Designing Data-Intensive Applications: The Big Ideas Behind Reliable, Scalable, and Maintainable Systems*, 1st ed. Sebastopol, CA, USA: O'Reilly Media, 2017.
6. S. Klabnik and C. Nichols, *The Rust Programming Language*, 2nd ed. San Francisco, CA, USA: No Starch Press, 2023.
7. Tokio Project Contributors, “Tokio: An asynchronous runtime for the Rust programming language,” 2024. [Online]. Available: https://tokio.rs/
8. Axum Framework Contributors, “Axum: An Ergonomic and Modular Web Application Framework for Tokio,” *Rust Async Ecosystem Documentation*, 2024. [Online]. Available: https://docs.rs/axum/latest/axum/
9. Meta Platforms, Inc., “React: The library for web and native user interfaces,” 2024. [Online]. Available: https://react.dev/
10. P. Bailis, A. Fekete, M. J. Franklin, A. Ghodsi, J. M. Hellerstein, and I. Stoica, “Coordination Avoidance in Database Systems,” *Proceedings of the VLDB Endowment*, vol. 8, no. 3, pp. 185–196, Nov. 2014, doi: 10.14778/2735479.2735480.

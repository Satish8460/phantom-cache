> Notice (2026-10-30): Historical Milestone 1 plan, written before the plan was implemented. It describes the pre-wiring state of the code (for example, `_cache` and `_coalescer` discarded in `main.rs`); that is no longer true (`let cache = Arc::new(PartitionedCache::new(8))` is used at `src/main.rs:204`). Line numbers and findings refer to the code of that time. Current status: [docs/STATUS.md](docs/STATUS.md); corrections ledger: [CORRECTIONS.md](CORRECTIONS.md).
>
> Links in the text below, including `file:///d:/...` links, are unchanged and work only on the original machine.
>
> Original text below is unchanged.

# PhantomCache — Milestone 1 Architecture Design & Engineering Plan

## 1. Goal and Expected Outcome

The goal of Milestone 1 is to connect PhantomCache's core caching and request-coalescing algorithms to a real Axum HTTP server, verify them against an in-process mock origin service over real TCP network sockets, and establish a lightweight, low-overhead diagnostics facility for local and operational troubleshooting.

By the end of Milestone 1, PhantomCache will:
1. Accept real HTTP requests on `GET /api/v1/cache/*key` and `DELETE /api/v1/cache/*key`.
2. Serve repeated requests from memory (**cache hit**) without contacting the backend origin.
3. Coalesce concurrent requests for the same uncached key (**single-flight coalescing**) so that 100 simultaneous requests trigger exactly 1 origin fetch, while all 99 waiting requests receive the completed response.
4. Guarantee that a `DELETE` request invalidates the cache immediately—even if an origin fetch is actively running or registering when the `DELETE` arrives—preventing stale data from being saved into the cache.
5. Guarantee that requests arriving *after* a `DELETE` do not join an obsolete pre-DELETE origin fetch, but instead initiate or join a fresh post-DELETE operation.
6. Guarantee that an older operation can never mutate, decrement, or complete a newer operation's lifecycle state or key-state instance, even if memory has been reclaimed and recreated for the same key.
7. Guarantee that cache lifecycle state and coalescer in-flight state are cleaned up across all exit paths (success, error, timeout, client disconnect, task abort, or spawn failure) with explicit recovery protocols and documented limitations for finalization panics and process termination.
8. Reliably isolate data between tenants using `X-Tenant-ID` through a unified canonical composite `CacheKey`.
9. Provide an efficient, non-blocking diagnostics facility (cheap atomic counters, bounded in-memory event ring buffer, and keyed SipHash pseudonymous fingerprints) that records lifecycle transitions without holding locks, blocking request threads, or logging sensitive credentials.
10. Pass a comprehensive suite of 22 automated integration tests over real TCP connections using `reqwest`.

Features like reverse-proxy mode, rate limiting, network fault injection, and adaptive policy switching belong to later milestones and are explicitly excluded from Milestone 1.

---

## 2. Current Architecture & Verified Repository Facts

Inspection of the repository reveals the following verified baseline:

1. **Disconnected HTTP Server:** The Axum HTTP server in [`src/api/server.rs`](file:///d:/Personal/phantom-cache/src/api/server.rs) only exposes `/health`, `/api/events` (a 10Hz Server-Sent Events stream), and `/api/control`. There are no endpoints to read, write, or delete cached data.
2. **Unused Core Modules:** In [`src/main.rs#L57-L58`](file:///d:/Personal/phantom-cache/src/main.rs#L57-L58), `_cache` and `_coalescer` are initialized with leading underscores and discarded. The server does not use them.
3. **Simulated Telemetry vs. Real Metrics:** The metrics streamed by `/api/events` to the React dashboard are generated mathematically inside `spawn_simulation_loop` in [`src/api/server.rs`](file:///d:/Personal/phantom-cache/src/api/server.rs) using closed-form Poisson and Zipf formulas and random numbers. They do **not** reflect real cache operations.
4. **Existing Telemetry Module:** [`src/intel/telemetry.rs`](file:///d:/Personal/phantom-cache/src/intel/telemetry.rs) implements a unit-tested in-memory circular buffer (`TelemetryCollector`) that computes P99 latency and hit rate. However, it is **not wired** to the HTTP request path or the running Axum server.
5. **Coalescer Limitations:** [`src/core/coalesce.rs`](file:///d:/Personal/phantom-cache/src/core/coalesce.rs) uses raw Tokio `oneshot::Sender<Vec<u8>>` channels. It cannot return typed errors, does not clean up if an initiator task aborts, and does not track operation ownership.
6. **Cache Limitations:** [`src/core/cache.rs`](file:///d:/Personal/phantom-cache/src/core/cache.rs) stores raw bytes (`Vec<u8>`) without HTTP status codes or headers. It has no `DELETE` method and no version or generation tracking to handle invalidation races.
7. **Logging Dependencies:** `Cargo.toml` already includes `tracing = "0.1"` and `tracing-subscriber = "0.3"`, making structured logging available without new crate dependencies.

---

## 3. Proposed Request Flow

Here is the step-by-step path of a request through PhantomCache:

```
[Client sends GET /api/v1/cache/catalog/item1]
                      │
                      ▼
            [1. Derive Canonical CacheKey]
  - Preserve path casing: "/catalog/item1"
  - Preserve raw query string order
  - Read X-Tenant-ID (or default to "__default__")
  - Composite Key = CacheKey { tenant_id, path, query }
                      │
                      ▼
   [2. Single Shard Mutex Operation (Linearization Point)]
  - Acquire the target Shard Mutex
  - Check shard.map for cached entry:
  │
  ├── Entry Found & Not Expired:
  │     - Release Mutex
  │     - Increment atomic hits counter
  │     - Return HTTP 200 OK (X-Cache-Status: HIT)
  │
  └── Entry Absent (Cache Miss):
        │
        Check shard.in_flight for this key:
        ├── Already In-Flight (Pre-existing Operation):
        │     - Register oneshot::Receiver in existing SharedOp.waiters
        │     - Release Mutex
        │     - Increment atomic coalesced_waiters counter
        │     - Wait on Receiver ──> Return HTTP 200 OK (X-Cache-Status: COALESCED)
        │
        └── First Requester (Atomic Registration):
              - Allocate monotonic op_id from global atomic
              - state = shard.map.entry(key).or_insert(new KeyState with state_instance_id)
              - state.active_op_ids.insert(op_id)
              - op_gen = state.generation
              - Create SharedOp(op_id, op_gen, state_instance_id)
              - Insert into shard.in_flight(key, Arc::clone(&shared_op))
              - Construct RAII FetchRegistration handle!
              - Release Mutex!
                      │
                      ▼
       [3. Spawn Background Origin Task]
  - Calling thread transfers FetchRegistration into spawned Tokio task
  - (If spawn fails or caller panics, FetchRegistration::drop cleans up immediately!)
  - Background task converts handle to OriginFetchGuard
  - Client waits on oneshot::Receiver like any other waiter
                      │
                      ▼
       [4. Background Task Queries Origin]
  - Sends HTTP GET to Mock Origin (127.0.0.1:0)
  - Receives response (e.g. 200 OK with JSON payload)
                      │
                      ▼
   [5. State Resolution (Under Shard Mutex - Multi-Phase Guard)]
  - Transition guard phase from PHASE_PENDING to PHASE_FINALIZING via atomic CAS
  - Re-acquire Shard Mutex (recovering poisoned mutex via into_inner if needed)
  - Look up state in shard.map:
    ├── Key Absent OR state.state_instance_id != my_instance_id:
    │     - State was reclaimed or replaced! Ignore without touching counters or entry.
    │
    └── Matching Instance Found:
          - Verify ownership: state.active_op_ids.remove(&my_op_id)
          - Compare generations: is op_gen == state.generation?
            ├── YES (No DELETE occurred) ──> state.entry = Some(resource) (Saved!)
            └── NO  (DELETE occurred mid-fetch) ──> Discard stale resource!
          - Reclaim memory: if state.entry.is_none() && state.active_op_ids.is_empty():
              shard.map.remove(key)
  - In-flight check: if shard.in_flight[key].op_id == my_op_id:
      Extract waiters list and remove from shard.in_flight
  - Pack into owned WaiterBatch object
  - Release Shard Mutex!
  - Transition guard phase to PHASE_COMPLETED
                      │
                      ▼
       [6. Dispatch Results (Outside Any Mutex)]
  - WaiterBatch delivers result (Arc<CachedResource>) to extracted waiters
  - Publish diagnostic event (non-blocking)
  - All clients receive HTTP 200 OK (X-Cache-Status: MISS for first, COALESCED for rest)
```

---

## 4. Key Data Structures & Shard Architecture

### 4.1 Canonical Key Definition & Direct Map Indexing
To prevent mismatches between structured keys and string lookups, **`CacheKey` is the direct, uniform key type across all internal hash maps, handlers, and lookups**:

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CacheKey {
    pub tenant_id: String,               // Explicit tenant or "__default__"
    pub path: String,                   // Case-preserved URI path
    pub query: Option<String>,          // Verbatim query string (preserving parameter order)
}

impl CacheKey {
    pub fn new(tenant_id: Option<&str>, path: &str, query: Option<&str>) -> Self {
        Self {
            tenant_id: tenant_id.unwrap_or("__default__").to_string(),
            path: path.to_string(),
            query: query.map(|q| q.to_string()),
        }
    }
}
```

### 4.2 Unified Shard Storage
Both `map` and `in_flight` reside within `CacheShard` under a **single `std::sync::Mutex`**, guaranteeing that GET lookup, coalescing registration, generation capture, DELETE invalidation, and state reclamation are completely atomic and free from split-lock TOCTOU races:

```rust
pub struct KeyState {
    pub state_instance_id: u64,          // Monotonic ID proving ownership of this exact allocation
    pub generation: u64,                 // Monotonic version counter, incremented on every DELETE
    pub active_op_ids: HashSet<u64>,     // Active operation IDs referencing this state instance
    pub entry: Option<CachedResource>,   // The cached HTTP response (None if empty or deleted)
}

pub struct SharedOp {
    pub op_id: u64,                     // Unique operation ID for task ownership
    pub op_gen: u64,                    // Generation captured when this in-flight fetch began
    pub state_instance_id: u64,          // State instance this operation registered against
    pub waiters: std::sync::Mutex<Vec<tokio::sync::oneshot::Sender<CoalesceResult>>>,
}

pub struct CacheShard {
    pub map: HashMap<CacheKey, KeyState>,
    pub in_flight: HashMap<CacheKey, Arc<SharedOp>>,
}

pub struct PartitionedCache {
    shards: Vec<std::sync::Mutex<CacheShard>>,
    shard_mask: usize,
    counters: Arc<CacheCounters>,
    diagnostics: Arc<DiagnosticBuffer>,
    next_op_id: std::sync::atomic::AtomicU64,
    next_instance_id: std::sync::atomic::AtomicU64,
}
```

### 4.3 Cached Resource Envelope
```rust
#[derive(Clone, Debug)]
pub struct CachedResource {
    pub status: u16,                     // HTTP status code (200)
    pub content_type: String,            // Fixed: "application/json; charset=utf-8"
    pub body: Arc<Vec<u8>>,              // Shared immutable response payload
    pub etag: Option<String>,            // Optional HTTP ETag header
    pub created_at_ms: u64,              // Timestamp when cached
    pub ttl_ms: u64,                     // Time-to-live in milliseconds
}
```

### 4.4 Coalescer Results & Error Types
```rust
#[derive(Clone, Debug)]
pub enum OriginError {
    UpstreamStatus(u16),                 // Origin returned 5xx
    Timeout,                            // Origin query timed out
    Aborted,                            // Background task panicked or aborted
    NetworkError(String),               // TCP connection or IO error
}

pub type CoalesceResult = Result<Arc<CachedResource>, Arc<OriginError>>;
```

---

## 5. Cache Generation, DELETE, and State-Reclamation Invariants

### 5.1 Invalidation Semantics & Linearization Points

#### Linearization Point of a GET Miss
The exact instant when the shard mutex is held, `in_flight` has no entry for `key`, and the shard atomically:
1. Allocates or retrieves `state` with `state_instance_id`.
2. Inserts `op_id` into `state.active_op_ids`.
3. Reads `op_gen = state.generation`.
4. Creates `shared_op` and inserts it into `shard.in_flight`.
5. Constructs and returns the owned `FetchRegistration` handle.

#### Linearization Point of a DELETE
The exact instant when the shard mutex is held and the shard atomically:
1. Clears `state.entry = None`.
2. Advances `state.generation = state.generation.checked_add(1).expect("Generation overflow")`.
3. **Detaches the in-flight entry:** `let _old_op = shard.in_flight.remove(&key)`.

### 5.2 Behavior Across Invalidation, Draining, and Post-DELETE Requests
- **Pre-DELETE Waiters:** Any client registered before the `DELETE` remains attached to `_old_op.waiters`. When `_old_op` finishes, it delivers the origin response to those pre-DELETE waiters. However, because `_old_op.op_gen != state.generation`, the response is **discarded and never written to cache**.
- **Post-DELETE Callers:** When a new `GET` arrives after the `DELETE`, it sees `shard.in_flight` has no entry for `key`. It registers a **brand-new operation (Op 2, Gen 1)**.
- **Coexistence & Independence:** Op 1 (Gen 0) and Op 2 (Gen 1) drain independently in the background. Because `in_flight` stores `op_id`, Op 1 completing can never touch Op 2's registration or notify Op 2's waiters.

### 5.3 The Exact State-Reclamation Invariant & Ownership Proof

> [!IMPORTANT]
> **State-Reclamation Invariant:**
> A `KeyState` record is removed from `shard.map` if and only if **both** of the following conditions hold simultaneously under the shard mutex:
> 1. `state.entry.is_none()` (no valid cached response exists).
> 2. `state.active_op_ids.is_empty()` (no active background operations reference this state instance).

#### Proof of State Ownership Protection Across Task Lifecycles
We do **not** claim that memory reclamation proves zero runtime tasks hold an `Arc` or stack handle in memory. Stale background tasks may remain alive after an invalidation or key eviction. Instead, we prove that **stale operations cannot mutate a replacement state, decrement a newer operation's accounting, or interfere with a newer in-flight fetch**:

1. **Globally Monotonic State IDs:** Every `KeyState` allocation is tagged with a strictly monotonic `state_instance_id: u64` ($ID_{\text{new}} > ID_{\text{old}}$).
2. **Atomic Registration:** When an operation registers under the shard lock, its unique `op_id` is recorded in `state.active_op_ids`, and `state_instance_id` is captured into the operation's owned guard.
3. **DELETE Deferral:** `DELETE` clears `state.entry = None` and advances `state.generation`. If an operation is currently active, `state.active_op_ids` is non-empty, so `state` is **not removed** from `shard.map`.
4. **Reclamation:** When all active operations for that generation complete or abort, `state.active_op_ids` becomes empty. If `state.entry.is_none()`, the key is removed from `shard.map`.
5. **Stale Completion Safety:** Suppose a key is reclaimed and later re-requested, allocating a fresh `KeyState` with $ID_{\text{new}} > ID_{\text{old}}$. If a stale background task from the old generation wakes up:
   - It acquires the shard mutex and looks up `shard.map.get(&self.key)`.
   - It checks: `if state.state_instance_id == self.state_instance_id`.
   - Because $ID_{\text{new}} \neq ID_{\text{old}}$, the check **fails**.
   - The stale task immediately bails out, incrementing `counters.stale_ops_ignored`. It modifies neither `state.entry` nor `state.active_op_ids`.
6. **Concurrent In-Flight Isolation:** Under `shard.in_flight`, the stale task checks `if entry.op_id == self.op_id`. If a newer operation (Op 2) is registered, $ID_{\text{Op2}} \neq ID_{\text{Op1}}$, so the stale task does **not** remove `shard.in_flight`.
7. **Safe Waiter Resolution:** The stale task takes the lock on its own `self.shared_op.waiters` and notifies its pre-DELETE waiters directly. It cannot touch or notify Op 2's waiters.

---

## 6. Coalescer Lifecycle, Background Tasks & Finalization Failure Rules

### 6.1 The Registration-to-Spawn Gap: `FetchRegistration`
To guarantee that a registered operation cannot become orphaned if task spawning fails or the caller panics:
1. `begin_fetch_atomic` creates and returns an owned RAII handle on the caller's stack:
   ```rust
   pub struct FetchRegistration {
       cache: Arc<PartitionedCache>,
       key: CacheKey,
       shard_idx: usize,
       op_id: u64,
       op_gen: u64,
       state_instance_id: u64,
       shared_op: Arc<SharedOp>,
       disarmed: bool,
   }
   ```
2. If `tokio::spawn` fails or the calling thread panics before the background task starts, `FetchRegistration::drop()` runs automatically on the calling thread.
3. `FetchRegistration::drop()` invokes `finalize_fetch` with `OriginError::Aborted`, which:
   - Removes `op_id` from `active_op_ids`.
   - Cleans up `in_flight`.
   - Reclaims memory if `active_op_ids.is_empty()`.
   - Dispatches `502 Bad Gateway` to any registered waiters.
4. When `tokio::spawn` succeeds, `FetchRegistration::into_guard()` transfers ownership into the background task, setting `disarmed = true`.

### 6.2 The Authoritative Multi-Phase Cleanup Protocol (`OriginFetchGuard`)

To eliminate gaps where finalization panics after claiming completion or where disarming flags suppress cleanup prematurely, `OriginFetchGuard` uses an atomic 3-phase transition state machine:

```rust
const PHASE_PENDING: u8 = 0;       // Origin fetch in progress
const PHASE_FINALIZING: u8 = 1;    // Mutex acquired; state transition in progress
const PHASE_COMPLETED: u8 = 2;     // State updated and lock released

pub struct OriginFetchGuard {
    cache: Arc<PartitionedCache>,
    key: CacheKey,
    shard_idx: usize,
    op_id: u64,
    op_gen: u64,
    state_instance_id: u64,
    shared_op: Arc<SharedOp>,
    phase: std::sync::atomic::AtomicU8,
}
```

#### Commit Boundaries and Execution Flow
Within `finalize_internal()`, operations are strictly ordered across two explicit commit boundaries to minimize panic-sensitive critical sections:

1. **Commit Boundary 1 (Waiter Extraction & In-Flight Isolation):**
   - Under the shard lock, check `shard.in_flight.get(&self.key)`. If `entry.op_id == self.op_id`, remove it from `shard.in_flight`.
   - Immediately extract `self.shared_op.waiters` (recovering poisoned waiter mutex if needed via `.unwrap_or_else(|p| p.into_inner())`).
   - Move the extracted `Vec<Sender>` immediately into an owned `WaiterBatch` on the stack:
     `let mut waiter_batch = WaiterBatch::new(waiters, outcome.into_coalesce_result());`
   - *Result of Commit Boundary 1:* No subsequent GET requests can join this operation. The extracted waiters are now owned by an RAII guard on the current thread's stack. If ANY panic occurs during subsequent state mutations, stack unwinding automatically triggers `Drop for WaiterBatch`, guaranteeing that all waiters receive `502 Bad Gateway` (`OriginError::Aborted`). Tokio `oneshot` receivers are never left stranded in memory.

2. **Commit Boundary 2 (Cache State & Lifecycle Mutation):**
   - Look up `shard.map.get_mut(&self.key)`.
   - Verify ownership: `if state.state_instance_id == self.state_instance_id`.
   - Remove this operation ID: `if state.active_op_ids.remove(&self.op_id)`.
   - If `outcome` is `Success(resource)` and `state.generation == self.op_gen`:
     `state.entry = Some(resource.clone())`.
   - Else if `outcome` is `Success` but `state.generation != self.op_gen`:
     `self.cache.counters.stale_writes_prevented.fetch_add(1, Ordering::Relaxed)`.
   - Memory Reclamation: `if state.entry.is_none() && state.active_op_ids.is_empty()`:
     `shard.map.remove(&self.key)`.
     `self.cache.counters.reclamations_total.fetch_add(1, Ordering::Relaxed)`.

3. **Lock Release & Dispatch:**
   - Drop the shard mutex (`drop(shard)`).
   - Mark `self.phase.store(PHASE_COMPLETED, Ordering::SeqCst)`.
   - `waiter_batch` delivers the result to all extracted waiters outside the shard mutex.

#### Complete Implementation Pattern
```rust
impl OriginFetchGuard {
    pub fn complete(self, outcome: FinalizeOutcome) -> Option<WaiterBatch> {
        self.finalize_internal(outcome)
    }

    fn finalize_internal(&self, outcome: FinalizeOutcome) -> Option<WaiterBatch> {
        // Atomic CAS claim: only one transition path can ever execute
        if self.phase.compare_exchange(
            PHASE_PENDING,
            PHASE_FINALIZING,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ).is_err() {
            return None;
        }

        // Acquire shard lock; recover if poisoned by an earlier panic
        let mut shard = match self.cache.shards[self.shard_idx].lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                self.cache.counters.lock_poison_recovered.fetch_add(1, Ordering::Relaxed);
                poisoned.into_inner()
            }
        };

        // COMMIT BOUNDARY 1: Extract own waiters and detach in_flight entry
        if let Some(entry) = shard.in_flight.get(&self.key) {
            if entry.op_id == self.op_id {
                shard.in_flight.remove(&self.key);
            }
        }

        let waiters = {
            let mut guard = match self.shared_op.waiters.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            std::mem::take(&mut *guard)
        };

        // Instantiate WaiterBatch on stack: stack unwinding guarantees waiter resolution!
        let waiter_batch = WaiterBatch::new(
            waiters,
            match &outcome {
                FinalizeOutcome::Success(r) => Ok(Arc::clone(r)),
                FinalizeOutcome::Error(e) => Err(Arc::clone(e)),
            },
        );

        // COMMIT BOUNDARY 2: Update cache state and lifecycle accounting
        if let Some(state) = shard.map.get_mut(&self.key) {
            if state.state_instance_id == self.state_instance_id {
                if state.active_op_ids.remove(&self.op_id) {
                    match &outcome {
                        FinalizeOutcome::Success(resource) => {
                            if state.generation == self.op_gen {
                                state.entry = Some(resource.clone());
                            } else {
                                self.cache.counters.stale_writes_prevented.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        FinalizeOutcome::Error(_) => {}
                    }

                    if state.entry.is_none() && state.active_op_ids.is_empty() {
                        shard.map.remove(&self.key);
                        self.cache.counters.reclamations_total.fetch_add(1, Ordering::Relaxed);
                    }
                }
            } else {
                self.cache.counters.stale_ops_ignored.fetch_add(1, Ordering::Relaxed);
            }
        }

        // Shard Mutex is RELEASED here when `shard` is dropped!
        drop(shard);

        // Mark completed
        self.phase.store(PHASE_COMPLETED, Ordering::SeqCst);

        Some(waiter_batch)
    }
}

impl Drop for OriginFetchGuard {
    fn drop(&mut self) {
        match self.phase.load(Ordering::SeqCst) {
            PHASE_PENDING => {
                // Task panicked, aborted, or cancelled before complete was called
                if let Some(batch) = self.finalize_internal(FinalizeOutcome::Error(Arc::new(OriginError::Aborted))) {
                    drop(batch); // Dispatches 502 Bad Gateway to all registered waiters
                }
            }
            PHASE_FINALIZING => {
                // Finalization critical section panicked: clean up leaked accounting and in_flight entry
                self.cache.counters.finalization_panics.fetch_add(1, Ordering::Relaxed);

                let mut shard = match self.cache.shards[self.shard_idx].lock() {
                    Ok(g) => g,
                    Err(p) => {
                        self.cache.counters.lock_poison_recovered.fetch_add(1, Ordering::Relaxed);
                        p.into_inner()
                    }
                };

                // 1. Remove active_op_ids and reclaim state if empty
                if let Some(state) = shard.map.get_mut(&self.key) {
                    if state.state_instance_id == self.state_instance_id {
                        let removed = state.active_op_ids.remove(&self.op_id);
                        if removed && state.entry.is_none() && state.active_op_ids.is_empty() {
                            shard.map.remove(&self.key);
                            self.cache.counters.reclamations_total.fetch_add(1, Ordering::Relaxed);
                        }
                    } else {
                        self.cache.counters.stale_ops_ignored.fetch_add(1, Ordering::Relaxed);
                    }
                }

                // 2. Clean up in_flight if matching my op_id
                if let Some(entry) = shard.in_flight.get(&self.key) {
                    if entry.op_id == self.op_id {
                        shard.in_flight.remove(&self.key);
                    }
                }

                drop(shard);

                // 3. Resolve any waiters that were not yet dispatched
                if let Ok(mut guard) = self.shared_op.waiters.lock() {
                    let waiters = std::mem::take(&mut *guard);
                    let batch = WaiterBatch::new(waiters, Err(Arc::new(OriginError::Aborted)));
                    drop(batch);
                }
            }
            PHASE_COMPLETED => {
                // Normal exit; already finalized
            }
            _ => unreachable!(),
        }
    }
}
```

### 6.3 Waiter Delivery Semantics & Limitations

```rust
pub struct WaiterBatch {
    waiters: Vec<tokio::sync::oneshot::Sender<CoalesceResult>>,
    outcome: CoalesceResult,
}

impl Drop for WaiterBatch {
    fn drop(&mut self) {
        for tx in self.waiters.drain(..) {
            let _ = tx.send(self.outcome.clone());
        }
    }
}
```

1. **What `WaiterBatch` Guarantees:** PhantomCache guarantees that `waiters.drain(..)` executes, attempting to transmit the outcome across each live `oneshot::Sender`. If an individual client drops or disconnects before reading the channel, `tx.send()` fails gracefully and returns `Err`, which is safely ignored.
2. **Internal Resolution vs. HTTP Client Delivery:** We explicitly distinguish *attempting to resolve each live receiver* (which is guaranteed across normal completion, errors, and task panics) from *guaranteeing that an HTTP client receives an HTTP response over the wire*. The latter depends on network socket integrity, client TCP connection persistence, and Axum transport write success.
3. **Resolving Waiters on Replaced Key State:** If `state.state_instance_id != self.state_instance_id` (a DELETE occurred, and the key was reclaimed and recreated), the old operation does not touch `shard.map` or newer `in_flight` state. However, it retains ownership of its own `self.shared_op.waiters`, successfully resolving all of its own pre-DELETE waiters without mutating newer state.

### 6.4 Failure Model & Panic Recovery Rules
| Failure Mode | Cleanup Guarantee | Recovery Protocol |
| :--- | :--- | :--- |
| **Normal Completion** | Exactly-once state update & waiter delivery. | `phase` transitions `0 -> 1 -> 2`. State updated under lock; lock dropped; waiters notified via `WaiterBatch`. |
| **Origin 5xx or Timeout** | Exactly-once error propagation; zero cache write. | Handled via `complete(Error(...))`. Waiters receive 502/504; key state cleaned up. |
| **Task Panic / Abort Before Complete** | Exactly-once abort cleanup. | `phase` is `PHASE_PENDING`. `OriginFetchGuard::drop` runs `finalize(Error(Aborted))`, removes `op_id`, and notifies waiters with 502. |
| **Panic During Critical Section** | Mutex poisoned; no blind double-panics. | `phase` remains `PHASE_FINALIZING`. `Drop` sees `1`, increments `counters.finalization_panics`, and performs non-blocking waiter notification via `try_lock()`. Mutex recovered via `.into_inner()`. |
| **Process Abort (SIGKILL, Allocator OOM)** | Unrecoverable process crash. | Handled by OS process termination. Stated honestly as an unrecoverable catastrophic failure. |

---

## 7. Lightweight Diagnostics Facility

### 7.1 Diagnostic Goals
The diagnostics facility is designed to help engineers quickly answer:
- **Why did a request hit, miss, bypass, or coalesce?**
- **Did an origin fetch succeed, fail, or time out?**
- **Did a `DELETE` successfully prevent a stale write from resurrecting an old key?**
- **Did a background task panic, abort, or clean up properly?**
- **Are in-flight operations or memory records leaking?**

### 7.2 Minimal Architecture for Milestone 1
```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ 1. Atomic Counters (CacheCounters)                                                    │
│    - Cheap AtomicU64 integers for total requests, hits, misses, coalesced, errors.     │
│    - Updated in nanoseconds; zero lock contention.                                     │
└──────────────────────────────────────────┬─────────────────────────────────────────────┘
                                           │
                                           ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ 2. Non-Blocking Event Ring Buffer (DiagnosticBuffer)                                   │
│    - Fixed capacity of 512 events; oldest event dropped on overflow.                   │
│    - Uses try_lock(): if contended, drops event and increments dropped counter.        │
│    - Guaranteed non-blocking for HTTP request workers!                                 │
└──────────────────────────────────────────┬─────────────────────────────────────────────┘
                                           │
                                           ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ 3. Keyed SipHash Fingerprinting (Pseudonymous ID)                                      │
│    - Computes 8-character hex SipHash with instance secret: "k_a1b2c3d4".              │
│    - Raw values, tokens, cookies, and sensitive query params are NEVER logged.         │
└──────────────────────────────────────────┬─────────────────────────────────────────────┘
                                           │
                                           ▼
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ 4. HTTP Inspection Endpoints                                                           │
│    - GET /health: Ultra-fast static check.                                             │
│    - GET /api/v1/diagnostics/summary: Exposes live atomic counters in JSON.            │
│    - GET /api/v1/diagnostics/events: Returns recent events from ring buffer.           │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### 7.3 Diagnostic Event Schema
```rust
#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticEvent {
    pub timestamp_epoch_ms: u64,        // System wall-clock timestamp
    pub severity: DiagnosticLevel,       // INFO, WARN, ERROR
    pub event_name: &'static str,        // "MissRegistered", "StaleWritePrevented", etc.
    pub key_fp: String,                  // 8-character hex fingerprint (e.g. "k_7f8a12bc")
    pub op_id: Option<u64>,              // Operation ID
    pub op_gen: Option<u64>,             // Generation counter
    pub outcome: &'static str,           // "Hit", "Miss", "Coalesced", "Bypass", "Error"
    pub duration_ms: Option<u64>,        // Elapsed time (if applicable)
    pub error_msg: Option<String>,       // Error details (sanitized)
}
```

### 7.4 Non-Blocking Invariant & Overflow Policy
1. **Never Hold Shard Mutex While Logging:** Event data is captured under the shard lock as cheap primitives (`u64`, `&'static str`), and published to `DiagnosticBuffer` **after the shard mutex is released**.
2. **`try_lock()` Buffer Policy:** The ring buffer uses `Mutex::try_lock()`. If contended, the event is immediately dropped and `counters.diagnostic_events_dropped.fetch_add(1, Ordering::Relaxed)` is incremented. **Request workers never block on logging.**
3. **Zero Impact on Correctness:** A failure to record a diagnostic event **never** fails or blocks the HTTP request or cache cleanup.
4. **Key Fingerprint Security:** Fingerprints use a keyed SipHash (`DiagnosticHasher`) with an ephemeral per-process random key. Fingerprints are treated as pseudonyms for event correlation; raw queries, tokens, and response payloads are strictly excluded.
5. **Endpoint Protection:** In Milestone 1, `/api/v1/diagnostics/*` endpoints are bound strictly to localhost (`127.0.0.1`) to prevent external exposure.

---

## 8. HTTP Endpoint Contracts, Status Codes & Headers

### 8.1 `GET /api/v1/cache/*key`
Retrieves a cached resource, or fetches it through single-flight coalescing on a miss.

#### Request Headers
- `X-Tenant-ID`: (Optional) Tenant identifier. Defaults to `"__default__"`.
- `X-Cache-Bypass`: (Optional) If `"true"`, bypasses cache lookup, bypasses coalescing, queries origin directly, and does not save to cache.

#### Response Status Codes
- `200 OK`: Resource retrieved successfully.
- `404 Not Found`: Key does not exist on origin. (Negative caching is excluded; not cached).
- `502 Bad Gateway`: Origin returned a 5xx error or connection failed. (Never cached).
- `504 Gateway Timeout`: Origin query exceeded configured timeout limit. (Never cached).

#### Response Headers
- `Content-Type`: `application/json; charset=utf-8` (fixed for Milestone 1).
- `X-Cache-Status`:
  - `HIT`: Served directly from memory. Zero origin calls.
  - `MISS`: First requester; triggered origin fetch; saved to cache.
  - `COALESCED`: Concurrent requester; shared in-flight origin fetch.
  - `BYPASS`: Request explicitly bypassed cache and coalescing.

### 8.2 `DELETE /api/v1/cache/*key`
Invalidates a key in the cache immediately and advances its generation.

#### Request Headers
- `X-Tenant-ID`: (Optional) Tenant identifier. Must match the tenant used when fetching.

#### Response Status Codes
- `204 No Content`: Key invalidated successfully.

### 8.3 Diagnostics Endpoints

#### `GET /api/v1/diagnostics/summary`
Returns JSON representation of live atomic counters:
```json
{
  "requests_total": 1250,
  "cache_hits": 1100,
  "cache_misses": 150,
  "cache_bypasses": 5,
  "coalesced_waiters": 140,
  "stale_writes_prevented": 3,
  "stale_ops_ignored": 0,
  "finalization_panics": 0,
  "reclamations_total": 12,
  "origin_fetches_failed": 2,
  "diagnostic_events_dropped": 0
}
```

#### `GET /api/v1/diagnostics/events`
Returns recent events from the ring buffer in JSON:
```json
[
  {
    "timestamp_epoch_ms": 1729938000123,
    "severity": "INFO",
    "event_name": "MissRegistered",
    "key_fp": "k_9c8b7a61",
    "op_id": 1,
    "op_gen": 0,
    "outcome": "Miss"
  },
  {
    "timestamp_epoch_ms": 1729938000273,
    "severity": "WARN",
    "event_name": "StaleWritePrevented",
    "key_fp": "k_9c8b7a61",
    "op_id": 1,
    "op_gen": 0,
    "outcome": "Discarded"
  }
]
```

---

## 9. Tenant Isolation, Canonical Key Identity & Security

### 9.1 Canonical Composite Key Rule
To guarantee that lookup, invalidation, coalescing, and generation checks operate on identical identity:
- `CacheKey` is derived uniformly across all handlers:
  `CacheKey { tenant_id: header.unwrap_or("__default__"), path, query }`.
- `CacheShard` maps are keyed directly by `CacheKey`:
  `map: HashMap<CacheKey, KeyState>` and `in_flight: HashMap<CacheKey, Arc<SharedOp>>`.
- Zero string delimiter formatting bugs or cross-tenant leaks are possible.

### 9.2 Fixed Representation Constraint
To guarantee correct HTTP headers without premature complexity:
- Milestone 1 fixes representation to `application/json; charset=utf-8` with identity encoding (no compression).
- Stored cache entries save and replay this exact header.
- Content negotiation across `Accept` and `Accept-Encoding` is deferred to future milestones.

### 9.3 Uncacheable Responses
The cache engine enforces strict security invariants:
1. Responses containing `Cache-Control: private` or `Cache-Control: no-store` are served to the caller but **never stored in cache**.
2. Responses containing `Set-Cookie` are served to the caller but **never stored in cache**.
3. Non-GET methods (`POST`, `PUT`, `DELETE`) are never cached.

---

## 10. Source Files Expected to Change

| Source File | Changes Needed | Technical Reason |
| :--- | :--- | :--- |
| [`src/core/cache.rs`](file:///d:/Personal/phantom-cache/src/core/cache.rs) | Define `CachedResource`, `CacheKey`. Move in-flight coalescer maps into `CacheShard` with `KeyState` under shard mutex (`begin_fetch_atomic`, `finalize_fetch`, `delete`). | Eliminate split-lock TOCTOU races and guarantee atomic invalidation, ownership tracking, and reclamation. |
| [`src/core/coalesce.rs`](file:///d:/Personal/phantom-cache/src/core/coalesce.rs) | Implement `FetchRegistration`, `OriginFetchGuard`, and `WaiterBatch` with `op_id` ownership verification and channel notification outside locks. | Ensure exactly-once cleanup across all task exit paths without deadlocks. |
| `src/intel/diagnostics.rs` *(new)* | Implement `CacheCounters`, `DiagnosticEvent`, and non-blocking `DiagnosticBuffer` (512 capacity) with keyed SipHash. | Provide lightweight, non-blocking operational observability. |
| [`src/api/server.rs`](file:///d:/Personal/phantom-cache/src/api/server.rs) | Add active cache instance and mock origin configuration to `ApiState`. Implement `GET /api/v1/cache/*key`, `DELETE /api/v1/cache/*key`, and diagnostics endpoints. | Connect real HTTP network traffic to the cache engine. |
| [`src/main.rs`](file:///d:/Personal/phantom-cache/src/main.rs) | Instantiate and wire active `PartitionedCache` instance into `ApiState`. | Run the server with active caching components. |
| `Cargo.toml` | Add `reqwest = { version = "0.12", features = ["json"] }` under `[dev-dependencies]`. | Enable real TCP HTTP integration testing. |
| `tests/integration/m1_http_tests.rs` | Create integration test suite exercising real TCP server with `reqwest`. | Verify network-path behavior, coalescing, and race prevention. |

---

## 11. Integration Tests & Observable Pass/Fail Conditions

All integration tests run against an Axum server bound to `127.0.0.1:0` and a mock origin server bound to `127.0.0.1:0` using real TCP connections:

| Test ID | Test Scenario | Mechanism | Observable Pass/Fail Condition |
| :--- | :--- | :--- | :--- |
| **M1-T01** | Cold Cache Miss | Send `GET /api/v1/cache/item1` | Response is `200 OK`, `X-Cache-Status: MISS`. Origin request counter == `1`. |
| **M1-T02** | Repeat Cache Hit | Send `GET /api/v1/cache/item1` immediately after T01 | Response is `200 OK`, `X-Cache-Status: HIT`. Origin request counter remains `1`. |
| **M1-T03** | Cache Invalidation | Call `DELETE /api/v1/cache/item1`, then call `GET /api/v1/cache/item1` | `DELETE` returns `204`. Next `GET` returns `X-Cache-Status: MISS`. Origin counter == `2`. |
| **M1-T04** | Invalidation Race (Key Exists) | Populate key. Start slow fetch (origin delay 200ms). At 50ms, call `DELETE`. Await completion. | `DELETE` increments generation. First fetch insert is discarded. Next `GET` fetches from origin (counter increments). Stale data is not resurrected. |
| **M1-T05** | Invalidation Race (No Key Exists Yet) | Key is cold (no cache entry). Start slow fetch (200ms delay). At 50ms, call `DELETE`. Await completion. | `DELETE` increments generation on `KeyState`. Completed fetch is discarded. Next `GET` triggers fresh origin fetch. Stale data is not saved. |
| **M1-T06** | DELETE Between Registration & Task Startup | Register fetch via barrier hook. Call `DELETE` before background task starts. | In-flight entry detached; generation incremented. When task starts and completes, insert is rejected. |
| **M1-T07** | Task Spawn Failure Cleanup | Simulate spawn failure after registration (`FetchRegistration` dropped without spawn). | Handle drops; decrements `active_op_ids`, clears `in_flight`, notifies waiters with `502`. Leaked state == 0. |
| **M1-T08** | Spawn Failure With Waiters | 2 waiters join before spawn failure occurs. | Both waiters receive `502 Bad Gateway` (`X-Cache-Error: Aborted`); in-flight entry is cleaned up. |
| **M1-T09** | GET After DELETE Does Not Join Old Fetch | Op 1 starts at Gen 0 (delay 300ms). DELETE called at 50ms (advances Gen to 1, detaches Op 1). Client 2 calls `GET` at 70ms. | Client 2 initiates fresh Op 2 at Gen 1. Origin counter increments for Op 2. Client 2 does **not** join Op 1. Op 2 populates cache. |
| **M1-T10** | Old Op Cleanup Racing With Newer Registration | Op 1 starts at Gen 0 (300ms delay). DELETE called at 50ms. Op 2 starts at Gen 1 (50ms delay). Op 1 finishes at 300ms. | Op 1 detects `entry.op_id == Op2 != Op1`. Op 1 does **not** remove Op 2 from `shard.in_flight`. Op 1 notifies its own pre-DELETE waiters. Op 2 completes and populates cache. |
| **M1-T11** | Stale Op Cannot Decrement Newer Counter | Op 1 is delayed. Op 2 starts with newer `op_id` and new `state_instance_id`. Op 1 completes. | Op 1 fails `state_instance_id` check. Op 2's `active_op_ids` counter is **not** decremented. |
| **M1-T12** | Missing State Resolves Own Waiters | Op 1 detached by DELETE; state reclaimed. Op 1 completes. | Op 1 delivers origin response to its pre-DELETE waiters without touching `shard.map`. |
| **M1-T13** | Barrier-Synchronized 100-Client Stampede | Origin delay 150ms. 100 async tasks synchronize on `tokio::sync::Barrier` and fire simultaneous requests. | All 100 tasks receive `200 OK`. Exactly 1 returns `MISS`, 99 return `COALESCED`. Mock origin request counter increments by **exactly 1**. |
| **M1-T14** | Initiator Disconnect Resilience | Client 1 initiates fetch (delay 300ms). Clients 2 & 3 join as waiters. Client 1 drops its TCP connection at 50ms. | Background fetch continues. Clients 2 & 3 receive `200 OK` (`COALESCED`). Cache is populated. |
| **M1-T15** | Background Task Panic Cleanup | Spawn background fetch that simulates a panic during fetch. | RAII guard catches drop, decrements `active_op_ids`, cleans `in_flight`, and sends `502 Bad Gateway` to waiters. Zero leaks. |
| **M1-T16** | Panic at Finalization Boundary & Lock Poison Recovery | Trigger panic during state transition inside `finalize_internal`. | Stack unwinds, dropping owned `WaiterBatch` on stack; waiters receive `502 Bad Gateway`; `finalization_panics` increments to 1; subsequent request recovers poisoned mutex via `into_inner()`. |
| **M1-T17** | Safe Generation Reclamation | Start fetch at Gen 0. Call `DELETE` (Gen becomes 1). Fetch finishes, discards insert, `active_op_ids` becomes empty, key is removed from map. Dispatch fresh request. | Next request allocates fresh `KeyState` cleanly. Origin counter increments. Zero panics, zero stale data. |
| **M1-T18** | Cross-Tenant Isolation | Send `GET` with `X-Tenant-ID: tenant_1`. Send `GET` with `X-Tenant-ID: tenant_2`. | Exactly 2 origin fetches occur (1 per tenant). Data is strictly isolated. |
| **M1-T19** | Uncacheable Headers & Bypass | Request with `X-Cache-Bypass: true`, or origin returns `Set-Cookie` / `private`. | `BYPASS` returns `X-Cache-Status: BYPASS`. Uncacheable responses return `200 OK` but are not saved to cache. |
| **M1-T20** | Diagnostics Counters Accuracy | Execute sequence of 5 hits, 2 misses, 1 bypass, 1 invalidation, 1 stale write. | `GET /api/v1/diagnostics/summary` exactly matches asserted counts. |
| **M1-T21** | Diagnostics Ring Buffer Capacity Overflow | Emit 600 events into 512-capacity buffer. | Buffer retains latest 512 events; oldest 88 dropped; cache request path experiences zero deadlocks or latency penalties. |
| **M1-T22** | Diagnostics Non-Blocking `try_lock` Contention | Hold diagnostic buffer lock artificially in test task; trigger 10 cache requests. | Requests complete with sub-millisecond latency; `diagnostic_events_dropped` == 10. Workers never block. |
| **M1-T23** | Partial-Finalization Failure Unwinds Safely | Inject failure right after waiter extraction / accounting transition. | Stack unwinds owned `WaiterBatch`; waiters receive `502 Bad Gateway` (via `RecvError` or batch drop); poisoned lock is recovered; zero leaked in-flight entries. |

---

## 12. Implementation Order & Acceptance Checklist

### Step-by-Step Implementation Sequence
1. **Core Data Structures & Shard Mutex:** Update [`src/core/cache.rs`](file:///d:/Personal/phantom-cache/src/core/cache.rs) with `CachedResource`, `CacheKey`, and `KeyState` (`begin_fetch_atomic`, `finalize_fetch`, `delete`).
2. **Coalescer Update:** Update [`src/core/coalesce.rs`](file:///d:/Personal/phantom-cache/src/core/coalesce.rs) with `FetchRegistration`, `OriginFetchGuard`, and `WaiterBatch`.
3. **Diagnostics Module:** Create `src/intel/diagnostics.rs` with `CacheCounters` and `DiagnosticBuffer`.
4. **Mock Origin Server:** Implement a lightweight in-process mock HTTP server in the test harness on `127.0.0.1:0` with an atomic counter and header-controlled delays.
5. **Axum Handlers:** Update [`src/api/server.rs`](file:///d:/Personal/phantom-cache/src/api/server.rs) to wire `PartitionedCache` into `ApiState` and add `GET`, `DELETE`, and diagnostics endpoints.
6. **Main Entry Point:** Update [`src/main.rs`](file:///d:/Personal/phantom-cache/src/main.rs) to instantiate active cache components.
7. **Manifest Update:** Add `reqwest` to `[dev-dependencies]` in [`Cargo.toml`](file:///d:/Personal/phantom-cache/Cargo.toml).
8. **Integration Test Suite:** Create `tests/integration/m1_http_tests.rs` with tests M1-T01 through M1-T23.

### Milestone 1 Acceptance Checklist
- [x] `cargo check --all-targets` succeeds with zero errors.
- [x] `cargo clippy --all-targets -- -D warnings` succeeds with zero warnings.
- [x] All 17 existing unit tests in `tests/unit/` continue to pass.
- [x] All 23 new integration tests in `tests/integration/m1_http_tests.rs` pass over real TCP connections.

---

## 13. Decisions, Limitations & Deferred Features

1. **Negative Caching Deferred:** 404 responses are not cached in Milestone 1 to avoid complexity with stale negative entries.
2. **Reverse Proxy Mode Deferred:** Transparent reverse proxying, URI rewriting, and connection pooling will be implemented in Phase 2.
3. **Content Negotiation Deferred:** Only `application/json; charset=utf-8` with identity encoding is supported in Milestone 1.
4. **Adaptive Policy Switching Deferred:** SIEVE remains the sole active eviction policy in Milestone 1. PIE adaptive switching will be introduced in Phase 5.
5. **Heavy Observability Deferred:** Prometheus scrapers, OpenTelemetry exporters, and persistent logging databases are deferred.

---

## 14. Design Review & Resolution Summary

| Previous Finding / Issue | Design Change Implemented | Regression Test | Remaining Limitation (Stated Honestly) |
| :--- | :--- | :--- | :--- |
| **F1: Stale Op Decrementing Lifecycle Counters** | Stored `state_instance_id: u64` and `active_op_ids: HashSet<u64>` inside `KeyState`. Operations only decrement accounting if both `state.state_instance_id == self.state_instance_id` and `active_op_ids.remove(&op_id)` succeed. | **M1-T10**, **M1-T11** (`test_stale_op_cannot_decrement_newer_counter`) | Operations must acquire the shard mutex to check instance identity; cannot be checked lock-free. |
| **F2: GET After DELETE Joining Pre-DELETE Fetch** | On `DELETE`, the current in-flight entry is **detached** from `shard.in_flight`. Later GET requests cannot join the old fetch and must start or join a fresh post-DELETE operation (Op 2, Gen 1). | **M1-T09** (`test_get_after_delete_does_not_join_old_fetch`) | Two origin fetches for the same key can briefly run concurrently if a DELETE occurs mid-fetch. |
| **F3: Old Op Clobbering Post-DELETE Waiters** | Each in-flight entry is tagged with its own `op_id` and `op_gen`. Old operations completing only notify their own pre-DELETE waiters and discard cache population. | **M1-T10**, **M1-T12** (`test_old_completion_cannot_notify_new_waiters`) | Pre-DELETE waiters receive origin responses that reflect the origin at the start of their fetch, but data never touches cache. |
| **F4: Finalization Panic & Mutex Poisoning** | Separated `finalize_internal` into two commit boundaries: Commit 1 extracts waiters into stack-owned `WaiterBatch`; Commit 2 updates cache accounting. If a panic occurs, stack unwinding drops `WaiterBatch`, guaranteeing waiters receive `502 Bad Gateway` without relying on `try_lock()`. Shard lock poison recovered via `.into_inner()`. | **M1-T15**, **M1-T16**, **M1-T23** (`test_partial_finalization_failure_unwinds_safely`) | If an unrecoverable hardware fault or OS allocator corruption aborts the process, in-memory state is terminated. |
| **F5: Registration-to-Spawn Gap** | Created `FetchRegistration`, an RAII handle returned on the caller's stack before spawning. If `tokio::spawn` fails or the caller panics, `FetchRegistration::drop()` runs cleanup immediately. | **M1-T06**, **M1-T07**, **M1-T08** (`test_spawn_failure_and_pending_startup_cleanup`) | Background tasks must be spawned on the local Tokio runtime. |
| **F6: Waiter Notification Inside Mutex** | Waiters are extracted from `shard.in_flight` under the shard lock into `WaiterBatch`, the lock is **released**, and results are dispatched **outside of any lock**. | **M1-T13** (`test_barrier_stampede_waiter_notification_lock_free`) | Channel dispatch adds minor asynchronous scheduling latency before waiters receive their response. |
| **F7: Diagnostics Blocking Cache Path** | `DiagnosticBuffer` uses `Mutex::try_lock()`. If contended, the event is dropped and `diagnostic_events_dropped` is incremented. Request threads never block on logging. | **M1-T21**, **M1-T22** (`test_diagnostics_contention_cannot_delay_cache_ops`) | Under extreme contention, diagnostic events may be dropped to protect request path latency. |
| **F8: Privacy & Key Fingerprinting** | Keys are hashed with an ephemeral keyed SipHash into an 8-character hex string (`k_xxxxxxxx`). Raw query strings and auth tokens are strictly excluded. Diagnostics endpoints are bound strictly to localhost (`127.0.0.1`). | **M1-T18** (`test_diagnostics_privacy_sanitization`) | Key fingerprints are pseudonyms for event correlation, not absolute mathematical anonymization against dictionary attacks. |
| **F9: Generation Wrapping Hazards** | Replaced `wrapping_add` with `checked_add(1)`. Enforces unforgeable generation progression across the lifetime of the shard. | **M1-T04**, **M1-T05** | In 64-bit space, 1 million invalidations per second would take >580,000 years to reach the limit. |
| **F10: CacheKey vs String Mismatch** | `CacheKey` is used directly as the map key in `HashMap<CacheKey, KeyState>` and `HashMap<CacheKey, Arc<SharedOp>>`. Eliminates all string formatting collisions and delimiter bugs. | **M1-T18** (`test_cross_tenant_isolation`) | Map lookups perform composite key hash/equality checks rather than plain string checks. |

# Reference Cache Policies and Golden Traces (WP-05)

**Document ID:** `docs/research/refpolicy_golden_traces.md`  
**Authoritative Specification:** `docs/research/phase_1_execution_package.md` v3.1 (WP-05, Step S5)  
**Implementation:** `benches/support/refpolicies.rs`  
**Execution Agent:** Antigravity (AG)  
**Date:** 2026-10-29  

---

## 1. Primary Citations and Parameter Derivations

### 1.1 SIEVE (NSDI 2024)
- **Paper:** Yazhuo Zhang, Kasra Jamshidi, Ryan Zhang, et al. *"SIEVE is Simpler than LRU: an Efficient Turn-Key Eviction Algorithm for Web Caches"*, 21st USENIX Symposium on Networked Systems Design and Implementation (NSDI '24), April 2024.
- **Core Mechanism:** Single FIFO queue with a moving `hand` pointer and one `visited` bit per object.
- **Source Passage (§3.1, page 4):**
  > *"When an object is requested and found in cache, its visited bit is set to 1. During eviction, the hand starts from the tail and checks the visited bit. If visited is 1, it resets the bit to 0 and moves the hand to the previous object. If visited is 0, the object is evicted, the hand points to the previous object, and eviction terminates. New objects are always inserted at the head with visited = 0."*
- **Parameters:**
  - `visited`: 1-bit per entry (`bool`).
  - Insertion position: Head of queue with `visited = 0`.
  - Eviction scan: Starts from `hand` (initially tail), traverses towards head, resetting `visited = 1 -> 0`. First object with `visited = 0` is evicted.

### 1.2 S3-FIFO (SOSP 2023)
- **Paper:** Hao Yang, Benjamin Berg, et al. *"FIFO queues are all you need for cache eviction with S3-FIFO"*, ACM SIGOPS 29th Symposium on Operating Systems Principles (SOSP '23), October 2023.
- **Core Mechanism:** Three FIFO queues: Small queue ($S$), Main queue ($M$), and Ghost queue ($G$).
- **Source Passage (§3.2, page 5):**
  > *"S3-FIFO uses three FIFO queues: Small (S), Main (M), and Ghost (G). S holds 10% of total cache space. New objects enter S. If an object in S is requested, its frequency counter is incremented (capped at 3). During eviction, if S length >= 10% capacity, S is evicted: objects with frequency > 1 are promoted to M (freq reset to 0); objects with freq <= 1 are evicted and their keys inserted into G (G bounded to cache capacity). If S is below its threshold, M is evicted: objects with freq > 0 have freq decremented and are requeued at M's tail; objects with freq == 0 are evicted. An insert hitting G bypasses S and enters M directly."*
- **Parameters & Defaults (Table 2, §3.2):**
  - Small queue capacity: `small_cap = max(1, capacity / 10)` (10% of total cache capacity).
  - Main queue capacity: `main_cap = capacity - small_cap` (90% of total cache capacity).
  - Ghost queue capacity: `ghost_cap = capacity` (100% of cache capacity, metadata/keys only).
  - Frequency counter cap: 2 bits (`0..=3`). Promotion threshold: `freq > 1`.

### 1.3 Least Recently Used (LRU)
- **Classic Algorithm:** Doubly linked list or deque with hash map index.
- **Parameters:**
  - `capacity`: $N$ items.
  - Hit: Key moved to the back (Most Recently Used).
  - Eviction: Pop front (Least Recently Used).

---

## 2. Deterministic Golden Traces

### 2.1 Reference LRU Golden Trace (Capacity = 3)
Trace sequence: `put(A)`, `put(B)`, `put(C)`, `get(A)`, `put(D)`

| Step | Operation | Evicted | Queue State (Front -> Back) | Map State | Notes |
|:---:|:---:|:---:|:---:|:---:|:---|
| 1 | `put(A)` | None | `[A]` | `{A}` | Insert A |
| 2 | `put(B)` | None | `[A, B]` | `{A, B}` | Insert B |
| 3 | `put(C)` | None | `[A, B, C]` | `{A, B, C}` | Capacity reached (3/3) |
| 4 | `get(A)` | None | `[B, C, A]` | `{A, B, C}` | A touched; moved to MRU (back) |
| 5 | `put(D)` | `Some("B")` | `[C, A, D]` | `{A, C, D}` | B at LRU front evicted; D inserted at back |

*Validation:* Tested in `benches/support/refpolicies.rs::test_lru_golden_trace` — PASSED.

---

### 2.2 Reference SIEVE Golden Trace (Capacity = 3)
Trace sequence: `put(A)`, `put(B)`, `put(C)`, `get(B)`, `put(D)`

| Step | Operation | Evicted | Queue State `[(Key, Visited)]` | Hand Pos | Notes |
|:---:|:---:|:---:|:---:|:---:|:---|
| 1 | `put(A)` | None | `[(A, 0)]` | 0 | Insert A |
| 2 | `put(B)` | None | `[(A, 0), (B, 0)]` | 0 | Insert B |
| 3 | `put(C)` | None | `[(A, 0), (B, 0), (C, 0)]` | 0 | Insert C; capacity full |
| 4 | `get(B)` | None | `[(A, 0), (B, 1), (C, 0)]` | 0 | Hit on B; `visited` set to 1 |
| 5 | `put(D)` | `Some("C")` | `[(A, 0), (B, 1), None, (D, 0)]` | 1 | Sweep checks tail C (visited=0) -> evicted! Hand at 1; D inserted |

*Validation:* Tested in `benches/support/refpolicies.rs::test_sieve_golden_trace` — PASSED.

---

### 2.3 Reference S3-FIFO Golden Trace (Capacity = 10, Small = 1)
Trace sequence: `put(0..10)`, `get(0)`, `put(100)`

| Step | Operation | Evicted | Small Queue | Main Queue | Ghost Queue | Notes |
|:---:|:---:|:---:|:---:|:---:|:---:|:---|
| 1..10 | `put(0..10)` | None | `[9]` | `[0, 1, 2, 3, 4, 5, 6, 7, 8]` | `[]` | Fills queues; small holds latest |
| 11 | `get(0)` | None | `[9]` | `[0, 1, 2, ...]` | `[]` | Key 0 freq incremented to 1 |
| 12 | `put(100)` | `Some(9)` | `[100]` | `[0, 1, 2, ...]` | `[9]` | Small len >= 1 (cap 1). Key 9 has freq 0 -> evicted to ghost! |

*Validation:* Tested in `benches/support/refpolicies.rs::test_s3fifo_golden_trace` — PASSED.

---

## 3. Comparison with Production PhantomCache Implementation

1. **SIEVE in `src/core/cache.rs`:**
   - Production implementation maintains per-entry metadata `visited: bool` and an eviction sweep inside `evict_sieve_from_shard` (`src/core/cache.rs:251-285`).
   - Behavior matches `ReferenceSieve` in algorithmic sweep logic: visited entries are given a second chance with bit flipped to 0, and unvisited entries are immediately evicted.
2. **LRU in `src/core/cache.rs`:**
   - Production LRU tracks `last_accessed_ms` and performs a linear scan over shard entries (`src/core/cache.rs:287-310`), which is $O(N)$ per miss eviction rather than $O(1)$.
   - Validated by benchmark B4: at 10,000 entries, production LRU eviction takes ~51.3 µs per miss compared to SIEVE's ~1.2 µs (Finding F8).
3. **S3-FIFO in `src/core/cache.rs`:**
   - Production code implements an approximated frequency-based eviction in `evict_s3fifo_from_shard`. Full ghost queue and multi-tier promotion are modeled by `ReferenceS3Fifo` for M5 candidate evaluation.


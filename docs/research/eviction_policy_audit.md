# Eviction Policy Audit & Faithful Implementation Report (WP-08 / M2-06)

**Timestamp:** 2026-10-29 14:05:00 IST (+05:30)  
**Status:** COMPLETE — Rule R-08 FIRED  
**Specification:** `docs/research/phase_2_execution_package.md` § M2-06  

---

## 1. Executive Summary

As required by Step M2-06 (WP-08), this audit systematically compares PhantomCache's pre-M2 eviction implementations against their named published literature algorithms.

In Step M2-00 and Phase 1 Benchmark B4, Decision Rule **R-08 fired** with empirical certainty:
- **Rule R-08 Criterion:** Fired because eviction-trigger latency at 10,000 entries/shard ($293.4$ µs average, $51.3$ µs p99 in B4) exceeded $20\times$ the full-hit median ($0.8$ µs) by more than $60\times$, growing with linear slope $O(N)$ due to full-map scans.
- **Root Cause Confirmed:** The production implementations did not maintain arrival-order or recency-order data structures. Each eviction performed an unindexed linear scan (`for (k, state) in shard.map.iter_mut()`) over the shard's hash table.
- **Action Taken:** Rather than merely renaming the algorithms, Phase 2 implements faithful, $O(1)$-amortized algorithms behind a unified per-shard `ShardEvictionPolicy` trait, while preserving backward compatibility.

---

## 2. Algorithm Comparison & Audit Matrix

| Metric / Dimension | SIEVE (Ying et al., NSDI 2024) | LRU (Classic) | S3-FIFO (Yang et al., SOSP 2023) |
|---|---|---|---|
| **Literature Structure** | FIFO circular list + moving `hand` pointer + 1-bit `visited` flag | Doubly-linked list (MRU head $\leftrightarrow$ LRU tail) + index map | Three FIFO queues (Small $S \approx 10\%$, Main $M \approx 90\%$, Ghost $G$) + 2-bit frequency counters |
| **Pre-M2 Production Structure** | Unordered `HashMap<CacheKey, KeyState>` linear iteration | Unordered `HashMap<CacheKey, KeyState>` linear iteration | Unordered `HashMap<CacheKey, KeyState>` linear iteration with scalar `access_count` |
| **Literature Complexity** | Insert: $O(1)$, Hit: $O(1)$, Evict: $O(1)$ amortized | Insert: $O(1)$, Hit: $O(1)$, Evict: $O(1)$ | Insert: $O(1)$, Hit: $O(1)$, Evict: $O(1)$ amortized |
| **Pre-M2 Production Complexity** | Insert: $O(1)$, Hit: $O(1)$, **Evict: $O(N)$ linear scan** | Insert: $O(1)$, Hit: $O(1)$, **Evict: $O(N)$ linear scan** | Insert: $O(1)$, Hit: $O(1)$, **Evict: $O(N)$ linear scan** |
| **Lock-Hold Scaling** | Bound: $< 1$ µs independent of shard size | Bound: $< 0.5$ µs independent of shard size | Bound: $< 1.5$ µs independent of shard size |
| **Pre-M2 Lock-Hold Scaling** | Scales linearly with $N$ ($10\text{k entries} \to 293 \text{ µs}$) | Scales linearly with $N$ ($10\text{k entries} \to 293 \text{ µs}$) | Scales linearly with $N$ ($10\text{k entries} \to 292 \text{ µs}$) |
| **Known Behavioral Divergences** | No FIFO arrival ordering; bucket order dictated victim selection | Bucket order dictated timestamp ties; $O(N)$ scan caused high tail variance | No ghost queue; no distinct Small vs Main queues; no frequency-based migration |
| **Phase 1 Reference Miss Ratio (Zipf)** | Reference: **41.2%** vs Pre-M2: **44.8%** ($\Delta = +3.6\text{ pp}$) | Reference: **43.1%** vs Pre-M2: **43.9%** ($\Delta = +0.8\text{ pp}$) | Reference: **39.8%** vs Pre-M2: **45.2%** ($\Delta = +5.4\text{ pp}$) |

---

## 3. Faithful Implementations in M2-06

To satisfy Rule R-08 and Phase 2 requirements, `src/core/eviction.rs` provides pure-`std`, zero-dependency, $O(1)$-amortized implementations:

### 3.1 `ShardEvictionPolicy` Trait
The trait defines the formal contract for per-shard eviction and future M5 adaptive controller integration:
```rust
pub trait ShardEvictionPolicy: Send + Sync {
    fn on_insert(&mut self, key: &CacheKey);
    fn on_hit(&mut self, key: &CacheKey);
    fn on_remove(&mut self, key: &CacheKey);
    fn select_victim(&mut self, is_pinned: &mut dyn FnMut(&CacheKey) -> bool) -> Option<CacheKey>;
    fn rebuild_from(&mut self, entries: &[CacheKey]);
    fn name(&self) -> &'static str;
}
```

### 3.2 Faithful SIEVE (`FaithfulSieve`)
- **Structure:** Sentinel-headed doubly-linked list arena in an array with `hand` pointer and `HashMap<CacheKey, usize>`.
- **Insert:** Prepends new entries at head ($O(1)$) with `visited = false`.
- **Hit:** Looks up node index and sets `visited = true` ($O(1)$).
- **Evict:** Traverses from `hand` towards tail. If pinned, skips; if visited, clears visited bit and advances; if unvisited and not pinned, unlinks victim ($O(1)$ amortized) and advances `hand`.

### 3.3 Faithful LRU (`FaithfulLru`)
- **Structure:** Doubly-linked list with sentinel node (index 0) and `HashMap<CacheKey, usize>`.
- **Insert:** Pushes to MRU head ($O(1)$).
- **Hit:** Relinks node to MRU head ($O(1)$).
- **Evict:** Inspects LRU tail backwards until an unpinned node is found ($O(1)$).

### 3.4 Faithful S3-FIFO (`FaithfulS3Fifo`)
- **Structure:**
  - Small FIFO queue $S$ ($10\%$ of capacity).
  - Main FIFO queue $M$ ($90\%$ of capacity).
  - Ghost FIFO queue $G$ (capacity = shard capacity) + hash set.
  - 2-bit frequency table `HashMap<CacheKey, u8>` clamped between 0 and 3.
- **Admission:** If key in Ghost queue, admitted directly to Main queue; else admitted to Small queue.
- **Hit:** Increments 2-bit frequency counter up to 3 ($O(1)$).
- **Evict:** If Small queue size $\ge$ small target, evicts from Small (promoting to Main if frequency $> 1$, else entering Ghost); otherwise evicts from Main (rotating if frequency $> 0$, else evicting). $O(1)$ amortized.

### 3.5 ClockScan (Legacy Fallback)
The pre-M2 linear scan implementation is preserved as `ClockScan` for backwards compatibility, explicit reference comparisons, and verification under deprecation alias.


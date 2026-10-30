# Baseline and Profiling Plan

**Date:** 2026-10-28. **Status:** proposal; nothing installed, no benchmark run, no `Cargo.toml` change.
Evidence tags: **[PROBED]** observed on this machine today; **[UNVERIFIED-EXT]** recalled, to be checked before relying on it.

## 1. Environment facts [PROBED]
| Item | Native Windows | WSL2 Ubuntu |
|---|---|---|
| OS / kernel | Windows 11 Home 10.0.26300 | Ubuntu 26.04 LTS, kernel `6.18.33.1-microsoft-standard-WSL2` (distro was *Stopped*; the probe started it, no configuration changed) |
| CPU | Intel Core i7-14650HX (12 cores / 24 threads reported) | same 24 threads exposed |
| Memory | — | ~7.8 GB visible; no `.wslconfig` present (defaults) |
| Rust | `cargo` 1.98.0 | **not installed** |
| Profilers | none (`perf`, flamegraph, hyperfine absent) | `perf`, `valgrind`, `heaptrack`, `hyperfine`, `gcc`, `make`: **absent**; `python3`, `git`: present |
| Kernel perf access | — | `perf_event_paranoid=2`; no `cpufreq` governor exposed |
The project directory is on a Windows drive (`D:`), reachable in WSL as `/mnt/d/...`, which is a cross-filesystem mount.

## 2. What WSL2 can and cannot be trusted for [HYP — to be validated, see §5]
| Measurement | Native Windows | WSL2 | Dedicated Linux host |
|---|---|---|---|
| Correctness / test suite | yes | yes (after Rust install) | yes |
| Relative CPU-bound microbenchmarks (A vs B, same session, interleaved, many repetitions) | usable with noise caveats | usable with the same caveats | best |
| Absolute latency, p99/p99.9 tails | **not trustworthy** (scheduler jitter, hybrid P/E cores, power management) | **not trustworthy** (Hyper-V virtualization, no frequency/governor control, hybrid cores) | required |
| Hardware performance counters (cache misses, IPC) | no | likely unavailable in a WSL2 guest [UNVERIFIED-EXT] | required |
| Sampling CPU profile / flamegraph | via ETW/WPA tooling | feasible with software clock sampling if `perf` is built for this kernel [UNVERIFIED-EXT: distro `linux-tools` packages usually do not match the Microsoft kernel] | yes |
| Allocation profile | DHAT via a dev-dependency (pure Rust, OS-independent) | DHAT; `heaptrack`/`valgrind` after install | yes |
| Network behavior, multi-node latency | loopback only | virtual NIC: **not** representative of physical networks | physical multi-host required |
Conclusion: WSL2 is acceptable for *relative* microbenchmarks and flamegraphs (with controls), not for absolute tail latency or network claims. Every result carries a platform label; WSL2 and native numbers are never pooled.

## 3. Setup proposal (requires approval for each install)
1. Install Rust toolchain in WSL (rustup) matching the Windows toolchain version.
2. Keep the source in sync via a clean copy or `git worktree`-free method that does **not** touch the Windows working tree; build with `CARGO_TARGET_DIR` on the WSL ext4 filesystem (avoids slow `/mnt/d` I/O).
3. Install `build-essential`, `hyperfine`, and a profiler path: first try the distribution's `perf`; if kernel-mismatched, either build `perf` from the matching WSL kernel source or use `samply`/DHAT.
4. Pin benchmarks to a fixed core set; disable other load; record the run manifest: kernel, CPU model, microcode if available, rustc version, build profile, `RUSTFLAGS`, git SHA, seed, and the full command line.
5. Optional later: dedicated Linux host for physical multi-node validation (RISK-P2-01).
No software has been installed or configured by this plan.

## 4. Benchmark design (before any optimization)
**Rule:** acceptance criteria are written before code changes; results are compared only within a platform.

Microbenchmarks (release profile, Criterion or equivalent; dev-dependency only, justification at adoption time):
- `get_resource` hit path, 1/4/16/64 threads, Zipf and uniform keys, key and body sizes varied.
- `begin_fetch_atomic` miss, coalesced-waiter, and rejected paths.
- Eviction latency distribution per policy at shard sizes 100, 1k, 10k (the default cap is 10,000 entries/shard).
- Epoch lookup contention: hit path with 1 vs many tenants (tests `code_findings.md` F9).
- Telemetry `record` and `calculate_percentiles` under concurrent load (F10).
- `CachedResource` clone cost; hasher cost.
End-to-end (separate, labeled): load generator → gateway → controllable local origin, with a fixed seed and open-loop arrival to avoid coordinated omission.

Statistics: ≥30 independent repetitions (process restarts), warm-up excluded, medians with bootstrap 95% CIs, paired/interleaved comparisons for A-vs-B; report raw samples. Detect and record outliers instead of dropping them silently.

Acceptance of the *baseline*: run-to-run variation of the key metrics ≤ a pre-stated bound (set from a first pilot) on two separate days; otherwise the platform is declared unfit for that metric.

Hypotheses to test (not assumptions): H-F9 global epoch mutex limits hit throughput scaling; H-F8 eviction scans cause p99 stalls growing with shard size; H-F10 telemetry sorting under lock stalls requests; H-clone `String` clone in `CachedResource` is a measurable hit-path cost.

## 5. Validating the platform itself
Before trusting WSL2: run a known-answer CPU benchmark and a fixed-work spin loop 30 times on native Windows and WSL2; compare variance, pinning effects, and time-of-day effects. If WSL2 variance exceeds native or exceeds the pre-stated bound, restrict WSL2 to profiling/flamegraphs only.

## 6. Dependencies (proposal only; none added)
| Purpose | Candidate | Dev-only | Alternatives | To check at adoption |
|---|---|---|---|---|
| Microbench harness | `criterion` | yes | `divan`, custom harness | maintenance status, MSRV with rustc 1.98, license, transitive deps |
| Property tests | `proptest` | yes | `quickcheck` | same |
| Interleaving tests | `loom` | yes | `shuttle` | works only for std-sync code under cfg; scope to cache/coalescer |
| Allocation profile | `dhat` | yes | heaptrack | feature-gated |
Each addition gets an entry recording the `Cargo.toml` and `Cargo.lock` diff. No runtime dependency without separate approval.

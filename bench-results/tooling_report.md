# PhantomCache Phase 1 — Tooling & Environment Report (S3)

**Authoritative Specification:** `docs/research/phase_1_execution_package.md` v3.1 §6.6  
**Timestamp:** `2026-10-28T19:28:00Z`  
**Execution Agent:** Antigravity (AG)

---

## 1. Primary Platform (Windows Native)

| Component | Detected Version / Specification |
|---|---|
| OS | Windows 11 Home (x86_64) |
| Rust toolchain | `rustc 1.98.0 (88d9e12ae 2026-05-02)` |
| Cargo | `cargo 1.98.0 (121e749ea 2026-04-28)` |
| Python | `Python 3.14.2` |
| Node.js | `v24.5.0` |
| npm | `11.5.1` |
| Git | `git version 2.47.1.windows.2` |
| Shell | PowerShell 7 / Git Bash (MSYS2) |

---

## 2. Dev-Dependency Additions & Justification

| Crate | Version | License | MSRV | Dev-Only | Purpose | Justification & Alternatives |
|---|---|---|---|---|---|---|
| `criterion` | `0.5.1` | Apache-2.0 / MIT | 1.70 | Yes (`[dev-dependencies]`) | Statistical micro-benchmark harness | Industry standard; default features off, no unneeded plotting dependencies. Evaluated `divan` (less established for multi-threaded tokio microbenches). |
| `hdrhistogram` | `7.6.0` | MIT / Apache-2.0 | 1.65 | Yes (`[dev-dependencies]`) | Tail-latency capture for concurrency and end-to-end runs | High-dynamic-range histogram with sub-microsecond precision across 1 ns–60 s range; avoids coarse bucket quantization. |
| `proptest` | `1.12.0` | MIT / Apache-2.0 | 1.65 | Yes (`[dev-dependencies]`) | Property tests for byte accounting invariants | Provides generative test shrinking over random operation sequences for byte-budget accounting (T-C7). |
| `dhat` | *None* | — | — | — | Allocation call-site profiling | **Not added.** Per §6.6 decision, local counting `GlobalAlloc` in `benches/support/alloc_counter.rs` is used with zero added dependencies. |

### Verification of Lockfile and Dependency Constraints:
- **`Cargo.lock` additions-only:** Verified via Python AST/set comparison: initial package count 235, final package count 288, added 53 packages, removed 0 packages (`initial ⊆ final`).
- **Runtime dependencies:** `cargo tree -e normal` compared against `runtime_deps_baseline.txt`: `diff` status is 0 (identical). Zero runtime dependencies added (AC-11 satisfied).
- **Test suite:** `cargo test --all-targets -j 1 --locked` passed: 248 passed, 0 failed, 4 ignored across 20 targets.
- **Clippy:** `cargo clippy --all-targets -j 1 --locked` compiled with 0 warnings, 0 errors.

---

## 3. UI Environment (Node / npm)

- Lockfile detected: `ui/package-lock.json` (npm package manager).
- Installed packages verified (`npm ls --depth=0`):
  - `@vitejs/plugin-react@4.7.0`
  - `lucide-react@0.300.0`
  - `react-dom@18.3.1`
  - `react@18.3.1`
  - `vite@5.4.21`
- Integrity: Zero missing packages; `npm ci` was never invoked in `ui/`. Builds will use `--outDir <temp>` to protect `ui/dist/`.

---

## 4. Secondary Platform (WSL2 Environment)

| Component | Status / Detected Version |
|---|---|
| Distribution | Ubuntu on WSL2 (`6.18.33.1-microsoft-standard-WSL2`) |
| User / Home | `root` / `/root` |
| Available disk | 955 GB free on root filesystem |
| Sudo status | `SUDO_OK` (passwordless) |
| System tools | `git 2.53.0`, `python3 3.14.3`, `rsync 3.4.1` |
| Destination path | `$HOME/phantom-bench` (verified non-existent; sentinel check ready) |
| Profiling | `perf` not installed; user-space counting allocator and Criterion will be used. |

---

## 5. Proof Application Runtime Environment Was Untouched

- **Source hash equality:** All 28 files in `src/**` hashed and verified byte-identical to `original_manifest.sha256` from the S0 snapshot.
- **Repository state:** Only `Cargo.toml`, `Cargo.lock`, `tests/integration/known_gaps.rs`, `docs/research/code_findings.md`, and `docs/research/phase_1_*` files were touched or added.
- **System configuration:** Windows PATH, system registry, power plans, and environment variables were untouched (all inspection commands were read-only).


# PhantomCache Control Center Remediation — Progress Tracking

**Document:** `docs/CONTROL_CENTER_REMEDIATION_PROGRESS.md`  
**Start Date:** 2026-10-31  
**Status:** IN PROGRESS  
**Execution Standard:** Pure engineering remediation, root-cause resolution, regression protection, zero weakened tests.

---

## Baseline Capture (Pre-Implementation)

- **Git Branch:** `feat/publish-phantomcache-core`
- **Git HEAD:** `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a`
- **Git Status Counts:** 159 total entries (48 modified tracked files, 0 staged, 111 untracked).
- **Backend Build:** `cargo check --bin phantom-cache-demo` passes (exit 0).
- **Research Freeze Integrity:** `cargo test --test m6_research_evaluation_tests freeze` passes (4/4 suites pass; 56/56 cryptographic hashes verified).
- **UI Test Suite Baseline (`node --test test` in `ui/`):**
  - Total Tests: **483**
  - Passing: **473**
  - Failing: **10**
  - Cause of Failures: Exactly 10 guard tests failed due to previous interactive console additions:
    1. `test/forbidden.test.mjs:11:1`: Fixed example numbers / client metric arithmetic.
    2. `test/forbidden.test.mjs:55:1`: Inline hex colors in `InteractiveConsole.jsx`.
    3. `test/forbidden.test.mjs:70:1`: Unexpected network calls in `InteractiveConsole.jsx`.
    4. `test/lab.guard.test.mjs:20:1`: Network access outside `lib/labApi.js`.
    5. `test/render.test.mjs:165:1`: Placeholder rules violated by inline HRW calculator in `Placeholder.jsx`.
    6. `test/a11y.test.mjs:33:1`: Screen linting failures (inline styles/table caption).
    7. `test/shell.test.mjs:175:1`: Overview header regex mismatch (`Traffic` altered).
    8. `test/shell.test.mjs:189:1`: Overview header regex mismatch (`Not reported (2)` altered).
    9. `test/shell.test.mjs:238:1`: Placeholder rules in `shell.test.mjs`.
    10. `test/func.gateway.test.mjs:202:1`: Storage access outside `gatewayConfig.js` in `useHistory.js`.

---

## Work Packages Status Overview

| Work Package | Objective | Status | Blockers / Risks |
| :--- | :--- | :---: | :--- |
| **WP1** | Authoritative cache response parser (`ui/src/lib/cacheStatus.js`) | READY | None |
| **WP2** | Truthful & reproducible stampede demonstration | QUEUED | Depends on WP1 |
| **WP3** | Traffic generator complete accounting & bounded concurrency | QUEUED | Depends on WP1 |
| **WP4** | Correct HRW calculator matching Rust router & shared vectors | QUEUED | None |
| **WP5** | Fix history persistence without storage guard violation | QUEUED | None |
| **WP6** | Restore all 10 failing UI guard tests to green | QUEUED | Depends on WP1-WP5 |
| **WP7** | Make demo script an assertion-based test suite | QUEUED | Depends on WP1-WP3 |
| **WP8** | Backend peer-invalidation observability | QUEUED | Invalidation safety boundaries |
| **WP9** | Peer-fill freshness investigation | QUEUED | Multi-process setup |
| **WP10** | Lower-severity presentation & terminology fixes | QUEUED | Polish phase |

---

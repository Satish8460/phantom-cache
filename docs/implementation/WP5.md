# WP5 — Productization & Launcher Plan (Revised)

## 1. Objective
Create the single-click batch script (`Start-Phantom-Demo.bat`), port conflict managers, offline pre-flight diagnostics screens, and presentation scenario scripts.

---

## 2. Process Management & Port Safety
The launcher handles background tasks cleanly:
* **Strict Port Identification:** Scans ports `8080` (API) and `3000` (UI). If occupied, it checks process owners. If it is an orphaned Phantom process, it terminates it. If it belongs to an external application, it halts and reports a port conflict warning rather than killing the process.
* **Orphan Cleanup:** Stores background PID tracking lists during startup. On exit, all registered PIDs are terminated sequentially to prevent memory leaks.

---

## 3. Pre-Flight Diagnostics Sweep
During boot, the script performs diagnostic readiness checks:
* **Cargo / Node Path Checks:** Verifies installation paths.
* **Artifact Checks:** Confirms compiled executable binary existence and local built React assets.
* **Offline Bundling:** Confirms all CSS, font, and icon assets are hosted locally (offline-first).

---

## 4. Startup Readiness Sequence
The launcher boots components sequentially, using endpoint health status checks rather than static sleep timers:
1. **Simulation API Boot:** Launches the background server.
2. **Health check Sweep:** Polls `http://localhost:8080/health` until returning HTTP `200`.
3. **UI Web server Boot:** Launches local Vite or static host.
4. **Browser Launch:** Opens default browser on port `3000`.

---

## 5. Implementation Files
* `Start-Phantom-Demo.bat`
* `docs/demo/README.md`
* `docs/implementation/WP5_GUIDE.md`

## 6. Completion Criteria
* `Start-Phantom-Demo.bat` coordinator script active.
* Process tracking and port safeguards verified.
* Offline asset bundling complete.

## Status
* **COMPLETED**

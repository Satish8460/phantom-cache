> Notice (2026-10-30): Historical work-package specification from the first (simulation-era) phase. "WP1" to "WP5" here are not the "WP-nn" work packages of `docs/research/work_packages.md`. The plan describes intended work, not verified current behaviour: for example, PIE canary rollouts and Count-Min sketch estimation are described here, but `src/control/pie.rs` is a legacy threshold stub (`pie: None` at `src/main.rs:433`) and `CountMinSketch` is used only by a unit test (`tests/unit/wp2_tests.rs`). Current status and evidence: [docs/STATUS.md](../STATUS.md), [handoff CORRECTIONS](../handoff/CORRECTIONS.md).
>
> Original text below is unchanged.

# WP4 Guide — Phantom Control Center UI

This guide details how to verify and run the React-based frontend dashboard connected to the Axum SSE server.

---

## 1. Running the API Server (Rust)
Ensure the server is running on port `8080` to stream simulated telemetry states:
```bash
cargo run --bin phantom-cache-demo
```

---

## 2. Launching the Control Center UI (React)
Move to the `ui/` subdirectory, install dependencies, and start the development server:
```bash
cd ui
npm install
npm run dev
```

The console opens automatically on `http://localhost:3000`.

---

## 3. Disconnection Verification
If the Rust server exits, the status indicator in the top header immediately transitions from green **CONNECTED** to red **DISCONNECTED**, verifying telemetry connection state.

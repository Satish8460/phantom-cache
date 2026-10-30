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

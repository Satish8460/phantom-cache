# Deferred integration items (M7)

## Done after the coordinator released `src/api/server.rs`

- `GET /health/live` and `GET /health/ready` are served by `src/health.rs` and merged in `create_app` with one line (`.merge(crate::health::router(crate::health::global()))`). `/health` is unchanged. Tests: `tests/integration/m7_health_tests.rs`. `main.rs` marks `config_valid` after configuration validation, `listening` after a successful TCP self-connect to the ingress address, and begins draining on a shutdown signal.
- Origin cap configuration (`PHANTOM_ORIGIN_CAP_*`) is validated by `src/config.rs` and applied in `main.rs` through `DefaultPolicyExecutor::configure_origin_cap`.

## Still open (need edits outside the M7 file set)

| # | Item | Exact change | Owner of target file |
|---|---|---|---|
| D-1 | Graceful shutdown with connection draining | `start_server` should take a shutdown future (or read `health::global()` draining) and use `axum::serve(listener, app).with_graceful_shutdown(fut)` / `axum_server::Handle::graceful_shutdown(Some(Duration))` for the TLS branch, with a bounded deadline. `main.rs` currently drops the server future on signal, which closes the listener but not accepted connections. A test must open a slow request, send the shutdown, and assert completion within the deadline. | `src/api/server.rs` |
| D-2 | Sweeper shutdown channel | `start_server` creates `let (_tx, rx) = watch::channel(false)` and drops `_tx` at the end of the `if let`; `PartitionedCache::spawn_sweeper_task` then has a `shutdown.changed()` branch that is always ready (`Err`), which probably spins the task. Keep `_tx` alive for the server lifetime (return it or store it in `ApiState`) and break the loop on `Err`. Verify with an idle-CPU measurement before and after. | `src/api/server.rs`, `src/core/cache.rs` |
| D-3 | Honor `PHANTOM_DEFAULT_TTL_MS` | Replace the literal `60_000` at the two call sites in `src/api/server.rs` (freshness evaluation) with a value read from shared state/config. `config.rs` already validates the key and marks it not honored. | `src/api/server.rs` |
| D-4 | ~~Authentication for `/api/v1/intelligence/status` and `/audit`~~ **DONE (SEC-M7-02, Alternative A)**: Observer role via `require_observer_if_secure`, audit pagination (`limit` 1..=100 default 50, `offset`), tests `m7_sec_fixes_tests::sec02_*`. Independent review pending. | done | `src/api/server.rs` |
| D-5 | ~~Anonymous GET in secure mode~~ **DONE (SEC-M7-01, Alternative A)**: `cache:read` required (401/403) before any cache or origin work; tests `m7_sec_fixes_tests::sec01_*`. Independent review pending. | done | `src/api/server.rs` |
| D-6 | ~~CORS policy~~ **DONE (SEC-M7-03, Alternative A)**: `PHANTOM_CORS_ORIGINS` allowlist, same-origin by default, dev profile lists the Vite origins; tests `m7_sec_fixes_tests::sec03_*`, `m7_config_tests::cors_origins_*`. Independent review pending. | done | `src/api/server.rs`, `src/config.rs`, `src/main.rs` |
| D-7 | `/metrics` scrape export and runtime kill switch | Owner decision O7-4; not implemented. | `src/api/server.rs` |
| D-8 | Server-side panics (`expect`) in `start_server` | `start_server` panics on bind/TLS failures; `main` maps any panic to exit 1. Returning `Result` would give clean messages. | `src/api/server.rs` |

# Phantom Lab HTTP API — contract (D-4)

**Status:** SPECIFICATION (2026-10-30). Implemented only where a later change log says so. The API starts the *same* experiment implementations used by tests and the `phantom-lab` binary (`phantom_cache_demo::lab` for T1 simulation, `phantom_cache_demo::lab_live::scenario_b` for the T2 real-gateway experiment). It never contains a second implementation or canned results.

## 1. Principles
- Every value carries a tier: `SIMULATED` (T1), `MEASURED_LOOPBACK` (T2), `RETAINED_*` (read from a retained artifact), or `UNAVAILABLE` with a reason. Missing is never zero.
- Sealed seeds are never exposed: `seed_set: "S"` is refused (422), the process refuses to enable the API if `PHANTOM_SEALED_SALT_HEX` is in its environment (the library validator would read it), and no response field can contain a seed of S.
- Security follows the gateway policy: with JWT enabled, reads need `Observer`, starting/cancelling runs needs `Operator`; in dev mode (no JWT) routes are anonymous and the capabilities response says so. No credentials in URLs (the SSE stream is consumed with an `Authorization` header via fetch streaming; the `EventSource` URL form is not supported).
- CORS follows `PHANTOM_CORS_ORIGINS`; error bodies are generic.

## 2. Routes (all under `/api/v1/lab`)
| Method + path | Role | Purpose |
|---|---|---|
| `GET /capabilities` | Observer | which run kinds can run here and why not: `t1_matrix` (library runner; dev/tuning seeds only), `scenario_b` (needs the gateway binary path; reports `available:false, reason` if missing, debug-build label if applicable), `max_concurrent_runs: 1`, limits, `security_mode` |
| `GET /evidence` | Observer | index of retained experiments found under `lab-results/` with kind, tier, verdict snapshot, timestamps, artifact list; includes the M6 sealed result (`m6_sealed_*`, verdict `NOT_SUPPORTED`) and dry runs as read-only evidence |
| `GET /evidence/{id}` | Observer | parsed `verdict.json`/`summary.json` + manifest provenance for one retained experiment |
| `POST /runs` | Operator | start a run: `{kind, spec}`; validated; `202 {run_id}`; `409` if a run is active; `422` on invalid/forbidden spec |
| `GET /runs` | Observer | persisted run index (newest first, paginated like the audit endpoint) |
| `GET /runs/{id}` | Observer | state (`queued|running|cancelling|completed|failed|cancelled|interrupted`), spec, progress, timestamps, exit status, summary link |
| `POST /runs/{id}/cancel` | Operator | cooperative cancel; terminates children; idempotent |
| `GET /runs/{id}/events` | Observer | SSE stream (see section 3) |
| `GET /runs/{id}/artifacts` | Observer | allowlisted artifact names with sizes and SHA-256 |
| `GET /runs/{id}/artifacts/{name}` | Observer | download one allowlisted artifact (size-capped, no traversal) |

## 3. Event stream
`text/event-stream`; each event has `id` (monotonic per run), `event` (`state`, `progress`, `result`, `done`, `error`) and a JSON `data` carrying `tier` where a number appears. `Last-Event-ID` resumes from the persisted event log (`events.jsonl`); a closed run replays then ends with `done`. Heartbeat comment line every 15 s. The server never emits an event it did not receive from the runner, with three stated exceptions that are state marks, not results: `queued` (run creation), `cancelling` (written when a cancel request is accepted) and, after a restart, `interrupted` followed by `done` (marked `source: server_recovery_on_startup`; not duplicated if the log already ends with `done`); a runner crash yields `state: interrupted/failed`, never a synthetic `completed`.

## 4. Persistence and restart
`lab-results/runs/<run_id>/` holds `run.json` (atomic write: spec, state, timestamps, exit code), `events.jsonl`, and the runner's artifacts. `run_id` = UTC timestamp + random suffix (no seeds). On startup, any run found in `running|queued|cancelling` is rewritten to `interrupted` with the restart time; nothing is resumed or fabricated. Index reads are bounded (newest 200).

## 5. Limits and validation
Body <= 16 KiB; `t1_matrix`: seed set in {T, D} only, <= 30 seeds, scenario/family/arm ids from the runner's catalogue, estimated cell count <= 20,000; `scenario_b`: repetitions 1..=30 (VALID label needs >= 10 and a release gateway), phase durations 1..=60 s, arrival rate and catalogue within documented bounds, arms from {BASE, STATIC-CAP, AIMD-CAP}. One active run; artifact download <= 64 MiB; unknown JSON fields rejected.

## 6. What the UI may rely on
Responses are versioned (`"schema": "lab-1"`); every metric object has `{v, unit, tier}` or `{unavailable:true, reason}`; `GET /evidence` is the only source for historical results (the UI never embeds results); the UI renders negative and inconclusive verdicts identically to positive ones.

## Change log
- **2026-10-30 (G2 implementation).** Implemented in `src/api/lab_api.rs` (declared in `src/api/server.rs`); user documentation `docs/user/lab_api.md`. Deviations from the specification above, all deliberate:
  1. Routes are added by `create_app_with_lab(state, cors, Some(service))`; `create_app`/`create_app_with_cors` delegate with `None`, so no lab route exists unless `PHANTOM_LAB_ENABLED=true` (404 otherwise). `main` installs the service in a process-wide `OnceLock` read by `start_server_with_cors`.
  2. Both POST routes require `Content-Type: application/json` (415) as a cross-site request guard.
  3. `scenario_b` accepts no `seed` (default seed only) and exposes phase lengths as `*_s` integer seconds; `t1_matrix` cannot set `out_dir`/`overwrite`.
  4. `GET /evidence*` returns artifact names and sizes only (no downloads); `evidence/{id}` ids are directory names, Scenario B ones are `scenario_b.<utc>`.
  5. Progress events of `scenario_b` carry the current `gateway_pid`; T1 progress events are throttled (about 5 per second).
  6. Restart recovery covers the newest 200 runs and appends `state`/`done` events marked `source: server_recovery_on_startup`.
  7. The run slot is released before the terminal state is written to `run.json`.
  8. `Last-Event-ID` was added to the CORS allowed request headers.
  9. D2: origin-cap saturation answers 503 with `X-Cache-Error: CapacityExceeded` (kept), `X-Cache-Error-Reason: OriginCapSaturated`, `Retry-After: 1` and its own body text; implemented as a marker carried in `OriginError::NetworkError` because `OriginError` lives in the frozen `src/core/coalesce.rs`.
- **2026-10-30 (G2b, review findings).** Anonymous mode adds a Host/Origin guard (421/403); one shared run handle serialises run.json and events.jsonl (no duplicate ids, monotone state machine, unique tmp files); gateway children are tied to the process lifetime by a job object on Windows and leftover pids are reaped on recovery only after an image-path and start-time check; Scenario B label rules tightened (profile from the directory above the executable, workload tolerance); stored artifact hashes, streamed downloads, SSE cap 16, body timeout 408; W-Z alpha range; server paths redacted from served JSON; the `log` event kind is not emitted and is removed from this contract; the API refuses to start with PHANTOM_SEALED_SALT_HEX set. Details: `docs/user/lab_api.md`.

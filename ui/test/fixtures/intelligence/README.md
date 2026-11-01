Fixtures for the Intelligence screen (lane L2).

- status_legacy_real.json is a REAL response of GET /api/v1/intelligence/status from the release binary D:\ClaudeTemp\audit\phantom_rev6.exe
  (secure mode, token with telemetry:read), captured 2026-10-31 on loopback. On that build /registry, /signals and /decisions answer 404.
- Every other file is CONTRACT-SHAPED: hand-written from docs/lanes/CONTRACT-pie-api-v1.md (lane L3). They were NOT captured from a running
  gateway, because no gateway build serving the v2 API existed when this lane ran. Numbers in them are test data only and never ship in the bundle.
  Replace them with real captures once lane L4 serves the endpoints: node scripts/capture-pie-fixtures.mjs <gateway> <tokens.json> <out-dir>.

#!/bin/sh
# Smoke test of an installed bundle: --version, --check-config on the shipped example,
# fail-fast on a bad value, then start the server on a loopback port and probe /health.
# Usage: packaging/smoke-test.sh <path-to-bundle-dir> [port]
set -eu
DIR=${1:?bundle directory required}
PORT=${2:-18085}
BIN="$DIR/phantom-cache-demo"

"$BIN" --version
"$BIN" --config "$DIR/phantom.example.conf" --check-config >/dev/null
echo "check-config on example: ok"

if PHANTOM_REQUIRE_AUTH=yes "$BIN" --check-config >/dev/null 2>&1; then
  echo "FAIL: invalid boolean was accepted" >&2; exit 1
fi
echo "invalid configuration rejected: ok"

PHANTOM_BIND_ADDR="127.0.0.1:$PORT" "$BIN" >"$DIR/smoke.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null || true' EXIT
i=0
until curl -fsS "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; do
  i=$((i+1)); [ "$i" -gt 60 ] && { echo "FAIL: server did not become healthy" >&2; exit 1; }
  sleep 0.5
done
echo "/health: $(curl -sS "http://127.0.0.1:$PORT/health")"
echo "/health/live: $(curl -sS -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/health/live")"
echo "/health/ready: $(curl -sS -o /dev/null -w '%{http_code}' "http://127.0.0.1:$PORT/health/ready")"
echo "smoke test passed"

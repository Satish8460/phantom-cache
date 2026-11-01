#!/bin/sh
# Reproduce the test suite and summarize it (POSIX sh). New in M7; does not modify the tree.
# Usage: scripts/reproduce_tests.sh [log-dir]    (default: ./repro-logs)
# Honors CARGO_TARGET_DIR. Always uses -j 1 (project rule).
set -u
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
LOGDIR=${1:-$ROOT/repro-logs}
mkdir -p "$LOGDIR"
LOG="$LOGDIR/cargo_test_all_targets.log"
{
  echo "date_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "rustc=$(rustc --version)"
  echo "head=$(git rev-parse HEAD 2>/dev/null || echo n/a)"
  echo "dirty_files=$(git status --short 2>/dev/null | wc -l | tr -d ' ')"
} > "$LOGDIR/environment.txt"
cargo test --all-targets -j 1 > "$LOG" 2>&1
RC=$?
echo "cargo test exit code: $RC"
echo "targets with results: $(grep -c '^test result' "$LOG")"
echo "passed:  $(sed -n 's/^test result: [a-z]*\. \([0-9]*\) passed.*/\1/p' "$LOG" | awk '{s+=$1} END {print s+0}')"
echo "failed:  $(sed -n 's/.* \([0-9]*\) failed;.*/\1/p' "$LOG" | awk '{s+=$1} END {print s+0}')"
echo "ignored: $(sed -n 's/.* \([0-9]*\) ignored;.*/\1/p' "$LOG" | awk '{s+=$1} END {print s+0}')"
echo "full log: $LOG"
echo "NOTE: cargo test --all-targets also executes the harness=false bench mains; see docs/release/ci_recommendations.md"
exit $RC

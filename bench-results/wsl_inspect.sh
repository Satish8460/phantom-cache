#!/usr/bin/env bash
set -euo pipefail
echo "=== Checking apt packages in WSL2 ==="
for pkg in build-essential pkg-config git python3 rsync; do
    dpkg -s "$pkg" 2>/dev/null | grep -E "Package:|Version:|Status:" || echo "$pkg: not installed"
done

echo "=== Checking rustc / cargo in WSL2 ==="
which rustc || echo "rustc not found"
which cargo || echo "cargo not found"
rustc --version 2>/dev/null || true
cargo --version 2>/dev/null || true

echo "=== Checking perf in WSL2 ==="
which perf || echo "perf not found"


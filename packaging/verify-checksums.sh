#!/bin/sh
# Verify a bundle directory against its SHA256SUMS (POSIX sh).
# Usage: packaging/verify-checksums.sh <bundle-dir>
set -eu
DIR=${1:?bundle directory required}
cd "$DIR"
if command -v sha256sum >/dev/null 2>&1; then sha256sum -c SHA256SUMS; else shasum -a 256 -c SHA256SUMS; fi

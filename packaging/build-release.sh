#!/bin/sh
# PhantomCache release build (POSIX sh). Produces dist/phantom-cache-<version>-<target>/ and a
# .tar.gz with SHA256SUMS. Does NOT commit, tag, push or publish anything.
#
# Environment (all optional):
#   CARGO_TARGET_DIR     build directory (use a fresh one for a clean-room build)
#   PHANTOM_BUILD_JOBS   cargo -j value (default 1)
#   PHANTOM_DIST_DIR     output directory (default ./dist)
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

JOBS=${PHANTOM_BUILD_JOBS:-1}
DIST=${PHANTOM_DIST_DIR:-$ROOT/dist}

VERSION=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -n 1)
TARGET=$(rustc -vV | sed -n 's/^host: //p')

# Version stamping: embedded at compile time via option_env! (see src/config.rs).
if command -v git >/dev/null 2>&1 && git rev-parse HEAD >/dev/null 2>&1; then
  COMMIT=$(git rev-parse HEAD)
  if [ -n "$(git status --porcelain 2>/dev/null)" ]; then COMMIT="${COMMIT}-dirty"; fi
else
  COMMIT=unknown
fi
BUILD_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)
export PHANTOM_BUILD_COMMIT="$COMMIT"
export PHANTOM_BUILD_DATE="$BUILD_DATE"

echo "Building phantom-cache-demo $VERSION ($COMMIT) for $TARGET with $(rustc --version)"
cargo build --release --locked -j "$JOBS" --bin phantom-cache-demo

BIN_DIR=${CARGO_TARGET_DIR:-$ROOT/target}/release
NAME="phantom-cache-$VERSION-$TARGET"
STAGE="$DIST/$NAME"
rm -rf "$STAGE"
mkdir -p "$STAGE/docs"

cp "$BIN_DIR/phantom-cache-demo" "$STAGE/"
cp packaging/phantom.example.conf packaging/phantom.production.example.conf "$STAGE/"
[ -f LICENSE ] && cp LICENSE "$STAGE/"
cp -R docs/user "$STAGE/docs/user"
cp -R docs/release "$STAGE/docs/release"
cp Cargo.lock "$STAGE/Cargo.lock.snapshot"
{
  echo "version=$VERSION"
  echo "commit=$COMMIT"
  echo "build_date_utc=$BUILD_DATE"
  echo "target=$TARGET"
  echo "rustc=$(rustc --version)"
  echo "cargo=$(cargo --version)"
  echo "cargo_flags=build --release --locked -j $JOBS"
} > "$STAGE/BUILD_INFO.txt"

( cd "$STAGE" && find . -type f ! -name SHA256SUMS | sort | while read -r f; do
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$f"; else shasum -a 256 "$f"; fi
  done > SHA256SUMS )

( cd "$DIST" && tar -czf "$NAME.tar.gz" "$NAME" )
( cd "$DIST" && if command -v sha256sum >/dev/null 2>&1; then sha256sum "$NAME.tar.gz"; else shasum -a 256 "$NAME.tar.gz"; fi > "$NAME.tar.gz.sha256" )

echo "Release bundle: $DIST/$NAME.tar.gz"
echo "Checksum file:  $DIST/$NAME.tar.gz.sha256"
"$STAGE/phantom-cache-demo" --version

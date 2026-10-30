# Install

## Option A: release bundle

A bundle `phantom-cache-<version>-<target>.zip` (Windows) or `.tar.gz` (POSIX) contains `phantom-cache-demo[.exe]`, two example configurations, `BUILD_INFO.txt`, `SHA256SUMS`, `Cargo.lock.snapshot` and these docs. Bundles are built with `packaging/build-release.*`; see [../release/reproduction_guide.md](../release/reproduction_guide.md).

1. Verify the archive against its `.sha256` file.
   - POSIX: `sha256sum -c phantom-cache-<version>-<target>.tar.gz.sha256`
   - PowerShell: compare `(Get-FileHash -Algorithm SHA256 <zip>).Hash` with the value in the `.sha256` file (case-insensitive).
2. Extract, then verify the contents: `packaging/verify-checksums.sh <dir>` (or `sha256sum -c SHA256SUMS` inside the directory).
3. Run `phantom-cache-demo --version`, then `phantom-cache-demo --config phantom.example.conf --check-config`.
4. Optional smoke test: `packaging/smoke-test.sh <dir>` or `packaging\smoke-test.ps1 -BundleDir <dir>`.

Checksums prove integrity relative to the checksum file you trust; they do not prove who built the bundle (no signing is provided).

## Option B: build from source

Requirements: a Rust toolchain (the reference build used rustc 1.98.0; the README states 1.75+ but only 1.98.0 has been used), and on Windows the MSVC linker. Several GB of disk for the build directory; the reference Windows machine needs `-j 1`.

```
cargo build --release --locked -j 1
target/release/phantom-cache-demo --version
```

The dashboard UI (`ui/`) is optional and separate: `cd ui && npm ci && npm run build` (Node.js 18+).

## Supported platforms

Only Windows x86-64 (development machine) has been exercised by the project's own runs. Linux and macOS build from the same source but are untested here.

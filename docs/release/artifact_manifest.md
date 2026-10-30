# Release artifact manifest

A release produced by `packaging/build-release.sh` or `packaging/build-release.ps1` contains:

| Path (inside `phantom-cache-<version>-<target>/`) | Origin | Purpose |
|---|---|---|
| `phantom-cache-demo[.exe]` | `cargo build --release --locked -j 1`, stamped with `PHANTOM_BUILD_COMMIT` / `PHANTOM_BUILD_DATE` | The gateway binary |
| `phantom.example.conf` | `packaging/` | Development example |
| `phantom.production.example.conf` | `packaging/` | Network deployment example (no secrets) |
| `LICENSE` | repository root, if present | MIT license text |
| `docs/user/**` | repository | User documentation |
| `docs/release/**` | repository | Reproduction guide, templates |
| `Cargo.lock.snapshot` | copy of `Cargo.lock` at build time | Dependency pin for audit/reproduction |
| `BUILD_INFO.txt` | build script | version, commit (with `-dirty` suffix if the tree had uncommitted changes), UTC build date, target, rustc and cargo versions, cargo flags |
| `SHA256SUMS` | build script | SHA-256 of every file above except itself |

Next to the bundle: `<bundle>.tar.gz` or `.zip` and `<bundle>.sha256` (SHA-256 of the archive).

Not included: UI build (`ui/dist`; build separately with `npm ci && npm run build`), third-party license notices (not generated; owner decision O7-2), SBOM, signatures, container image.

## Per-release record (fill in)

| Field | Value |
|---|---|
| Archive name | |
| Archive SHA-256 | |
| Binary SHA-256 | |
| Commit / dirty flag | |
| Build host (OS, CPU), toolchain | |
| Build twice and compare binary hashes (reproducibility) | identical / different (explain) |
| Verification run (reproduction_guide.md section 5) | |

# PhantomCache release notes: <version> (TEMPLATE, not a release)

Fill every section from evidence. Delete a section only if it is genuinely empty. An agent does not tag, commit or publish; the owner does.

- **Version / commit / build date (UTC):** from `phantom-cache-demo --version` and `BUILD_INFO.txt`
- **Release variant:** MVP-A or MVP-B as recorded by the owner (D: <owner decision, link>)
- **Platforms built and tested:** list exactly what was run (OS, arch, toolchain). Untested platforms are listed as untested.

## Changes
- (reference commits or the diff; group as added / changed / fixed / removed)

## Supported configurations
- Link `docs/user/configuration.md`. State the tested profile(s): development / production, TLS on/off, single node / cluster (cluster is experimental).

## Claims and evidence
Every performance, freshness, safety or intelligence statement needs a row. No row, no statement.

| Statement | Label (measured / simulated / inferred / literature) | Conditions (release build, hardware, OS, workload, seed, repetitions, interval) | Evidence file |
|---|---|---|---|
| | | | |

Do NOT include: adaptive-controller benefit (not established), "first"/"novel" claims beyond the recorded novelty snapshot, numbers from debug builds, single runs or simulations presented as measurements.

## Known issues
- Copy open items from `docs/release/deferred_integration.md` and `docs/security/m7_security_review.md`.

## Security
- Open findings by severity and owner disposition (accepted / fixed).

## Upgrade and rollback
- Stateless in-memory cache: restart semantics, cold-start effect, configuration changes since the previous version (`--check-config` output diff).

## Artifacts
- File names, sizes and SHA-256 (see `artifact_manifest.md`). Signing status: unsigned.

## Verification record
- Commands, exit codes and test counts as run for this release (see `reproduction_guide.md`). State anything not verified.

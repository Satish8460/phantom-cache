# M6 Pre-registration — Amendment 1 (protocol v2.1)

**Authored:** 2026-10-30 (local, +05:30), by the technical lead. **Amends:** `preregistration_m6_v2.md` (unchanged, byte-identical, still pinned by SHA-256) and supersedes the validity of `preregistration_m6_v2_freeze.json` (preserved unchanged as historical evidence; its SHA-256 is pinned in the v2.1 freeze record). This amendment is a dated addition. It does not rewrite or backdate anything.

## 1. Why this amendment exists
The v2 freeze record (`frozen_at_utc` 2026-10-29T14:09:47Z) hashes the lab code, `src/core/*`, `src/intelligence/*`, the M6 test file and the tuning artifacts. After it was written, those files changed legitimately (correctness fixes, the lab extraction, lint fixes). `verify_freeze` therefore correctly refuses to run the sealed evaluation against the v2 record. The v2 salt was also generated inside an earlier agent session. Rather than edit the v2 record, protocol v2.1 adds a new record, a new sealed-salt commitment and a stricter verifier.

## 2. What changed since the v2 freeze (frozen-file inventory)
| Area (files in the freeze inventory) | Change since 2026-10-29T14:09:47Z |
|---|---|
| `src/core/cache.rs` | fenced metadata setters and 304 completion (S-1, S-2, S-9), guard-conditioned delete, sweeper retention inside stale windows (H2) and interval clamp, saturating TTL arithmetic, poisoned-shard recovery, key validation rules (control characters, dot segments, 4096-byte canonical cap, tenant alphabet), background-slot guard |
| `src/core/coalesce.rs`, `src/core/freshness.rs` | new origin error variants, parked freshness metadata installed by `finalize_fetch`, RFC 9111 304 merge |
| `src/intelligence/executor.rs` | typed `OriginCapSaturated` error and permit API (legacy wrapper kept) |
| `src/lab/**`, `tests/support/lab/**` | extraction of the runner into the library (`phantom-lab`), lint fixes, freeze verification v2.1, sealed commitment and derivation domain |
| `tests/integration/m6_research_evaluation_tests.rs` | lint fixes; freeze tests rewritten for v2.1 (valid path and 13 fail-closed paths) |
| `docs/research/preregistration_m6_v2.md` | **unchanged** |
| Hypotheses, endpoints, baselines, analysis plan, success criteria | **unchanged** since v2. No change was made after seeing any sealed-set data (none exists) |

Evidence that the T1 simulation's behavior was not altered by these changes (scope: what was actually run, debug and release, loopback-free virtual time): (a) a fixed smoke matrix (3 development seeds, O-DEG/O-BRN, W-DI/W-Z, 5 arms) produced byte-identical `cells.csv`, `fetches.csv` and `requests.csv` (SHA-256 prefixes `9f4b48ce5fe0b614`, `9ceea345848ed7c6`, `1aede70e8fd6c7ad`) before the lab extraction, after it, after the cache/stale fixes, and again after the cluster, validator-forwarding and security changes (checked 2026-10-30 ~03:5x +05:30); (b) the tuning run on tuning seeds T was repeated in the **release** profile on 2026-10-29T22:22:41Z and produced a byte-identical `tuning_trials.csv` (487 rows) and identical `frozen_arm_params.json` parameters compared with the original run of 2026-10-29T14:06:23Z (debug). This supports, but does not prove, that no T1-relevant behavior changed; it does not cover configurations not run.

## 3. Seed exposure, salt retirement and limitations
- **Burned (v1):** the plaintext range 30001..=30030 is public and was executed against synthetic data. It is rejected by every leak check. Unchanged from v2.
- **Retired (v2 salt):** commitment `bc7c9683fb46b1e4d8f3b2c01612668c9591427402e49c576220bba3724f0e75`. The salt was generated inside an earlier agent session. It was never used to derive an evaluation seed (only for a repository leak scan that printed nothing). It is declared exposed and retired. It is not claimed to have been secret or pristine.
- **New (v2.1 salt):** generated from the operating-system CSPRNG (.NET `RandomNumberGenerator`) at **2026-10-29T22:12:07Z (2026-10-30 03:42:07 +05:30)**, by the technical lead's process, never printed or logged, stored outside the repository at `D:\Personal\.phantom-sealed-m6\m6_salt_v2_1.hex` with an owner-only ACL. Only its SHA-256 commitment `60167cd235911e0c10377b68fc725c6c6a892f41480707482f43ec9578f667cc` is in the repository (`SEALED_SALT_COMMITMENT_SHA256`). The derivation domain string changed to `phantomcache-m6-sealed-v2.1`.
- **Limitation (stated, not hidden):** the file is readable by any process running as the same Windows user, including every agent session on this machine. The protection is process discipline (nobody reads it; the sealed run receives it only through the `PHANTOM_SEALED_SALT_HEX` environment variable at run time and the numeric seeds are never written), not an enforced security boundary. The salt was also created by an agent process, so a reader who requires a human-generated secret should regenerate it, update the commitment and re-amend before relying on the sealed run.
- **Status of sealed material at freeze:** no sealed seed has been derived or used by any run; the leak scan has not found any numeric sealed seed in the repository.

## 4. Validity of historical results
| Result | Status under v2.1 |
|---|---|
| v1 M6 results (`m6_research_evaluation.md`, completion report) | **RETRACTED** (synthetic arithmetic); unchanged banners |
| v2 tuning on seeds T (original, 2026-10-29T14:06:23Z) | Preserved in `lab-results/tuning_v2_original/`; reproduced byte-identically in release on 2026-10-29T22:22:41Z; the re-run files in `lab-results/tuning/` are the ones frozen here |
| Development-seed smoke, sensitivity checks, candidate dispositions (`lab-results/smoke_*`, `candidates_m5/`) | **Exploratory only.** Produced with pre-fix code and a v2 manifest (they carry the retired commitment). Not evidence for H1-H6. The candidate dispositions must be recomputed on current code before being cited |
| Any T2/loopback or product-path claim | Not covered by this protocol (M6 is Tier 1, SIMULATED) |

## 5. Procedure (unchanged from v2 except the verifier)
`m6_full_sealed_evaluation` (ignored test, release profile) calls `verify_freeze`, which now fails closed unless: the v2.1 record exists and `protocol_version` equals the compiled `2.1`; the v2 preregistration, this amendment and the original v2 freeze record still hash as recorded; the recorded commitment equals the compiled one; a timestamp is present; and every listed file hash matches. It then derives seeds from `PHANTOM_SEALED_SALT_HEX` (rejecting any salt not matching the commitment), scans the repository for numeric sealed seeds, loads the frozen tuning artifact, runs the pre-registered matrix, and analyses the raw per-cell data (paired by seed, BCa 95% CI, sign-flip permutation test, Holm adjustment). No result is interpreted as a claim until `claims_ledger.md` is updated from the retained artifacts. Negative and inconclusive outcomes are reported identically.

## 6. Not claimed
This amendment does not claim that the adaptive controller works, that T1 results transfer to a real gateway, that the old freeze was valid after the files changed, or that the sealed salt is a strong secret. Overhead hypotheses (H3, H5, H6) remain not testable in T1.

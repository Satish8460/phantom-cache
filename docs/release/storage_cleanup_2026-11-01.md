# Storage audit and cleanup, 2026-11-01

**Status:** completed for the authorised scope. Linked to board D-060 and section J8. Append dated amendments rather than editing. Earlier record: `docs/research/storage_cleanup_2026-10-30.md`.
**Evidence tags:** all sizes are measured file-size sums (logical bytes) taken in the session; free-space figures come from the drive and are the ones to rely on.

## 1. Result
| Measure | Value |
|---|---|
| D: free before | 12.37 GB |
| D: free after | 198.82 GB |
| **Recovered (measured free-space gain)** | **186.45 GB** |
| Logical bytes of deleted items | 200.0 GB (the gap is the same unexplained logical-vs-free difference recorded on 2026-10-30; cause not established) |

## 2. Deleted (15 items, all regenerable)
Each was a Cargo target directory (top level only `debug`, `release`, `tmp`, `.rustc_info.json`, `CACHEDIR.TAG`; no unusual files in `debug`/`release`/`tmp`) with no running cargo, rustc, node or phantom process, and none referenced by a launcher. Exception: the zip, see below. Per-item size and free-space trace: `D:\ClaudeTemp\audit\storage_cleanup_2026-11-01.log`.

| Path | Files | GB |
|---|---|---|
| `D:\Personal\_m7_target` | 35,176 | 33.937 |
| `D:\Personal\_sbx_m3_target` | 33,951 | 37.102 |
| `D:\Personal\_sbx_m3rev_target` | 18,026 | 21.203 |
| `D:\Personal\_sbx_u01int_target` | 22,258 | 21.131 |
| `D:\Personal\_sbx_u01rev_target` | 4,203 | 4.187 |
| `D:\Personal\_sbx_u01_target` | 4,122 | 4.085 |
| `D:\Personal\_sbx_race_target` | 3,784 | 3.869 |
| `D:\Personal\_dr029_target` | 3,614 | 3.414 |
| `D:\Personal\_sbx_harden_target` (empty) | 0 | 0 |
| `D:\Personal\_m7_target.zip` | 1 | 6.927 |
| `D:\ClaudeTemp\impl\target` | 24,617 | 23.576 |
| `D:\ClaudeTemp\audit\target` | 22,131 | 21.761 |
| `D:\ClaudeTemp\lanes\target_L4` | 14,359 | 16.645 |
| `D:\ClaudeTemp\lanes\target_L3` | 5,482 | 2.718 |
| `D:\Temp\cf_old_target` | 4,579 | 4.181 |

`_m7_target.zip` was a truncated archive (valid local header, no central directory, so standard tools cannot open it). Its first entries show the root `_m7_target/`, the same build directory deleted above; this is an inference from the first two entries only, not a full listing. The earlier cleanup record already called it a corrupt, unidentified archive.

Consequence: recorded binary hashes in the ledgers remain as text; the deleted binaries (for example the `_m7_target\release\phantom-lab.exe` named in `docs/demo/scenario_b_runbook.md`) are gone and must be rebuilt (rebuilds are not bit-identical).

## 3. Preserved (large or uncertain)
| Path | GB | Why kept |
|---|---|---|
| `D:\Temp\sbx_m3fix` (includes `target`, 16.3 GB) | 16.6 | Protected M3 sandbox |
| `D:\Personal\_sbx_m3fix_target`, `_fp`, `_fprel`, `_rel` | 10.7 | Build caches of the protected M3 sandbox; ownership boundary uncertain |
| `D:\Personal\phantom-cache\target` (`debug` 8.67, `release` 0.62) | 9.3 | `release` holds the launcher binary (`target\release\phantom-cache-demo.exe`); `debug` needed for tests; regenerable but kept to avoid disrupting Phase 1 |
| `D:\docker` (`docker_data.vhdx` 53.5, `ext4.vhdx` 0.09) | 53.6 | Docker Desktop WSL disk; the daemon is not running and the repo has no Dockerfile or compose file, so ownership could not be established |
| `D:\DeveloperCaches` (Gradle) | 10.9 | Not PhantomCache |
| `D:\Personal\phantom-cache-backups` | 1.5 | Recovery copies |
| `D:\Personal\phantom-cache-snapshots`, `.phantom-sealed-m6`, `phantom-cache-academic` | 0.15, 0, 0.27 | Protected |
| `D:\ClaudeTemp\audit` (evidence, `phantom_rev*.exe`, tools), `impl\backups`, `backups`, `snapshots`, `docs_work` | about 2 | Evidence, recovery copies, demo binaries |
| `D:\Temp\sbx_*` source copies, `D:\Temp\*` browser profiles | about 3 | Small, ownership partly uncertain |
| Root files `D:\h1.json`, `h2.json`, `rr.patch`, `wtb_*.js` and the other root documents | small | Unidentified or not PhantomCache |

## 4. Decisions for the owner
1. `D:\Personal\_sbx_m3fix_target*` (10.7 GB) and `D:\Temp\sbx_m3fix\target` (16.3 GB): regenerable build output of the M3 sandbox, kept because the sandbox is protected. Say the word to remove just the target directories.
2. `phantom-cache\target\debug` (8.67 GB): regenerable; removing it makes the next test run rebuild.
3. `D:\docker\...\docker_data.vhdx` (53.5 GB): start Docker Desktop and run read-only `docker system df -v` to see whether any PhantomCache image or volume exists; nothing was started or changed.

## 5. Verification
- Git: HEAD `804a7b8d51d0577f67a04e3bb9fa9d2950ae8f8a`, branch `feat/publish-phantomcache-core`, `git status --short` identical before and after (189 lines, 0 differences; lists in `D:\ClaudeTemp\audit\storage_git_status_before.txt` and `..._after.txt`). `git fsck` printed nothing.
- Present after cleanup: board and monitor-state documents, `Cargo.toml`, `Cargo.lock`, `ui/package-lock.json`, both start batch files, `target\release\phantom-cache-demo.exe`, `phantom_rev10.exe`, evidence folder, academic workspace, M3 sandbox, backups, snapshots, sealed M6 folder.
- Limitations: no rebuild or test run was done (not requested); file contents of the preserved items were not re-hashed; the 14 other pre-existing, unrelated files at the `D:\` root were not examined.

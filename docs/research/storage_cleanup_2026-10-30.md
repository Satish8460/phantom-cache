# Storage cleanup record, 2026-10-30

**Status:** Completed for the authorised scope (Category A partially, by owner decision; Category B fully). Historical record; append dated amendments rather than editing.
**Authority:** owner instructions of 2026-10-30 (Stage 1 audit, conditional approval, acceptance of the cleanup). The progress ledger carries the chronological entries; this file is the consolidated record.
**Evidence tags:** every figure below was measured in the session unless marked otherwise.

## 1. Category B: regenerable build outputs removed
Each target was re-checked immediately before removal against the pre-cleanup inventory (file count and bytes identical), no process referenced it, and removal was verified afterwards (path no longer exists).

| Path | Files | Logical bytes |
|---|---|---|
| `D:\Personal\_m3_target` | 6,539 | 8,025,217,404 |
| `D:\Personal\_m3_target_base` | 3,377 | 3,057,214,080 |
| `D:\Personal\_ttl_target` | 13,193 | 16,531,807,799 |
| `D:\Personal\_m7_target_g2b` | 24,217 | 26,526,108,764 |
| `D:\Personal\_m7_target_before` | 3,414 | 2,740,371,366 |
| `D:\Personal\_m7_target_cpl` | 3,361 | 2,757,485,891 |
| `D:\Personal\_m7_target_scratch` | 4,261 | 4,023,502,033 |
| `D:\CargoTarget\m7_clean` | 2,013 | 622,212,687 |
| `D:\Personal\phantom-cache\target` | 52,087 | 43,519,746,717 |
| `D:\Personal\phantom-cache\Personal_m7_target` | 7 | 2,419 |
| **Total** | **112,469** | **107,803,669,160** (about 100.4 GiB) |

The per-target inventory was captured before removal in the session scratchpad (`pre_cleanup_inventory.json`, outside the repo).

- Pre-scan found no non-build files in these targets other than each `.rustc_info.json` (cargo's own metadata).
- `Personal_m7_target` held 7 files (2,419 bytes of cargo metadata), matching the audit; it was removed as authorised.
- **Recorded binary hashes** in the ledger and reports remain as text. The deleted binaries can no longer be independently re-verified, and a rebuild will not reproduce them bit for bit.

### Disk-space recovery (measured separately from the logical sum)
| Measurement | Value |
|---|---|
| Logical bytes deleted (sum of file sizes) | 107,803,669,160 bytes = 100.4 GiB |
| Free space on D: immediately before deletion | 107.41 GiB |
| Free space on D: immediately after | 197.50 GiB |
| Free space on D: at the later final check | 197.42 GiB |
| **Measured free-space gain** | **about 90.1 GiB** (90.0 GiB at the later check) |

The measured gain is about 10 GiB smaller than the logical sum. The cause has **not** been established. Hardlinks within cargo output directories are one possible explanation (a hypothesis only; it was not tested). Other disk activity between the two measurements (agent runs, the snapshot copies of about 0.15 GB, new reports) and filesystem accounting differences are also possible. The free-space figure is the one to rely on for "space recovered".

## 2. Category A: snapshot consolidation (partial by owner decision)
Source: `D:\Personal\phantom-cache-snapshots` (11 phase snapshot folders). Destination: `D:\Personal\phantom-cache\.local\snapshots\` (untracked; `.gitignore` unchanged).

### 2.1 Completed: seven verified copies
Verified twice (after copying, and again on 2026-10-30 after the owner's decision) by comparing the SHA-256 of every file in the original with the copy and the file counts. Zero mismatches. The originals were preserved.

| Folder | Files | Bytes | Hash mismatches | Manifest SHA-256 (path+file-hash list) |
|---|---|---|---|---|
| `phase1-20261028T185855Z-456bb25` | 312 | 35,517,809 | 0 | 1802E3B0F2A1BF5C6FED4D3D2497BEDA55620C5CDC249FB048F5559BE091FA26 |
| `phase2-20261029T063500Z-456bb25` | 173 | 18,441,102 | 0 | D9EADBEE036B5373CA50658A25848D38F9DFC742CC963F617C96284AC7BB3574 |
| `phase3-exit-20261029T104000Z-456bb25` | 233 | 20,415,183 | 0 | A38F0FB73DE3A10E4DAE7BF7A0CAB19A345A52C7FAE1BD5694818F26BD376CE1 |
| `phase4-entry-20261029T104500Z-456bb25` | 233 | 20,415,183 | 0 | A38F0FB73DE3A10E4DAE7BF7A0CAB19A345A52C7FAE1BD5694818F26BD376CE1 |
| `phase4-exit-20261029T113000Z-456bb25` | 247 | 20,761,572 | 0 | F2F39C5706713A90A063F7A0327D2F3D205B436159A9EC316157A0232FDBF0DE |
| `phase5-entry-20261029T113500Z-456bb25` | 247 | 20,761,572 | 0 | F2F39C5706713A90A063F7A0327D2F3D205B436159A9EC316157A0232FDBF0DE |
| `phase5-exit-20261029T124500Z-456bb25` | 285 | 22,363,528 | 0 | D064402BD48617D494081336A38ED570487CC654B27A7FD84E4E9621ACE315A1 |

Identical manifest hashes for phase3-exit/phase4-entry and phase4-exit/phase5-entry are expected: each entry snapshot is a re-capture of the preceding exit state (observation, consistent with identical file counts and bytes).

### 2.2 Intentionally retained: four protected originals (not copied)
`phase2-exit-20261029T094000Z-456bb25`, `phase3-20261029T094500Z-456bb25`, `phase6-entry-20261029T181500Z-456bb25`, `phase6-exit-20261029T185000Z-456bb25` under `D:\Personal\phantom-cache-snapshots\`.

- They carry an `Everyone: DENY (W,D)` access-control entry (observed via `icacls`, read-only inspection) and cannot be listed or read by the session account; copying failed with "Access denied".
- They appear to be deliberately protected; the owner decided to leave them in place. Ownership was not taken, permissions were not changed, and no other access mechanism was attempted.
- They therefore remain **outside** the project directory. All existing references to their original locations are preserved (no document was edited).
- Their contents are unverified by this cleanup; nothing is known about them beyond their names and the ACL.

### 2.3 Known artefact
`.local\snapshots\phase2-exit-20261029T094000Z-456bb25` exists as an **empty directory** created by the first copy attempt before it failed. It is not a copy and holds no data. It was left in place because the owner authorised no further filesystem changes; it can be removed on request.

## 3. Retained untouched (exclusions)
`D:\Personal\_m7_target` (in use by browser verification), `D:\Personal\_m7_target.zip` (corrupt archive, unidentified), `D:\Temp\cf_old_target`, active TEMP and agent scratch files, unidentified root files (`D:\h1.json`, `h2.json`, `rr.patch`, `wtb_*.js`), `D:\Personal\.phantom-sealed-m6` (never opened), the original snapshots directory, and the `D:\CargoTarget` root.

## 4. Git and integrity state after cleanup
- HEAD `456bb25930004f1896a84c53e776e7b3ef29ee16`, branch `feat/publish-phantomcache-core`, no stashes.
- `git status --short`: 72 lines before and after the cleanup. The 21 tracked modified files match the session-start list (`git diff --stat`: 21 files). New untracked entries from this period: `.local/`, the vision documents, the round-3 browser report and this record; removed untracked entry: `Personal_m7_target/`.
- No tracked source, frozen experiment record or unrelated file was changed by the cleanup.
- No research evidence was among the deleted items. The M3 and TTL patches and logs live in `docs/research/m3_durability/` and `docs/research/ttl_decision/`.

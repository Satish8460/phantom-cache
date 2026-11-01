# PhantomCache Phase 1 — Preservation Snapshot Manifest (S0)

**Snapshot Directory:** `D:\Personal\phantom-cache-snapshots\phase1-20261028T185855Z-456bb25`  
**POSIX Path:** `/d/Personal/phantom-cache-snapshots/phase1-20261028T185855Z-456bb25`  
**Timestamp Created:** `2026-10-28T18:58:55Z`  
**Timestamp Verified:** `2026-10-28T19:08:08Z`  
**Repository Working Baseline:** `456bb25930004f1896a84c53e776e7b3ef29ee16` on `feat/publish-phantomcache-core`  
**Verification Verdict:** **PASS** (V1: PASS, V2: PASS, V3: PASS)

---

## 1. Toolchain and Environment

| Tool | Version / Spec |
|---|---|
| `git` | git version 2.47.1.windows.2 |
| `tar` | tar (GNU tar) 1.35 |
| `sha256sum` | sha256sum (GNU coreutils) 8.32 |
| `python` | Python 3.14.2 |
| `ripgrep` | ripgrep 15.1.0 (+pcre2) |
| OS / Shell | Windows 11 / Git Bash (MSYS2) |
| Snapshot ACL | Restricted: `MSI\Satish:(OI)(CI)(F)`, inheritance removed |

---

## 2. Inventory and Counts

| Category | Count | Notes |
|---|---|---|
| Total working-tree paths (`all_paths.z`) | 138 | Captured from `git ls-files` + `git ls-files -o --exclude-standard` |
| `COPIED` files | 134 | Byte-exact copies in `tree/` |
| `REDACTED` files | 4 | Text files with sensitive patterns redacted in `tree/` |
| `EXCLUDED` files | 0 | Binary/filename class matches excluded |
| Files in snapshot `tree/` | 138 | `COPIED` (134) + `REDACTED` (4) |
| Tracked modified entries | 16 | Matches §2 specification exactly |
| Untracked non-ignored entries | 16 | Matches §2 specification exactly |
| Tracked unmodified entries in tree | 106 | (138 - 16 untracked - 16 modified) |
| Ignored entries inventoried | 7 | `api.log`, `build.log`, `ui/ui.log`, 2 in `ui/dist/`, `target/`, `ui/node_modules/` |
| Relevant ignored copies | 2 | `ui/dist/` copied to `ignored_relevant/ui_dist/` (0 scan hits) |

---

## 3. Cryptographic Manifests and Checksums

| File | SHA-256 Checksum |
|---|---|
| `meta/original_manifest.sha256` | `a87fd80a888ec623878b67b98bc421969cf3edfb31975f3f3adf20f91628e7c3` |
| `meta/tree_copy_manifest.sha256` | `8b67cfcb602a3e998e23e6d9f80975853de0bcce15914c250617440c0b7ac7ef` |
| `meta/snapshot_files.sha256` | `69e888e4d3a4fac2f70bdc6b7b6e7a49a8e22db749edeff8615ff8c247d3b8ba` |
| `tools/verify_snapshot.sh` | `1eba4816ad42046c6d9b1eba252c35dc4deef88831d09a103a4680b81bba3c57` |

---

## 4. Secret Screening and Handling (No Values Recorded)

All candidate files were screened against private key headers, JWT patterns, credential/token/secret assignments, cloud tokens, URL credentials, and filename patterns.

### 4.1 Redacted Files (`tree/`)

1. **`docs/handoff/transcripts/ag_conversation_session_2_oct25_to_oct28.jsonl`**
   - Handling: `REDACTED`
   - Matched classes: `assignment`, `url_credential` (27 lines matched)
   - Reason: Assistant conversation transcript containing simulated/test auth tokens and URL credentials.
2. **`src/main.rs`**
   - Handling: `REDACTED`
   - Matched class: `assignment` (line 286)
   - Reason: Client authorization configuration assignment.
3. **`src/security/jwt.rs`**
   - Handling: `REDACTED`
   - Matched class: `assignment` (lines 125, 144)
   - Reason: JWT secret rotation map assignment.
4. **`tests/integration/security_phase1_tests.rs`**
   - Handling: `REDACTED`
   - Matched class: `assignment` (lines 302, 326, 346, 388, 420, 458, 662)
   - Reason: Test JWT tokens and test secret assignments.

### 4.2 Excluded / Omitted Artifacts

1. **`meta/tracked_changes.patch`**
   - Handling: `OMITTED` (deleted; documented in `meta/tracked_changes_omitted.txt`)
   - Reason: `git diff HEAD --binary` contained 1 hit on line 7371 (`assignment` in `src/main.rs`). All changes remain preserved in `tree/` and in the original working tree.
2. **Ignored Logs (`api.log`, `build.log`, `ui/ui.log`)**
   - Handling: `INVENTORY_ONLY` (not copied per A-14)
   - Reason: Ephemeral/regenerable logs; screened clean (0 hits), inventoried with size, mtime, and sha256 in `meta/ignored_inventory.tsv`.
3. **Build Directories (`target/`, `ui/node_modules/`)**
   - Handling: `INVENTORY_ONLY`
   - Reason: Large reproducible build caches.
4. **`meta/git_config_removed_keys.txt`**
   - Total keys removed: `0` (allow-list filtered; no secret key names found in local config).

---

## 5. Verification Results (V1, V2, V3)

The automated script `$SNAP/tools/verify_snapshot.sh` was executed from a fresh shell:

- **V1 (Copy Integrity):** **PASS**
  - 134 `COPIED` files match `original_manifest.sha256` byte-for-byte.
  - 4 `REDACTED` files exist in `tree/` and their hashes differ from original hashes.
  - 0 `EXCLUDED` files.
  - Total `tree/` files: 138 == `COPIED + REDACTED`.
  - 0 failures.
- **V2 (Restore Simulation):** **PASS**
  - Simulated restore to `$SNAP/verify/restore`.
  - Check A: All 134 restored `COPIED` files match `original_manifest.sha256`.
  - Check B: Unmodified tracked files (36 in this subset) match their git index blob IDs via `git hash-object --path`.
  - Check C: Exactly 16 tracked modified files identified with `differs-from-index = yes`.
  - Check D: Patch check skipped with recorded explanation (secret hit in diff).
  - 0 failures.
- **V3 (Readability and Fresh-Shell Verification):** **PASS**
  - `v3_hashes.txt` generated from fresh shell matches `snapshot_files.sha256` with 0 diffs.
  - 169 non-verify snapshot files read-tested with 0 read failures.
  - Total snapshot size: 35,517,351 bytes.
  - ACL verified: Owner-only full control `MSI\Satish:(OI)(CI)(F)`, inheritance removed.

---

## 6. Exact Restore Recipe

To restore the working state from this snapshot:
1. Clone or check out HEAD: `git checkout 456bb25930004f1896a84c53e776e7b3ef29ee16`
2. Overlay files from snapshot `tree/`:
   ```bash
   tar -cf - -C "D:/Personal/phantom-cache-snapshots/phase1-20261028T185855Z-456bb25/tree" . | tar -xpf - -C "D:/Personal/phantom-cache"
   ```
3. For the 4 `REDACTED` files, restore the unredacted originals from the pristine working directory copy or apply the unredacted working tree copies preserved under owner custody.
4. If `ui/dist/` is required, copy from `ignored_relevant/ui_dist/`:
   ```bash
   cp -r "D:/Personal/phantom-cache-snapshots/phase1-20261028T185855Z-456bb25/ignored_relevant/ui_dist" "D:/Personal/phantom-cache/ui/dist"
   ```
5. Re-run `cargo test --all-targets -j 1 --locked` to confirm 242 passing tests.


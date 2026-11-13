# Evidence kept outside Git (board D-062, owner decision 2026-11-01)

These files exist on the author's machine and are deliberately **not** committed. Their bytes are protected by an archive copy and by the SHA-256 manifests in this folder.

| Manifest | Content | Files | Size |
|---|---|---|---|
| `manifest_g6_browser_ui_evidence.sha256.csv` | screenshots and driver output under `docs/design/remediation`, `docs/demo/browser_verification`, `docs/demo/live_demo_shots` | 2,028 | 241.4 MB |
| `manifest_g7_generated_pdfs.sha256.csv` | generated PDFs under `docs/learning` (regenerable from `docs/learning/src`) | 7 | 36.7 MB |
| `manifest_g8_local_only.sha256.csv` | raw AI-session transcripts (`docs/handoff/transcripts`) and exploratory smoke outputs (`lab-results/smoke_*`); kept local and ignored | 28 | 18.5 MB |

Columns: `path` (repository-relative), `size` (bytes), `sha256` (lower-case hex). Archive copy: `D:\Personal\phantom-cache-backups\evidence_archive_20261101\files\<path>` plus a copy of the manifests; the full recovery checkpoint `D:\Personal\phantom-cache-backups\20261101T223413_pre_commit_plan_D062` also contains every one of these files.

Verify a working tree or the archive: for each row, hash the file with SHA-256 and compare. The matching `.gitignore` rules keep these paths out of `git status`; markdown indexes beside them (for example `docs/design/remediation/00_progress.md`) stay tracked. Re-generating the screenshots needs the drivers in `ui/scripts/browser-*.mjs`; re-generating the PDFs needs the sources in `docs/learning/src`. Regenerated files will not be byte-identical, so keep the archive copy as the evidence of record.

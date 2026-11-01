# Browser verification evidence (D-7)

**Status:** Active (evidence directory index; the contents are retained, unedited records)
**Audience:** reviewers of the Lab UI and anyone citing browser results
**Last reviewed:** 2026-10-30
**Authority and related:** the report in each round folder is the record; this file only describes the directory. Ledger entries: [../../research/progress_ledger.md](../../research/progress_ledger.md) (2026-10-30 10:06 for round 1, 11:16 for round 2, and the later round 3 entry). UI design: [../../design/phantom_lab_ui.md](../../design/phantom_lab_ui.md). Test strategy: [../../testing/strategy.md](../../testing/strategy.md).

## What this is

Retained output of three rounds of automated browser verification of the Lab UI, run on 2026-10-30:

| Round folder | Report | Note |
|---|---|---|
| `20261030T041006Z/` | [report.md](20261030T041006Z/report.md) | round 1; includes `aborted_attempt1/` |
| `20261030T045605Z/` | [report.md](20261030T045605Z/report.md) | round 2, after the first UI fixes; includes `superseded/` (with `matrix_contaminated`) |
| `20261030T055008Z/` | [report.md](20261030T055008Z/report.md) | round 3 re-verification; observation only, nothing was fixed |

Each folder holds `*.results.json` and `.jsonl` driver output, `*.browser.json`, `logs/` (console, network and gateway logs) and `shots/` (screenshots).

## Tier and labels

Not a performance or experiment result. The observations are of the UI in headless Chrome (156.0.8078.79 per the round 1 report) driven over the Chrome DevTools Protocol, against a debug-build gateway on one Windows 11 machine, loopback. Accessibility observations are automated checks, not an audit; no assistive technology was used. Values shown in the screenshots keep the UI's own provenance labels (see [../../glossary.md](../../glossary.md) section 5). Do not generalize to other browsers, release builds or other hosts.

## How it was produced

By the dependency-free scripts in [../../../ui/scripts/](../../../ui/scripts/) (named in the round 1 report: `browser-verify`, `cdp-lib`, `browser-journey`, `browser-a11y`, `browser-recheck`), driven by a verification agent. Each report states that the lead wrote it from the agent's returned report. Gateway binary hashes per round are in the ledger entries. Folders marked `superseded`, `aborted_attempt1` or `matrix_contaminated` are runs that were not used for the conclusions; they are kept as evidence.

## Rules

- Do not edit, rename or delete anything here. A correction is a new dated note elsewhere that links to the file.
- Hash and manifest pointers: no checksum manifest exists for this directory (unverified whether one is planned). Binary hashes are in the ledger.
- Retention: about 121 MB measured with `du` on 2026-10-30, mostly screenshots (round 1 alone reports 275 screenshots, about 36.7 MB, in the ledger). The logs were not scanned for secrets for this README. Run the secret scan required before any commit (delivery step G5, see [../../research/phantomcache_milestone_accountability_report.md](../../research/phantomcache_milestone_accountability_report.md) section 5) before committing this directory. Whether to commit all screenshots is an owner decision.

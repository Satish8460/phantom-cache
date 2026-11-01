# Package 07: multi-host review note

**Status:** Draft for owner review. **Audience:** the owner. **Last reviewed:** 2026-10-30 18:17 IST (machine clock, from `date`). **Index and rules:** `00_README.md`.

## Pointer
The plan for running and reviewing PhantomCache across more than one physical host is being written by another agent in `docs/research/multi_host/`. At the time this note was written that folder did not exist yet. This note contains no plan and must not be read as one. Follow the plan there when it exists: `[FILL: path of the final multi-host plan file(s) once written]`.

## Why it is mentioned here
- Every measurement and cluster test the project has is loopback or single-machine (Windows 11 development laptop). The handbook says plainly: no multi-host or network run exists, and loopback, simulated, WSL2 and physical-host results must be kept separate and labelled (project rule 4).
- The external security review (package 05) and the browser matrix (package 04) both run on single machines. Neither establishes anything about multiple hosts, a real network, clock skew between hosts, or partial failure between hosts.
- A review of cluster behaviour on real hosts will need its own reviewers, hosts, date and sign-off. Record them in the log in `00_README.md` section 6 by adding rows; do not reuse the single-machine reviewers' entries.

## What to do when the plan exists
1. Check that the plan names its hosts, network, workload, seeds, baselines and repetitions (project rule 4) before approving any run.
2. Add its human-review steps (if it has any) to this folder as a new numbered package, not inside the plan.
3. Until a multi-host human review is recorded, anything about multi-host behaviour stays **UNVERIFIED** or **TESTED (loopback)** as the case is.

## Blank fields for the owner
Plan location `[FILL]`; owner approval of the plan `[FILL: date]`; reviewers for multi-host work `[FILL]`; hosts `[FILL]`; date `[FILL]`.

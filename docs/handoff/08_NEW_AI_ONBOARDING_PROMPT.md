> Notice (2026-10-30): This handoff file is a historical record of the state on 2026-10-28 and is not edited. Parts of it are superseded: "Phase 4 accepted / zero P0 / formally accepted" and similar statements are not supported by the later audit ([handoff CORRECTIONS](CORRECTIONS.md), `CLAUDE.md` sections 6 and 6b); the test count "242 tests / 19 targets" is a 2026-10-28 baseline, not the current one ([docs/STATUS.md](../STATUS.md)); and descriptions of cluster cache sharing may predate the mTLS cluster path, where the local node shares the serving cache (`src/main.rs:332`). The "Do NOT begin Phase 5" instruction and the "Current Milestone Status" line are obsolete; work continued beyond Phase 4 afterwards (see the status table).
>
> Original text below is unchanged.

# Copy-Ready Onboarding Prompt for Incoming AI Assistant

**Document ID:** `docs/handoff/08_NEW_AI_ONBOARDING_PROMPT.md`  
**Usage:** The user can paste the block below into the initial prompt of the next AI assistant session.

---

```markdown
# PHANTOMCACHE — INCOMING AI ONBOARDING DIRECTIVE

You are taking over as the lead systems and concurrency engineer for **PhantomCache**, an enterprise-grade in-memory reverse-proxy caching and request coalescing engine written in Rust.

**Repository:** `D:\Personal\phantom-cache`  
**Active Working Branch:** `feat/publish-phantomcache-core`  
**Expected Git HEAD:** `456bb25930004f1896a84c53e776e7b3ef29ee16`  
**Current Milestone Status:** Phase 4 Remediated, Verified, and Formally Accepted (242/242 tests passing across 19 targets; Clippy clean).

---

## 1. MANDATORY SAFETY RULES (DO NOT VIOLATE)

1. **NEVER execute destructive Git operations:** Do NOT run `git reset`, `git clean`, `git checkout --`, `git restore`, `git stash`, or discard modifications. All modified tracked files and untracked directories represent intentional, accepted project work.
2. **NEVER commit or push without explicit user authorization:** Do NOT run `git commit` or `git push`. The user authorizes commits separately.
3. **Inspect the actual repository state first:** Always run:
   ```powershell
   git rev-parse HEAD
   git branch --show-current
   git status --short
   ```
4. **Append `-j 1` to all Cargo commands:** This Windows environment requires single-threaded test scheduling (`cargo test --all-targets -j 1`).
5. **Do NOT restart completed phases:** P1–P4 and distributed Stages 3A–4C are completed and accepted. Do not re-architect or refactor existing code.
6. **Do NOT begin Phase 5 implementation:** Await explicit user authorization and scope definition before starting Phase 5.
7. **Trust code and reproducible output over unsupported claims:** Do not claim global linearizability, hard staleness bounds, or strict memory ceilings without evidence.

---

## 2. KNOWLEDGE BASE NAVIGATION

Before writing any code or answering architectural questions, read the handoff package in `docs/handoff/`:
1. `docs/handoff/00_START_HERE.md` — Operating rules and quick start guide.
2. `docs/handoff/01_PROJECT_MASTER_CONTEXT.md` — Deep architecture, data flows, and subsystem specifications.
3. `docs/handoff/02_PHASE_HISTORY_AND_STATUS.md` — Chronological history, audit findings, and reconciliations.
4. `docs/handoff/03_ARCHITECTURE_AND_PROTOCOL_REFERENCE.md` — Wire framing, generational fencing, and anti-entropy.
5. `docs/handoff/04_TESTS_BENCHMARKS_AND_EVIDENCE.md` — Authoritative test inventory (242 tests) and benchmark evidence.
6. `docs/handoff/05_OPEN_RISKS_AND_NEXT_STEPS.md` — Prioritized risk register and proposed roadmap.
7. `docs/handoff/06_AGENT_WORKFLOW_AND_GIT_SAFEGUARDS.md` — Engineering workflow, audit standards, and Git rules.
8. `docs/handoff/07_CONVERSATION_AND_DECISION_INDEX.md` — Chronological timeline and transcript references.
9. `docs/handoff/MANIFEST.md` — Master inventory of all handoff documents.

Raw transcripts of previous AI sessions are preserved in `docs/handoff/transcripts/`.

---

## 3. YOUR IMMEDIATE FIRST ACTION

1. Verify repository status and run the baseline test suite:
   ```powershell
   git rev-parse HEAD; git branch --show-current; git status --short
   cargo test --all-targets -j 1
   cargo clippy --all-targets -j 1 -- -D warnings
   ```
2. Confirm that 242 tests pass and Clippy produces 0 warnings.
3. Confirm to the user that you have read `docs/handoff/00_START_HERE.md` and are ready to receive their next directive.
```

